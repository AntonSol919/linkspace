// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::io;
use std::io::Result;

use super::db::MutHashCursor;
use super::db::MutPktCursor;
use super::db::MutTreeCursor;
use super::db::PktLogCursor;
use super::misc::IterDirection;
use super::misc::Refreshable;
use super::db::WriteTxn;
use super::misc::assert_align;
use crate::env::RecvPktPtr;
use crate::env::tree_key::*;
use crate::env::write_result::WriteResult;
use crate::env::write_trait::SWrite;
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

pub struct ReadTxn(pub(crate) IReadTxn);
impl Refreshable for ReadTxn {
    fn refresh(&mut self) {
        self.0.refresh()
    }
}
impl std::ops::Deref for ReadTxn {
    type Target = IReadTxn;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Drop for ReadTxn {
    fn drop(&mut self) {
        tracing::trace!("Dropping read txn");
    }
}
pub struct IReadTxn<C = super::db::ReadTxn> {
    pub(crate) btree_txn: C,
}

impl<C: super::misc::Cursors> IReadTxn<C> {
    pub(crate) fn new(btree_txn: C) -> Self {
        tracing::trace!("Open txn");
        IReadTxn { btree_txn }
    }

    /// read a pkt and use the local net header
    pub fn read_ptr(&self, hash: &LkHash) -> Result<Option<Stamp>> {
        match self.btree_txn.hash_cursor().read_uniq(hash)? {
            Some(idx) => Ok(Some(Stamp::new(idx))),
            None => Ok(None),
        }
    }
    /// read a pkt and use the local net header
    pub fn read(&self, hash: &LkHash) -> Result<Option<RecvPktPtr>> {
        tracing::trace!(hash = ?hash, "Read hash");
        match self.read_ptr(hash)? {
            Some(idx) => read_pkt(&self.btree_txn.pkt_cursor(), idx),
            None => Ok(None),
        }
    }
    pub fn log_head(&self) -> Stamp {
        let i = self.btree_txn.pkt_cursor().last().0;
        Stamp::new(i)
    }
    pub fn pkts_after(&self, after: Stamp) -> impl Iterator<Item = RecvPktPtr> {
        self.local_pkt_log(Stamp::new(after.get() + 1))
    }

    pub fn log_range(&self, q: StampRange) -> impl Iterator<Item = RecvPktPtr> {
        let dir = IterDirection::from(q.start, q.end);
        if dir.is_forward() {
            Either::Left(
                self.btree_txn
                    .pkt_cursor()
                    .range_uniq(&q.start)
                    .map(as_recv_ptr),
            )
        } else {
            Either::Right(
                self.btree_txn
                    .pkt_cursor()
                    .range_uniq_rev(&q.start)
                    .map(as_recv_ptr),
            )
        }
    }

    pub fn local_pkt_log(&self, from: Stamp) -> impl Iterator<Item = RecvPktPtr> {
        self.btree_txn
            .pkt_cursor()
            .range_uniq(&from.get())
            .map(as_recv_ptr)
    }
    pub fn get_pkts_by_logidx(
        &self,
        idx: impl Iterator<Item = Stamp>,
    ) -> impl Iterator<Item = &NetPktPtr> {
        let c = self.btree_txn.pkt_cursor();
        idx.filter_map(move |p| c.read_uniq(&p.get()).ok().flatten())
            .map(as_netpkt)
    }
    /*
    fn fixed_size_prefix_iter<const N:usize>(&self,_start:[u8;N]) -> impl Iterator<Item = TreeEntryRef<'_>>{
        #[allow(unreachable_code)]
        std::iter::once(todo!())
        let mut c2 = self.btree_txn.tree_cursor();
        let mut prefix = Some(start);
        use crate::pkt::uint_native::u8_be;
        std::iter::from_fn(move || {
            match c2.read_next(&prefix?).expect("Read ok"){
                Some((k,val)) => {
                    let entry = TreeEntry {
                        val,
                        btree_key: TreeKey::new(k),
                    };
                    prefix = u8_be::add(k[0..N].try_into().unwrap(),u8_be::one());
                    Some(entry)
                },
                None => None,
            }
        })
    }
    pub fn iter_group_domains_entries(&self,start:GroupID) -> impl Iterator<Item = TreeEntryRef<'_>> {
        let mut it = start.into_bytes().into_iter().chain(Domain::default().into_iter());
        let start_bytes : [u8;40] =std::array::from_fn(|_| it.next().unwrap());
        self.fixed_size_prefix_iter(start_bytes).take_while(move |p| p.btree_key.group() == start)
    }
    pub fn iter_group_domains(&self, start:GroupID) -> impl Iterator<Item=Domain> +'_{
        self.iter_group_domains_entries(start).map(|e|e.btree_key.domain())
    }
    pub fn iter_groups_entries(&self,start: [u8;32]) -> impl Iterator<Item = TreeEntryRef<'_>> {
        self.fixed_size_prefix_iter(start)
    }
    pub fn iter_groups(&self) -> impl Iterator<Item=GroupID> +'_ {
        let start = [0;32];
        self.iter_groups_entries(start).map(|e| e.btree_key.group())
    }
     */

