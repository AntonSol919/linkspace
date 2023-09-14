// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_pkt::*;

use crate::prelude::{TestOp };

use super::pkt_predicates::PktPredicates;

impl PktPredicates {
    pub fn from_gdp(group: GroupID, domain: Domain, space: &Space,exact:bool) -> Self {
        let mut r = Self::DEFAULT
            .group(group)
            .unwrap()
            .domain(domain)
            .unwrap();
        r.prefix(space).unwrap();
        if exact {
            r.depth.try_add(TestOp::Equal, *r.rspace_prefix.space_depth()).unwrap();
        }
        r
    }
    

    pub fn from_gd(group: GroupID, domain: Domain) -> Self {
        Self::DEFAULT.group(group).unwrap().domain(domain).unwrap()
    }
    pub fn space(mut self, space: impl AsRef<Space>) -> anyhow::Result<Self> {
        self.prefix(space)?;
        self.depth.try_add(TestOp::Equal, *self.rspace_prefix.space_depth())?;
        Ok(self)
    }
    pub fn prefix(&mut self, prefix: impl AsRef<Space>) -> anyhow::Result<()> {
        let sp = prefix.as_ref();
        if self.rspace_prefix.starts_with(sp) {
            tracing::trace!(old=%self.rspace_prefix,new=%sp,"Current prefix already more specific")
        } else if sp.starts_with(&self.rspace_prefix) {
            let sp = sp.try_into_rooted()?;
            tracing::trace!(old=%self.rspace_prefix,new=%sp,new_len=sp.space_depth(),"Setting more specific prefix");
            self.rspace_prefix = sp;
            self.check_space()?;
        } else {
            anyhow::bail!("disjoint space {:?} <> {:?}", sp, &*self.rspace_prefix);
        };
        Ok(())
    }
    pub fn key(mut self, k: PubKey) -> anyhow::Result<Self> {
        self.pubkey.try_add(TestOp::Equal,k.into())?;
        Ok(self)
    }
    pub fn group(mut self, g: GroupID) -> anyhow::Result<Self> {
        self.group.try_add(TestOp::Equal, g.into())?;
        Ok(self)
    }
    pub fn domain(mut self, domain: impl AsRef<[u8]>) -> anyhow::Result<Self> {
        let domain = Domain::try_fit_byte_slice(domain.as_ref())?.uint().get();
        self.domain.try_add(TestOp::Equal, domain)?;
        Ok(self)
    }
    pub fn create_before(mut self, create: Stamp) -> anyhow::Result<Self> {
        self.create.try_add(TestOp::Less, create.get())?;
        Ok(self)
    }
    
}
