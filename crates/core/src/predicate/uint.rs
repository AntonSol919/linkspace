// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::ops::Not;

use crate::pkt::{AB, B64};

pub trait UInt
where
    Self: Copy + PartialEq + Eq + Ord + PartialOrd + std::fmt::Debug + Send + Sync + 'static,
{
    const MIN: Self;
    const MAX: Self;
    const ONE: Self;
    const BITS: u32;
    fn not(self) -> Self;
    fn bit_and(self, other: Self) -> Self;
    fn bit_or(self, other: Self) -> Self;
    fn inc(self) -> Option<Self> {
        self.checked_add(Self::ONE)
    }
    fn overflowing_add(self, rhs: Self) -> (Self, bool) ;
    fn checked_add(self, o: Self) -> Option<Self>;
    fn checked_sub(self, o: Self) -> Option<Self>;
    fn decr(self) -> Option<Self> {
        self.checked_sub(Self::ONE)
    }

    fn overflowing_shl(self, rhs: u32) -> (Self, bool) ;
    fn overflowing_shr(self, rhs: u32) -> (Self, bool) ;
    fn leading_zeros(&self) -> u32 ;
    fn trailing_zeros(&self) -> u32 ;
    fn trailing_ones(&self) -> u32 ;
    fn leading_ones(&self) -> u32 ;

    fn as_be_bytes(&self, _out: &mut dyn FnMut(&[u8]));
    fn to_be_vec(&self) -> Vec<u8> {
        let mut v = vec![];
        self.as_be_bytes(&mut |o| v.extend_from_slice(o));
        v
    }

    //fn as_bytes(self) -> Vec<u8>;
    //fn print_bytes(self) as bytes ...
}

macro_rules! impl_native_uint {
    ($k:ident) => {
        impl UInt for $k {
            const MIN: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = $k::max_value();
            const BITS: u32 = std::mem::size_of::<Self>() as u32 * 8;

            fn as_be_bytes(&self, out: &mut dyn FnMut(&[u8])){out(&$k::to_be_bytes(*self))}
            #[inline(always)]
            fn not(self) -> Self {
                !self
            }
            #[inline(always)]
            fn bit_and(self, other: Self) -> Self {
                self & other
            }
            #[inline(always)]
            fn bit_or(self, other: Self) -> Self {
                self | other
            }
            fn overflowing_add(self, other: Self) -> (Self, bool) {
                self.overflowing_add(other)
            }
            fn checked_add(self, other: Self) -> Option<Self> {
                self.checked_add(other)
            }
            fn checked_sub(self, _other: Self) -> Option<Self> {
                todo!()
            }

            fn inc(self) -> Option<Self> {
                self.checked_add(1)
            }
            fn decr(self) -> Option<Self> {
                self.checked_sub(1)
            }
            fn leading_zeros(&self) -> u32 {
                $k::leading_zeros(*self)
            }
            fn trailing_zeros(&self) -> u32 {
                $k::trailing_zeros(*self)
            }
            fn trailing_ones(&self) -> u32 {
                $k::trailing_ones(*self)
            }
            fn leading_ones(&self) -> u32 {
                $k::leading_ones(*self)
            }
            fn overflowing_shl(self, rhs: u32) -> (Self, bool) {
                $k::overflowing_shl(self, rhs)
            }
            fn overflowing_shr(self, rhs: u32) -> (Self, bool) {
                $k::overflowing_shr(self, rhs)
            }
        }
    };
}
impl_native_uint!(u128);
impl_native_uint!(u64);
impl_native_uint!(u32);
impl_native_uint!(u16);
impl_native_uint!(u8);