    pub fn partial_hashes_entries(
        &self,
        starts_with: PartialHash,
    ) -> impl Iterator<Item = RecvPktPtr> {
        let c1 = self.btree_txn.pkt_cursor();
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
        self.btree_txn
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

impl<X> IReadTxn<X>
where
    X: Refreshable,
{
    pub fn refresh(&mut self) {
        self.btree_txn.refresh()
    }
}

// TODO batch insertion can be optimized
pub fn insert<B: NetPkt + ?Sized>(
    (log, hash, spk): &mut (MutPktCursor, MutHashCursor, MutTreeCursor),
    idx: Stamp,
    pkt: &B,
) -> Result<WriteResult<()>> {
    let last_idx = { log.last().0 };
    if last_idx > idx.get() {
        todo!()
    }
    let result = hash.try_put_unique(&pkt.hash(), idx.get())?;
    if result.is_new() {
        log.insert(idx.get(), pkt.size() as usize , |dest| {
            assert_align(dest);
            let segments = pkt.byte_segments();
            unsafe { segments.write_segments_unchecked(dest.as_mut_ptr()) };
            if cfg!(debug_assertions){
                let pkt = unsafe { NetPktFatPtr::from_bytes_unchecked(dest) };
                pkt.check(false).unwrap();
            }
        })?;
        if let Some((key, val)) =
            TreeEntry::from_pkt(idx, pkt).map(|te| (te.btree_key.take(), te.val))
        {
            spk.put_append(&key, &val).unwrap();
        }
        return Ok(WriteResult::New(()));
    }
    Ok(WriteResult::Old(()))
}

impl<'x> WriteTxn<'x> {
    pub fn write_impl<'o, P: NetPkt + ?Sized + 'o>(
        &mut self,
        it: impl Iterator<Item = &'o P>,
        mut cb: impl FnMut(&'o P, bool) -> std::result::Result<bool, ()>,
    ) -> io::Result<(usize, Option<Stamp>)> {
        let mut new = 0;
        let _g = tracing::trace_span!("Start write").entered();
        let mut last = None;
        {
            let mut writer = self.mut_cursors();
            let mut now = now();
            for pkt in it {
                let is_new = match insert(&mut writer, now, pkt) {
                    Ok(r) => r.unref().is_new(),
                    Err(e) => return Err(e),
                };
                if is_new {
                    tracing::trace!(logidx=?now,hash=%pkt.hash(),"New Written");
                    new += 1;
                    now = Stamp::new(now.get() + 1);
                    last = Some(now);
                }
                match cb(pkt, is_new) {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(_) => return Ok((0, None)),
                }
            }
        }
        self.commit()?;
        Ok((new, last))
    }
}

impl<'x> SWrite for WriteTxn<'x> {
    fn write_many_state<'o>(
        &mut self,
        pkts: &'o mut dyn Iterator<Item = &'o dyn NetPkt>,
        out: Option<&'o mut dyn FnMut(&'o dyn NetPkt, bool) -> std::result::Result<bool, ()>>,
    ) -> io::Result<(usize, Option<Stamp>)> {
        self.write_impl(pkts, out.unwrap_or(&mut |_, _| Ok(true)))
    }
}
