// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::*;
use crate::stamp_range::StampRange;
use either::Either;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, fmt::Write, ops::RangeInclusive, str::FromStr};
use thiserror::Error;

pub trait TestTrait {
    const TOKEN: u8;
    const ENUM: TestOp;
    fn test_ref<U: UInt>(left: &U, right: &U) -> bool;
    fn test_uint_slice(left: &[u8], right: &[u8]) -> bool;
}
pub trait TestVal<V>
where
    Self: std::fmt::Debug,
{
    fn test(&self, val: &V) -> bool;
    fn iter(&self) -> Box<dyn Iterator<Item = (TestOp, V)>>
    where
        V: 'static;
}
impl<V> TestVal<V> for () {
    #[inline(always)]
    fn test(&self, _val: &V) -> bool {
        true
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (TestOp, V)>>
    where
        V: 'static,
    {
        Box::new(std::iter::empty())
    }
}
impl<V, A: TestVal<V>, B: TestVal<V>> TestVal<V> for (A, B) {
    #[inline]
    fn test(&self, val: &V) -> bool {
        self.0.test(val) && self.1.test(val)
    }

    fn iter(&self) -> Box<dyn Iterator<Item = (TestOp, V)>>
    where
        V: 'static,
    {
        Box::new(self.0.iter().chain(self.1.iter()))
    }
}

macro_rules! top  {
    ([$( ($fname:ident,$token:expr,$test:expr, $test_vec:expr) ),* $(,)?]) => {
        $(
            #[derive(Copy,Clone,Eq,PartialEq,Debug)]
            pub struct $fname<V=()>(pub V);
            impl<V> $fname<V> {
                pub fn unwrap(self) -> (TestOp,V) { (TestOp::$fname, self.0)}
            }
            impl std::fmt::Display for $fname {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_char($token as char )
                }
            }
            impl TestTrait for $fname{
                const TOKEN: u8 = $token;
                const ENUM : TestOp = TestOp::$fname;
                fn test_ref<U:UInt>(left: &U,right:&U) -> bool { $test(left,right)}
                fn test_uint_slice(left: &[u8], right:&[u8]) -> bool{
                    assert_eq!(left.len(),right.len());
                    $test_vec(left,right)
                }
            }
            impl<U:UInt> TestVal<U> for $fname<U> {
                #[inline(always)]
                fn test(&self, val: &U) -> bool { $test(val,&self.0)}
                fn iter(&self) -> Box<dyn Iterator<Item=(TestOp,U)>> {
                    Box::new(std::iter::once((TestOp::$fname,self.0)))
                }
            }
        )*
        #[derive(Copy,Clone,Eq,PartialEq,Serialize,Deserialize)]
        #[derive(Debug)]
        pub enum TestOp{
            $( $fname ),*
        }
        pub const TEST_OP_CHARS : [u8;6] = [$($token as u8),*];
        impl TestOp {
            pub const fn into_byte(self) -> u8 {
                match self {
                    $( TestOp::$fname => $token ),*
                }
            }
            pub const fn from_byte(byte:u8) -> Option<Self>{
                match byte {
                    $( $token =>  Some(TestOp::$fname),)*
                        _ => None
                }
            }
            pub fn uint_func<U:UInt>(self) -> fn(&U,&U) -> bool {
                match self {
                    $( TestOp::$fname => $fname::test_ref::<U> ),*
                }
            }
            pub fn slice_func(self) -> fn(&[u8],&[u8]) -> bool {
                match self {
                    $( TestOp::$fname => $fname::test_uint_slice ),*
                }
            }
        }
        impl TryFrom<&[u8]> for TestOp{
            type Error = OpErr;
            fn try_from(op:&[u8]) -> Result<Self,OpErr> {
                if op.len() != 1 { return Err(OpErr::Parse)}
                let op = op[0];
                $( if op == $token { return Ok(TestOp::$fname);})*
                Err(OpErr::Unknown(op))
            }
        }

    };
}

impl std::fmt::Display for TestOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(self.into_byte() as char)
    }
}

#[derive(Error, Debug)]
pub enum OpErr {
    #[error("Unknown Op")]
    Unknown(u8),
    #[error("? expected byte")]
    Parse,
}

