// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::io::Result;

use super::db::LMDBTxn;
use super::db::PktLogCursor;
use super::misc::IterDirection;
use crate::env::RecvPktPtr;
use crate::partial_hash::PartialHash;
use crate::stamp_range::StampRange;
use either::Either;
use linkspace_pkt::*;

pub fn as_recv_ptr((llp, bytes): (u64, &[u8])) -> RecvPktPtr {
    RecvPktPtr {
        pkt: as_netpkt(bytes),
        recv: Stamp::new(llp),
    }
}


fn as_netpkt(bytes: &[u8]) -> &NetPktPtr {
    unsafe { NetPktPtr::from_bytes_unchecked(bytes) }
}
pub(crate) fn read_pkt<'txn>(
    cur: &PktLogCursor<'txn>,
    recv: Stamp,
) -> Result<Option<RecvPktPtr<'txn>>> {
    cur.read_uniq(&recv.get()).map(|opt| {
        opt.map(|bytes| RecvPktPtr {
            pkt: as_netpkt(bytes),
            recv,
        })
    })
}

pub struct ReadTxn<'env>(pub(crate) LMDBTxn<'env>);

impl<'env> ReadTxn<'env> {
    pub fn refresh(&mut self){
        self.0.refresh_inplace().unwrap()
    }
    /// read a pkt and use the local net header
    pub fn read_ptr(&self, hash: &LkHash) -> Result<Option<Stamp>> {
        match self.0.hash_cursor().read_uniq(hash)? {
            Some(idx) => Ok(Some(Stamp::new(idx))),
            None => Ok(None),
        }
    }
    /// read a pkt and use the local net header
    pub fn read(&self, hash: &LkHash) -> Result<Option<RecvPktPtr>> {
        tracing::trace!(hash = ?hash, "Read hash");
        match self.read_ptr(hash)? {
            Some(idx) => read_pkt(&self.0.pkt_cursor(), idx),
            None => Ok(None),
        }
    }
    pub fn log_head(&self) -> Stamp {
        let i = self.0.pkt_cursor().last().0;
        Stamp::new(i)
    }
    pub fn pkts_after(&self, after: Stamp) -> impl Iterator<Item = RecvPktPtr> {
        self.local_pkt_log(Stamp::new(after.get() + 1))
    }

    pub fn log_range(&self, q: StampRange) -> impl Iterator<Item = RecvPktPtr> {
        let dir = IterDirection::from(q.start, q.end);
        if dir.is_forward() {
            Either::Left(
                self.0
                    .pkt_cursor()
                    .range_uniq(&q.start)
                    .map(as_recv_ptr),
            )
        } else {
            Either::Right(
                self.0
                    .pkt_cursor()
                    .range_uniq_rev(&q.start)
                    .map(as_recv_ptr),
            )
        }
    }

    pub fn local_pkt_log(&self, from: Stamp) -> impl Iterator<Item = RecvPktPtr> {
        tracing::trace!(%from,"getting packets after");
        self.0
            .pkt_cursor()
            .range_uniq(&from.get())
            .map(as_recv_ptr)
    }
    pub fn get_pkts_by_logidx(
        &self,
        idx: impl Iterator<Item = Stamp>,
    ) -> impl Iterator<Item = &NetPktPtr> {
        let c = self.0.pkt_cursor();
        idx.filter_map(move |p| c.read_uniq(&p.get()).ok().flatten())
            .map(as_netpkt)
    }
    pub fn partial_hashes_entries(
        &self,
        starts_with: PartialHash,
    ) -> impl Iterator<Item = RecvPktPtr> {
        let c1 = self.0.pkt_cursor();
        self.partial_hashes(starts_with).map(move |(_, ptr)| {
            read_pkt(&c1, Stamp::new(ptr))
                .expect("BTree Is inconsistent")
                .expect("BTree Is inconsistent")
        })
    }
    pub fn uniq_partial(
        &self,
        starts_with: PartialHash,
    ) -> Option<std::result::Result<RecvPktPtr, Vec<LkHash>>> {
        let mut it = self.partial_hashes_entries(starts_with);
        match it.next() {
            None => None,
            Some(first) => match it.next() {
                Some(sec) => {
                    let mut lst = vec![first.hash(), sec.hash()];
                    lst.extend(it.map(|p| p.hash()));
                    Some(Err(lst))
                }
                None => Some(Ok(first)),
            },
        }
    }
    pub fn partial_hashes(&self, starts_with: PartialHash) -> impl Iterator<Item = (&LkHash, u64)> {
        let _g = tracing::trace_span!("Partial Matching",partial=%starts_with.0.as_str()).entered();
        let start = starts_with.aprox_btree_idx();
        tracing::trace!(start=?start, start_b64=%starts_with, "start at");
        self.0
            .hash_cursor()
            .range_uniq(&start)
            .map(|(k, v)| (k, v, base64(k as &[u8])))
            .skip_while(move |(_, _, b64)| {
                let skip = b64.as_str() < starts_with.0.as_str();
                tracing::trace!(skip=?skip,b64=%b64);
                skip
            })
            .take_while(move |(_, _, b64)| {
                let cont = b64.starts_with(starts_with.0.as_str());
                tracing::trace!(whiel=%cont,b64=%b64);
                cont
            })
            .map(|(h, idx, _)| (B64::from_ref(h), idx))
    }
}
