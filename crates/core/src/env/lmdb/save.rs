
use std::fmt::Display;

use linkspace_pkt::{NetPkt, now,  NetPktExt};
use lmdb::{RwCursor ,  WriteFlags, Transaction};

use crate::env::{ tree_key::{TreeEntry, TreeValueBytes}};

use super::{  db::LMDBEnv};


#[derive(Debug,Default,Copy,Clone,PartialEq)]
#[repr(u32)]
pub enum SaveState {
    #[default]
    Pending = 0 ,
    Error   = 0b001,
    Exists  = 0b010,
    Written = 0b110,
}
impl SaveState {
    pub fn is_new(&self) -> bool { matches!(self,SaveState::Written)}
}
impl Display for SaveState{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            SaveState::Pending => f.write_str("pending"),
            SaveState::Error => f.write_str("error"),
            SaveState::Exists => f.write_str("exists"),
            SaveState::Written => f.write_str("written"),
        }
    }
}



impl LMDBEnv{
    pub fn save<P:NetPkt>(&self, pkts: &mut [(P,SaveState)]) -> lmdb::Result<(u64,usize)>{
        use lmdb::Error;
        use lmdb_sys::*;

        let lmdb_e = &self;
        tracing::trace!(pkts_list_len=pkts.len(),"opening write txn");
        let txn = lmdb_e.env.begin_rw_txn()?;


        let pktlog = RwCursor::new(&txn,lmdb_e.pktlog)?;

        let mut start = now().get();
        match pktlog.ro().get(None, None, lmdb_sys::MDB_LAST){
            Ok((Some(recv), _)) => {
                let last : u64=  super::db::pktlog::val(recv.try_into().unwrap());
                if last > start {
                    eprintln!("db log saved entries from the future? - this could become undefined behavior")
                }
                tracing::trace!(last,start,"attempting to write");
                start = last.max(start);
            }
            Ok((None,_)) => unreachable!(),
            Err(e) if matches!(e,Error::NotFound) => {},
            Err(e) => return Err(e)
        };

        let mut hash = RwCursor::new(&txn,lmdb_e.hash)?;
        let mut at = start;

        for (p,state) in pkts.iter_mut() {
            if matches!(state,SaveState::Pending){
                match hash.put(p.hash_ref(),&at.to_ne_bytes(),  WriteFlags::NO_OVERWRITE){
                    Ok(()) => {
                        at +=1;
                    },
                    Err(e) if matches!(e,Error::KeyExist) => {
                        *state = SaveState::Exists;
                        tracing::trace!(p=%p.hash_ref(),"already exists");
                    },
                    Err(e) => {
                        tracing::trace!(?e,"write err");
                        Err(e)?
                    }
                }
            }
        }
        std::mem::drop(hash);
        let total_new = at-start;
        tracing::trace!(total_new,at,"hashes inserted");

        if total_new == 0 { return Ok((at,0));}
        at = start;
        for (pkt,state) in pkts.iter() {
            if matches!(state,SaveState::Pending){
                let mut at_val = super::db::pktlog::bytes(at);
                let mut key_val: MDB_val = MDB_val {
                    mv_size: 8 ,
                    mv_data: std::ptr::from_mut(&mut at_val).cast()
                };
                let len = pkt.size() as usize;
                let mut data_val: MDB_val = MDB_val {
                    mv_size: len,
                    mv_data: std::ptr::null_mut::<libc::c_void>(),
                };

                let flags = MDB_NODUPDATA | MDB_NOOVERWRITE | MDB_RESERVE | MDB_APPEND;
                let r = unsafe {mdb_cursor_put(pktlog.ro().cursor(),&mut key_val,&mut data_val, flags)};
                if r != MDB_SUCCESS {
                    return Err(lmdb::Error::from_err_code(r))
                }
                let segments = pkt.byte_segments();
                unsafe { segments.write_segments_unchecked(data_val.mv_data.cast());}

                at +=1;
            }
        }

        tracing::trace!("pktlog inserted");
        std::mem::drop(pktlog);
        at = start;

        let mut tree = RwCursor::new(&txn,lmdb_e.tree)?;

        for (pkt,state) in pkts.iter_mut() {
            if matches!(state,SaveState::Pending) {
                *state = SaveState::Written;
                let entry = TreeEntry::from_pkt(at.into(), &pkt);
                tracing::trace!(pkt=%linkspace_pkt::PktFmtDebug(&pkt),?entry,"tree entry");
                let te = entry.map(|te| (te.btree_key.take(), te.val));
                if let Some((key,val)) = te{
                    tree.put(&key , &val,WriteFlags::empty())?;
                    assert!(val.len() == std::mem::size_of::<TreeValueBytes>());
                    tracing::trace!(?key,?val,"save ok");
                }
                at +=1;
            }
        }
        std::mem::drop(tree);
        tracing::trace!("tree entries ok");

        txn.commit()?;
        Ok((at-1, total_new as usize))
    }
}