impl FromStr for TestOp {
    type Err = OpErr;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.as_bytes().try_into()
    }
}

top! {[
    (Nop,b'?',
     |_a,_b| true,
     |_a,_b| true),
    (Equal,b'=',
     |a,b| a == b,
     |a,b| a ==b),
    (Less,b'<',
     |a,b| a < b,
     |a,b| a <b),
    (Greater,b'>',
     |a,b| a > b,
     |a,b| a > b ),
    (Mask0,b'0',
     |a:&U,b:&U|     (a.bit_or(*b) == *b),
     |a:&[u8],b:&[u8]|       a.iter().zip(b).all(|(a,b)| a | b == *b)
     ),
    (Mask1,b'1',
     |a:&U,b:&U|     (a.bit_and(*b) ) == *b,
     |a:&[u8],b:&[u8]|       a.iter().zip(b).all(|(a,b)| a & b == *b)
    ),
]}

#[test]
fn mask0() {
    //maskzero ( 0b1111_0000 , 0b0101_0000, ) == true;
    //maskzero ( 0b1111_0000 , 0b0101_0001, ) == false;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SetValueInfo<V> {
    pub in_set: bool,
    pub val: Option<V>,
}
impl<V> SetValueInfo<V> {
    pub fn into<A>(self) -> SetValueInfo<A>
    where
        V: Into<A>,
    {
        self.map(Into::into)
    }
    pub fn map<A>(self, f: impl FnOnce(V) -> A) -> SetValueInfo<A> {
        SetValueInfo {
            in_set: self.in_set,
            val: self.val.map(f),
        }
    }
}

/// the set of numbers between bound.greater_eq <= number <= bound.less_eq
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct Bound<U> {
    pub low: U,
    pub high: U,
}
impl<U: UInt> Bound<U> {
    pub const DEFAULT: Self = Bound {
        high: U::MAX,
        low: U::MIN,
    };
    pub const EMPTY: Self = Bound {
        high: U::MIN,
        low: U::MAX,
    };
    pub fn from(r: RangeInclusive<U>) -> Self
    where
        U: Clone,
    {
        let (low, high) = if r.start() <= r.end() {
            (*r.start(), *r.end())
        } else {
            (*r.end(), *r.start())
        };
        Bound { low, high }
    }
    /// Numbers in the set are always less than this
    pub fn less(&self) -> Option<U> {
        self.high.inc()
    }
    /// Numbers in the set are always greater than this
    pub fn greater(&self) -> Option<U> {
        self.low.decr()
    }
    pub fn from_eq(v: U) -> Bound<U> {
        Bound { high: v, low: v }
    }
    pub fn as_eq(&self) -> Option<U> {
        if self.high == self.low {
            Some(self.high)
        } else {
            None
        }
    }
    pub fn iter(self) -> impl Iterator<Item = (TestOp, U)> {
        let eq = self.as_eq().map(|v| (TestOp::Equal, v));
        let less_it = if eq.is_some() {
            None
        } else {
            self.less().map(|v| (TestOp::Less, v))
        };
        let greater_it = if eq.is_some() {
            None
        } else {
            self.greater().map(|v| (TestOp::Greater, v))
        };
        eq.into_iter().chain(less_it).chain(greater_it)
    }
    #[inline(always)]
    pub fn test(&self, val: U) -> bool {
        val <= self.high && val >= self.low
    }
    pub fn bound_cmp(&self, i: U) -> Ordering {
        let start = i.cmp(&self.low);
        let end = i.cmp(&self.high);
        match (start, end) {
            (Ordering::Less, Ordering::Less) => Ordering::Less,
            (Ordering::Greater, Ordering::Greater) => Ordering::Greater,
            (Ordering::Less, Ordering::Greater) => unreachable!("cmp on empty set is meaningless"),
            _ => Ordering::Equal,
        }
    }
    pub fn take_high_bound(&mut self) -> U {
        std::mem::replace(&mut self.high, Self::DEFAULT.high)
    }
    pub fn take_low_bound(&mut self) -> U {
        std::mem::replace(&mut self.low, Self::DEFAULT.low)
    }
}


