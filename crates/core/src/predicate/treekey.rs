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
    stamp_range::{ StampRange},
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

