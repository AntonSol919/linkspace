// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// The lns:[#:0] lookup entries.

use crate::{prelude::*, protocols::lns::{GROUP_TAG, LNS, BY_TAG_P, PUBKEY_TAG}};
use linkspace_core::prelude::query_mode::Order;
use tracing::instrument;

use super::claim::Claim;


// only call this with a valid claim
#[instrument(ret,skip(lk),level="debug")]
pub(crate) fn save_private_claim(lk: &Linkspace,new_claim:&Claim,admin:Option<&SigningKey>,and: &[&dyn NetPkt],priority:bool) -> anyhow::Result<bool>{
    let wr = lk.get_writer();
    let read = lk.get_reader();
    let admin_k = admin.map(|v| v.pubkey());
    let now = now();
    // This claim is being overwritten. This means its old by-tag ptrs must be removed as well. 
    let old_claim = super::lookup_claim(lk, &new_claim.name)?;
    let old_claim = old_claim.as_ref();
    let old_chash = old_claim.map(|o|o.pkt.hash());
    if let Some(c) = old_claim{ if c.pkt.hash() == new_claim.pkt.hash(){ return Ok(false)} }

    let old_claim_grp = old_claim.and_then(|v| v.group()).cloned();

    let drop_old_group = old_claim_grp
        .filter(|_| old_claim_grp != new_claim.group().cloned())// we can skip this step if they new_ptr pkt will overwrite anyways
        .and_then(|grp| ptr_lookup_entry(&read, GROUP_TAG,grp , admin_k).into_opt())
        .transpose()?
        .map(|old| mut_ptrlookup_entry(old.get_links(), old.get_ipath(), old_chash, None, admin, now));

    tracing::debug!(?drop_old_group);
    let mut add_new_group = None;
    if let Some(grp) = new_claim.group(){
        let p = ptr_lookup_entry(&read, GROUP_TAG, *grp, admin_k).into_ok()?;
        let links = p.as_ref().map(|v|v.get_links()).unwrap_or(&[]);
        let path = BY_TAG_P.into_spathbuf().push(GROUP_TAG).push(grp).ipath();
        let new_link = Link{tag: create_rtag(new_claim.until()),ptr:new_claim.pkt.hash()};
        add_new_group = Some(mut_ptrlookup_entry(links, &path, old_chash, Some((priority,new_link)), admin, now));
    }
    tracing::debug!(?add_new_group);


    // this should take into account what the public key prefers
    let old_claim_pubkey = old_claim.and_then(|v| v.pubkey()).cloned();
    let drop_old_pubkey = old_claim_pubkey
        .filter(|_| old_claim_pubkey != new_claim.pubkey().cloned())// we can skip this step if they new_ptr pkt will overwrite anyways
        .and_then(|pubkey| ptr_lookup_entry(&read, PUBKEY_TAG,pubkey , admin_k).into_opt())
        .transpose()?
        .map(|old| mut_ptrlookup_entry(old.get_links(), old.get_ipath(),  old_chash, None, admin, now));

    tracing::debug!(?drop_old_pubkey);

    let mut add_new_pubkey = None;
    if let Some(pubkey) = new_claim.pubkey(){
        let p = ptr_lookup_entry(&read, PUBKEY_TAG, *pubkey, admin_k).into_ok()?;
        let links = p.as_ref().map(|v|v.get_links()).unwrap_or(&[]);
        let path = BY_TAG_P.into_spathbuf().push(PUBKEY_TAG).push(pubkey).ipath();
        let new_link = Link{tag: create_rtag(new_claim.until()),ptr:new_claim.pkt.hash()};
        add_new_pubkey = Some(mut_ptrlookup_entry(links, &path, old_chash, Some((priority,new_link)), admin, now));
    }
    tracing::debug!(?add_new_pubkey);

    fn as_p(p:&Option<NetPktBox>) -> Option<&dyn NetPkt> {
        match &p {
            Some(o) => Some(o as &dyn NetPkt),
            None => None,
        }
    }

    let txn : Vec<&dyn NetPkt>= [&new_claim.pkt as &dyn NetPkt].into_iter()
        .chain(as_p(&drop_old_group))
        .chain(as_p(&add_new_group))
        .chain(as_p(&drop_old_pubkey))
        .chain(as_p(&add_new_pubkey))
        .chain(and.iter().copied())
        .collect();
    save_pkts(wr, &txn)?;
    Ok(true)
}

