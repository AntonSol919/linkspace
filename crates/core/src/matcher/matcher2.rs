// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::{ensure, Context};
use linkspace_pkt::{abe::TypedABE, reroute::RecvPkt, NetPkt, NetPktExt, Stamp};
use tracing::Span;

use crate::{
    env::queries::RecvPktPtr,
    predicate::{
        pkt_predicates::StatePredicates,
        test_pkt::{compile_predicates, PktStreamTest},
        TestSet,
    },
    prelude::{Bound, Query},
};

pub use super::{Cf, DSpan};

pub type WatchID = Vec<u8>;
pub type WatchIDRef = [u8];
pub type WatchIDExpr = TypedABE<Vec<u8>>;

/// [[WatchEntry]] with no associated context
pub type BareWatch = WatchEntry<()>;

#[derive(Debug)]
/// Stored predicates, predicate state, identity, and associated ctx \<C\> ( usually a callback )
pub struct WatchEntry<C> {
    pub id: WatchID,
    pub tests: Vec<Box<dyn PktStreamTest>>,
    pub nth_query: u32,
    pub i_query: TestSet<u32>,
    pub nth_new: u32,
    pub i_new: TestSet<u32>,
    pub recv_bounds: Bound<u64>,
    pub query: Box<Query>,
    pub last_test: (bool, Cf),
    pub ctx: C,
    pub span: DSpan,
}
impl<C> WatchEntry<C> {
    pub fn new(
        id: WatchID,
        query: Query,
        nth_query: u32,
        ctx: C,
        span: Span,
    ) -> anyhow::Result<Self> {
        let (it, recv_bounds) = compile_predicates(&query.predicates);
        let tests = it.map(|(t, _)| t).collect();
        let StatePredicates {
            mut i_new, i_query, ..
        } = query.predicates.state;
        let nth_new = 0;
        ensure!(
            recv_bounds.as_eq().is_none(),
            "watching for 'eq' recv is nonsense"
        );
        // With this, we only need to check i_new.less_eq < i_new to determine if our watch has ended
        if i_query.bound.high != u32::MAX {
            let less = (i_query.bound.high + 1)
                .checked_sub(nth_query)
                .context("Empty i_new and i_query combo")?;
            i_new
                .try_add(crate::predicate::TestOp::Less, less)
                .context("empty i_new and i_query combination")?;
        }
        tracing::trace!(?i_new, ?i_query, ?recv_bounds, ?nth_query, "Watch budgets");

        ensure!(i_new.has_any(), "query budget empty");
        ensure!(i_query.info(nth_query).val.is_some(), "watch budget empty");
        Ok(WatchEntry {
            id,
            query: Box::new(query),
            tests,
            recv_bounds,
            i_new,
            nth_new,
            i_query,
            nth_query,
            ctx,
            span: DSpan(span),
            last_test: (false, Cf::CONTINUE),
        })
    }
    pub fn map<N>(self, new_ctx: N) -> (C, WatchEntry<N>) {
        let WatchEntry {
            tests,
            id,
            query,
            ctx,
            span,
            recv_bounds,
            nth_query,
            i_query,
            nth_new,
            i_new,
            last_test,
        } = self;
        (
            ctx,
            WatchEntry {
                tests,
                id,
                query,
                ctx: new_ctx,
                span,
                recv_bounds,
                nth_query,
                i_query,
                nth_new,
                i_new,
                last_test,
            },
        )
    }

    pub fn test_dyn(&mut self, pkt: &dyn NetPkt) -> (bool, Cf) {
        // this is terribly inefficient.
        let p = pkt.as_netbox();
        self.test(RecvPkt {
            recv: pkt.get_recv(),
            pkt: &p,
        })
    }
    // return bool : Is Match and Ctr::BReak if testing is done
    pub fn test(&mut self, db_pkt: RecvPktPtr) -> (bool, Cf) {
        self.last_test = self._test(db_pkt);
        self.last_test
    }
    fn _test(&mut self, pkt: RecvPktPtr) -> (bool, Cf) {
        if self.recv_bounds.high < pkt.get_recv().get() {
            tracing::trace!("break: Recv Out of upper bound");
            return (false, Cf::BREAK);
        }
        if self.recv_bounds.low > pkt.get_recv().get() {
            tracing::trace!("cnt: Recv Out of lower bound");
            return (false, Cf::CONTINUE);
        }
        let accepted = self.tests.result(&pkt);
        if let Err(kind) = accepted {
            tracing::trace!(?kind, "Test failed");
            return (false, Cf::CONTINUE);
        }
        let accepted_nth = self.i_new.test(self.nth_new) && self.i_query.test(self.nth_query);
        tracing::trace!(accepted_nth, "accepted");
        self.nth_new += 1;
        self.nth_query += 1;
        (
            accepted_nth,
            if self.i_new.bound.high < self.nth_new {
                Cf::BREAK
            } else {
                Cf::CONTINUE
            },
        )
    }
}

