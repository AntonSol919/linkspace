// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/*
lmdb-rs is broken. All cursor methods have an incorrect lifetime.
Make sure you capture a cursor on use
*/

use ::lmdb::{self, *};
use ffi::MDB_NEXT_NODUP;
use lmdb_sys as ffi;
use std::io;
use std::io::{ErrorKind, Result};
use std::{fmt::Debug, io::Write, marker::PhantomData, mem::size_of, path::Path, sync::Arc};

use crate::{env::write_result::WriteResult, prelude::TreeValueBytes};

#[derive(Clone)]
pub struct RawBTreeEnv(Arc<LMDBEnv>);
pub use lmdb::Error;

use super::misc::{ Refreshable, assert_align,  Cursors};

pub type WriteTxn<'o> = LMDBTxn<lmdb::RwTransaction<'o>>;
pub type MutHashCursor<'o> = UniqCursor<'o, [u8; 32], RwCursor<'o>>;
pub type MutTreeCursor<'o> = MultiCursor<'o, RwCursor<'o>>;
pub type MutPktCursor<'o> = UniqCursor<'o, u64, RwCursor<'o>>;

pub type ReadTxn = LMDBTxn<lmdb::RoTransaction<'static>>;
pub type HashCursor<'o> = UniqCursor<'o, [u8; 32], RoCursor<'o>>;
pub type PktLogCursor<'o> = UniqCursor<'o, u64, RoCursor<'o>>;
pub type TreeCursor<'o> = MultiCursor<'o, RoCursor<'o>>;

/*
impl<'o,A:Cursor<'o>> Drop for MultiCursor<'o, A>{
    fn drop(&mut self) {
        tracing::trace!("Dropping multicursor {}",self.0.inspect());
    }
}
impl<'o,K,A:Cursor<'o>> Drop for UniqCursor<'o,K,A>{
    fn drop(&mut self) {
        tracing::trace!("Dropping {} Cursor{}",self.2,self.0.inspect());
    }
}
*/

#[cfg(target_pointer_width = "32")]
const DEFAULT_MAP_SIZE: usize = 2usize.pow(31) - 4;
#[cfg(not(target_pointer_width = "32"))]
const DEFAULT_MAP_SIZE: usize = 2usize.pow(31) * 128;
fn open_env(path: &Path, mut mapsize: usize, flags: EnvironmentFlags) -> Environment {
    let mut err = Ok(());
    if cfg!(target_pointer_width = "32") && mapsize > 2usize.pow(31) - 4 {
        tracing::warn!("lmdb on 32-bit is capped at {DEFAULT_MAP_SIZE} ( 2^31 )");
        mapsize = DEFAULT_MAP_SIZE;
    }
    for i in 0..5 {
        err = match Environment::new()
            .set_max_dbs(4)
            .set_flags(flags)
            .set_map_size(mapsize)
            .open(path)
        {
            Ok(env) => return env,
            Err(e) => Err(e),
        };
        let os_err = std::io::Error::last_os_error();
        if os_err.kind() == ErrorKind::OutOfMemory {
            panic!("{os_err:?}\nLK_LMDB_MAPSIZE={mapsize}");
        }
        tracing::warn!(?i, ?err, ?os_err, "DB Open");
        if i == 5 {
            tracing::error!(?i, ?err, ?os_err, "DB Open");
        }
        std::thread::sleep(std::time::Duration::from_millis(50 + 200 * i));
    }
    let err = err.unwrap_err();
    let error_str = if let Error::Other(16) = err {
        format!("Error opening {path:?} {err:?} ( A process can not have multiple connections to the same database)")
    } else {
        format!("Error opening {path:?} {err:?}")
    };
    if cfg!(target = "unix") {
        tracing::error!("Checking lock");
        let o = std::process::Command::new("lsof")
            .current_dir(path)
            .args(["+D", "./"])
            .spawn()
            .expect(&error_str)
            .wait_with_output();
        println!("{o:?}");
    }
    panic!("{} \n {:?}", error_str, std::io::Error::last_os_error());
}

pub fn as_io(e: lmdb::Error) -> std::io::Error {
    if let Error::Other(i) = e {
        io::Error::from_raw_os_error(i)
    } else {
        io::Error::other(e)
    }
}

