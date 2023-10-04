// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/**
LNS is complex. In part because it supports 3 modes.

The modes are :
- public roots (:com , :dev etc) - claims are created and authorities vote - (compromised) keys are easy/cheap to replace and fast path lookup.
- a :local admin - creates keypoints to other 'roots' creating a chain of names - a simple forward lookup where each (group of) keys names others keys.

Each has a different concept of when a claim is 'live'.

Not all part of are fully implemented.

**/
use abe::eval::ApplyResult;
use anyhow::ensure;
use byte_fmt::{ab, AB};
use linkspace_argon2_identity::pubkey;
use linkspace_pkt::{
    rspace1, Domain, GroupID, Link, LkHash, Point, PubKey, RootedStaticSpace, Stamp, Tag,
};
use tracing::instrument;

use crate::runtime::Linkspace;

use self::{
    claim::{read_enckey, Claim, LiveClaim},
    name::{Name, NameType},
    public_claim::IssueHandler,
};

pub mod claim;
pub mod name;

pub mod local_claim;
pub mod public_claim;

pub mod admin;
pub mod eval;
pub mod utils;

pub const LNS: Domain = ab(b"lns");
pub const CLAIM_PREFIX: RootedStaticSpace<15> = rspace1::<6>(b"claims");
/// tag expected for local claims pointing to a (live) lns:[#:pub] claim
pub const PUB_CLAIM_TAG: [u8; 9] = *b"pub-claim";
pub const PUBKEY_TAG: [u8; 7] = *b"pubkey@";
pub const PUBKEY_AUTH_TAG: [u8; 7] = *b"pubkey^";
pub const GROUP_TAG: [u8; 6] = *b"group#";
pub const ENCKEY_TAG: [u8; 6] = *b"enckey";
/// A linkpoint at lns:[#:0]:by-tag/../PTR will contain by-claim:CLAIM_HASH
pub const BY_CLAIM_TAG: Tag = ab(b"by-claim");
pub const VOTE_TAG: Tag = ab(b"vote");

pub const BY_TAG_P: linkspace_pkt::RootedStaticSpace<15> = linkspace_pkt::rspace1::<6>(b"by-tag");
pub static BY_GROUP_TAG: [&[u8]; 2] = [b"by-tag", &GROUP_TAG];
pub static BY_PUBKEY_TAG: [&[u8]; 2] = [b"by-tag", &PUBKEY_TAG];

/// (Until stamp,_)
#[inline(always)]
pub fn as_stamp_tag(tag: Tag) -> (Stamp, [u8; 8]) {
    (
        Stamp::try_from(&tag.0[0..8]).unwrap(),
        tag.0[8..].try_into().unwrap(),
    )
}

#[inline(always)]
pub fn stamp_tag(stamp: Stamp, rest: [u8; 8]) -> Tag {
    let mut t = Tag::default();
    t[0..8].copy_from_slice(&stamp.0);
    t[8..].copy_from_slice(&rest);
    t
}

pub fn lnstag(stamp: Stamp, rest: &[u8], kind: u8) -> anyhow::Result<Tag> {
    anyhow::ensure!(rest.len() < 8);
    let mut t = Tag::default();
    t[0..8].copy_from_slice(&stamp.0);
    t[0..15][15 - rest.len()..].copy_from_slice(rest);
    t[15] = kind;
    Ok(t)
}
/// Get the parent claim
pub fn lookup_authority_claim(
    lk: &Linkspace,
    name: &Name,
    issue_handler: IssueHandler,
) -> anyhow::Result<Result<Claim, Claim>> {
    match name.name_type() {
        NameType::Local => todo!(),
        NameType::Public => {
            let (parent, _val) = name.space().pop();
            let name = Name::from_space(parent).ok().unwrap_or_else(Name::root);
            Ok(lookup_live_chain(lk, &name, issue_handler)?
                .map(|p| p.claim)
                .map_err(|p| p.claim))
        }
    }
}

fn dummy_root(name: &Name) -> LiveClaim {
    LiveClaim {
        claim: Claim::new(
            name.clone(),
            Stamp::MAX,
            &mut [Link {
                tag: [255; 16].into(),
                ptr: [0; 32].into(),
            }],
            &[],
        )
        .unwrap(),
        signatures: vec![],
        parent: None,
    }
}

