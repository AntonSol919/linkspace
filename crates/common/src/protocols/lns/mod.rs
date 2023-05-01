// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/**
LNS is a little complex because it supports 3 modes.
A mode defines when a claim is 'live'.

The modes are :
- the public roots (:com , :dev, :free etc) - authorities vote and claims have fast path based lookup.
- a :local admin, creates keypoints to other 'roots' creating a chain of names. 
- the :env claims are claims stored as files in the LK_DIR directory

an admin key/process creates a local lookup & reverse-lookup table such that each can quickly be resolved.
Every part of this is not yet fully implemented.

**/



use abe::eval::{clist, ApplyResult};
use anyhow::{Context };
use byte_fmt::ab;
use linkspace_argon2_identity::pubkey;
use linkspace_pkt::{spath, Domain, Tag, PubKey, GroupID, Ptr, Stamp, Link };
use tracing::instrument;

use crate::runtime::Linkspace;

use self::{claim::{LiveClaim, Claim, resolve_enckey}, name::{Name, SpecialName}, public_claim::IssueHandler};


pub mod name;
pub mod claim;

pub mod public_claim;
pub mod local_claim;
pub mod env_claim;

pub mod eval;
pub mod utils;
pub mod admin;

pub const LNS: Domain = ab(b"lns");
spath!(pub const CLAIM_PREFIX = [b"claims"]);
/// tag expected for local claims pointing to a (live) lns:[#:pub] claim
pub const PUB_CLAIM_TAG : Tag = ab(b"pub-claim");
pub const PUBKEY_TAG : Tag = ab(b"pubkey@");
pub const VOTE_TAG : Tag = ab(b"vote");
pub const GROUP_TAG: Tag = ab(b"group#");
pub const ENCKEY_TAG : Tag = ab(b"enckey");
/// A linkpoint at lns:[#:0]:by-tag/../PTR will contain by-claim:CLAIM_HASH
pub const BY_CLAIM_TAG : Tag = ab(b"by-claim");

pub const BY_TAG_P : linkspace_pkt::IPathC<15> = linkspace_pkt::ipath1(b"by-tag");
spath!(pub const BY_GROUP_TAG = [b"by-tag",&GROUP_TAG.0]);
spath!(pub const BY_PUBKEY_TAG= [b"by-tag",&PUBKEY_TAG.0]);


pub fn auth_tag(b:&[u8]) -> Tag {
    let mut auth = ab(b"^");
    auth[0..15][15-b.len()..].copy_from_slice(b);
    auth
}
/// Get the parent claim
pub fn lookup_authority_claim(lk:&Linkspace,name:&Name,issue_handler:IssueHandler) -> anyhow::Result<Result<Claim,Claim>>{
    match name.special() {
        Some(SpecialName::Env) => Ok(Ok(Claim::new(name.clone(),Stamp::MAX,&[],vec![]).unwrap())),
        Some(SpecialName::Local) => todo!(),
        None => {
            let (parent,_val) = name.spath().pop();
            let name = Name::from_spath(parent).ok().unwrap_or_else(Name::root);
            Ok(lookup_live_chain(lk, &name, issue_handler)?.map(|p| p.claim).map_err(|p|p.claim))
        },
    }
}

fn dummy_root(name:&Name) -> LiveClaim{
    LiveClaim{
        claim: Claim::new(name.clone(),Stamp::MAX,&[],vec![]).unwrap(),
        signatures: vec![],
        parent: None,
    }
}