pub fn open(path: &Path, make_dir: bool) -> std::io::Result<RawBTreeEnv> {
    tracing::trace!(?path,make_dir,"open db");
    path.as_os_str().to_str().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Path must be valid utf8",
    ))?; // not really but it makes some api's easier
    if make_dir {
        std::fs::create_dir_all(path)?
    };
    if let Ok(mut f) = std::fs::File::create_new(path.join("type")) {
        f.write_all(b"lmdb")?;
        f.flush()?;
    }
    if std::fs::read(path.join("type"))? != b"lmdb" {
        return Err(io::Error::other("db type mismatch"));
    }
    let idfile = path.join("id");
    if let Ok(mut file) = std::fs::File::create_new(&idfile) {
        let id: [u8; 8] = linkspace_pkt::now().0;
        file.write_all(&id)?;
        file.flush()?;
    }
    let mapsize = std::env::var("LK_LMDB_MAPSIZE")
        .map(|v| v.parse().expect("LK_LMDB_MAPSIZE to be u32"))
        .unwrap_or(DEFAULT_MAP_SIZE);
    let env = Arc::new(open_env(
        path,
        mapsize,
        EnvironmentFlags::empty() | EnvironmentFlags::WRITE_MAP | EnvironmentFlags::NO_TLS,
    ));
    let primary = env
        .create_db(Some("pktlog"), DatabaseFlags::INTEGER_KEY)
        .unwrap();
    let hash = env.create_db(Some("hash"), DatabaseFlags::empty()).unwrap();
    let tree = env
        .create_db(
            Some("tree"),
            DatabaseFlags::DUP_SORT | DatabaseFlags::DUP_FIXED,
        )
        .unwrap();
    let uid: [u8; 8] = std::fs::read(idfile)
        .unwrap()
        .try_into()
        .expect("expect 8 bytes");
    let uid = u64::from_be_bytes(uid);
    Ok(RawBTreeEnv(Arc::new(LMDBEnv {
        primary,
        tree,
        hash,
        env,
        uid,
    })))
}

impl RawBTreeEnv {
    pub fn uid(&self) -> u64 {
        self.0.uid
    }
    pub(crate) fn write_txn(&self) -> std::io::Result<WriteTxn> {
        let txn = self.0.env.begin_rw_txn().map_err(as_io)?;
        Ok(LMDBTxn {
            txn: Some(txn),
            env: self.clone(),
        })
    }

    pub(crate) fn read_txn(&self) -> Result<ReadTxn> {
        tracing::debug!("Get reader");
        let txn = unsafe { std::mem::transmute(Some(self.0.env.begin_ro_txn().map_err(as_io)?)) };
        Ok(LMDBTxn {
            txn,
            env: self.clone(),
        })
    }
}

struct LMDBEnv {
    env: Arc<lmdb::Environment>,
    uid: u64,
    primary: Database,
    tree: Database,
    hash: Database,
}
pub struct LMDBTxn<T> {
    txn: Option<T>,
    env: RawBTreeEnv,
}
pub struct MultiCursor<'o, A: Cursor<'o>>(A, PhantomData<&'o ()>);
impl<'o, A: Cursor<'o>> MultiCursor<'o, A> {
    fn new(a: A) -> MultiCursor<'o, A> {
        //tracing::trace!(p=%a.inspect(),"New treecur");
        MultiCursor(a, PhantomData)
    }
}

pub struct UniqCursor<'o, K, A: Cursor<'o>>(A, PhantomData<(K, &'o ())>, &'static str);
impl<'o, A: Cursor<'o>> UniqCursor<'o, [u8; 32], A> {
    fn new_hash(a: A) -> UniqCursor<'o, [u8; 32], A> {
        //tracing::trace!(p=%a.inspect(),"New Hash");
        UniqCursor(a, PhantomData, "hash")
    }
}
impl<'o, A: Cursor<'o>> UniqCursor<'o, u64, A> {
    fn new_pkt(a: A) -> UniqCursor<'o, u64, A> {
        //tracing::trace!(p=%a.inspect(),"New Pkt");
        UniqCursor(a, PhantomData, "pkt")
    }
}

