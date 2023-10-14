// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use parse_display::Display;
#[derive(Debug, Default, Copy, Clone, PartialEq, Display)]
#[repr(u8)]
pub enum SaveState {
    #[default]
    Pending = 0,
    Error = 0b001,
    Exists = 0b010,
    Written = 0b110,
}
impl SaveState {
    pub fn is_written(&self) -> bool {
        matches!(self, SaveState::Written)
    }
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
            "Bug - unaligned bytes"
        )
    };
    b
}