pub struct Matcher<C> {
    watch_entries: Vec<WatchEntry<C>>,
}
impl<C> Default for Matcher<C> {
    fn default() -> Self {
        Self {
            watch_entries: Vec::new(),
        }
    }
}
impl<C> Matcher<C> {
    pub fn register(&mut self, watch_e: WatchEntry<C>) -> Option<WatchEntry<C>> {
        let ok = watch_e.recv_bounds.high > linkspace_pkt::now().get();
        tracing::debug!(register_ok=?ok);
        match (
            ok,
            self.watch_entries
                .binary_search_by_key(&watch_e.id.as_ref(), |v| &v.id),
        ) {
            (true, Ok(i)) => Some(::std::mem::replace(&mut self.watch_entries[i], watch_e)),
            (false, Ok(i)) => Some(self.watch_entries.remove(i)),
            (true, Err(i)) => {
                self.watch_entries.insert(i, watch_e);
                None
            }
            (false, Err(_i)) => None,
        }
    }
    pub fn deregister(
        &mut self,
        watch_id: &WatchIDRef,
        range: bool,
        mut on_drop: impl FnMut(WatchEntry<C>),
    ) -> usize {
        match self
            .watch_entries
            .binary_search_by_key(&watch_id, |e| &e.id)
        {
            Ok(i) => {
                if !range {
                    let w = self.watch_entries.remove(i);
                    on_drop(w);
                    1
                } else {
                    let c = self.watch_entries[i..]
                        .iter()
                        .take_while(|v| v.id.starts_with(watch_id))
                        .count();
                    for w in self.watch_entries.drain(i..c + i) {
                        on_drop(w)
                    }
                    c
                }
            }
            Err(_) => 0,
        }
    }
    pub fn trigger(
        &mut self,
        pkt: RecvPktPtr,
        mut on_match: impl FnMut(&mut C) -> Cf,
        on_drop: impl FnMut(WatchEntry<C>),
    ) {
        self.watch_entries
            .drain_filter(|e| {
                let _g = e.span.0.clone().entered();
                let (test_ok, test_finish) = e.test(pkt);
                let callback_finish =
                    (test_ok && on_match(&mut e.ctx).is_break()) || test_finish.is_break();
                tracing::debug!(test_ok, ?test_finish, callback_finish);
                callback_finish
            })
            .for_each(on_drop);
    }
    /// clears out of bound watches and determine when the next gc should be run ( none means empty, Some(MAX) means no oob set)
    pub fn gc(&mut self, logptr: Stamp) -> Option<Stamp> {
        let mut min: u64 = u64::MAX;
        self.watch_entries.retain(|e| {
            if e.recv_bounds.high > logptr.get() {
                min = e.recv_bounds.high.min(min);
                true
            } else {
                false
            }
        });
        if self.watch_entries.is_empty() {
            None
        } else {
            Some(min.into())
        }
    }
    pub fn entries(&self) -> &[WatchEntry<C>] {
        &self.watch_entries
    }

    /*
    pub fn any_match(&mut self, _pkt: impl NetFields, _locallogptr: Stamp,mut on_drop:impl FnMut(WatchEntry<C>))  -> bool {
        let mut ok = false;
        self.watch_entries
            .drain_filter( |_e| {
                todo!()
            })
            .for_each(|e| on_drop(e));
        ok
    }
     */
}
/*
impl<E:std::fmt::Debug> std::fmt::Debug for WatchEntry<E>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WatchEntry")
            .field("id", &self.id).field("tests", &self.tests.len())
            .field("q_budget", &self.q_budget).field("new_budget", &self.new_budget)
            .field("recv_bounds", &self.recv_bounds).field("rules", &self.rules)
            .field("ctx", &self.ctx).field("span", &self.span).finish()
    }
}
*/
