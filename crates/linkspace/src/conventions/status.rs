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

A request is a packet in the form DOMAIN:[#:0]:/\fstatus/GROUP/type(/instance?) , with no data and no links.
A reply is of the form DOMAIN:[#:0]/\status/GROUP/type/instance with some data and at least some links.

A request without 'instance' should be answered by all instances.

The reply must have an 'instance' set. It defaults to 'default'.
The reply data should be either "OK\n" or "ERR\n" followed by more info.
The reply process links can start with init:[#:0] at first and should point to previous replies after that.

A new request is not made if one was made after now-timeout.
I.e. a process checks if a request was made since now-timeout, before making a new request, and returns after last_req+timeout.
A reply is accepted if it was made now-timeout.

This might change
**/
use crate::{*};
/// const '/status' spacename
pub static STATUS_SPACE: RootedStaticSpace<16> = rspace1::<7>(concat_bytes!([255], b"status"));

#[derive(Copy, Clone)]
#[repr(C)]
/// Options for a local status request.
pub struct LkStatus<'o> {
    /// the domain of interests
    pub domain: Domain,
    /// the group of interests (NOT the group to query, see mod level doc on the usage)
    pub group: GroupID,
    /// the object type of interests
    pub objtype: &'o [u8],
    /// the instance
    pub instance: Option<&'o [u8]>,
    /// qid to use for query - reuse of qid will remove the listening query
    pub qid: &'o [u8]
}
impl Default for LkStatus<'static> {
    fn default() -> Self {
        Self { domain: domain(), group: group(), objtype: &[], instance: None, qid: b"status" }
    }
}
impl<'o> std::fmt::Debug for LkStatus<'o>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LkStatus")
            .field("domain", &self.domain)
            .field("group", &self.group)
            .field("objtype", &AB(&self.objtype))
            .field("instance",&self.instance.map(AB)).finish()
    }
}
#[doc(hidden)]
pub fn lk_status_space(status: LkStatus) -> LkResult<RootedSpaceBuf> {
    let mut space = STATUS_SPACE.into_buf();
    space.try_append_component(&*status.group)?;
    space.try_append_component(status.objtype)?;
    if let Some(v) = status.instance {
        space.try_append_component(v)?;
    }
    Ok(space)
}

/// A query that returns both requests and updates
pub fn lk_status_request(status:LkStatus) -> LkResult<NetPktBox>{
    lk_linkpoint(&[],status.domain, PRIVATE, &lk_status_space(status)?,&[], None)
}

/// A query that returns both requests and updates
pub fn lk_status_overwatch(status:LkStatus,max_age:Stamp) -> LkResult<Query> {
    let LkStatus { domain,  ..} = status;
    let space = lk_status_space(status)?;
    let mut q = lk_query(&Q);
    let create_after = now().saturating_sub(max_age);
    q = lk_query_push(q, "group", "=", &*PRIVATE)?;
    q = lk_query_push(q, "domain", "=", &*domain)?;
    q = lk_query_push(q, "create", ">", &*create_after)?;
    q = lk_query_push(q, "prefix", "=", space.space_bytes())?;
    Ok(q)
}

