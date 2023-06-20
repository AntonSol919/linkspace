use super::db::{PktLogCursor, TreeCursor, HashCursor};


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
            b.as_ptr().align_offset(std::mem::size_of::<usize>()) == 0,
            "BTree Bug, unaligned bytes"
        )
    };
    b
}
