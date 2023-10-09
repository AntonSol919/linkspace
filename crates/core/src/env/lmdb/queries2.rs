// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/*
Predicate compilation needs a overhaul as its probably the worst of both worlds.

Currently:
A PktPredicates struct is big and has a lot of usless fields.
compile_predicates turns each [TestSet] into a [NetPktPredicate<A,B,C>] that holds only relevant fields and the type impls [PkStreamTest].
NetPktPredicate ensures a field to test is only request once or never.
FIXME: At the moment everything is boxed
Two alternatives come to mind:
- An arena allocator
- A Small stack machine. Something like  (Mem:[u8;256]) ++ vec![Enum{ChangeField, (TestOp,&[u8])}]


*/
use crate::pkt::field_ids::FieldEnum;
use crate::predicate::pkt_predicates::PktPredicates;
use either::Either;
use linkspace_pkt::tree_order::TreeEntryRef;
use linkspace_pkt::{Stamp, B64, U256};

use crate::predicate::{
    exprs::RuleType,
    test_pkt::{compile_predicates, PktStreamTest},
};
use crate::prelude::{Bound, TestSet};

use crate::env::query_mode::{Mode, Order, Table};
use crate::env::tree_key::treekey_checked;

use super::queries::{read_pkt, ReadTxn};
use super::tree_iter::TreeKeysIter;
use crate::env::RecvPktPtr;

impl<'txn> ReadTxn<'txn> {
    pub fn scope_iter(
        &'txn self,
        rules: &PktPredicates,
        order: Order,
    ) -> Option<(TreeEntryRef<'txn>, TreeKeysIter<'txn>)> {
        let req = rules.compile_tree_keys(order.is_asc()).unwrap();
        let lower_bound = req.lower_bound().unwrap();
        let iter_dup = self.0.tree_cursor().iter_dup(order.is_asc());
        let at = iter_dup
            .set_range(&lower_bound)
            .map(super::tree_iter::spd)?;
        let mut it = TreeKeysIter {
            req,
            iter_dup,
            lower_bound,
        };
        let at = it.set_pointer_at_match(at)?;
        Some((at, it))
    }
    pub fn query_tree_entries(
        &'txn self,
        rules: &PktPredicates,
        ord: Order,
    ) -> impl Iterator<Item = TreeEntryRef<'txn>> + 'txn {
        let nth_find_set = rules.state.i_branch;
        let mut yields = nth_find_set.iter(0);
        assert!(yields.peek().is_some(), "i_branch is empty");
        let (mut key_ptr, mut keys_iter) = self.scope_iter(rules, ord).unzip();
        let mut cnt = true;
        let pkt_stamp = rules.create;
        let recv_stamp = rules.recv_stamp;
        let hash = rules.hash;
        let data_size = rules.data_size;
        let links_len = rules.links_len;
        std::iter::from_fn(move || {
            if yields.peek().is_none() {
                key_ptr = keys_iter.as_mut().and_then(|v| v.next_entry());
            }
            tracing::trace!(?key_ptr, "Walk Branch");
            while cnt {
                let next_item = {
                    key_ptr
                        .take()
                        .or_else(|| keys_iter.as_mut().and_then(|v| v.next_entry()))
                };
                if next_item.is_none() {
                    yields = nth_find_set.iter(0);
                    let next_range = keys_iter
                        .as_mut()
                        .map(|v| {
                            key_ptr = v.next_scope();
                            key_ptr.is_some()
                        })
                        .unwrap_or(false);
                    if next_range {
                        tracing::trace!("Reset");
                        continue;
                    } else {
                        tracing::trace!("No more ranges");
                        cnt = false;
                        return None;
                    };
                }
                let next_item = next_item.unwrap();
                let ok = pkt_stamp.test(next_item.create().get())
                    && recv_stamp.test(next_item.local_log_ptr().get())
                    && hash.test(next_item.hash().into())
                    && links_len.test(next_item.links_len().into())
                    && data_size.test(next_item.data_size().into());
                tracing::trace!(ok, ?next_item, "key entry");
                if ok {
                    let yielding = yields.next();
                    tracing::trace!(?yielding,hash=?next_item.hash(),"Branch entry");
                    match yielding {
                        Some(true) => return Some(next_item),
                        Some(false) => {}
                        None => {
                            key_ptr = keys_iter.as_mut().and_then(|v| v.next_scope());
                            yields = nth_find_set.iter(0);
                        }
                    }
                }
            }
            None
        })
    }
    pub fn query_tree(
        &'txn self,
        ord: Order,
        predicates: &PktPredicates,
    ) -> impl Iterator<Item = RecvPktPtr<'txn>> + 'txn {
        let pkt_filter = compile_predicates(predicates)
            .0
            .filter(|(_test, kind)| !treekey_checked(*kind))
            .map(|(test, _)| test)
            .collect::<Vec<_>>();
        let c1 = self.0.pkt_cursor();
        let it = self
            .query_tree_entries(predicates, ord)
            .map(move |v| {
                super::queries::read_pkt(&c1, v.local_log_ptr())
                    .map_err(|e| ("Btree Error - tree query", v.local_log_ptr(), e))
                    .unwrap()
                    .ok_or_else(|| ("BTree inconsistent - cant find", v.local_log_ptr()))
                    .unwrap()
            })
            .filter(move |pkt| pkt_filter.test(pkt));
        let nth_log_set = predicates.state.i_db.iter(0);

        it.zip(nth_log_set).filter_map(|(v, ok)| ok.then_some(v))
    }