impl<'e> WriteTxn<'e> {
    pub(crate) fn mut_cursors<'o>(
        &'o mut self,
    ) -> (MutPktCursor<'o>, MutHashCursor<'o>, MutTreeCursor<'o>) {
        let txn = self.txn.as_mut().unwrap();
        let [t1, t2, t3]: [&'o mut RwTransaction; 3] = unsafe {
            [
                &mut *(txn as *mut RwTransaction),
                &mut *(txn as *mut RwTransaction),
                &mut *(txn as *mut RwTransaction),
            ]
        };
        let primary: RwCursor<'o> = t1.open_rw_cursor(self.env.0.primary).unwrap();
        let hash: RwCursor<'o> = t2.open_rw_cursor(self.env.0.hash).unwrap();
        let tree: RwCursor<'o> = t3.open_rw_cursor(self.env.0.tree).unwrap();
        (
            UniqCursor::new_pkt(primary),
            UniqCursor::new_hash(hash),
            MultiCursor::new(tree),
        )
    }

    pub(crate) fn commit(&mut self) -> Result<()> {
        tracing::trace!("commit txn");
        self.txn.take().unwrap().commit().map_err(as_io)?;
        self.txn =
            unsafe { std::mem::transmute(Some(self.env.0.env.begin_rw_txn().map_err(as_io)?)) };
        Ok(())
    }
}
impl<'o, TXN: 'o + lmdb::Transaction> Cursors for LMDBTxn<TXN> {
    fn pkt_cursor(&self) -> PktLogCursor {
        UniqCursor::new_pkt(
            self.txn
                .as_ref()
                .unwrap()
                .open_ro_cursor(self.env.0.primary)
                .unwrap(),
        )
    }
    fn tree_cursor(&self) -> TreeCursor {
        MultiCursor::new(
            self.txn
                .as_ref()
                .unwrap()
                .open_ro_cursor(self.env.0.tree)
                .unwrap(),
        )
    }
    fn hash_cursor(&self) -> HashCursor {
        UniqCursor::new_hash(
            self.txn
                .as_ref()
                .unwrap()
                .open_ro_cursor(self.env.0.hash)
                .unwrap(),
        )
    }
}
impl Refreshable for ReadTxn {
    fn refresh(&mut self) {
        tracing::trace!("Refresh");
        self.txn = Some(self.txn.take().unwrap().reset().renew().unwrap());
    }
}

impl<'txn> PktLogCursor<'txn> {
    pub(crate) fn range_uniq(mut self, start: &u64) -> impl Iterator<Item = (u64, &'txn [u8])> {
        let c = self.0.iter_from(start.to_ne_bytes());
        c.map(move |kv| {
            let _tmp = self.0.cursor(); // We MUST capture self to work around lmdb-rs soundess bug
            let (k, v) = kv.unwrap();
            let k = match k.try_into() {
                Ok(k) => u64::from_ne_bytes(k),
                _ => panic!("bug: lmdb dsync? ( cursors outlived iter?)"),
            };
            let v = assert_align(v);
            (k, v)
        })
    }
    pub(crate) fn range_uniq_rev(self, start: &u64) -> impl Iterator<Item = (u64, &'txn [u8])> {
        let start = *start;
        let it = match self.0.get(Some(&start.to_ne_bytes()), None, ffi::MDB_LAST) {
            Ok(_) | Err(Error::NotFound) => Iter::Ok {
                cursor: self.0.cursor(),
                op: ffi::MDB_GET_CURRENT,
                next_op: ffi::MDB_PREV,
                _marker: PhantomData,
            },
            Err(error) => Iter::Err(error),
        };
        it.map_while(|kv| kv.ok()).map(move |(k, v)| {
            let _tmp = self.0.cursor(); // We MUST capture self to work around lmdb-rs soundess bug
            let k = u64::from_ne_bytes(k.try_into().unwrap());
            let v = assert_align(v);
            (k, v)
        })
    }
    pub(crate) fn read_uniq(&self, key: &u64) -> Result<Option<&'txn [u8]>> {
        match self.0.get(Some(&key.to_ne_bytes()), None, ffi::MDB_SET) {
            Err(lmdb::Error::NotFound) => Ok(None),
            Ok((_, v)) => Ok(Some(assert_align(v))),
            Err(e) => Err(as_io(e)),
        }
    }
    pub fn last(&self) -> (u64, &'txn [u8]) {
        match self.0.get(None, None, ffi::MDB_LAST) {
            Ok((Some(v), bytes)) => return (u64::from_ne_bytes(v.try_into().unwrap()), bytes),
            Ok((None, _)) => tracing::trace!("Error getting last idx"),
            Err(Error::NotFound) => {}
            Err(e) => tracing::trace!(e=?e,"Error getting last idx"),
        };
        (0, &[])
    }
}
impl<'txn> HashCursor<'txn> {
    pub(crate) fn range_uniq(
        mut self,
        start: &[u8; 32],
    ) -> impl Iterator<Item = (&'txn [u8; 32], u64)> {
        let it = self.0.iter_from(start);
        it.map(move |kv| {
            let _tmp = self.0.cursor(); // We MUST capture self to work around lmdb-rs soundess bug
            let (k, v) = kv.unwrap();
            let v = u64::from_ne_bytes(v.try_into().unwrap());
            (k.try_into().unwrap(), v)
        })
    }
    pub(crate) fn read_uniq(&self, key: &[u8; 32]) -> Result<Option<u64>> {
        match self.0.get(Some(key), None, ffi::MDB_SET) {
            Err(lmdb::Error::NotFound) => Ok(None),
            Ok((_, v)) => Ok(Some(u64::from_ne_bytes(v.try_into().unwrap()))),
            Err(e) => Err(as_io(e)),
        }
    }
}

