// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use core::fmt;
use std::{cmp::Ordering, hint::unreachable_unchecked, ops::RangeInclusive, str::FromStr};

use linkspace_pkt::Stamp;

use crate::stamp_fmt::stamp_from_str;

/// By default runs from MAX value to 0 ( new to old )
#[derive(Debug, Clone, Eq, PartialEq, Copy, Hash)]
pub struct StampRange {
    pub start: u64,
    pub end: u64,
}
impl Default for StampRange {
    fn default() -> StampRange {
        StampRange::ALL_DSC
    }
}
impl StampRange {
    pub const fn new_u64s(start: u64, end: u64) -> StampRange {
        StampRange { start, end }
    }
    pub const fn new(start: Stamp, end: Stamp) -> StampRange {
        StampRange {
            start: start.get(),
            end: end.get(),
        }
    }
    pub const ALL_ASC: StampRange = StampRange::new(Stamp::ZERO, Stamp::MAX);
    pub const ALL_DSC: StampRange = StampRange::new(Stamp::MAX, Stamp::ZERO);
    #[must_use]
    pub const fn rev(self) -> StampRange {
        StampRange {
            start: self.end,
            end: self.start,
        }
    }
    pub fn is_ascending(&self) -> bool {
        self.start < self.end
    }
    pub fn is_new_first(&self) -> bool {
        !self.is_ascending()
    }
    pub const fn as_bound(self) -> RangeInclusive<u64> {
        self.start..=self.end
    }
    pub fn contains_u64(&self, i: u64) -> bool {
        self.bound_cmp(i) == Ordering::Equal
    }
    pub fn contains(&self, i: Stamp) -> bool {
        self.contains_u64(i.get())
    }
    pub fn bound_cmp(&self, i: u64) -> Ordering {
        let start = self.start.cmp(&i);
        let end = self.end.cmp(&i);
        match (start, end) {
            (Ordering::Less, Ordering::Less) => Ordering::Less,
            (Ordering::Greater, Ordering::Greater) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }

    pub fn iter_cmp(&self, i: Stamp) -> IterCmp {
        self.iter_cmp_u64(i.get())
    }
    /// The Range is either ascending or descending.
    /// This function determines where 'i' is if increment/decrementing along with the range direction.
    /// By default if start == end the range is considered to be ascending
    pub fn iter_cmp_u64(&self, i: u64) -> IterCmp {
        let start = self.start.cmp(&i) as i8;
        let end = self.end.cmp(&i) as i8;
        let i = if start == end { start } else { 0 };
        let dir = self.start.cmp(&self.end) as i8;
        let r = match (dir, i) {
            (0, v) => -v,
            (a, b) => a * b,
        };
        IterCmp::unchecked_from(r)
    }
    pub fn lower_bound(&self) -> Stamp {
        Stamp::new(self.start.min(self.end))
    }

    pub fn upper_bound(&self) -> Stamp {
        Stamp::new(self.start.max(self.end))
    }
}

impl StampRange {
    pub fn from_str_at(s: &str, now: Option<Stamp>) -> anyhow::Result<Self> {
        let mut i = s.split("..");
        let mut default = StampRange::ALL_DSC;
        default.start = i
            .next()
            .filter(|v| !v.is_empty())
            .map(|v| stamp_from_str(v, now))
            .transpose()
            .map_err(|_| anyhow::anyhow!("Invalid start"))?
            .map(Stamp::get)
            .unwrap_or(default.start);
        default.end = i
            .next()
            .filter(|v| !v.is_empty())
            .map(|v| stamp_from_str(v, now))
            .transpose()
            .map_err(|_| anyhow::anyhow!("invalid end"))?
            .map(Stamp::get)
            .unwrap_or(default.end);
        Ok(default)
    }
}

impl FromStr for StampRange {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        StampRange::from_str_at(s, None)
    }
}
impl fmt::Display for StampRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.start, self.end) {
            (u64::MAX, v) => write!(f, "++..{v}"),
            (v, u64::MAX) => write!(f, "{v}..++"),
            (a, b) => write!(f, "{a}..{b}"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(i8)]
pub enum IterCmp {
    Pre = -1,
    In = 0,
    Post = 1,
}
impl IterCmp {
    pub fn unchecked_from(i: i8) -> Self {
        match i {
            -1 => IterCmp::Pre,
            0 => IterCmp::In,
            1 => IterCmp::Post,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

#[test]
fn cmp() {
    use IterCmp::*;
    fn c(a: u64, b: u64, c: u64, r: IterCmp) {
        println!("Expect {:?}", r);
        assert_eq!(
            r,
            StampRange::new_u64s(a, b).iter_cmp(c.into()),
            "NOT {}..{} cont {} = {:?}",
            a,
            b,
            c,
            r
        );
    }
    c(0, 0, 1, Post);
    c(0, 1, 0, In);
    c(0, 1, 1, In);
    c(0, 1, 2, Post);

    c(1, 10, 0, Pre);
    c(1, 10, 1, In);
    c(1, 10, 2, In);
    c(1, 10, 11, Post);

    c(10, 1, 0, Post);
    c(10, 1, 1, In);
    c(10, 1, 2, In);
    c(10, 1, 11, Pre);

    c(2, 1, 0, Post);
    c(2, 1, 1, In);
    c(2, 1, 2, In);
    c(2, 1, 4, Pre);

    c(2, 2, 1, Pre);
    c(2, 2, 2, In);
    c(2, 2, 3, Post);
}