#[cfg(feature="runtime")]
/// watch for any points matching 
pub fn lk_status_watch(lk:&Linkspace,status:LkStatus, d_timeout:Stamp,
                      mut cb: impl crate::runtime::cb::PktHandler + 'static) -> LkResult<bool>{
    use crate::runtime::{lk_watch2,lk_get_all};
    use linkspace_common::prelude::{U16, U32};
    use tracing::debug_span;

    let span = debug_span!("status_poll",?status,?d_timeout);
    let _ = span.enter();
    let mut ok = false;
    let mut last_request = Stamp::ZERO;
    let mut query : Query= lk_status_overwatch(status, d_timeout)?;
    // We want to capture any old request, so we first lk_get_all both requests and replies.
    lk_get_all(lk, &query, &mut |pkt| {
        if pkt.get_links().is_empty() && pkt.data().is_empty() {
            last_request = *pkt.get_create_stamp();
            tracing::debug!(pkt=%PktFmt(&pkt),"recently requested");
            false
        }else {
            ok =true;
            let cnt = (cb).handle_pkt(pkt,lk);
            tracing::debug!("recently replied");
            cnt.is_break()
        }
    })?;
    if last_request == Stamp::ZERO{
        tracing::debug!("creating new req");
        let req = lk_status_request(status).unwrap();
        last_request = *req.get_create_stamp();
        lk_save(lk,&req)?;
    }
    let wait_until = last_request.saturating_add(d_timeout);
    tracing::debug!(?wait_until,"Waiting until");
    query = lk_query_push(query, "data_size", ">", &*U16::ZERO)?;
    query = lk_query_push(query, "links_len", ">", &*U16::ZERO)?;
    query = lk_query_push(query, "recv", "<", &*wait_until)?;
    query = lk_query_push(query, "i_db", "<", &*U32::ZERO)?;
    query = lk_query_push(query, "", "qid", status.qid)?;
    lk_watch2(lk, &query, cb,span)?;
    Ok(ok)
}

fn is_status_reply(status:LkStatus,rspace:&RootedSpace,pkt:&NetPktPtr) -> LkResult<()>{
    anyhow::ensure!(*pkt.get_domain() == status.domain
                    && *pkt.get_group() == PRIVATE
                    && pkt.get_rooted_spacename() == rspace
                    && !pkt.get_links().is_empty()
                    && !pkt.data().is_empty()
                    ,"invalid status update");
    Ok(())
}

/// Insert a callback that is triggered on a request. Must yield a valid response. Don't forget to process
#[cfg(feature="runtime")]
pub fn lk_status_set(lk:&Linkspace,status:LkStatus,mut update:impl FnMut(&Linkspace,Domain,GroupID,&RootedSpace,Link) -> LkResult<NetPktBox> +'static)-> LkResult<()>{
    use tracing::debug_span;
    use linkspace_common::prelude::{U16, U32};
    use crate::runtime::{lk_watch2};

    let span = debug_span!("status_set",?status);
    let _ = span.enter();
    let LkStatus { domain, group, objtype, instance,qid }= status;

    let objtype = objtype.to_vec();
    let instance = instance.or(Some(b"default")).map(Vec::from);
    let status = LkStatus { instance: instance.as_deref(), domain , group, objtype:&objtype, qid};
    let space = lk_status_space(status)?;
    let link = Link{tag:ab(b"init"),ptr:PRIVATE};
    let initpkt = update(lk,status.domain, PRIVATE, &space,link)?;
    is_status_reply(status, &space, &initpkt)?;
    let mut prev = initpkt.hash();
    tracing::debug!(?initpkt,"init status");
    lk_save(lk,&initpkt )?;
    std::mem::drop(initpkt);

    let mut q = lk_query(&Q);
    let prefix = lk_status_space(LkStatus { instance:None, ..status})?;
    q = lk_query_push(q, "data_size", "=", &*U16::ZERO)?;
    q = lk_query_push(q, "links_len", "=", &*U16::ZERO)?;
    q = lk_query_push(q, "prefix", "=", prefix.space_bytes())?;
    // We only care about new packets. Worst case a request was received and timeout between our init and this cb.
    q = lk_query_push(q, "i_db", "<", &*U32::ZERO)?;
    q = lk_query_push(q, "", "qid", qid)?;
    lk_watch2(lk, &q, try_cb(move |pkt:&dyn NetPkt, lk:&Linkspace| -> LkResult<()>{
        let status = LkStatus { instance: instance.as_deref(), domain , group, objtype:&objtype,qid:&[]};
        let p = pkt.get_rooted_spacename();
        if p.depth() == space.depth() && p.space() != space.as_ref() { return Ok(())}
        let link = Link{tag:ab(b"prev"),ptr:prev};
        let reply = update(lk,status.domain,PRIVATE,&space,link)?;
        is_status_reply(status, &space, &reply)?;
        prev  = reply.hash();
        tracing::debug!(?reply,"Reply status");
        lk_save(lk,&reply)?;
        Ok(())
    }),span)?;
    Ok(())
}