/// Lookup the chain of claims that gave a name
pub fn lookup_live_chain(lk:&Linkspace, name: &Name,issue_handler:IssueHandler) -> anyhow::Result<Result<LiveClaim,LiveClaim>>{
    match name.special(){
        Some(SpecialName::Env) => {
            let path = name.fs_env_path()?;
            let claim : anyhow::Result<Claim>= try {
                let pbytes = match lk.env().env_data(&path, false)?{
                    Some(p) => p,
                    None => return Ok(Err(dummy_root(name)))
                };
                let pkt = linkspace_pkt::read::parse_netpkt(&pbytes, false)?.map_err(|_| anyhow::anyhow!("not a valid packet"))?;
                Claim::from(pkt)?
            };
            Ok(Ok(LiveClaim{
                claim: claim.with_context(||anyhow::anyhow!("Reading claim at {}",path.to_string_lossy()))?,
                signatures: vec![],
                parent: None,
            }))
        },
        Some(SpecialName::Local) => {
            // No admin process exists yet so we pretend something setup the correct :local claims
             match local_claim::get_private_claim(&lk.get_reader(), name, None).into_ok()?{
                Some(claim) => {
                    Ok(Ok(LiveClaim{
                        claim:Claim::from(claim)?,
                        signatures: vec![],parent:None
                    }))
                }
                None => Ok(Err(dummy_root(name))),
             }
        }
        // The admin process doesn't exist yet so we walk the chain for now
        None => public_claim::walk_live_claims(&lk.get_reader(), public_claim::root_claim(), &mut name.spath().iter(), issue_handler),
    }
}

pub fn lookup_enckey(lk:&Linkspace,name:&Name) -> anyhow::Result<Option<(PubKey,String)>>{
    let claim = lookup_claim(lk, name)?;
    match claim{
        None => return Ok(None),
        Some(c) => match c.enckey()?{
            Some(k) => resolve_enckey(&lk.get_reader(), k).map(Some),
            None => Ok(None)
        },
    }
}


pub fn lookup_pubkey(lk:&Linkspace,name:&Name) -> anyhow::Result<Option<PubKey>>{
    lookup_claim(lk, name).map(|m|m.and_then(|c| c.pubkey().copied()))
}

pub fn lookup_group(lk:&Linkspace,name:&Name) -> anyhow::Result<Option<GroupID>>{
    lookup_claim(lk, name).map(|m|m.and_then(|c| c.group().copied()))
}

pub fn lookup_claim(lk:&Linkspace,name:&Name) -> anyhow::Result<Option<Claim>>{
    lookup_live_chain(lk, name, &mut |_|Ok(())).map(|o|o.ok().map(|o|o.claim))
}

#[instrument(skip(lk),ret)]
pub fn reverse_lookup(lk:&Linkspace,tag:Tag,ptr:Ptr) -> ApplyResult<Claim>{
    // Because we can't yet trust the admin, we have to do a forward lookup as well to validate this is a valid claim.
    let claim = admin::ptr_lookup(&lk.get_reader(), tag, ptr, None)?;
    let name = &claim.name;
    let by_name = lookup_claim(lk, name)??;
    if by_name.links().first_eq(tag).map(|p|p.ptr == ptr).unwrap_or(false){
        Some(by_name).into()
    }else {
        Err(anyhow::anyhow!("found {claim} pointing to {name} but this is set to {by_name} with a different {tag} link")).into()
    }
}

pub fn setup_special_keyclaim(
    lk: &Linkspace,
    name: Name,
    enckey: &str,
    overwrite:bool
) -> anyhow::Result<PubKey> {
    let sp = name.special().context("will only setup :local or :env keys")?;
    if let Ok(Some(c)) = lookup_claim(lk, &name){
        if !overwrite {anyhow::bail!("claim already set but overwrite is false")}
        else { tracing::debug!(old_claim=%c)}
    }
    let pubkey = pubkey(enckey)?.into();
    let claim = Claim::new(name, Stamp::MAX, &[Link{tag: PUBKEY_TAG,ptr:pubkey}], vec![clist(&["enckey",enckey])])?;
    match sp {
        SpecialName::Local => {
            if claim.name.spath().collect().len() > 2 { anyhow::bail!("Local is currently limited to single component name")}
            local_claim::setup_local_keyclaim(lk, claim,None)?;
        },
        SpecialName::Env => env_claim::setup(lk,claim,overwrite)?
    }
    Ok(pubkey)
}
