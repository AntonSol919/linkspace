
use std::{io, mem};
use anyhow::{ensure };
use linkspace_pkt::{Stamp, NetPktExt, LkHash, PRIVATE, B64};
use lmdb::Transaction;
use lmdb_sys::{ MDB_stat};
pub use lmdb_sys::MDB_envinfo;

use super::{db::LMDBEnv, BTreeEnv};

/// Contains information about the environment.
#[derive(Debug, Clone, Copy,Default)]
pub struct LMDBVersion{
    pub major: libc::c_int,
    pub minor: libc::c_int,
    pub patch: libc::c_int,
}

#[derive(Debug)]
pub struct DbInfo{
    pub pktlog: MDB_stat,
    pub tree: MDB_stat,
    pub hash: MDB_stat,
}

impl LMDBEnv{
    pub fn real_disk_size(&self) -> io::Result<u64> {
        // taken from https://github.com/meilisearch/heed
        use std::fs::{Metadata,File};
        #[cfg(unix)]
        use std::os::unix::{
            io::{ BorrowedFd, RawFd},
        };
        #[cfg(windows)]
        use std::{
            lmdb_sys::OsStr,
            os::windows::io::{AsRawHandle, BorrowedHandle, RawHandle},
        };
        #[cfg(unix)]
        unsafe fn metadata_from_fd(raw_fd: RawFd) -> io::Result<Metadata> {
            let fd = BorrowedFd::borrow_raw(raw_fd);
            let owned = fd.try_clone_to_owned()?;
            File::from(owned).metadata()
        }

        #[cfg(windows)]
        unsafe fn metadata_from_fd(raw_fd: RawHandle) -> io::Result<Metadata> {
            let fd = BorrowedHandle::borrow_raw(raw_fd);
            let owned = fd.try_clone_to_owned()?;
            File::from(owned).metadata()
        }


        let mut fd = std::mem::MaybeUninit::uninit();
        unsafe { lmdb::lmdb_result(lmdb_sys::mdb_env_get_fd(self.env.env(), fd.as_mut_ptr())).map_err(crate::env::lmdb::db::as_io)? };
        let fd = unsafe { fd.assume_init() };
        let metadata = unsafe { metadata_from_fd(fd)? };
        Ok(metadata.len())
    }
    /// Returns some basic informations about this environment.
    pub fn env_info(&self) -> MDB_envinfo{
        let mut raw_info = mem::MaybeUninit::uninit();
        unsafe { lmdb_sys::mdb_env_info(self.env.env(), raw_info.as_mut_ptr()) };
        unsafe { raw_info.assume_init() }
    }

    /// Returns the size used by all the databases in the environment without the free pages.
    pub fn db_info(&self) -> lmdb::Result<DbInfo> {
        let mut pktlog = mem::MaybeUninit::uninit();
        let mut tree = mem::MaybeUninit::uninit();
        let mut hash = mem::MaybeUninit::uninit();
        let txn = self.env.begin_ro_txn()?;
        unsafe {
            lmdb_sys::mdb_stat(txn.txn(), self.pktlog.dbi(), pktlog.as_mut_ptr());
            lmdb_sys::mdb_stat(txn.txn(), self.tree.dbi(), tree.as_mut_ptr());
            lmdb_sys::mdb_stat(txn.txn(), self.hash.dbi(), hash.as_mut_ptr());

            Ok(DbInfo { pktlog: pktlog.assume_init(), tree: tree.assume_init(), hash: hash.assume_init() })
        }
    }
    pub fn version_info(&self) -> LMDBVersion{
        let mut v = LMDBVersion::default();
        unsafe {
            lmdb_sys::mdb_version(
                std::ptr::from_mut(&mut v.major),
                std::ptr::from_mut(&mut v.minor),
                std::ptr::from_mut(&mut v.patch),
            );
        }
        v
    }
    
}

impl BTreeEnv{
    pub fn linkspace_info(&self) -> anyhow::Result<()>{
        let reader = self.new_read_txn()?;
        use linkspace_pkt::tree_order::TreeEntry;
        let log : Vec<(u64,LkHash)> = reader.local_pkt_log(Stamp::ZERO)
            .inspect(|o| tracing::trace!(
                is_ok=?o.pkt.check(false),
                recv=%o.recv,
                hash=%o.pkt.hash(),
                tree=%TreeEntry::from_pkt(o.recv, &o.pkt).map(|o|o.to_string()).unwrap_or_default(),
                address=?o.pkt.as_netpkt_bytes().as_ptr(),
            )
            )
            .map(|o| (o.recv.get(),o.pkt.hash())).collect();

        let mut hashes : Vec<(u64,LkHash)> = reader.0.hash_cursor().range_uniq(&PRIVATE)
            .map(|(hash, stamp)| (stamp,B64(*hash)))
            .collect();
        hashes.sort_by_key(|(s,_)| *s);
        let hashes_ok = log == hashes;

        let iter_dup = reader.0.tree_cursor().iter_dup(true);

        let mut tree_ok = true;
        while let Some(e) = iter_dup.get_next_entry(){
            let te = super::tree_iter::spd(e);
            let ok = log.binary_search_by_key(&te.local_log_ptr().get(), |o|o.0).is_ok();
            if ok{
                tracing::trace!(?te,?ok)
            }else {
                tree_ok = false;
                tracing::warn!(?te,?ok)
            }
        }
        ensure!( tree_ok && hashes_ok, "Corruption: tree {tree_ok} - hashes : {hashes_ok}");

        let now = linkspace_pkt::now();
        let it = reader.local_pkt_log(now);
        for pkt in it {
            tracing::warn!("host - pkt inserted in the future ? {} {}", now, linkspace_pkt::PktFmtDebug(&pkt))
        }

        let head = reader.log_head();
        let it = reader.pkts_after(head);
        for pkt in it {
            tracing::warn!("lmdb-rs error - looped around? {} {}", now, linkspace_pkt::PktFmtDebug(&pkt))
        }

        Ok(())
    }
}