impl<U: UInt> Mask<U> {
    pub const DEFAULT: Self = {
        Mask {
            ones: U::MIN,
            zeros: U::MAX,
        }
    };
    pub const EMPTY: Self = {
        Mask {
            ones: U::MAX,
            zeros: U::MIN,
        }
    };
    pub fn ones(&self) -> Option<U> {
        if self.ones == Self::DEFAULT.ones {
            None
        } else {
            Some(self.ones)
        }
    }
    pub fn zeros(&self) -> Option<U> {
        if self.zeros == Self::DEFAULT.zeros {
            None
        } else {
            Some(self.zeros)
        }
    }
    pub fn has_any(&self) -> bool {
        self.zeros.bit_and(self.ones) == self.ones
    }
    pub fn new(ones: U, zeros: U) -> Self {
        Mask { ones, zeros }
    }
    pub fn as_opts(&self) -> Mask<Option<U>> {
        Mask {
            ones: self.ones(),
            zeros: self.zeros(),
        }
    }
    pub fn add(&mut self, op: TestOp, val: U) {
        *self = match op {
            TestOp::Mask0 => Mask::new(self.ones, self.zeros.bit_and(val)),
            TestOp::Mask1 => Mask::new(self.ones.bit_or(val), self.zeros),
            _ => *self,
        };
    }
}
impl<U: UInt> Mask<U> {
    #[inline(always)]
    pub fn test(&self, val: &U) -> bool {
        TestOp::Mask0.uint_func()(val, &self.zeros) && TestOp::Mask1.uint_func()(val, &self.ones)
    }
    pub fn min_val(&self) -> U {
        self.zeros.bit_and(self.ones)
    }
    pub fn max_val(&self) -> U {
        self.zeros.bit_and(U::MAX)
    }
    pub fn wrong_bits(&self, val: U) -> U {
        self.zeros
            .not()
            .bit_and(val)
            .bit_or(self.ones.bit_and(val.not()))
    }

    pub fn in_set(&self, val: U) -> Option<U> {
        let (overflow, v) = self.overflowing_in_set(val);
        if overflow {
            None
        } else {
            Some(v)
        }
    }

    pub fn overflowing_in_set(self: &Mask<U>, val: U) -> (bool, U) {
        // if val was known to be in set this could be done faster. But we dont so we cant.
        debug_assert!(self.has_any());
        let Mask { zeros, .. } = self;
        let wrong = self.wrong_bits(val);
        if wrong == U::MIN {
            return (false, val);
        }
        let min = self.min_val();
        let wrongmask = U::MAX
            .overflowing_shl(U::BITS - 1 - wrong.leading_zeros())
            .0
            .not();
        let vnz = val.bit_or(zeros.not()).bit_or(wrongmask);
        let pivot = vnz.trailing_ones();
        let (freemask, over) = U::MAX.overflowing_shr((U::BITS - 1).wrapping_sub(pivot));
        let (incval, _) = val
            .bit_or(freemask.overflowing_shr(1).0)
            .overflowing_add(U::ONE);
        let fix_mask = U::MAX.overflowing_shl(pivot).0.not();
        let v = incval.bit_and(fix_mask.not()).bit_or(min.bit_and(fix_mask));
        (over, v)
    }
}
#[test]
fn test_inset() {
    #![allow(unreachable_code)]
    return;
    for zeros in 0..=255 {
        for ones in 0..=255 {
            for val in 0..=255 {
                let mask = Mask { zeros, ones };
                if !mask.has_any() {
                    continue;
                };
                let iter_find = (val..=u8::MAX).filter(|v| mask.test(v)).next();
                assert_eq!(mask.in_set(val), iter_find)
            }
        }
    }
}

