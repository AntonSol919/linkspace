use linkspace_common::prelude::{U16, pkt_fmt, U32 };

// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
Status queries allow us to communicate if a process exists that is handling a specific type and a specific instance.

Note that this is only for local status updates.
The group argument does not ask inside GROUP, it only signals which group the query is about.
Other processes are meant to answer a request.

The following are common status types.
A group exchange process will reply to these requests:
The current expected formats are:

exchange GROUP process
exchange GROUP connection PUBKEY
exchnage GROUP pull PULL PULL_HASH

A request is a packet in the form DOMAIN:{#:0}:/\fstatus/GROUP/type[/instance?] , with no data and no links.
A reply is of the form DOMAIN:{#:0}/\status/GROUP/type/instance with some data and links.

A request without 'instance' is be answered by all instances.

The reply must have an 'instance' set. It defaults to 'default'.
The reply data should be either "OK\n" or "ERR\n" followed by more info.
The reply process links can start with init:{#:0} at first and should point to previous replies after that.

A request is only made 'once' per timeout.
I.e. a process checks if a request was made since now-timeout, before making a new request, and returns after last_req+timeout.
**/
use crate::{*, runtime::lk_get_all};
pub const STATUS_PATH: IPathC<16> = ipath1(concat_bytes!([255], b"status"));

#[derive(Copy, Clone)]
#[repr(C)]
pub struct LkStatus<'o> {
    pub domain: Domain,
    pub group: GroupID,
    pub objtype: &'o [u8],
    pub instance: Option<&'o [u8]>,
}
#[doc(hidden)]
pub fn lk_status_path(status: LkStatus) -> LkResult<IPathBuf> {
    let mut path = STATUS_PATH.into_ipathbuf();
    path.try_append_component(&*status.group)?;
    path.try_append_component(status.objtype)?;
    if let Some(v) = status.instance {
        path.try_append_component(v)?;
    }
    Ok(path)
}

/// A query that returns both requests and updates
pub fn lk_status_request(status:LkStatus) -> LkResult<NetPktBox>{
    lk_linkpoint(status.domain, LOCAL_ONLY_GROUP, &lk_status_path(status)?, &[],&[], None)
}

/// A query that returns both requests and updates
pub fn lk_status_overwatch(status:LkStatus,max_age:Stamp) -> LkResult<Query> {
    let LkStatus { domain,  ..} = status;
    let path = lk_status_path(status)?;
    let mut q = lk_query();
    let create_after = now().saturating_sub(max_age);
    lk_query_push(&mut q, "group", "=", &*LOCAL_ONLY_GROUP)?;
    lk_query_push(&mut q, "domain", "=", &*domain)?;
    lk_query_push(&mut q, "create", ">", &*create_after)?;
    lk_query_push(&mut q, "prefix", "=", path.spath_bytes())?;
    Ok(q)
}


/// It is up to the caller to ensure 'lk_process' is called accodingly. Impl [PktHandler::stopped] to capture the timeout event.
pub fn lk_status_poll(lk:&Linkspace,status:LkStatus, timeout:Stamp, mut cb: impl PktHandler + 'static) -> LkResult<()>{
    let mut ok = false;
    let mut last_request = Stamp::ZERO;
    let mut query : Query= lk_status_overwatch(status, timeout)?;

    lk_get_all(lk, &query, &mut |pkt| {
        if pkt.get_links().is_empty() && pkt.data().is_empty() {
            last_request = *pkt.get_create_stamp();
            tracing::debug!(pkt=%pkt_fmt(&pkt),"recently requested");
            true
        }else {
            ok =true;
            let cnt = (cb).handle_pkt(pkt,lk);
            cnt.is_continue()
        }
    })?;
    if last_request == Stamp::ZERO{
        tracing::debug!("creating new req");
        let req = lk_status_request(status).unwrap();
        last_request = *req.get_create_stamp();
        lk_save(lk,&req)?;
    }
    let wait_untill = last_request.saturating_add(timeout);
    tracing::debug!(?wait_untill,"Waiting untill");
    lk_query_push(&mut query, "data_size", ">", &*U16::ZERO)?;
    lk_query_push(&mut query, "links_len", ">", &*U16::ZERO)?;
    lk_query_push(&mut query, "recv", "<", &*wait_untill)?;
    let w = [b"status" as &[u8],&*now()].concat();
    lk_query_push(&mut query, "", "watch", &w)?;
    lk_watch(lk, &query, cb)?;
    Ok(())
}

fn is_status_reply(status:LkStatus,path:&IPath,pkt:&NetPktPtr) -> LkResult<()>{
    anyhow::ensure!(*pkt.get_domain() == status.domain
                    && *pkt.get_group() == LOCAL_ONLY_GROUP
                    && pkt.get_ipath() == path
                    && !pkt.get_links().is_empty()
                    && !pkt.data().is_empty()
                    ,"invalid status update");
    Ok(())
}

pub fn lk_status_set(lk:&Linkspace,status:LkStatus,mut update:impl FnMut(&Linkspace,Domain,GroupID,&IPath,Link) -> LkResult<NetPktBox> +'static)-> LkResult<()>{
    let LkStatus { domain, group, objtype, instance }= status;
    let objtype = objtype.to_vec();
    let instance = instance.or(Some(b"default")).map(Vec::from);
    let status = LkStatus { instance: instance.as_deref(), domain , group, objtype:&objtype};
    let path = lk_status_path(status)?;
    let link = Link{tag:ab(b"init"),ptr:LOCAL_ONLY_GROUP};
    let initpkt = update(lk,status.domain, LOCAL_ONLY_GROUP, &path,link)?;
    is_status_reply(status, &path, &initpkt)?;
    let mut prev = initpkt.hash();
    lk_save(&lk,&initpkt )?;
    std::mem::drop(initpkt);

    let mut q = lk_query();
    let prefix = lk_status_path(LkStatus { instance:None, ..status})?;
    lk_query_push(&mut q, "data_size", "=", &*U16::ZERO)?;
    lk_query_push(&mut q, "links_len", "=", &*U16::ZERO)?;
    lk_query_push(&mut q, "prefix", "=", prefix.spath_bytes())?;
    // We only care about new packets. Worst case a request was received and timeout between our init and this cb.
    lk_query_push(&mut q, "i_index", "<", &*U32::ZERO)?;
    lk_query_push(&mut q, "", "watch", &[b"status-update" as &[u8],&*now()].concat())?;
    lk_watch(&lk, &q, cb(move |pkt:&dyn NetPkt, lk:&Linkspace| -> LkResult<()>{
        let status = LkStatus { instance: instance.as_deref(), domain , group, objtype:&objtype};
        let p = pkt.get_ipath();
        if p.len() == path.len() && p.spath() != path.as_ref() { return Ok(())}
        let link = Link{tag:ab(b"prev"),ptr:prev};
        let reply = update(lk,status.domain,LOCAL_ONLY_GROUP,&path,link)?;
        is_status_reply(status, &path, &reply)?;
        prev  = reply.hash();
        lk_save(lk,&reply)?;
        Ok(())
    }))?;
    lk_process_while(&lk, Stamp::MAX, Stamp::MAX)?;
    Ok(())
}


