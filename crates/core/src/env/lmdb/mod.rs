use std::{sync::{Arc }, path::{Path,PathBuf},fmt::Debug, io::{Write, self} };

use anyhow::Context;
pub use ipcbus::ProcBus;
use linkspace_pkt::{ Stamp, PUBLIC_GROUP_PKT, NetPkt, AB, NetPktExt, NetPktPtr };
use lmdb_sys::MDB_envinfo;
use tracing::instrument;
use crate::{LNS_ROOTS};

use self::{queries::{ ReadTxn}, save::SaveState, db::LMDBEnv, db_info::{DbInfo, LMDBVersion}};

pub mod db;
pub mod misc;
pub mod tree_iter;
pub mod queries;
pub mod queries2;
pub mod save;
pub mod db_info;

/// A [NetPktPtr] and a recv stamp


#[derive(Clone)]
pub struct BTreeEnv(pub Arc<Inner>);

pub struct Inner {
    lmdb: LMDBEnv,
    location: PathBuf,
    pub log_head: Arc<ipcbus::ProcBus>,
}
impl Debug for BTreeEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BTreeEnv").finish()
    }
}
impl BTreeEnv {
    
    pub fn dir(&self) -> &Path {
        &self.0.location
    }
    pub fn open(path: PathBuf, make_dir: bool) -> io::Result<BTreeEnv> {
        let lmdb= db::open(&path, make_dir)?;
        let location = path.canonicalize()?;
        tracing::debug!(?location, "Opening BTreeEnv");
        let log_head = Arc::new(ipcbus::ProcBus::new(lmdb.uid));
        let env = BTreeEnv(Arc::new(Inner{
            lmdb,
            log_head,
            location,
        }));
        {
            let new = save_ptr_one(&env, &***PUBLIC_GROUP_PKT)?.is_new();
            if new && std::env::var_os("LK_NO_LNS").is_none(){
                let mut bytes = LNS_ROOTS;
                let mut roots:Vec<_> = std::iter::from_fn(||{
                    if bytes.is_empty() { return None;}
                    let pkt = crate::pkt::read::read_pkt(bytes, true).unwrap();
                    bytes = &bytes[pkt.size() as usize..];
                    match pkt{
                        std::borrow::Cow::Borrowed(o) =>Some((o,SaveState::Pending)),
                        std::borrow::Cow::Owned(_) => panic!(),
                    }
                }).collect();
                save_ptr(&env, &mut roots)?;
            }
        }
        Ok(env)
    }

    pub fn local_enckey(&self) -> anyhow::Result<String> {
        // TODO this should prob check for read only access
        Ok(std::fs::read_to_string(self.0.location.join("local_auth"))?.lines().next().context("missing enckey")?.to_owned())
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
    pub fn get_reader(&self) -> anyhow::Result<ReadTxn> {
        Ok(ReadTxn(self.0.lmdb.read_txn()?))
    }

    pub async fn log_head(&self) -> Stamp {
        let v = self.0.log_head.next_async().await;
        Stamp::new(v)
    }

    fn save<P:NetPkt>(&self, pkts: &mut [(P,SaveState)]) -> io::Result<usize>{
        if pkts.is_empty() { return Ok(0);}
        let (last_idx, total) = self.0.lmdb.save(pkts).map_err(db::as_io)?;
        if total > 0 {
            let _ = self.0.log_head.emit(last_idx);
        }
        Ok(total)
    }
    pub fn real_disk_size(&self) -> io::Result<u64> {
        self.0.lmdb.real_disk_size()
    }
    pub fn env_info(&self) -> MDB_envinfo{
        self.0.lmdb.env_info()
    }
    pub fn db_info(&self) -> DbInfo{
        self.0.lmdb.db_info().unwrap()
    }
    pub fn lmdb_version(&self) -> LMDBVersion{
        self.0.lmdb.version_info()
    }
}


pub fn save_ptr(env: &BTreeEnv,pkts:&mut [(&NetPktPtr,SaveState)]) -> io::Result<usize>{
    env.save(pkts)
}
pub fn save_dyn(env: &BTreeEnv,pkts:&mut [(&dyn NetPkt,SaveState)]) -> io::Result<usize>{
    env.save(pkts)
}
pub fn save_ptr_one(env:&BTreeEnv,pkt:&NetPktPtr) -> io::Result<SaveState>{
    let mut o = [(pkt,SaveState::Pending)];
    save_ptr(env,&mut o)?;
    Ok(o[0].1)
}
pub fn save_dyn_one(env:&BTreeEnv,pkt:&dyn NetPkt) -> io::Result<SaveState>{
    let mut o = [(pkt,SaveState::Pending)];
    save_dyn(env,&mut o)?;
    Ok(o[0].1)
}
pub fn save_ptr_iter<'o>(env: &BTreeEnv, it : impl Iterator<Item=&'o NetPktPtr>) -> io::Result<usize>{
    let mut lst = smallvec::SmallVec::<[(&NetPktPtr,SaveState);8]>::new_const();
    lst.extend(it.map(|o|(o,SaveState::Pending)));
    save_ptr(env,&mut lst)
}
pub fn save_dyn_iter<'o>(env: &BTreeEnv, it : impl Iterator<Item=&'o dyn NetPkt>) -> io::Result<usize>{
    let mut lst = smallvec::SmallVec::<[(&dyn NetPkt,SaveState);8]>::new_const();
    lst.extend(it.map(|o|(o,SaveState::Pending)));
    save_dyn(env,&mut lst)
}
fn check_path(path:&std::path::Path) -> anyhow::Result<&std::path::Path>{
    if let Some(c) = path.components().find(|v| !matches!(v,std::path::Component::Normal(_))){
        anyhow::bail!("path can not contain a {c:?} component")
    }
    Ok(path)
}
