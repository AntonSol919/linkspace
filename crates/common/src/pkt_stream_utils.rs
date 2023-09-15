// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/// Utility for buffering data packets until a spoint references them.
/// Useful for filtering unlinked datapackets
use linkspace_core::{
    pkt::{NetPkt, NetPktBox},
    prelude::{utils::LkHashSet, LkHash, B64},
};
use linkspace_pkt::PointExt;

// FIXME, this should look in NetPktHeader.flags
#[derive(Default)]
pub struct Buffer {
    pub forward_buf: Vec<NetPktBox>,
    pub latest_links: LkHashSet,
}

impl Buffer {
    pub fn push(&mut self, pkt: NetPktBox) -> Vec<NetPktBox> {
        match pkt.links() {
            None => {
                // DataBlock
                if self.latest_links.contains(pkt.hash_ref()) {
                    vec![pkt]
                } else {
                    self.forward_buf.push(pkt);
                    vec![]
                }
            }
            Some(links) => {
                self.latest_links = links.iter().map(|r| r.ptr).collect();
                //self.forward_buf.drain(..).filter(|v| self.latest_links.remove(&v.hash())).chain(pkt).collect()
                let (mut unlocked, dropped): (Vec<_>, Vec<_>) = self
                    .forward_buf
                    .drain(..)
                    .partition(|v| self.latest_links.contains(v.hash_ref()));
                unlocked.push(pkt);
                if dropped.is_empty() {
                    tracing::warn!("Dropping {:#?}", dropped);
                }
                unlocked
            }
        }
    }
}

/// Extremely light best-effort deduplication. 
pub struct QuickDedup {
    bufs: Box<[LkHash]>,
}
impl QuickDedup {
    pub fn new(cap: usize) -> Self {
        QuickDedup {
            bufs: vec![B64([0; 32]); cap].into_boxed_slice(),
        }
    }
    pub fn probable_contains(&mut self, h: LkHash) -> bool {
        let i: usize = unsafe { *(&h.0 as *const [u8; 32] as *const usize) };
        let idx = i % self.bufs.len();
        let r = self.bufs[idx] == h;
        self.bufs[idx] = h;
        r
    }
}
