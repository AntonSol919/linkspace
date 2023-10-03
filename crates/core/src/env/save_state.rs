// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[derive(Debug, Default, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum SaveState {
    #[default]
    Pending = 0,
    Error = 0b001,
    Exists = 0b010,
    Written = 0b110,
}
impl SaveState {
    pub fn is_new(&self) -> bool {
        matches!(self, SaveState::Written)
    }
}
impl std::fmt::Display for SaveState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveState::Pending => f.write_str("pending"),
            SaveState::Error => f.write_str("error"),
            SaveState::Exists => f.write_str("exists"),
            SaveState::Written => f.write_str("written"),
        }
    }
}
