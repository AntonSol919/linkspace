// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/*
TODO: Write txn should open its cursor once and keep them around.
*/

/*
#[cfg(all(feature = "inmem",not(feature = "lmdb'")))]
pub mod inmem;
#[cfg(all(feature = "inmem",not(feature = "lmdb'")))]
pub use self::inmem::*;
*/

#[cfg(feature = "lmdb")]
pub mod lmdb;
#[cfg(feature = "lmdb")]
pub use self::lmdb::*;

pub trait Cursors {
    fn pkt_cursor(&self) -> PktLogCursor;
    fn tree_cursor(&self) -> TreeCursor;
    fn hash_cursor(&self) -> HashCursor;
}
impl<X: Cursors> Cursors for &X {
    fn pkt_cursor(&self) -> PktLogCursor {
        (*self).pkt_cursor()
    }
    fn tree_cursor(&self) -> TreeCursor {
        (*self).tree_cursor()
    }
    fn hash_cursor(&self) -> HashCursor {
        (*self).hash_cursor()
    }
}
pub trait Refreshable {
    fn refresh(&mut self);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum IterDirection {
    Forwards,
    Backwards,
}
impl IterDirection {
    pub fn is_forward(&self) -> bool {
        matches!(self, IterDirection::Forwards)
    }
    pub fn from<X: PartialOrd>(start: X, end: X) -> Self {
        if start <= end {
            IterDirection::Forwards
        } else {
            IterDirection::Backwards
        }
    }
}

#[track_caller]
pub fn assert_align(b: &[u8]) -> &[u8] {
    if !b.is_empty() {
        assert!(
            b.as_ptr().align_offset(4) == 0,
            "BTree Bug, unaligned bytes"
        )
    };
    b
}
