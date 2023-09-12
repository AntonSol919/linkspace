// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::Query;
use linkspace_common::core::query::Query as QueryImpl;
#[doc(hidden)]
impl From<crate::Query> for QueryImpl {
    fn from(val: crate::Query) -> Self {
        val.0
    }
}
#[doc(hidden)]
impl From<QueryImpl> for crate::Query {
    fn from(value: QueryImpl) -> Self {
        crate::Query(value)
    }
}
impl crate::Query {
    #[doc(hidden)]
    pub fn as_impl(&self) -> &QueryImpl {
        unsafe { &*(self as *const Query as *const QueryImpl) }
    }
    #[doc(hidden)]
    pub fn from_impl(q: &QueryImpl) -> &Query {
        unsafe { &*(q as *const QueryImpl as *const Query) }
    }
}

#[cfg(feature = "runtime")]
pub mod rt_interop {
    use std::ops::ControlFlow;

    use linkspace_common::{
        prelude::NetPkt,
        runtime::{handlers::StopReason},
    };
    pub use linkspace_common::runtime::Linkspace as LinkspaceImpl;
    #[doc(hidden)]
    impl From<LinkspaceImpl> for crate::Linkspace {
        fn from(value: LinkspaceImpl) -> Self {
            crate::Linkspace(value)
        }
    }
    #[doc(hidden)]
    impl From<crate::Linkspace> for LinkspaceImpl {
        fn from(val: crate::Linkspace) -> Self {
            val.0
        }
    }
    impl crate::Linkspace {
        #[doc(hidden)]
        pub fn as_impl(&self) -> &LinkspaceImpl {
            unsafe { &*(self as *const Linkspace as *const LinkspaceImpl) }
        }
        #[doc(hidden)]
        pub fn from_impl(lk: &LinkspaceImpl) -> &Linkspace {
            unsafe { &*(lk as *const LinkspaceImpl as *const Linkspace) }
        }
    }

    // Wrapper to hide PktStreamHandler and its type arguments
    pub(crate) struct Handler<T: PktHandler + ?Sized>(pub(crate) T);
    

    use crate::{Linkspace, runtime::cb::PktHandler, Query};
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
}
