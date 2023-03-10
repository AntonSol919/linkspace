// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/*
TODO. components needs to be integrated here


*/

use anyhow::Context;
use linkspace_pkt::*;

use crate::{
    prelude::{TreeEntryRef, TreeKey, TreeValueBytes},
    stamp_range::{IterCmp, StampRange},
};

use super::{bitset_test::BitTestSet, TestSet, UInt};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TreeKeys {
    pub domain: TestSet<u128>,
    pub group: TestSet<U256>,
    pub ipath: IPathBuf,
    pub depth: BitTestSet,
    pub pubkey: TestSet<U256>,
    pub cstamp: StampRange,
}

impl TreeKeys {
    pub fn lower_bound(&self) -> anyhow::Result<Vec<u8>> {
        let mut btree_key = vec![];
        let (group, domain, depth, spath, pubkey) = (
            self.group.info(UInt::MIN).val.context("Empty group set")?,
            self.domain
                .info(UInt::MIN)
                .val
                .context("Empty domain set")?,
            self.depth.info(0).val.context("Empty depthset")?,
            &self.ipath,
            self.pubkey
                .info(UInt::MIN)
                .val
                .context("Empty pubkey set")?,
        );
        tracing::trace!(?group,?domain,?depth,%spath,?pubkey, "lowerbounds");
        btree_key.extend_from_slice(&group.to_be_bytes::<32>());
        btree_key.extend_from_slice(&Domain::from(domain).0);
        btree_key.push(depth);
        btree_key.extend(spath.spath_bytes());
        btree_key.extend_from_slice(&pubkey.to_be_bytes::<32>());
        Ok(btree_key)
    }
}

use crate::env::db::IterDup;
#[derive(Debug)]
pub struct TreeKeysIter<'txn> {
    pub iter_dup: IterDup<'txn>,
    pub req: TreeKeys,
    pub lower_bound: Vec<u8>,
}
pub fn spd<'o>(kv: (&'o [u8], &'o TreeValueBytes)) -> TreeEntryRef<'o> {
    TreeEntryRef::from_db(kv)
}

impl<'txn> TreeKeysIter<'txn> {
    pub fn next_entry(&mut self) -> Option<TreeEntryRef<'txn>> {
        self.iter_dup
            .get_next_entry()
            .map(spd)
            .filter(|v| self.req.cstamp.contains(v.create()))
    }
    fn next_stamp_match(&self, at: &mut TreeEntryRef<'txn>) -> Result<(), ()> {
        let mut cmp = self.req.cstamp.iter_cmp(at.create());
        while cmp == IterCmp::Pre {
            *at = self.iter_dup.get_next_entry().map(spd).ok_or(())?;
            cmp = self.req.cstamp.iter_cmp(at.create());
        }
        if cmp == IterCmp::In {
            Ok(())
        } else {
            Err(())
        }
    }

    pub(crate) fn set_pointer_at_match(
        &mut self,
        mut at: TreeEntryRef<'txn>,
    ) -> Option<TreeEntryRef<'txn>> {
        let mut jump: Vec<u8> = vec![];
        loop {
            jump.clear();
            let (group, domain, path_len, key) = {
                let (group, domain, sp_len, _, key) = at.btree_key.fields();
                tracing::trace!(?at, "Fields");
                (
                    self.req.group.info(group.into()),
                    self.req.domain.info(domain.into()),
                    self.req.depth.info(sp_len),
                    self.req.pubkey.info(key.into()),
                )
            };
            let prefix_ok = at.btree_key.spath().starts_with(&self.req.ipath);
            tracing::trace!(at=%at.btree_key.spath(), req=%self.req.ipath, prefix_ok,"PrefixOk ");
            tracing::trace!(group=?group.into::<GroupID>(),dom=?domain.into::<Domain>(),?path_len,?prefix_ok,?key,"Scope Jump");
            if key.in_set && path_len.in_set && prefix_ok && domain.in_set && group.in_set {
                match self.next_stamp_match(&mut at) {
                    Ok(_) => {
                        tracing::trace!("Match found");
                        return Some(at);
                    }
                    Err(_) => {
                        jump.extend_from_slice(at.btree_key.as_bytes());
                        jump.push(255);
                        let next = self.iter_dup.set_range(&jump).map(spd)?;
                        tracing::trace!(?at, ?next, " Create stamp OOR, jumped ");
                        at = next;
                        continue;
                    }
                }
            }

            // valueinfo returns this or next matching value of a set, and in_set = true if it equals the input
            // We build up the next key to jump to in pieces.
            // For each field we append it to the 'jump' value.
            // if the next field has no value ( its not in set and no incr will ever be ), we break and copy the rest from our 'lower_bound'

            jump.extend_from_slice(&group.val?.to_be_bytes::<32>());
            if group.in_set {
                jump.extend_from_slice(&domain.val.map(u128::to_be_bytes).unwrap_or([255; 16]));
                if domain.val.is_none() {
                    jump.push(255);
                } else if domain.in_set {
                    match (path_len.in_set, prefix_ok, key.in_set) {
                        (true, true, true) => unreachable!(),
                        (false, _, _) => {
                            tracing::trace!(?path_len, "Pathlen OOB, setting next depth");
                            jump.push(
                                self.req
                                    .depth
                                    .next_depth(path_len.val.unwrap_or(255))
                                    .unwrap_or(255),
                            );
                        }
                        (true, false, _) => {
                            jump.push(
                                self.req
                                    .depth
                                    .next_depth(path_len.val.unwrap_or(255))
                                    .unwrap_or(255),
                            );
                            tracing::trace!("Prefix OOB, extinding with lowerbound ");
                        }
                        (true, true, false) => {
                            jump.push(path_len.val.unwrap_or(255));
                            jump.extend_from_slice(at.btree_key.spath().spath_bytes());
                            jump.extend_from_slice(
                                &key.val.map(|v| v.to_be_bytes::<32>()).unwrap_or([255; 32]),
                            );
                        }
                    }
                } else {
                    tracing::trace!("Domain OOB")
                }
            } else {
                tracing::trace!("Group OOB")
            }

            if let Some(ext) = self.lower_bound.get(jump.len()..) {
                let old_len = jump.len();
                jump.extend_from_slice(ext);
                tracing::trace!(old_len, len = jump.len(), "Extending jump val");
            }
            let next = self.iter_dup.set_range(&jump).map(spd);
            tracing::trace!(?next,?at,?jump,jump_str=?TreeKey::from(&jump),"Jumpped");
            at = next?;
        }
    }

    pub fn next_scope(&mut self) -> Option<TreeEntryRef<'txn>> {
        let at = self.iter_dup.get_next_range().map(spd)?;
        tracing::trace!(?at, "next scope");
        self.set_pointer_at_match(at)
    }
}
