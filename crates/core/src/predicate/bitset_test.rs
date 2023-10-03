// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::{SetValueInfo, TestSet};
use linkspace_pkt::MAX_SPACE_DEPTH;
use std::{fmt::Debug, str::FromStr};
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct BitTestSet<const MAX: u8 = 8>(pub u32);
pub type SPLenTestSet = BitTestSet<{ MAX_SPACE_DEPTH as u8 }>;
impl Default for BitTestSet {
    fn default() -> Self {
        Self::ALL
    }
}
impl Debug for BitTestSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries((0..32).filter(|i| self.contains(*i)))
            .finish()
    }
}

impl BitTestSet {
    pub fn try_combine(self, other: Self) -> anyhow::Result<Self> {
        let r = BitTestSet(self.0 & other.0);
        anyhow::ensure!(r.0 != 0, "requested bittestsets have no overlap");
        Ok(r)
    }
}

impl FromStr for BitTestSet {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(BitTestSet::NONE);
        }
        if s == "*" {
            return Ok(BitTestSet(1));
        }
        if s == "**" {
            return Ok(BitTestSet::ALL);
        }
        anyhow::bail!("TODO depthset {}", s);
    }
}

impl<const MAX: u8> BitTestSet<MAX> {
    pub const NONE: BitTestSet<MAX> = BitTestSet(u32::MIN);
    pub const ALL: BitTestSet<MAX> = BitTestSet(u32::MAX >> (32 - MAX));
    pub fn greater_eq(ge: u32) -> Self {
        match u32::MAX.checked_shl(ge) {
            Some(g) => Self::ALL.and(BitTestSet(g)),
            None => Self::NONE,
        }
    }
    pub fn from_rules(t: &TestSet<u8>) -> Self {
        BitTestSet::from_el_iter((0..=MAX_SPACE_DEPTH as u8).filter(|v| t.test(*v)))
    }
    pub fn contains_min(self, depth: u8) -> bool {
        self.contains(depth) || self.next_depth(depth).is_some() // This is inefficient
    }
    pub fn contains(&self, depth: u8) -> bool {
        self.0 & 1u32.checked_shl(depth as u32).unwrap_or(0) != 0
    }
    pub fn from_el_iter(it: impl IntoIterator<Item = u8>) -> Self {
        let mut this = 0;
        it.into_iter()
            .for_each(|i| this |= 1u32.checked_shl(i as u32).unwrap_or(0));
        BitTestSet(this)
    }

    #[must_use]
    pub fn checked_shift(self, base: u8) -> Option<Self> {
        self.0.checked_shl(base as u32).map(BitTestSet)
    }
    pub fn next_depth(&self, at: u8) -> Option<u8> {
        let mask = u32::MAX.checked_shl(at as u32 + 1)?;
        let new = self.0 & mask;
        if new == 0 {
            return None;
        }
        Some(new.trailing_zeros() as u8)
    }
    pub fn info(self, val: u8) -> SetValueInfo<u8> {
        let in_set = self.contains(val);
        SetValueInfo {
            in_set,
            val: if in_set {
                Some(val)
            } else {
                self.next_depth(val)
            },
        }
    }
    #[must_use]
    pub fn and(mut self, other: Self) -> Self {
        self.0 &= other.0;
        self
    }
}
#[test]
fn bitset() {
    let set = SPLenTestSet::greater_eq(4);
    assert!(!set.contains(0));
    assert!(!set.contains(3));
    assert!(set.contains(4));
    assert!(set.contains(5));
    let mut set = SPLenTestSet::from_el_iter([0, 1, 2, 3, 4, 10, 31]);
    assert!(set.contains(0));
    assert!(set.contains(4));
    assert!(!set.contains(5));
    assert!(set.contains(10));
    assert!(!set.contains(12));
    assert!(set.contains(31));
    assert!(!set.contains(32));
    assert_eq!(set.next_depth(0), Some(1));
    assert_eq!(set.next_depth(1), Some(2));
    assert_eq!(set.next_depth(4), Some(10));
    assert_eq!(set.next_depth(32), None);
    set = set.checked_shift(2).unwrap();
    assert!(!set.contains(0));
    assert!(set.contains(2));
    assert!(set.contains(12));
    assert!(!set.contains(32));
    assert_eq!(set.next_depth(11), Some(12));
    assert_eq!(set.next_depth(12), None);
}
