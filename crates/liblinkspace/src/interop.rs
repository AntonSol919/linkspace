// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#[doc(hidden)]
impl Into<linkspace_common::core::query::Query> for crate::Query {
    fn into(self) -> linkspace_common::core::query::Query {
        self.0
    }
}
#[doc(hidden)]
impl From<linkspace_common::core::query::Query> for crate::Query {
    fn from(value: linkspace_common::core::query::Query) -> Self {
        crate::Query(value)
    }
}
#[doc(hidden)]
impl From<linkspace_common::runtime::Linkspace> for crate::Linkspace {
    fn from(value: linkspace_common::runtime::Linkspace) -> Self {
        crate::Linkspace(value)
    }
}
#[doc(hidden)]
impl Into<linkspace_common::runtime::Linkspace> for crate::Linkspace {
    fn into(self) -> linkspace_common::runtime::Linkspace {
        self.0
    }
}
impl crate::Linkspace{
    #[doc(hidden)]
    pub fn as_impl(&self) -> &LinkspaceImpl{
        unsafe{&*(self as *const Linkspace as *const LinkspaceImpl)}
    }
    #[doc(hidden)]
    pub fn from_impl(lk:&LinkspaceImpl) -> &Linkspace{
        unsafe { &*(lk as *const LinkspaceImpl as *const Linkspace) }
    }
}

// Wrapper to hide PktStreamHandler and its type arguments
pub(crate) struct Handler<T: PktHandler + ?Sized>(pub(crate) T);
use std::ops::ControlFlow;

use linkspace_common::{runtime::{Linkspace as LinkspaceImpl, handlers::StopReason}, prelude::NetPkt};

use crate::{PktHandler, Linkspace, Query};
impl<T: PktHandler + ?Sized> linkspace_common::runtime::handlers::PktStreamHandler for Handler<T> {
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, rx: &LinkspaceImpl) -> ControlFlow<()> {
        let rx = unsafe { &*(rx as *const LinkspaceImpl as *const Linkspace) };
        self.0.handle_pkt(pkt, rx)
    }
    fn stopped(
        &mut self,
        watch: linkspace_common::prelude::BareWatch,
        rx: &LinkspaceImpl,
        reason: StopReason,
    ) {
        let query = Query(*watch.query);
        let rx = unsafe { &*(rx as *const LinkspaceImpl as *const Linkspace) };
        self.0
            .stopped(query, rx, reason, watch.nth_query, watch.nth_new)
    }
}