/// Tests to test a value. To pass the test value must have 1's for all 1's in ones, and all 0's for all 0's in zeros!!
/// I.e. : Mask { zeros : 0b0000_1111}.test(0b0000_1010) == true
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Mask<U> {
    pub ones: U,
    pub zeros: U,
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct TestSet<U> {
    pub bound: Bound<U>,
    pub mask: Mask<U>,
}
fn bits(v: &[u8]) -> String {
    std::iter::once("b".to_string())
        .chain(v.iter().map(|v| format!("_{v:0>8b}")))
        .collect()
}
impl<U: UInt> std::fmt::Debug for TestSet<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = f.debug_struct(&format!("TestSet<{}>", std::any::type_name::<U>()));
        if !self.bound.has_any() {
            s.field("empty", &true);
            return s.finish();
        }
        if let Some(eq) = self.bound.as_eq() {
            s.field("eq", &eq);
        } else {
            if let Some(less) = self.bound.less() {
                s.field("less", &less);
            }
            if let Some(greater) = self.bound.greater() {
                s.field("greater", &greater);
            }
        }
        if self.mask.ones != Self::DEFAULT.mask.ones {
            s.field("ones", &bits(&self.mask.ones.to_be_vec()));
        }
        if self.mask.zeros != Self::DEFAULT.mask.zeros {
            s.field("zeros", &bits(&self.mask.zeros.to_be_vec()));
        }
        s.finish()
    }
}
impl<U: UInt> std::fmt::Debug for Mask<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("Mask<{}>", std::any::type_name::<U>()))
            .field("ones", &bits(&self.ones.to_be_vec()))
            .field("zeros", &bits(&self.zeros.to_be_vec()))
            .finish()
    }
}

impl<U: UInt> TestSet<U> {
    pub fn as_eq(&self) -> Option<U> {
        // FIXME check mask
        self.bound.as_eq()
    }
}

impl<U: UInt> Default for TestSet<U> {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl<U: UInt> Bound<U> {
    pub fn has_any(&self) -> bool {
        self.high >= self.low
    }
    pub fn try_add(&mut self, op: TestOp, rh: U) -> anyhow::Result<()> {
        self.add(op, rh);
        if !self.has_any() {
            anyhow::bail!("incompatible bound {:?} {:?}", op, rh)
        }
        Ok(())
    }
    pub fn add(&mut self, op: TestOp, rh: U) {
        match op {
            TestOp::Equal => {
                self.low = self.low.max(rh);
                self.high = self.high.min(rh);
            }
            TestOp::Less => match rh.decr() {
                Some(v) => self.high = self.high.min(v),
                None => *self = Self::EMPTY,
            },
            TestOp::Greater => match rh.inc() {
                Some(v) => self.low = self.low.max(v),
                None => *self = Self::EMPTY,
            },
            _ => {}
        }
    }
}

impl<U: UInt> TestSet<U> {
    pub const DEFAULT: Self = {
        TestSet {
            bound: Bound::<U>::DEFAULT,
            mask: Mask::<U>::DEFAULT,
        }
    };

    #[inline]
    /// Warning - This is only valid if TestSet.has_any;
    pub fn info(&self, val: U) -> SetValueInfo<U> {
        let cmp = self.bound.bound_cmp(val);
        match cmp {
            std::cmp::Ordering::Less => SetValueInfo {
                in_set: false,
                val: self.mask.in_set(self.bound.low),
            },
            std::cmp::Ordering::Equal => match self.mask.in_set(val) {
                Some(v) => {
                    if v > self.bound.high {
                        SetValueInfo {
                            in_set: false,
                            val: None,
                        }
                    } else {
                        SetValueInfo {
                            in_set: v == val,
                            val: Some(v),
                        }
                    }
                }
                None => SetValueInfo {
                    in_set: false,
                    val: None,
                },
            },
            std::cmp::Ordering::Greater => SetValueInfo {
                in_set: false,
                val: None,
            },
        }
    }
    pub fn is_empty(&self) -> bool {
        !self.has_any()
    }
    pub fn min_value(&self) -> Option<U> {
        let v = self.mask.in_set(self.bound.low)?;
        if self.bound.high < v {
            None
        } else {
            Some(v)
        }
    }
    pub fn has_any(&self) -> bool {
        self.min_value().is_some()
    }

    pub fn rules(self) -> impl Iterator<Item = (TestOp, U)> {
        if self.is_empty() {
            return Either::Left(std::iter::once((TestOp::Less, U::MIN)));
        }
        let as_opt = |test, v, default| {
            {
                if v == default {
                    None
                } else {
                    Some((test, v))
                }
            }
            .into_iter()
        };
        let it = self
            .bound
            .iter()
            .chain(as_opt(
                TestOp::Mask1,
                self.mask.ones,
                Self::DEFAULT.mask.ones,
            ))
            .chain(as_opt(
                TestOp::Mask0,
                self.mask.zeros,
                Self::DEFAULT.mask.zeros,
            ));
        Either::Right(it)
    }