#[instrument(ret,skip(reader),level="debug")]
pub fn ptr_lookup(reader: &ReadTxn, tag: Tag, ptr: Ptr,admin:Option<PubKey>) -> ApplyResult<Claim> {
    let ple = ptr_lookup_entry(reader, tag, ptr, admin);
    read_claims(reader, ple.into_ok()??.pkt, now()).find_map(|(_,p)| p.ok()?).into()
}
#[instrument(ret,skip(reader),level="debug")]
pub fn ptr_lookup_entry(reader: &ReadTxn, tag: Tag, ptr: Ptr,admin:Option<PubKey>) -> ApplyResult<RecvPktPtr> {
    let path = BY_TAG_P.into_spathbuf().push(tag).push(ptr);
    let mut preds = PktPredicates::from_gd(PRIVATE, LNS).path(path)?.create_before(now()).unwrap();
    // entries have the form /by-tag/TAG/PTR
    if let Some(v) = admin {
        preds.pubkey.add(TestOp::Equal, v.into())
    }
    tracing::debug!(%preds,"by-tag");
    reader.query_tree(query_mode::Order::Desc, &preds).next().into()
}


pub type TaggedClaim = ((Stamp,[u8;8]),anyhow::Result<Option<Claim>>);
fn read_claims<'o>(reader: &'o ReadTxn,pkt: &'o impl NetPkt,valid_at:Stamp) -> impl Iterator<Item=TaggedClaim> +'o{
    pkt.get_links().iter()
        .map(|v| (rtag(v.tag),v.ptr))
        .filter(move |((until,_),_)| *until > valid_at)
        .map(|(rt,p)| (rt,Claim::read(reader,&p)))
}

pub fn list_ptr_lookups<'o>(reader: &'o ReadTxn, tag: AB<[u8; 16]>,ptr:Option<Ptr>,admin:Option<PubKey>) -> impl Iterator<Item=Vec<TaggedClaim>> +'o {
    let path = BY_TAG_P.into_spathbuf().push(tag);
    let path = if let Some(p) = ptr { path.push(&*p)} else { path}; 
    let mut preds = PktPredicates::from_gd(PRIVATE, LNS).create_before(now()).unwrap();
    preds.prefix(path).unwrap();
    preds.path_len.add(TestOp::Equal,3);
    preds.state.i_branch.add(TestOp::Equal,0);
    if let Some(v) = admin {
        preds.pubkey.add(TestOp::Equal, v.into())
    }
    tracing::debug!(%preds,"by-tag");
    let it = reader.query_tree(Order::Desc, &preds).peekable();
    let now = now();
    it.map(move |tagpkt| read_claims(reader,&tagpkt.pkt,now).collect())
}


#[instrument(ret,level="trace")]
pub fn mut_ptrlookup_entry(links:&[Link],path:&IPath,remove:Option<Ptr>,add_link_first:Option<(bool,Link)>,admin:Option<&SigningKey>,now:Stamp) -> NetPktBox{
    let (pre,post) = match add_link_first {
        Some((true,link)) => (Some(link),None),
        Some((false,link)) => (None,Some(link)),
        None => (None,None),
    };
    let lks =  links.iter().copied().filter(|l| Some(l.ptr) != remove).filter(|l| rtag(l.tag).0 > now);
    let new : Vec<_> = pre.into_iter().chain(lks).chain(post.into_iter()).collect();
    point(PRIVATE,LNS,path,&new,&[],now,admin,()).as_netbox()
}




/// (Until stamp,_)
pub fn rtag(tag:Tag) -> (Stamp,[u8;8]) {
    (Stamp::try_from(&tag.0[0..8]).unwrap(),tag.0[8..].try_into().unwrap())
}
fn create_rtag(stamp:Stamp) -> Tag {
    let mut t = Tag::default();
    t[0..8].copy_from_slice(&stamp.0);
    t
}

