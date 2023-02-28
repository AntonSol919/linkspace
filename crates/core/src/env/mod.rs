// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// the db is duck type compatible between inmem and lmdb
use crate::consts::PUBLIC_GROUP_PKT;
use linkspace_pkt::{NetPkt, Stamp };
use std::{
    fmt::Debug,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    sync::OnceLock,
};
use write_trait::save_pkt;

use self::queries::IReadTxn;
pub use self::{
    db::{Error, RawBTreeEnv, Refreshable, WriteTxn},
    queries::ReadTxn,
    write_trait::SWrite,
};

pub mod db;
pub mod tree_key;
//pub mod tree_query;
pub mod queries;
pub mod queries2;
pub mod query_mode;
pub mod write_result;
pub mod write_trait;

pub type BusCall = Arc<dyn Fn(Stamp) + Send + Sync + 'static>;
pub static BUS: OnceLock<BusCall> = OnceLock::new();
#[derive(Clone)]
pub struct BTreeEnv {
    inner: RawBTreeEnv,
    location: Arc<PathBuf>,
    pub log_head: Arc<bus::ProcBus>,
}
impl Debug for BTreeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BTreeEnv").finish()
    }
}
pub use bus::ProcBus;

impl BTreeEnv {
    pub fn location(&self) -> &Path {
        &self.location
    }
    pub fn open(path: PathBuf, make_dir: bool) -> io::Result<BTreeEnv> {
        let inner = db::open(&path, make_dir)?;
        let location = Arc::new(path.canonicalize()?);
        tracing::debug!(?location, "Opening BTreeEnv");
        let log_head = bus::ProcBus::new(inner.uid());
        let env = BTreeEnv {
            inner,
            log_head: Arc::new(log_head),
            location,
        };
        {
            let mut writer = env.inner.write_txn()?;
            save_pkt(&mut writer, &**PUBLIC_GROUP_PKT)?;
        }
        Ok(env)
    }

    pub fn conf_data(&self, id: &str) -> io::Result<Vec<u8>> {
        let p = std::path::Path::new(id);
        if !p.is_absolute() {
            return Err(std::io::Error::other("only absolute paths allowed"));
        }
        let path = self.location().join(p);
        std::fs::read(path)
    }
    #[track_caller]
    pub fn get_reader(&self) -> io::Result<ReadTxn> {
        let btree_txn = self.inner.read_txn()?;
        Ok(ReadTxn(IReadTxn::new(btree_txn)))
    }
    pub fn get_writer(&self) -> io::Result<WriteTxn2> {
        tracing::trace!("Open write txn");
        Ok(WriteTxn2 {
            txn: Some(self.inner.write_txn()?),
            update: &self.log_head,
            last: None,
        })
    }

    pub async fn log_head(&self) -> Stamp {
        let v = self.log_head.next_async().await;
        Stamp::new(v)
    }
}
/// TODO fix name
/// Needs to broadcast updates and expose a ReadTxn ref
pub struct WriteTxn2<'o> {
    txn: Option<db::WriteTxn<'o>>,
    update: &'o bus::ProcBus,
    last: Option<Stamp>,
}

impl<'o> WriteTxn2<'o> {
    pub fn reader(&self) -> queries::IReadTxn<&(impl db::Cursors + '_)> {
        tracing::debug!("Peek reader of write txn");
        IReadTxn::new(self.txn.as_ref().unwrap())
    }
    fn set_last(
        &mut self,
        result: io::Result<(usize, Option<Stamp>)>,
    ) -> io::Result<(usize, Option<Stamp>)> {
        if let Ok((_writes, Some(last_writes))) = &result {
            self.last = Some(*last_writes);
        }
        result
    }
}
impl<'txn> Drop for WriteTxn2<'txn> {
    fn drop(&mut self) {
        std::mem::drop(self.txn.take());
        if let Some(last) = self.last {
            let _ = self.update.emit(last.get());
        }
    }
}

impl<'txn> SWrite for WriteTxn2<'txn> {
    fn write_many_state<'o>(
        &mut self,
        pkts: &'o mut dyn Iterator<Item = &'o dyn NetPkt>,
        out: Option<&'o mut dyn FnMut(&'o dyn NetPkt, bool) -> Result<bool, ()>>,
    ) -> io::Result<(usize, Option<Stamp>)> {
        let r = self.txn.as_mut().unwrap().write_many_state(pkts, out);
        self.set_last(r)
    }
}
