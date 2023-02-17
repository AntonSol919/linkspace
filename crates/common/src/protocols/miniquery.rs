// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
A request-reply optimized for pulling specific hashes or the latest on a given path without parsing any request header.
Can essential emulate http like request and replies.
Mostl usefull for publicly facing services.

Servers should operate on port 2023 serving UDP and TCP requests

- a linkpacket containing only data is interpeted as a list of hashes to reply with
- a linkpoint with a signle link is interpreted as the DOMAIN:PUBKEY to look in the same path/group the linkpoint is in.
- A invalid request ( the empty packet ) packet should return a errorpoint

TODO: add keypoint costs for proof of work

**/
use linkspace_core::prelude::*;

pub const MINIQ_PORT: u16 = 2023;
pub const MINIQ: Domain = bytefmt::abx(b"miniq-protocol");

pub fn packedq_pull_path(domain: Domain, path: &IPath) -> NetPktBox {
    packedq_pull_path2(PUBLIC_GROUP, domain, None, path)
}
pub fn packedq_pull_path2(
    group: GroupID,
    domain: Domain,
    key: Option<PubKey>,
    path: &IPath,
) -> NetPktBox {
    let now = now();
    let links = [Link {
        tag: domain,
        ptr: key.unwrap_or([0; 32].into()),
    }];
    linkpoint(
        group,
        MINIQ,
        path,
        &links,
        &[],
        now,
        NetOpts::ValidUntil(now),
    )
    .as_netbox()
}

pub fn packedq_pull(hashes: &[Ptr]) -> NetPktParts {
    let now = now();
    let data: &[u8] = unsafe {
        std::slice::from_raw_parts(
            hashes.as_ptr() as *const u8,
            hashes.len() * std::mem::size_of::<LkHash>(),
        )
    };
    linkpoint(
        PUBLIC_GROUP,
        MINIQ,
        IPath::empty(),
        &[],
        data,
        now,
        NetOpts::ValidUntil(now),
    )
}

//pub fn packedq_pull_reply<B:Try>(lk:&ReadTxn,pkt: impl NetPkt, mut cb: impl FnMut(RecvPktPtr) -> B) -> B{

/*
pub fn get_gdpk(addr:IpAddr,group:GroupID,domain:Domain,path:&IPath,key:Option<PubKey>) -> std::io::Result<Option<()>>{
    todo!()
}
*/
