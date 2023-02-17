// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(unused_variables, unused_mut)]
use anyhow::ensure;
use linkspace_pkt::*;

use crate::prelude::TestOp;

use super::pkt_predicates::PktPredicates;

impl PktPredicates {
    pub fn from_gdp(group: GroupID, domain: Domain, path: &SPath) -> Self {
        Self::DEFAULT
            .group(group)
            .unwrap()
            .domain(domain)
            .unwrap()
            .path(path)
            .unwrap()
    }
    pub fn from_gd(group: GroupID, domain: Domain) -> Self {
        Self::DEFAULT.group(group).unwrap().domain(domain).unwrap()
    }
    pub fn path(mut self, path: impl AsRef<SPath>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        ensure!(path.starts_with(&self.path_prefix), "incompatible path");
        self.path_prefix = path.into_spathbuf().try_idx()?;
        self.path_len
            .try_add(TestOp::Equal, *self.path_prefix.path_len())?;
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
