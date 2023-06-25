
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