pub struct IterDup<'txn> {
    cur: RoCursor<'txn>,
    value_asc: bool,
}
impl<'o> Debug for IterDup<'o> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IterDup").finish()
    }
}
type E<'txn> = (&'txn [u8], &'txn TreeValueBytes);
impl<'txn> IterDup<'txn> {
    #[track_caller]
    fn iget(&self, key: Option<&[u8]>, op: u32) -> Option<E<'txn>> {
        match self.cur.get(key, None, op) {
            Ok((Some(k), val)) => Some((k, val.try_into().unwrap())),
            Err(Error::NotFound) => None,
            e => panic!("{e:?}"),
        }
    }
    pub fn set_range(&self, start: &[u8]) -> Option<E<'txn>> {
        if self.value_asc {
            self.iget(Some(start), lmdb_sys::MDB_SET_RANGE)
        } else {
            let (k, _) = self.iget(Some(start), lmdb_sys::MDB_SET_RANGE)?;
            let (_, v) = self.cur.get(None, None, lmdb_sys::MDB_LAST_DUP).unwrap();
            Some((k, v.try_into().unwrap()))
        }
    }
    pub fn get_next_entry(&self) -> Option<E<'txn>> {
        self.iget(
            None,
            if self.value_asc {
                lmdb_sys::MDB_NEXT_DUP
            } else {
                lmdb_sys::MDB_PREV_DUP
            },
        )
    }
    pub fn get_next_range(&self) -> Option<E<'txn>> {
        if self.value_asc {
            self.iget(None, MDB_NEXT_NODUP)
        } else {
            let (k, _) = self.iget(None, MDB_NEXT_NODUP)?;
            let (_, v) = self.cur.get(None, None, lmdb_sys::MDB_LAST_DUP).unwrap();
            Some((k, v.try_into().unwrap()))
        }
    }
    pub fn get_current(&self) -> Option<E<'txn>> {
        self.iget(None, lmdb_sys::MDB_GET_CURRENT)
    }
}

impl<'txn> TreeCursor<'txn> {
    pub fn iter_dup(self, value_asc: bool) -> IterDup<'txn> {
        IterDup {
            cur: self.0,
            value_asc,
        }
    }

    /*
    pub(crate) fn range_multi(
        self,
        start: &[u8],
        dir: IterDirection,
        uniq_keys: bool,
    ) -> impl Iterator<Item = (&'txn [u8], &'txn [u8; size_of::<TreeValueBytes>()])> {
        let mut op = ffi::MDB_GET_CURRENT;
        match self.0.get(Some(start), None, ffi::MDB_SET_RANGE) {
            Ok(_) if dir == IterDirection::Backwards => {
                // This is prob suboptimal, but we have to check if the cursor is exactly at this key and jump to the back if true.
                if let Ok((Some(key), _)) = self.0.get(None, None, ffi::MDB_GET_CURRENT) {
                    if key == start {
                        let _s = self.0.get(None, None, ffi::MDB_LAST_DUP);
                    } else if let Err(Error::NotFound) = self.0.get(None, None, ffi::MDB_PREV) {
                        op = ffi::MDB_PREV; // if at the first key make sure 'next' also returns not found
                    }
                }
            }
            Ok(_) => {}
            Err(Error::NotFound) => (),
            Err(_error) => todo!(),
        };
        let next = match (dir, uniq_keys) {
            (IterDirection::Forwards, true) => ffi::MDB_NEXT_NODUP,
            (IterDirection::Forwards, false) => ffi::MDB_NEXT,
            (IterDirection::Backwards, true) => ffi::MDB_PREV_NODUP,
            (IterDirection::Backwards, false) => ffi::MDB_PREV,
        };
        let mut i = 0;
        std::iter::from_fn(move || {
            if op == ffi::MDB_GET_CURRENT {
                let first = self.0.get(None, None, ffi::MDB_GET_CURRENT);
                tracing::trace!(i,r=?first,"first");
                op = next;
                if let Ok((Some(key), val)) = first {
                    i += 1;
                    return Some((key, val.try_into().unwrap()));
                }
            }
            let r = self.0.get(None, None, op);
            i += 1;
            match r {
                Err(Error::NotFound) => None,
                Err(e) => {
                    tracing::error!("LMDB ERROR!!! {:?}", e);
                    None
                }
                Ok((None, _v)) => panic!(),
                Ok((Some(key), val)) => {
                    //tracing::trace!(i,key=?key,"next");
                    match val.try_into() {
                        Ok(v) => Some((key, v)),
                        Err(e) => panic!("Cast error? {val:?} {e:?}"),
                    }
                }
            }
        })
    }

    pub(crate) fn read_next(
        &mut self,
        start: &[u8],
    ) -> Result<Option<(&'txn [u8], &'txn [u8; size_of::<TreeValueBytes>()])>> {
        match self.0.get(Some(start), None, lmdb_sys::MDB_SET_RANGE) {
            Ok((Some(key), val)) => Ok(Some((key, val.try_into().unwrap()))),
            Err(Error::NotFound) => Ok(None),
            e => todo!("{:?}", e),
        }
    }
    */
}