/*
macro_rules! impl_bigendian_uint {
    ($k:ident,$native:ident) => {
        impl UInt for $k {
            const MIN: Self = $k::ZERO;
            const MAX: Self = $k::MAX;
            const ONE: Self = $k::new(1);
            #[inline(always)]
            fn negate(self) -> Self { !self }
            #[inline(always)]
            fn bit_and(self,other: Self) -> Self { self & other }
            #[inline(always)]
            fn bit_or(self,other: Self) -> Self { self | other }
            #[inline(always)]
            fn checked_add(self,other:Self) -> Option<Self> { self.get().checked_add(other.get()).map(Into::into)}
            fn checked_sub(self,_other:Self) -> Option<Self> { todo!()}
            #[inline(always)]
            fn inc(self) -> Option<Self> { self.get().checked_add(1).map(Into::into)}
            #[inline(always)]
            fn decr(self) -> Option<Self> { self.get().checked_sub(1).map(Into::into)}
        }
    };
}

use linkspace_pkt::uint_native::NativeArrayConstants;
use linkspace_pkt::{LU64,LU32,LU16   };


impl_bigendian_uint!(LU64,u64);
impl_bigendian_uint!(LU32,u32);
impl_bigendian_uint!(LU16,u16);

macro_rules! impl_aligned_bigendian {
    ($native:ident) => {
        impl<const L:usize> UInt for [$native;L]{
            const MIN: Self = [0;L];
            const MAX: Self = [$native::MAX;L];
            const ONE: Self = { let mut v = Self::MIN; v[L-1] = $native::from_ne_bytes((1 as $native).to_be_bytes()); v};
            #[inline(always)]
            fn bit_and(self,other: Self) -> Self { self.zip(other).map(|(a,b)| a&b) }
            #[inline(always)]
            fn bit_or(self,other: Self) -> Self { self.zip(other).map(|(a,b)| a|b) }
            #[inline(always)]
            fn negate(self) -> Self { self.map(|i| !i)}
            fn checked_sub(self,_o:Self) -> Option<Self> {
                todo!()
            }
            fn checked_add(self,_o:Self) -> Option<Self> {
                todo!()
            }
        }
    };
}
impl_aligned_bigendian!(u16);
impl_aligned_bigendian!(u32);
impl_aligned_bigendian!(u64);
impl_aligned_bigendian!(u128);


impl<const L:usize> UInt for BigUInt<L,u64> where Self: NativeArrayConstants + Copy + PartialEq+Eq+Ord+PartialOrd +std::fmt::Debug + Send + Sync + 'static{
    const MIN: Self = <Self as NativeArrayConstants>::MIN;
    const MAX: Self = <Self as NativeArrayConstants>::MAX;
    const ONE: Self = <Self as NativeArrayConstants>::ONE;
    fn negate(self) -> Self {
        Self::negate(self)
    }
    fn bit_and(self, other:Self) -> Self {
        Self::bit_and(self, other)
    }
    fn bit_or(self, other:Self) -> Self {
        Self::bit_or(self, other)
    }
    fn checked_add(self,o:Self) -> Option<Self> {
        Self::checked_add(self, o)
    }

    fn checked_sub(self,o:Self) -> Option<Self> {
        Self::checked_sub(self, o)
    }

}
*/

impl UInt for B64<[u8; 32]> {
    const MIN: Self = B64([0; 32]);
    const MAX: Self = B64([255; 32]);
    const ONE: Self = B64(u8_be::one());
    const BITS: u32 = 32 * 8;

    #[inline(always)]
    fn bit_and(self, other: Self) -> Self {
        B64(self.0.zip(other.0).map(|(a, b)| a & b))
    }
    #[inline(always)]
    fn bit_or(self, other: Self) -> Self {
        B64(self.0.zip(other.0).map(|(a, b)| a | b))
    }
    #[inline(always)]
    fn not(self) -> Self {
        B64(self.0.map(|i| !i))
    }
    fn inc(self) -> Option<Self> {
        u8_be::add(self.0, u8_be::one()).map(B64)
    }
    fn decr(self) -> Option<Self> {
        u8_be::sub(self.0, u8_be::one()).map(B64)
    }


    fn checked_sub(self, other: Self) -> Option<Self> {
        UInt::checked_sub(self.to_u256(),other.to_u256()).map(Self::from_u256)
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        UInt::checked_add(self.to_u256(),other.to_u256()).map(Self::from_u256)
    }

    fn overflowing_shl(self, rhs: u32) -> (Self, bool)  {
        let (v,over) = UInt::overflowing_shl(self.to_u256(), rhs);
        (v.into(),over)
    }

