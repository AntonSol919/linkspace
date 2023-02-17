// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    hash::{BuildHasherDefault, Hasher},
    mem::size_of,
};

pub(crate) fn none<X, Y>(_f: Y) -> Option<X> {
    None
}
pub(crate) fn as_bytes<V>(b: &V) -> &[u8; size_of::<V>()] {
    unsafe { &*(b as *const V as *const [u8; size_of::<V>()]) }
}

use crate::LkHash;
#[derive(Copy, Clone, Default)]
pub struct NoHash(u64);
impl Hasher for NoHash {
    #[inline]
    fn write(&mut self, value: &[u8]) {
        self.0 ^= u64::from_ne_bytes(value[0..8].try_into().unwrap());
    }
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}
pub type PktHashBuildHasher = BuildHasherDefault<NoHash>;
pub type PktHashMap<V> = ::std::collections::HashMap<LkHash, V, PktHashBuildHasher>;
pub type PktHashSet = ::std::collections::HashSet<LkHash, PktHashBuildHasher>;