impl<'txn> MutPktCursor<'txn> {
    pub(crate) fn last(&mut self) -> (u64, &'txn [u8]) {
        match self.0.get(None, None, ffi::MDB_LAST) {
            Ok((Some(v), bytes)) => return (u64::from_ne_bytes(v.try_into().unwrap()), bytes),
            Ok((None, _)) => tracing::error!("Error getting last idx"),
            Err(Error::NotFound) => {}
            Err(e) => tracing::error!(e=?e,"Error getting last idx"),
        };
        (0, &[])
    }
    pub(crate) fn insert(
        &mut self,
        idx: u64,
        len: usize,
        insert: impl FnOnce(&mut [u8]),
    ) -> Result<()> {
        let cur = self.0.cursor();
        use std::ptr;
        let len = ((len + 3) / 4) * 4; // proper allignment
        let key = idx.to_ne_bytes();
        tracing::trace!(idx=?key,"Write new Pkt");
        let mut key_val: ffi::MDB_val = ffi::MDB_val {
            mv_size: key.len() as libc::size_t,
            mv_data: key.as_ptr() as *mut libc::c_void,
        };
        let mut data_val: ffi::MDB_val = ffi::MDB_val {
            mv_size: len,
            mv_data: ptr::null_mut::<libc::c_void>(),
        };

        let r = unsafe {
            ffi::mdb_cursor_put(
                cur,
                &mut key_val,
                &mut data_val,
                ffi::MDB_NODUPDATA | ffi::MDB_NOOVERWRITE | ffi::MDB_RESERVE | ffi::MDB_APPEND,
            )
        };
        match r {
            ffi::MDB_SUCCESS => {
                let dest = unsafe {
                    std::slice::from_raw_parts_mut(data_val.mv_data as *mut u8, data_val.mv_size)
                };
                insert(dest);
                Ok(())
            }
            ffi::MDB_KEYEXIST => {
                panic!("insert logic error")
            }
            e => Err(as_io(Error::from_err_code(e))),
        }
    }
}
impl<'txn> MutHashCursor<'txn> {
    pub(crate) fn try_put_unique(
        &mut self,
        key: &[u8; 32],
        pkt_idx: u64,
    ) -> Result<WriteResult<()>> {
        match self.0.put(
            &key,
            &pkt_idx.to_ne_bytes(),
            WriteFlags::NO_DUP_DATA | WriteFlags::NO_OVERWRITE,
        ) {
            Ok(_) => {
                tracing::trace!("New Pkt");
                Ok(WriteResult::New(()))
            }
            Err(Error::KeyExist) => {
                tracing::trace!("OldPkt");
                Ok(WriteResult::Old(()))
            }
            Err(e) => Err(as_io(e)),
        }
    }
}

impl<'txn> MutTreeCursor<'txn> {
    pub(crate) fn put_append(
        &mut self,
        key: &[u8],
        val: &[u8; size_of::<TreeValueBytes>()],
    ) -> Result<()> {
        match self.0.put(&key, &val, WriteFlags::empty()) {
            Ok(_v) => Ok(()),
            Err(e) => Err(as_io(e)),
        }
    }
}
