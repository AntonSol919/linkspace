// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
Status queries allow us to communicate if a process exists that is handling a specific type and a specific instance.

Note that this is specifically only local status updates.
The group argument does not ask inside GROUP, it only signals which group the query is about.
Other processes on your computer are meant to answer a request.

The following are common status types.
A group exchange process will reply to these requests:
The current expected formats are:

exchange GROUP network
exchange GROUP connection PUBKEY
exchnage GROUP pull PULL PULL_HASH

They should reply with at least the data with the first line either "ok\n"  or "err\n"
The rest (Links and data) can give more information

**/
use crate::*;
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
pub fn lk_status_path(status: &LkStatus) -> LkResult<IPathBuf> {
    let mut path = STATUS_PATH.into_ipathbuf();
    path.try_append_component(&*status.group)?;
    path.try_append_component(status.objtype)?;
    if let Some(v) = status.instance {
        path.try_append_component(v)?;
    }
    Ok(path)
}

/*
/** setup a status reply (requested with [lk_status_poll]) - !!Requires [lk_process] or [lk_process_while] to function

Returns the ID you can use with [lk_stop].
Will automatically drop after duration
 **/
pub fn lk_status_setup(lk:&Linkspace,status:&LkStatus,duration:Stamp,value:Vec<u8>)-> LkResult<Vec<u8>>{
    let domain = status.domain;
    let path = status.path(false)?;
    let mut q = lk_query();
    lk_query_push(&mut q, "group",     "=" , &*LOCAL_ONLY_GROUP)?;
    lk_query_push(&mut q, "domain",    "=" , &*domain)?;
    lk_query_push(&mut q, "recv",      "<=", &*now().saturating_add(duration))?;
    lk_query_push(&mut q, "create",    ">" , &*now() )?;
    lk_query_push(&mut q, "data_size", "=" , &*U16::ZERO)?;
    lk_query_push(&mut q, "links_len", "=" , &*U16::ZERO)?;
    lk_query_push(&mut q, "recv", ">", &*now())?;

    match status.instance{
        Some(_) => todo!(),
        None => {
            std::mem::drop(status);
            let init = lk_linkpoint_ref(domain, LOCAL_ONLY_GROUP, &path, &[], &value, None)?;
            lk_save(lk, &init)?;
            let init_hash = init.hash();
            std::mem::drop(init);
            lk_query_push(&mut q, "path", "=", path.spath_bytes())?;
            let id = path.spath_bytes().to_vec();
            lk_query_push(&mut q, "","id",&id)?;
            lk_view(lk, &q, misc::TryCb(move |pkt:&dyn NetPkt,lk:&Linkspace|  -> LkResult<()>{
                if pkt.get_links().is_empty(){
                    let links = [Link{tag:ab(b"init"),ptr:init_hash},Link{tag:ab(b"req"),ptr:pkt.hash()}];
                    let lp = lk_linkpoint_ref(domain, LOCAL_ONLY_GROUP, &path, &links,&value, None).unwrap();
                    lk_save(lk,&lp)?;
                }
                Ok(())
            }))?;
            return Ok(id)
        },
    }
}


pub fn lk_status_request(status:&LkStatus) -> LkResult<NetPktBox>{
    lk_linkpoint(status.domain, LOCAL_ONLY_GROUP, &status.path(true)?, &[],&[], None)
}

/// A query that checks for the status events for a given domain,group,objtype,instance? no older than max_duration
/// A request is any packet without data or links.
/// reply_only adds the 'data_size:>:0 & links_len:>:0' predicate, effectivly ignoreing requests
pub fn lk_status_query(status:&LkStatus,max_age:Stamp) -> LkResult<Query> {
    let LkStatus { domain, instance , ..} = status;
    let path = status.path(false)?;
    let mut q = lk_query();
    let create_after = now().saturating_sub(max_age);
    lk_query_push(&mut q, "group", "=", &*LOCAL_ONLY_GROUP)?;
    lk_query_push(&mut q, "domain", "=", &**domain)?;
    lk_query_push(&mut q, "data_size", ">", &*U16::ZERO)?;
    lk_query_push(&mut q, "links_len", ">", &*U16::ZERO)?;
    lk_query_push(&mut q, "create", ">", &*create_after)?;
    match instance{
        Some(_) => todo!(),
        None => {
            lk_query_push(&mut q, "path", "=", path.spath_bytes())?;
        }
    }
    Ok(q)
}
*/