    fn overflowing_shr(self, rhs: u32) -> (Self, bool)  {
        let (v,over) = UInt::overflowing_shr(self.to_u256(), rhs);
        (v.into(),over)
    }

    fn leading_zeros(&self) -> u32  {
        UInt::leading_zeros(&self.to_u256())
    }
    fn trailing_zeros(&self) -> u32  {
        UInt::trailing_zeros(&self.to_u256())
    }

    fn trailing_ones(&self) -> u32  {
        UInt::trailing_ones(&self.to_u256())
    }

    fn leading_ones(&self) -> u32  {
        UInt::leading_ones(&self.to_u256())
    }

    fn overflowing_add(self, rhs: Self) -> (Self, bool)  {
        let (v,over) = UInt::overflowing_add(self.to_u256(), rhs.to_u256());
        (v.into(),over)
    }

    fn as_be_bytes(&self, out: &mut dyn FnMut(&[u8])) {
        out(&self.0)
    }


}

impl UInt for AB<[u8; 16]> {
    const MIN: Self = AB([0; 16]);
   const MAX: Self = AB([255; 16]);
    const ONE: Self = AB(u8_be::one());
    const BITS: u32 = 16 * 8;
    fn as_be_bytes(&self, out: &mut dyn FnMut(&[u8])) {
        out(&self.0)
    }
  
    #[inline(always)]
    fn bit_and(self, other: Self) -> Self {
        AB(self.0.zip(other.0).map(|(a, b)| a & b))
    }
    #[inline(always)]
    fn bit_or(self, other: Self) -> Self {
        AB(self.0.zip(other.0).map(|(a, b)| a | b))
    }
    #[inline(always)]
    fn not(self) -> Self {
        AB(self.0.map(|i| !i))
    }
    fn inc(self) -> Option<Self> {
        u8_be::add(self.0, u8_be::one()).map(AB)
    }
    fn decr(self) -> Option<Self> {
        u8_be::sub(self.0, u8_be::one()).map(AB)
    }
    fn overflowing_add(self, rhs: Self) -> (Self, bool)  {
        let (v,over) = UInt::overflowing_add(self.to_u128(), rhs.to_u128());
        (v.into(),over)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        UInt::checked_sub(self.to_u128(),other.to_u128()).map(Self::from_u128)
    }
    fn checked_add(self, other: Self) -> Option<Self> {
        UInt::checked_add(self.to_u128(),other.to_u128()).map(Self::from_u128)
    }

    fn overflowing_shl(self, rhs: u32) -> (Self, bool)  {
        let (v,over) = UInt::overflowing_shl(self.to_u128(), rhs);
        (v.into(),over)
    }

    fn overflowing_shr(self, rhs: u32) -> (Self, bool)  {
        let (v,over) = UInt::overflowing_shr(self.to_u128(), rhs);
        (v.into(),over)
    }

    fn leading_zeros(&self) -> u32  {
        UInt::leading_zeros(&self.to_u128())
    }
    fn trailing_zeros(&self) -> u32  {
        UInt::trailing_zeros(&self.to_u128())
    }

    fn trailing_ones(&self) -> u32  {
        UInt::trailing_ones(&self.to_u128())
    }

    fn leading_ones(&self) -> u32  {
        UInt::leading_ones(&self.to_u128())
    }

}

pub const fn one<const N: usize>() -> [u64; N] {
    let mut r = [0; N];
    r[0] += 1;
    r
}
use linkspace_pkt::{ruint::Uint, U256, U512};

