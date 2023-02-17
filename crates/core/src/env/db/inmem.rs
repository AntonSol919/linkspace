// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(unused_imports,dead_code)]
/// Reference implementation!
/// A better inmem storage uses Arcs instead of pointers
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, RwLock, RwLockWriteGuard, Weak, RwLockReadGuard}, borrow::BorrowMut, mem::size_of,
};
use either::Either;
use rpds::{ListSync, QueueSync, RedBlackTreeMapSync};
use linkspace_pkt::{Stamp, NetPktPtr};

use crate::{env::{tree_key::TreeValueBytes, write_result::WriteResult}, consts::{PUBLIC_GROUP_PKT, PUBLIC_GROUP_PKT_PTR, PUBLIC_GROUP_PKT_BYTES}};

use super::{IterDirection,  assert_align, Refreshable, Cursors};

type PktLog = RedBlackTreeMapSync<u64, Vec<u8>>;
type HashDB = RedBlackTreeMapSync<[u8; 32], u64>;
type TreeDB = RedBlackTreeMapSync<Vec<u8>, Vec<TreeValueBytes>>;

#[derive(Clone)]
pub struct RawBTreeEnv(Arc<ImDb>);

use thiserror::*;

#[derive(Error, Debug,Copy,Clone)]
pub enum Error {}

pub type WriteTxn<'o> = Writer<'o>;
pub struct MutPktCursor<'o>(&'o mut PktLog);
pub struct MutHashCursor<'o>(&'o mut HashDB);
pub struct MutTreeCursor<'o>(&'o mut TreeDB);

pub(crate) type ReadTxn = Reader;
pub struct PktLogCursor<'o>(&'o PktLog);
pub struct HashCursor<'o>(&'o HashDB);
pub struct TreeCursor<'o>(&'o TreeDB);


pub fn open(_ignored: impl AsRef<std::path::Path>) -> anyhow::Result<RawBTreeEnv>{
    tracing::info!("Using INMEM db");
    let db = RawBTreeEnv(Arc::new(ImDb {
        writer: Default::default(),
        reader: RwLock::new(Reader{dbs:Default::default(),env:Weak::new()}),
    }));
    db.0.reader.write().unwrap().env = Arc::downgrade(&db.0);
    Ok(db)
}
#[derive(Debug)]
struct ImDb {
    writer: RwLock<DBs>,
    reader: RwLock<Reader>,
}
#[derive(Debug)]
pub struct Writer<'a>(
    RwLockWriteGuard<'a, DBs>,
    &'a RwLock<Reader>,
);
#[derive(Clone,Debug,Default)]
pub struct DBs{
    pkts: PktLog,
    hash: HashDB,
    tree: TreeDB,
}
#[derive(Clone,Debug)]
pub struct Reader{
    dbs: DBs,
    env: Weak<ImDb>
}

impl<'txn> PktLogCursor<'txn> {
    pub(crate) fn last(&mut self) -> (u64,&'txn [u8]){
        self.0.last().map(|(i,v)|(*i,v.as_slice())).unwrap_or((0,PUBLIC_GROUP_PKT_BYTES.bytes()))
    }
    pub(crate) fn range_uniq(&mut self, start: &u64) ->  impl Iterator<Item = (u64, &'txn [u8])>{
        let start= *start;
        self.0.range(start..).map(|(k,pkt)| (*k,pkt.as_slice()))
    }
    pub(crate) fn range_uniq_rev(&mut self, start: &u64) ->  impl Iterator<Item = (u64, &'txn [u8])>{
        let start= *start;
        self.0.range(start..).rev().map(|(k,pkt)| (*k,pkt.as_slice()))
    }
    pub(crate) fn read_uniq(&self, idx: &u64) -> Result<Option<&'txn [u8]>,Error> {
        Ok(self.0.get(&idx).map(|v| v.as_slice()))
    }
}

impl<'txn> HashCursor<'txn> {
    pub(crate) fn range_uniq(&mut self, start: &[u8; 32]) ->  impl Iterator<Item = (&'txn [u8; 32],u64)>{
        self.0.range(*start..).map(|(k,v)|(k,*v))
    }

    pub(crate) fn read_uniq(&self, key: &[u8; 32]) -> Result<Option<u64>,Error> {
        Ok(self.0.get(key).cloned())
    }
}
impl<'txn> MutHashCursor<'txn>{
    pub(crate) fn try_put_unique(
        &mut self,
        key: &[u8; 32],
        pkt_idx:u64 
    ) -> Result<WriteResult<()>,Error> {
        if self.0.get(key).is_some() {
            return Ok(WriteResult::Old(()));
        }
        self.0.insert_mut(*key, pkt_idx);
        Ok(WriteResult::New(()))
    }
}
impl<'txn> MutPktCursor<'txn, >{
    pub(crate) fn last(&mut self) -> (u64,&[u8]){
        self.0.last().map(|(i,v)|(*i,v.as_slice())).unwrap_or((0,&[]))
    }
    pub(crate) fn insert(
        &mut self,
        idx: u64,
        len: usize,
        insert: impl FnOnce(&mut [u8]),
    ) -> Result<(),Error> 
    {
        let vec = into_vec(len, insert);
        assert_align(vec.as_slice());
        self.0.insert_mut(idx, vec);
        Ok(())
    }
}