/// Lookup the chain of claims that gave a name
pub fn lookup_live_chain(
    lk: &Linkspace,
    name: &Name,
    issue_handler: IssueHandler,
) -> anyhow::Result<Result<LiveClaim, LiveClaim>> {
    match name.name_type() {
        NameType::Local => {
            // No admin process exists yet so we pretend something setup the correct :local claims
            match local_claim::get_private_claim(&lk.get_reader(), name, None).into_ok()? {
                Some(claim) => Ok(Ok(LiveClaim {
                    claim: Claim::from(claim)?,
                    signatures: vec![],
                    parent: None,
                })),
                None => Ok(Err(dummy_root(name))),
            }
        }
        // The admin process doesn't exist yet so we walk the chain for now
        NameType::Public => public_claim::walk_live_claims(
            &lk.get_reader(),
            public_claim::root_claim(),
            &mut name.space().iter(),
            issue_handler,
        ),
    }
}

pub fn lookup_enckey(lk: &Linkspace, name: &Name) -> anyhow::Result<Option<(PubKey, String)>> {
    let claim = lookup_claim(lk, name)?;
    if let Some(c) = claim {
        return match c.enckey() {
            Ok(k) => Ok(Some(k)),
            Err(None) => Ok(None),
            Err(Some(link)) => Ok(lk
                .get_reader()
                .read(&link.ptr)?
                .map(|pkt| read_enckey(pkt.data()))
                .transpose()?),
        };
    }
    Ok(None)
}

pub fn lookup_pubkey(lk: &Linkspace, name: &Name) -> anyhow::Result<Option<PubKey>> {
    lookup_claim(lk, name).map(|m| m.and_then(|c| c.pubkey().copied()))
}

pub fn lookup_group(lk: &Linkspace, name: &Name) -> anyhow::Result<Option<GroupID>> {
    lookup_claim(lk, name).map(|m| m.and_then(|c| c.group().copied()))
}

pub fn lookup_claim(lk: &Linkspace, name: &Name) -> anyhow::Result<Option<Claim>> {
    lookup_live_chain(lk, name, &mut |_| Ok(())).map(|o| o.ok().map(|o| o.claim))
}

#[instrument(skip(lk), ret)]
pub fn reverse_lookup(lk: &Linkspace, tag: &[u8], ptr: LkHash) -> ApplyResult<Claim> {
    // Because we can't yet trust the admin, we have to do a forward lookup as well to validate this is a valid claim.
    let claim = admin::ptr_lookup(&lk.get_reader(), tag, ptr, None)?;
    let name = &claim.name;
    let by_name = lookup_claim(lk, name)??;
    if by_name
        .links()
        .first_tailmask(tag)
        .map(|p| p.ptr == ptr)
        .unwrap_or(false)
    {
        Some(by_name).into()
    } else {
        Err(anyhow::anyhow!("found {claim} pointing to {name} but this is set to {by_name} with a different {} link",AB(tag))).into()
    }
}

pub fn setup_special_keyclaim(
    lk: &Linkspace,
    name: Name,
    enckey: &str,
    overwrite: bool,
) -> anyhow::Result<PubKey> {
    let sp = name.name_type();
    ensure!(
        sp == NameType::Local,
        "you can only setup :local key claims this way"
    );
    if let Ok(Some(c)) = lookup_claim(lk, &name) {
        if !overwrite {
            anyhow::bail!("claim already set but overwrite is false")
        } else {
            tracing::debug!(old_claim=%c)
        }
    }
    let pubkey = pubkey(enckey)?.into();
    let claim = Claim::new(
        name,
        Stamp::MAX,
        &mut [Link {
            tag: ab(&PUBKEY_TAG),
            ptr: pubkey,
        }],
        enckey.as_bytes(),
    )?;
    if claim.name.space().to_array().len() > 2 {
        anyhow::bail!("Local is currently limited to single component name")
    }
    local_claim::setup_local_keyclaim(lk, claim, None)?;
    Ok(pubkey)
}
