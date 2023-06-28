// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.




use ::lmdb::{self, *};
use ffi::MDB_NEXT_NODUP;
use lmdb_sys as ffi;
use std::io;
use std::io::{ErrorKind, Result};
use std::{fmt::Debug, io::Write, marker::PhantomData, path::Path };

use crate::{ prelude::TreeValueBytes};

pub use lmdb::Error;

use super::misc::{  assert_align  };

#[cfg(target_pointer_width = "32")]
const DEFAULT_MAP_SIZE: usize = 2usize.pow(31) - 4;
#[cfg(not(target_pointer_width = "32"))]
const DEFAULT_MAP_SIZE: usize = 2usize.pow(31) * 128;


fn open_env(path: &Path, flags: EnvironmentFlags) -> Environment {
    let mut err = Ok(());
    let mapsize : Option<usize> = std::env::var("LK_LMDB_MAPSIZE").ok().map(|v| v.parse()).transpose().expect("can't parse LK_LMDB_MAPSIZE");

    if mapsize.is_none(){
        if let Ok(env) = Environment::new().set_max_dbs(4).set_flags(flags).set_map_size(DEFAULT_MAP_SIZE).open(path){
            return env;
        }
    }

    for i in 0..5 {
        let mut env = Environment::new();
        env.set_max_dbs(4).set_flags(flags);
        if let Some(ms) = mapsize {
            tracing::info!("{path:?} setting mapsize {ms}");
            env.set_map_size(ms);
        }
        match env.open(path){
            Ok(env) => return env,
            Err(e) => { err = Err(e)},
        };
        let os_err = std::io::Error::last_os_error();
        if os_err.kind() == ErrorKind::OutOfMemory {
            panic!("{os_err:?} (maybe set LK_LMDB_MAPSIZE)");
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

pub(crate) fn open(path: &Path, make_dir: bool) -> std::io::Result<LMDBEnv> {
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
    let env = open_env(
        path,
        EnvironmentFlags::empty() | EnvironmentFlags::WRITE_MAP | EnvironmentFlags::NO_TLS,
    );
    let pktlog = env
        .create_db(Some("pktlog"), pktlog::PKTLOG_FLAGS)
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
    Ok(LMDBEnv {
        pktlog,
        tree,
        hash,
        env,
        uid,
    })
}

impl LMDBEnv{
    pub(crate) fn read_txn(&self) -> Result<LMDBTxn> {
        let txn = self.env.begin_ro_txn().map_err(as_io)?;
        Ok(LMDBTxn { txn,env:self})
    }


}
pub(crate) struct LMDBEnv {
    pub(crate) env: lmdb::Environment,
    pub(crate) uid: u64,
    pub(crate) pktlog: Database,
    pub(crate) tree: Database,
    pub(crate) hash: Database,
}
pub struct LMDBTxn<'env> {
    pub(crate) txn: RoTransaction<'env>,
    pub(crate) env: &'env LMDBEnv
}
pub struct MultiCursor<'o, A>(A, PhantomData<&'o ()>);
pub struct UniqCursor<'o, K, A>(A, PhantomData<(K, &'o ())>, &'static str);

pub type HashCursor<'o> = UniqCursor<'o, [u8; 32], RoCursor<'o>>;
pub type PktLogCursor<'o> = UniqCursor<'o, u64, RoCursor<'o>>;
pub type TreeCursor<'o> = MultiCursor<'o, RoCursor<'o>>;



impl<'env> LMDBTxn<'env> {