impl<'txn> TreeCursor<'txn>{
    pub(crate) fn range_multi(self, start: &[u8], dir: IterDirection, uniq_keys: bool) -> impl Iterator<Item = (&'txn [u8], &'txn TreeValueBytes)>{
        let start = start.to_vec();
        match dir {
            IterDirection::Forwards => {
                let it = self.0.range(start..).flat_map(move |(key, vals)| {
                    vals.iter().map(|v| (key.as_slice(), v)).take(if uniq_keys {
                        1
                    } else {
                        vals.len()
                    })
                });
                Either::Left(it)
            }
            IterDirection::Backwards => {
                let it = self.0.range(..=start).rev().flat_map(move |(key, vals)| {
                    vals.iter()
                        .rev()
                        .map(|v| (key.as_slice(), v))
                        .take(if uniq_keys { 1 } else { vals.len() })
                });
                Either::Right(it)
            }
        }
    }

    pub(crate) fn read_next(&mut self, start: &[u8]) -> Result<Option<(&'txn [u8],&'txn TreeValueBytes)>,Error> {
        // TODO This is rather slow.
        Ok(self.0.range(start.to_vec()..)
            .flat_map(move |(key, vals)| vals.iter().map(|v| (key.as_slice(), v))).next())
    }
}


impl<'o> MutTreeCursor<'o>{
    pub(crate) fn put_append(&mut self, key: &[u8], val: &TreeValueBytes) -> Result<(),Error> {
        match self.0.get_mut(key) {
            Some(lst) => {
                lst.push(*val);
                lst.sort_unstable();
            }
            None => {
                self.0.insert_mut(key.to_vec(), vec![*val]);
            }
        };
        Ok(())
    }
}

impl<'o> Writer<'o> {
    pub(crate) fn last(&mut self) -> u64{ MutPktCursor(&mut self.0.deref_mut().pkts).last().0}
    pub(crate) fn mut_cursors(&mut self) -> (MutPktCursor,MutHashCursor,MutTreeCursor) {
        let DBs { pkts, hash, tree } = self.0.deref_mut();
        (MutPktCursor(pkts),MutHashCursor(hash), MutTreeCursor(tree))
    }
    pub fn reader(&self) -> Reader{
        self.1.read().unwrap().clone()
    }

    pub(crate) fn commit(&mut self) -> Result<(),Error> {
        let mut dest = self.1.write().unwrap();
        dest.dbs = (self.0).clone();
        Ok(())
    }
}

impl<'o> super::Cursors for Writer<'o>{
    fn hash_cursor(&self) -> HashCursor{
        HashCursor(&self.0.hash)
    }
    fn tree_cursor(&self) -> TreeCursor{
        TreeCursor(&self.0.tree)
    }
    fn pkt_cursor(&self) -> PktLogCursor{
        PktLogCursor(&self.0.pkts)
    }
}
impl super::Cursors for Reader {
    fn hash_cursor(&self) -> HashCursor{
        HashCursor(&self.dbs.hash)
    }
    fn tree_cursor(&self) -> TreeCursor{
        TreeCursor(&self.dbs.tree)
    }
    fn pkt_cursor(&self) -> PktLogCursor{
        PktLogCursor(&self.dbs.pkts)
    }
}
impl Refreshable for Reader {
    fn refresh(&mut self){
        let arc = Weak::upgrade(&self.env).unwrap();
        let r = arc.reader.read().unwrap();
        self.dbs = r.dbs.clone();
        self.env = r.env.clone();
    } 
}
impl Reader {
    pub(crate) fn last(&self) -> u64{ self.pkt_cursor().last().0}
}

impl RawBTreeEnv{
    pub fn location(&self) -> &str {
        "inmem"
    }
    pub fn uid(&self) -> u64 {
        0
    }
    pub(crate) fn write_txn(&self) -> Writer<'_> {
        Writer(self.0.writer.write().unwrap(), &self.0.reader)
    }
    pub(crate) fn read_txn(&self) -> Result<Reader,Error> {
        Ok(self.0.reader.read().unwrap().clone())
    }

}
fn into_vec(len: usize, fill: impl FnOnce(&mut [u8])) -> Vec<u8> {
    let mut v = vec![0; len];
    fill(&mut v);
    v
}



