use std::ops::Range;

use linkspace_pkt::{
    now,
    tree_order::{TreeEntry, TreeValueBytes},
    NetPkt, NetPktExt,
};
use lmdb::{RwCursor, Transaction, WriteFlags};

use crate::env::misc::SaveState;

use super::db::LMDBEnv;

impl LMDBEnv {
    /// return first stamp used and last stamp. If first == last then nothing was written.
    #[tracing::instrument(skip_all, err)]
    pub fn save<P: NetPkt>(&self, pkts: &mut [(P, SaveState)]) -> lmdb::Result<Range<u64>> {
        use lmdb::Error;
        use lmdb_sys::*;

        let lmdb_e = &self;
        let txn = lmdb_e.env.begin_rw_txn()?;

        let pktlog = RwCursor::new(&txn, lmdb_e.pktlog)?;

        let mut start = now().get();
        match pktlog.ro().get(None, None, lmdb_sys::MDB_LAST) {
            Ok((Some(recv), _)) => {
                let last: u64 = super::db::pktlog::val(recv.try_into().unwrap());
                if last > start {
                    eprintln!("db log saved entries from the future? - this could become undefined behavior");

                    start = last + 1;
                }
            }
            Ok((None, _)) => unreachable!(),
            Err(Error::NotFound) => {}
            Err(e) => return Err(e),
        };

        let mut hash = RwCursor::new(&txn, lmdb_e.hash)?;
        let mut at = start;

        for (p, state) in pkts.iter_mut() {
            if matches!(state, SaveState::Pending) {
                match hash.put(p.hash_ref(), &at.to_ne_bytes(), WriteFlags::NO_OVERWRITE) {
                    Ok(()) => {
                        at += 1;
                    }
                    Err(Error::KeyExist) => {
                        *state = SaveState::Exists;
                        tracing::trace!(p=%p.hash_ref(),"already exists");
                    }
                    Err(e) => Err(e)?,
                }
            }
        }
        std::mem::drop(hash);
        let total_new = at - start;
        tracing::trace!(total_new, start, end = at, "new txn for");

        if total_new == 0 {
            return Ok(start..at);
        };
        at = start;
        for (pkt, state) in pkts.iter() {
            if matches!(state, SaveState::Pending) {
                let mut at_val = super::db::pktlog::bytes(at);
                let mut key_val: MDB_val = MDB_val {
                    mv_size: 8,
                    mv_data: std::ptr::from_mut(&mut at_val).cast(),
                };
                let len = pkt.size() as usize;
                let mut data_val: MDB_val = MDB_val {
                    mv_size: len,
                    mv_data: std::ptr::null_mut::<libc::c_void>(),
                };

                let flags = MDB_NODUPDATA | MDB_NOOVERWRITE | MDB_RESERVE | MDB_APPEND;
                let r = unsafe {
                    mdb_cursor_put(pktlog.ro().cursor(), &mut key_val, &mut data_val, flags)
                };
                if r != MDB_SUCCESS {
                    return Err(lmdb::Error::from_err_code(r));
                }
                let segments = pkt.byte_segments();
                unsafe {
                    segments.write_segments_unchecked(data_val.mv_data.cast());
                }

                at += 1;
            }
        }

        std::mem::drop(pktlog);
        at = start;

        let mut tree = RwCursor::new(&txn, lmdb_e.tree)?;

        for (pkt, state) in pkts.iter_mut() {
            if matches!(state, SaveState::Pending) {
                *state = SaveState::Written;
                let entry = TreeEntry::from_pkt(at.into(), &pkt);
                let te = entry.map(|te| (te.btree_key.take(), te.val));
                if let Some((key, val)) = te {
                    tree.put(&key, &val, WriteFlags::empty())?;
                    assert!(val.len() == std::mem::size_of::<TreeValueBytes>());
                }
                at += 1;
            }
        }
        std::mem::drop(tree);

        txn.commit()?;
        Ok(start..at)
    }
}