    pub fn try_add(&mut self, op: TestOp, val: U) -> anyhow::Result<()> {
        self.bound.add(op, val);
        self.mask.add(op, val);
        if self.has_any() {
            Ok(())
        } else {
            anyhow::bail!("incompatible {:?} {:?}", op, val)
        }
    }
    pub fn add(&mut self, op: TestOp, val: U) {
        self.bound.add(op, val);
        self.mask.add(op, val);
    }
    #[allow(clippy::complexity)]
    pub fn unpack(
        &self,
    ) -> (
        Option<Equal<U>>,
        Option<Less<U>>,
        Option<Greater<U>>,
        Option<Mask0<U>>,
        Option<Mask1<U>>,
    ) {
        (
            self.bound.as_eq().map(Equal),
            self.bound.less().map(Less),
            self.bound.greater().map(Greater),
            self.mask.zeros().map(Mask0),
            self.mask.ones().map(Mask1),
        )
    }
    pub fn map<O>(self, f: impl Fn(U) -> O) -> TestSet<O> {
        TestSet {
            bound: Bound {
                low: f(self.bound.low),
                high: f(self.bound.high),
            },
            mask: Mask {
                ones: f(self.mask.ones),
                zeros: f(self.mask.zeros),
            },
        }
    }
}
impl Bound<u64> {
    pub fn stamp_range(&self, ascending: bool) -> StampRange {
        let (start,end) = if ascending {(self.low,self.high) }else {(self.high,self.low)};
        StampRange{start,end}
    }
}
impl<U: UInt> TestSet<U> {
    pub fn test(&self, val: U) -> bool {
        self.rules()
            .all(|(op, rh_val)| op.uint_func()(&val, &rh_val))
    }
}

impl<U: UInt> Bound<U> {
    pub fn try_combine(self, other: Self) -> anyhow::Result<Self> {
        anyhow::ensure!(self.high >= other.low);
        anyhow::ensure!(other.low <= self.high);
        Ok(Bound {
            high: self.high.min(other.high),
            low: self.low.max(other.low),
        })
    }
}

impl<V: UInt> TestSet<V> {
    /**
    Returns an iterator of booleans starting signifying if the element is in set.
    ```
    TestSet {+:1, <:8} == [1,3,5,7]
    iter_contains(3) -> [true,false,true,false,true]
    //The 'at' param tracks the last yield,
    //This is usefull in situations such as
    let mut at = 3;
    let i = vec![4,2,6].iter().zip(set.iter_contains(&mut at)).filter_map(|(v,b)| b.and_then(v)).collect();
    assert_eq!(i,[4,6]); assert_eq(at,6);
    ```
     **/
    pub fn iter_contains(self, at: &mut V) -> impl Iterator<Item = bool> + '_ {
        std::iter::from_fn(move || {
            if self.bound.high < *at {
                return None;
            }
            let r = self.test(*at);
            if let Some(nt) = at.inc() {
                *at = nt;
            } else if !r {
                return None;
            };
            Some(r)
        })
    }

    pub fn iter(self, at: V) -> MembershipIter<V> {
        MembershipIter { set: self, at }
    }
    pub fn enumerate(self, start: V) -> impl Iterator<Item = V> {
        let mut next = self.info(start).val;
        std::iter::from_fn(move || {
            let result = next?;
            next = result.inc();
            if let Some(v) = next {
                next = self.info(v).val
            }
            Some(result)
        })
    }
}

pub struct MembershipIter<V> {
    set: TestSet<V>,
    at: V,
}
impl<V: UInt> MembershipIter<V> {
    pub fn peek(&self) -> Option<bool> {
        if self.set.bound.high < self.at {
            return None;
        }
        Some(self.set.test(self.at))
    }
}
impl<V: UInt> Iterator for MembershipIter<V> {
    type Item = bool;
    fn next(&mut self) -> Option<Self::Item> {
        if self.set.bound.high < self.at {
            return None;
        }
        let r = self.set.test(self.at);
        if let Some(nt) = self.at.inc() {
            self.at = nt;
        } else if !r {
            return None;
        };
        Some(r)
    }
}