    pub fn query_log2(
        &'txn self,
        ord: Order,
        rules: &'txn PktPredicates,
    ) -> impl Iterator<Item = RecvPktPtr<'txn>> {
        let (it, recv) = compile_predicates(rules);
        let tests = it.map(|(t, _)| t).collect::<Vec<_>>().into_boxed_slice();
        let log_range = recv.stamp_range(ord.is_asc());
        let it = self.log_range(log_range);

        let nth_find_set = rules.state.i_branch.iter(0);
        let it = it.zip(nth_find_set).filter_map(|(v, ok)| ok.then_some(v));
        let it = it.filter(move |pkt| tests.test(**pkt));

        let nth_log_set = rules.state.i_db.iter(0);
        it.zip(nth_log_set).filter_map(|(v, ok)| ok.then_some(v))
    }

    pub fn query_hash_entries(
        &'txn self,
        hashset: TestSet<U256>,
        recv_bound: Bound<u64>,
        ord: Order,
    ) -> impl Iterator<Item = Stamp> + '_ {
        use crate::predicate::value_test::*;
        match ord {
            Order::Asc => {
                let TestSet {
                    bound:
                        Bound {
                            low: greater_eq,
                            high: less_eq,
                        },
                    mask,
                } = hashset;
                if mask != Mask::DEFAULT {
                    tracing::warn!("todo impl hash mask jumping");
                }
                let greater_eq: B64<[u8; 32]> = greater_eq.into();
                self.0
                    .hash_cursor()
                    .range_uniq(&greater_eq)
                    .map(|(hash, stamp)| (B64(*hash).into(), stamp))
                    .take_while(move |(v, _)| *v <= less_eq)
                    .filter(move |(v, stamp): &(U256, _)| recv_bound.test(*stamp) && mask.test(v))
                    .map(|(_, stamp)| stamp.into())
            }
            Order::Desc => {
                todo!("hash desc not yet impl");
            }
        }
    }
    pub fn query_hash(
        &'txn self,
        ord: Order,
        rules: &'txn PktPredicates,
    ) -> impl Iterator<Item = RecvPktPtr<'txn>> {
        let (it, recv_bound) = compile_predicates(rules);
        let pkt_filter = it
            .filter(|(_, kind)| *kind != RuleType::Field(FieldEnum::PktHashF))
            .map(|(test, _)| test)
            .collect::<Vec<_>>();
        let c1 = self.0.pkt_cursor();
        let it = self
            .query_hash_entries(rules.hash, recv_bound, ord)
            .map(move |v| {
                read_pkt(&c1, v)
                    .expect("BTree Is inconsistent")
                    .expect("BTree Is inconsistent")
            })
            .filter(move |pkt| pkt_filter.test(pkt));
        let nth_log_set = rules.state.i_db.iter(0);

        it.zip(nth_log_set).filter_map(|(v, ok)| ok.then_some(v))
    }

    pub fn query(
        &'txn self,
        mode: Mode,
        pred: &'txn PktPredicates,
        nth_pkt: &'txn mut u32,
    ) -> anyhow::Result<impl Iterator<Item = RecvPktPtr<'txn>>> {
        let span = tracing::debug_span!("query db",?mode,%pred);
        let e = span.enter();

        let it = match mode.table {
            Table::Hash => {
                let it = self.query_hash(mode.order, pred);
                let filter = pred.state.i_query.iter_contains(nth_pkt);
                let it = it.zip(filter).filter_map(|(v, ok)| ok.then_some(v));
                Either::Left(it)
            }
            Table::Tree => {
                let it = self.query_tree(mode.order, pred);
                let filter = pred.state.i_query.iter_contains(nth_pkt);
                let it = it.zip(filter).filter_map(|(v, ok)| ok.then_some(v));
                Either::Right(Either::Left(it))
            }
            Table::Log => {
                let it = self.query_log2(mode.order, pred);
                let filter = pred.state.i_query.iter_contains(nth_pkt);
                let it = it.zip(filter).filter_map(|(v, ok)| ok.then_some(v));
                Either::Right(Either::Right(it))
            }
        };
        std::mem::drop(e);
        let it = it.inspect(move |pkt| {
            tracing::debug!(parent:&span,pkt=?linkspace_pkt::PktFmtDebug(pkt),"yielding")
        });
        Ok(it)
    }
}