macro_rules! ruint_impl {
    ($uid:ident) => {
        impl UInt for $uid {
            const MIN: Self = $uid::ZERO;
            const MAX: Self = $uid::MAX;
            const ONE: Self = $uid::from_limbs(one());
            const BITS: u32 = $uid::BITS as u32;
            fn not(self) -> Self {
                Not::not(self)
            }
            fn bit_and(self, other: Self) -> Self {
                self & other
            }
            fn bit_or(self, other: Self) -> Self {
                self | other
            }
            fn checked_add(self, o: Self) -> Option<Self> {
                self.checked_add(o)
            }
            fn checked_sub(self, o: Self) -> Option<Self> {
                self.checked_sub(o)
            }
            fn inc(self) -> Option<Self> {
                self.checked_add(Self::ONE)
            }

            fn overflowing_add(self, rhs: Self) -> (Self, bool) {
                self.overflowing_add(rhs)
            }

            fn decr(self) -> Option<Self> {
                self.checked_sub(Self::ONE)
            }

            fn overflowing_shl(self, rhs: u32) -> (Self, bool) {
                let (over, mask) = (rhs as usize / Self::BITS, rhs as usize % Self::BITS);
                (self.overflowing_shl(mask as usize).0, over > 0)
            }

            fn overflowing_shr(self, rhs: u32) -> (Self, bool) {
                let (over, mask) = (rhs as usize / Self::BITS, rhs as usize % Self::BITS);
                (self.overflowing_shr(mask as usize).0, over > 0)
            }

            fn leading_zeros(&self) -> u32 {
                Uint::leading_zeros(&self) as u32
            }

            fn trailing_zeros(&self) -> u32 {
                Uint::trailing_zeros(&self) as u32
            }

            fn trailing_ones(&self) -> u32 {
                Uint::trailing_ones(&self) as u32
            }

            fn leading_ones(&self) -> u32 {
                Uint::leading_ones(&self) as u32
            }

            fn as_be_bytes(&self, _out: &mut dyn FnMut(&[u8])) {
                _out(&self.to_be_bytes_vec())
            }
        }
    };
}

#[test]
pub fn wtf() {}

ruint_impl!(U256);
ruint_impl!(U512);

/// Big endian bytes.
pub mod u8_be {
    #[inline(always)]
    pub fn cmp<const N: usize>(lhs: [u8; N], rhs: [u8; N]) -> std::cmp::Ordering {
        let mut i = 0;
        loop {
            let c = lhs[i].cmp(&rhs[i]);
            if c != std::cmp::Ordering::Equal || i == N - 1 {
                return c;
            }
            i += 1;
        }
    }
    pub const fn zero<const N: usize>() -> [u8; N] {
        [0; N]
    }
    pub const fn one<const N: usize>() -> [u8; N] {
        let mut r = [0; N];
        r[N - 1] += 1;
        r
    }
    #[inline(always)]
    pub const fn add<const N: usize>(mut bytes: [u8; N], val: [u8; N]) -> Option<[u8; N]> {
        let mut carry = false;
        let mut idx = N - 1;
        loop {
            let (ni, nc) = bytes[idx].carrying_add(val[idx], carry);
            bytes[idx] = ni;
            carry = nc;
            if idx == 0 {
                break;
            }
            idx -= 1;
        }
        if carry {
            None
        } else {
            Some(bytes)
        }
    }
    #[inline(always)]
    pub const fn sub<const N: usize>(mut bytes: [u8; N], val: [u8; N]) -> Option<[u8; N]> {
        let mut carry = false;
        let mut idx = N - 1;
        loop {
            let (ni, nc) = bytes[idx].borrowing_sub(val[idx], carry);
            bytes[idx] = ni;
            carry = nc;
            if idx == 0 {
                break;
            }
            idx -= 1;
        }
        if carry {
            None
        } else {
            Some(bytes)
        }
    }

    #[inline(always)]
    pub fn add_one(bytes: &mut [u8]) -> Option<&mut [u8]> {
        let mut carry = true;
        let mut idx = bytes.len() - 1;
        loop {
            // TODO: break on no carry
            let (ni, nc) = bytes[idx].carrying_add(0, carry);
            bytes[idx] = ni;
            carry = nc;
            if idx == 0 {
                break;
            }
            idx -= 1;
        }
        if carry {
            None
        } else {
            Some(bytes)
        }
    }
    #[inline(always)]
    pub fn sub_one(bytes: &mut [u8]) -> Option<&mut [u8]> {
        let mut carry = true;
        let mut idx = bytes.len() - 1;
        loop {
            let (ni, nc) = bytes[idx].borrowing_sub(0, carry);
            bytes[idx] = ni;
            carry = nc;
            if idx == 0 {
                break;
            }
            idx -= 1;
        }
        if carry {
            None
        } else {
            Some(bytes)
        }
    }
}
