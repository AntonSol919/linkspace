use std::{sync::{Arc, OnceLock}, path::{Path,PathBuf},fmt::Debug, io::{Write, self} };

use anyhow::Context;
pub use ipcbus::ProcBus;
use linkspace_pkt::{ Stamp, PUBLIC_GROUP_PKT, NetPkt, AB, NetPktExt };
use tracing::instrument;
use crate::{env::write_trait::save_pkt, LNS_ROOTS};

use self::{db::RawBTreeEnv, queries::{IReadTxn, ReadTxn}};

use super::write_trait::SWrite;


pub mod db;
pub mod misc;
pub mod tree_iter;
pub mod queries;
pub mod queries2;

/// A [NetPktPtr] and a recv stamp


pub type BusCall = Arc<dyn Fn(Stamp) + Send + Sync + 'static>;
pub static BUS: OnceLock<BusCall> = OnceLock::new();
#[derive(Clone)]
pub struct BTreeEnv {
    inner: RawBTreeEnv,
    location: Arc<PathBuf>,
    pub log_head: Arc<ipcbus::ProcBus>,
}
impl Debug for BTreeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BTreeEnv").finish()
    }
}
impl BTreeEnv {
    pub fn dir(&self) -> &Path {
        &self.location
    }
    pub fn open(path: PathBuf, make_dir: bool) -> io::Result<BTreeEnv> {
        let inner = db::open(&path, make_dir)?;
        let location = Arc::new(path.canonicalize()?);
        tracing::debug!(?location, "Opening BTreeEnv");
        let log_head = ipcbus::ProcBus::new(inner.uid());
        let env = BTreeEnv {
            inner,
            log_head: Arc::new(log_head),
            location,
        };
        {
            let mut writer = env.inner.write_txn()?;
            let new = save_pkt(&mut writer, &**PUBLIC_GROUP_PKT)?;
            if new && std::env::var_os("LK_NO_LNS").is_none(){
                let mut bytes = LNS_ROOTS;
                let roots:Vec<_> = std::iter::from_fn(||{
                    if bytes.is_empty() { return None;}
                    let pkt = crate::pkt::read::read_pkt(bytes, true).unwrap();
                    bytes = &bytes[pkt.size() as usize..];
                    Some(pkt)
                }).collect();
                let mut it = roots.iter().map(|p| p.as_ref() as &dyn NetPkt);
                let (_i,_) = writer.write_many_state(&mut it, None).unwrap();
            }
        }
        Ok(env)
    }

    pub fn local_enckey(&self) -> anyhow::Result<String> {
        // TODO this should prob check for read only access
        Ok(std::fs::read_to_string(self.location.join("local_auth"))?.lines().next().context("missing enckey")?.to_owned())
    }
    #[instrument(ret,skip(bytes))]
    pub fn set_files_data(&self, path: impl AsRef<std::path::Path>+std::fmt::Debug, bytes: &[u8],overwrite:bool) -> anyhow::Result<()>{
        tracing::trace!(bytes=%AB(bytes));
        let path = self.dir().join("files").join(check_path(path.as_ref())?);
        let r: anyhow::Result<()> = try {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let mut file = if overwrite {
                std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(&path)?
            }else {
                std::fs::OpenOptions::new().create_new(true).write(true).open(&path)?
            };
            file.write_all(bytes)?;
        };
        r.with_context(|| anyhow::anyhow!("Target {}",path.to_string_lossy()))
    }
    #[instrument(ret)]
    // notfound_err simplifies context errors
    pub fn files_data(&self, path: impl AsRef<std::path::Path>+std::fmt::Debug,notfound_err:bool) -> anyhow::Result<Option<Vec<u8>>> {
        let path = self.dir().join("files").join(check_path(path.as_ref())?);
        use std::io::ErrorKind::*;
        match std::fs::read(&path){
            Ok(k) => Ok(Some(k)),
            Err(e) if !notfound_err && e.kind() == NotFound =>Ok(None),
            Err(e) => Err(e).with_context(|| anyhow::anyhow!("could not open {}",path.to_string_lossy()))
        }
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
    update: &'o ipcbus::ProcBus,
    last: Option<Stamp>,
}

impl<'o> WriteTxn2<'o> {
    pub fn reader(&self) -> queries::IReadTxn<&(impl misc::Cursors + '_)> {
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

fn check_path(path:&std::path::Path) -> anyhow::Result<&std::path::Path>{
    if let Some(c) = path.components().find(|v| !matches!(v,std::path::Component::Normal(_))){
        anyhow::bail!("path can not contain a {c:?} component")
    }
    Ok(path)
}
