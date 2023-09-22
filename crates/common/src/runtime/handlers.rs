// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::Linkspace;
pub use linkspace_core::matcher::BareWatch;
use linkspace_core::{
    pkt::NetPkt, query::{Query, KnownOptions},
};
use linkspace_pkt::{
    reroute::{ReroutePkt},
    NetFlags, PointExt, NetPktBox,
};
use std::ops::{ControlFlow, Try};

pub struct SinglePktHandler<T>(pub Option<T>);
impl<T> PktStreamHandler for SinglePktHandler<T>
where
    T: FnOnce(&dyn NetPkt, &Linkspace),
{
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, rx: &Linkspace) -> ControlFlow<()> {
        if let Some(func) = self.0.take() {
            func(pkt, rx);
        }
        ControlFlow::Break(())
    }
}

pub enum StopReason {
    Break,
    Finish,
    Replaced,
    Closed,
}

pub trait PktStreamHandler {
    /// Handles an event.
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()>;
    /// Called when break, finished, or replaced
    fn stopped(&mut self, _watch: BareWatch, _lk: &Linkspace, _reason: StopReason) {}
}
impl PktStreamHandler for Box<dyn PktStreamHandler> {
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, rx: &Linkspace) -> ControlFlow<()> {
        (**self).handle_pkt(pkt, rx)
    }
    fn stopped(&mut self, watch: BareWatch, rx: &Linkspace, reason: StopReason) {
        (**self).stopped(watch, rx, reason)
    }
}

impl<F, R: Try<Output = (), Residual = E>, E: std::fmt::Debug> PktStreamHandler for F
where
    F: FnMut(&dyn NetPkt, &Linkspace) -> R + 'static,
{
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, rx: &Linkspace) -> ControlFlow<()> {
        match (self)(pkt, rx).branch() {
            ControlFlow::Continue(_) => ControlFlow::Continue(()),
            ControlFlow::Break(_e) => ControlFlow::Break(()),
        }
    }
}

/// Warning - The notify will be done regardless of the handle_pkt break return;
pub struct NotifyClose<F> {
    pub inner: F,
    pub origin: Option<NetPktBox>,
}
impl<F> NotifyClose<F> {
    pub fn new(inner: F, q:&Query, origin: &dyn NetPkt) -> Self {
        let origin = if matches!(q.get_known_opt(KnownOptions::NotifyClose), Ok(Some(_))){
            Some(origin.as_netbox())
        } else { None};
        NotifyClose {
            inner,
            origin
        }
    }
    
}

impl<F: PktStreamHandler> PktStreamHandler for NotifyClose<F> {
    fn handle_pkt(&mut self, pkt: &dyn NetPkt, lk: &Linkspace) -> ControlFlow<()> {
        self.inner.handle_pkt(pkt, lk)
    }

    fn stopped(&mut self, _watch: BareWatch, _rx: &Linkspace, _reason: StopReason) {
        if let Some(echo) = self.origin.take(){
            self.inner.handle_pkt(&echo, _rx);
        }
        self.inner.stopped(_watch, _rx, _reason)
    }
}

pub struct FollowHandler<F> {
    pub inner: F,
}
impl<F: PktStreamHandler> PktStreamHandler for FollowHandler<F> {
    fn handle_pkt(&mut self, origin_pkt: &dyn NetPkt, rx: &Linkspace) -> ControlFlow<()> {
        let mut pkt = ReroutePkt::new(origin_pkt);
        pkt.net_header.flags.remove(NetFlags::LINKED_IN_FUTURE_PKT);
        pkt.net_header
            .flags
            .remove(NetFlags::LINKED_IN_PREVIOUS_PKT);
        self.inner.handle_pkt(&pkt, rx)?;
        if pkt.pkt.as_point().get_links().is_empty() {
            return ControlFlow::Continue(());
        }
        let r = rx.get_reader();
        tracing::trace!(?origin_pkt, "getting links");

        for link in pkt.pkt.as_point().get_links() {
            let result = r.read(&link.ptr);
            tracing::trace!(?link, ?result, "link");
            match result {
                Ok(Some(follow_pkt)) => {
                    let mut pkt = ReroutePkt::new(follow_pkt);
                    pkt.net_header.flags.insert(NetFlags::LINKED_IN_FUTURE_PKT);
                    pkt.net_header
                        .flags
                        .remove(NetFlags::LINKED_IN_PREVIOUS_PKT);
                    self.inner.handle_pkt(&pkt, rx)?;
                }
                e => tracing::debug!(?origin_pkt, ?link, ?e, "cant follow pkt"),
            }
        }
        ControlFlow::Continue(())
    }
}