    pub fn pkt_cursor(&self) -> PktLogCursor {
        let cur = self.txn.open_ro_cursor(self.env.pktlog).unwrap();
        UniqCursor(cur,PhantomData,"pktlog")
    }
    pub fn tree_cursor(&self) -> TreeCursor {
        let cur = self.txn.open_ro_cursor(self.env.tree).unwrap();
        MultiCursor(cur,PhantomData)
    }
    pub fn hash_cursor(&self) -> HashCursor {
        let cur = self.txn.open_ro_cursor(self.env.hash).unwrap();
        UniqCursor(cur,PhantomData,"hash")
    }
}
impl<'o> LMDBTxn<'o>{
    pub fn refresh(self) -> lmdb::Result<LMDBTxn<'o>>{
        tracing::trace!("Refresh");
        let LMDBTxn { txn, env } = self;
        Ok(LMDBTxn{txn: txn.reset().renew()?,env})
    }
    pub fn refresh_inplace(&mut self) -> lmdb::Result<()>{
        let txn = self.txn.txn();
        let result  = unsafe {
            lmdb_sys::mdb_txn_reset(txn);
            lmdb_sys::mdb_txn_renew(txn)
        };
        lmdb::lmdb_result(result)
    }
}

#[cfg(target_pointer_width = "64")]
pub mod pktlog{
    pub const PKTLOG_FLAGS : lmdb::DatabaseFlags = lmdb::DatabaseFlags::INTEGER_KEY;
    pub fn bytes(b:u64) -> [u8;8]{ b.to_ne_bytes()}
    pub fn val(v:[u8;8]) -> u64 {u64::from_ne_bytes(v)}
}
#[cfg(not(target_pointer_width = "64"))]
pub mod pktlog{
    pub const PKTLOG_FLAGS : lmdb::DatabaseFlags = lmdb::DatabaseFlags::empty();
    pub fn bytes(b:u64) -> [u8;8]{ b.to_be_bytes()}
    pub fn val(v:[u8;8]) -> u64{u64::from_be_bytes(v)}
}

impl<'txn> PktLogCursor<'txn> {
    pub(crate) fn range_uniq(self, start: &u64) -> impl Iterator<Item = (u64, &'txn [u8])> {
        let c = self.0.iter_from(pktlog::bytes(*start));
        c.map(move |kv| {
            let (k, v) = kv.unwrap();
            let k = match k.try_into() {
                Ok(k) => pktlog::val(k),
                _ => panic!("bug: lmdb dsync? ( cursors outlived iter?)"),
            };
            let v = assert_align(v);
            (k, v)
        })
    }
    pub(crate) fn range_uniq_rev(self, start: &u64) -> impl Iterator<Item = (u64, &'txn [u8])> {

        let start = *start;
        let it = match self.0.get(Some(&pktlog::bytes(start)), None, ffi::MDB_LAST) {
            Ok(_) | Err(Error::NotFound) => Iter::Ok {
                cursor: self.0,
                op: ffi::MDB_GET_CURRENT,
                next_op: ffi::MDB_PREV,
            },
            Err(error) => Iter::Err(error),
        };
        it.map_while(|kv| kv.ok()).map(|(k, v)| {
            let k = pktlog::val(k.try_into().unwrap());
            let v = assert_align(v);
            (k, v)
        })
    }
    pub(crate) fn read_uniq(&self, key: &u64) -> Result<Option<&'txn [u8]>> {
        match self.0.get(Some(&pktlog::bytes(*key)), None, ffi::MDB_SET) {
            Err(lmdb::Error::NotFound) => Ok(None),
            Ok((_, v)) => Ok(Some(assert_align(v))),
            Err(e) => Err(as_io(e)),
        }
    }
    pub fn last(&self) -> (u64, &'txn [u8]) {
        match self.0.get(None, None, ffi::MDB_LAST) {
            Ok((Some(v), bytes)) => return (pktlog::val(v.try_into().unwrap()), bytes),
            Ok((None, _)) => tracing::trace!("Error getting last idx"),
            Err(Error::NotFound) => {}
            Err(e) => tracing::trace!(e=?e,"Error getting last idx"),
        };
        (0, &[])
    }
}
impl<'txn> HashCursor<'txn> {
    pub(crate) fn range_uniq(
        self,
        start: &[u8; 32],
    ) -> impl Iterator<Item = (&'txn [u8; 32], u64)> {
        let it = self.0.iter_from(start);
        it.map(move |kv| {
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
}
