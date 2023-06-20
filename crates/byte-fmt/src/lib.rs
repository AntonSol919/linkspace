// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(incomplete_features)]
#![feature(
    ptr_from_ref,
    array_windows,
    slice_flatten,
    int_roundings,
    generic_const_exprs,
    bigint_helper_methods,
    const_bigint_helper_methods,
    const_option_ext
)]
pub mod endian_types;

pub use base64 as base64_crate;
use base64_crate::DecodeError;
use core::ops::{Deref, DerefMut};
use std::{
    borrow::{Borrow, Cow},
    mem::size_of,
    str::FromStr, ptr,
};


pub use abe;
pub use abe::eval;
use abe::{
    abtxt::{ABTxtError, MAX_STR, as_abtxt},
    ast::{no_ctrs, MatchError},
    cut_prefix_nulls, cut_prefixeq,
    eval::{ABList },
    fit_back, thiserror, ABEValidator, FitSliceErr, ToABE, ABE,
};

use std::fmt::{self, Debug, Display};

pub fn as_abtxt_c(mut b: &[u8], cut: bool) -> std::borrow::Cow<'_, str> {
    if cut {
        b = cut_prefix_nulls(b)
    }
    abe::abtxt::as_abtxt(b)
}

pub fn ab_slice<X>(i: &[X]) -> &[AB<X>] {
    unsafe { std::slice::from_raw_parts(i.as_ptr().cast(), i.len())}
}

use base64_crate::prelude::*;

pub fn b64(b: &[u8], mini: bool) -> String {
    if mini{
        mini_b64(b)
    } else {
        base64(b)
    }
}
pub use abe::scope::base::{base64,base64_decode,mini_b64};

use bytemuck::{Pod,Zeroable};

#[derive(Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord,Pod,Zeroable)]
#[repr(transparent)]
/// newtype around bytes to print/parse [[abe]] text
pub struct AB<N: ?Sized = Vec<u8>>(pub N);
#[derive(Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord,Pod,Zeroable)]
#[repr(transparent)]
/// newtype around bytes to print/parse b64 (url-safe no-padding)
pub struct B64<N = [u8; 32]>(pub N);

impl<'o> From<&'o [u8]> for &'o AB<[u8]> {
    fn from(d: &'o [u8]) -> Self {
        AB::from_ref(d)
    }
}
impl<N:?Sized> AB<N>{
    pub fn from_ref(b: &N) -> &AB<N> {
        unsafe { std::mem::transmute(b)}
    }
}

impl<N> AB<N>
where
    Self: AsRef<[u8]>,
{
    pub fn cut_prefix_nulls(&self) -> &[u8] {
        cut_prefix_nulls(self.as_ref())
    }
    pub fn as_ref_cut(&self, cut_nulls: bool) -> &[u8] {
        if cut_nulls {
            cut_prefix_nulls(self.as_ref())
        } else {
            self.as_ref()
        }
    }
    pub fn as_str(&self, cut_nulls: bool) -> Cow<str> {
        abe::abtxt::as_abtxt(self.as_ref_cut(cut_nulls))
    }

    // deterimine prefix mode and cut
    fn x_prefix_cut(&self) -> (bool, &[u8]) {
        let b = self.as_ref();
        if b.first().copied().unwrap_or(0) == 255 {
            (true, cut_prefixeq::<255>(b))
        } else {
            (false, cut_prefix_nulls(b))
        }
    }
}
/*
impl<N: PartialEq> PartialEq<str> for AB<N>
where
    Self: FromStr,
{
    fn eq(&self, other: &str) -> bool {
        match Self::from_str(other) {
            Ok(v) => self.0.eq(&v.0),
            Err(_) => {
                panic!("string equality check for AB ( e.g. Domain, Tag) must be a valid abtxt")
            }
        }
    }
}
*/
impl<N> ToABE for AB<N>
where
    Self: AsRef<[u8]>,
{
    fn write_abe(&self, out: &mut dyn FnMut(ABE)) {
        let (max_padded, bytes) = self.x_prefix_cut();
        let mode = if max_padded { MAX_STR } else { "a" };
        let st = abe::abtxt::as_abtxt(bytes);
        let st : &str= st.borrow();
        if self.as_ref().len() == 16 {
            abe::abe!({ mode : st  }).for_each(out)
        } else {
            abe::abe!({ mode : st : (self.as_ref().len().to_string()) }).for_each(out)
        }
    }
    fn to_abe_str(&self) -> String {
        let (max_padded, bytes) = self.x_prefix_cut();
        let mode = if max_padded { MAX_STR } else { "a" };
        let st = abe::abtxt::as_abtxt(bytes);
        let st : &str= st.borrow();
        if self.as_ref().len() == 16 {
            format!("[{mode}:{st}]")
        } else {
            format!("[{mode}:{st}:{}]",self.as_ref().len())
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub struct ParseErr<E: std::error::Error> {
    pub source: E,
    pub st: String,
}
impl<E: std::error::Error> Display for ParseErr<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {}", self.st, self.source)
    }
}

impl<'o> AB<&'o [u8]> {
    pub fn parse_ctx<V>(&self) -> Result<V, ParseErr<<V as FromStr>::Err>>
    where
        V: FromStr,
        <V as FromStr>::Err: std::error::Error,
    {
        let st = self.as_str(false);
        st.parse().map_err(|source| ParseErr {
            source,
            st: st.to_string(),
        })
    }
    pub fn parse<V>(&self) -> Result<V, <V as FromStr>::Err>
    where
        V: FromStr,
    {
        let st = self.as_str(false);
        st.parse()
    }
}
fn _fit() {
    let a: [u8; 2] = AB::try_fit_byte_slice(&[]).unwrap().0;
    assert_eq!(a, [0, 0]);
    let b: [u8; 2] = AB::try_fit_byte_slice(&[1]).unwrap().0;
    assert_eq!(b, [0, 1]);
    let c: [u8; 2] = AB::try_fit_byte_slice(&[1, 1]).unwrap().0;
    assert_eq!(c, [1, 1]);
    let d = AB::<[u8; 2]>::try_fit_byte_slice(&[1, 1, 1]);
    assert!(d.is_err())
}

/**
copy `val` into array of N bytes prepending 0's as needed. panic if val > N

```
    assert_eq!(ab::<8>("abcd"),AB([0,0,0,0,97,98,99,100]));
```
**/
pub const fn ab<const N: usize>(val: &[u8]) -> AB<[u8; N]> {
    match AB::try_fit_byte_slice(val) {
        Ok(o) => o,
        Err(_e) => panic!("cant fit into 16 bytes"),
    }
}
pub const fn try_ab<const N: usize>(val: &[u8]) -> Result<AB<[u8; N]>, FitSliceErr> {
    AB::try_fit_byte_slice(val)
}
/// [ab] with \xff as padding
pub const fn abx<const N: usize>(val: &[u8]) -> AB<[u8; N]> {
    match AB::try_fit_slice_filled::<255>(val) {
        Ok(o) => o,
        Err(_e) => panic!("cant fit into 16 bytes"),
    }
}

impl<const L: usize> AB<[u8; L]> {
    pub fn utf8(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(self.cut_prefix_nulls())
    }
    /// [Self::utf8] but fallback to abtxt
    pub fn try_utf8(&self) -> Cow<str> {
        self.utf8()
            .map(Cow::Borrowed)
            .unwrap_or_else(|_v| self.as_str(true))
    }
    pub const fn try_fit_byte_slice(slice: &[u8]) -> Result<Self, FitSliceErr> {
        match fit_back::<0, L>(slice) {
            Ok(o) => Ok(AB(o)),
            Err(e) => Err(e),
        }
    }
    pub const fn try_fit_slice_filled<const FILL: u8>(slice: &[u8]) -> Result<Self, FitSliceErr> {
        match fit_back::<FILL, L>(slice) {
            Ok(o) => Ok(AB(o)),
            Err(e) => Err(e),
        }
    }
    pub fn parse_abtxt(st: impl AsRef<[u8]>) -> Result<Self, ABTxtError> {
        let mut this = [0; L];
        let i = abe::abtxt::parse_abtxt_into(st.as_ref(), &mut this)?;
        Self::try_fit_byte_slice(&this[..i]).map_err(ABTxtError::FitSlice)
    }
}
impl<const L: usize> TryFrom<ABList> for AB<[u8; L]> {
    type Error = FitSliceErr;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        let bytes = value.as_exact_bytes().map_err(|_| FitSliceErr {
            size: Some(L),
            got: Err("Delimited bytes"),
        })?;
        Self::try_fit_byte_slice(bytes)
    }
}
impl<N> From<AB<N>> for ABList
where
    AB<N>: AsRef<[u8]>,
{
    fn from(val: AB<N>) -> Self {
        val.as_ref().to_vec().into()
    }
}
impl<const L: usize> ABEValidator for AB<[u8; L]> {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        no_ctrs(b)?;
        Ok(())
    }
}


impl<N: AsRef<[u8]>> AsRef<[u8]> for AB<N> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl<N> AsRef<N> for AB<N> {
    fn as_ref(&self) -> &N {
        &self.0
    }
}
impl<N: Borrow<[u8]>> Borrow<[u8]> for AB<N> {
    fn borrow(&self) -> &[u8] {
        self.0.borrow()
    }
}
impl<N> Deref for AB<N> {
    type Target = N;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<N> DerefMut for AB<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<N> From<N> for AB<N> {
    fn from(v: N) -> Self {
        AB(v)
    }
}
impl<const L: usize> TryFrom<&[u8]> for AB<[u8; L]> {
    type Error = FitSliceErr;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::try_fit_byte_slice(value)
    }
}
impl std::ops::BitXor for B64<[u8; 32]> {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        unsafe {
            let [a1, b1]: [u128; 2] = std::mem::transmute(self.0);
            let [a2, b2]: [u128; 2] = std::mem::transmute(rhs);
            std::mem::transmute([a1.bitxor(a2), b1.bitxor(b2)])
        }
    }
}

impl FromStr for AB<Vec<u8>> {
    type Err = ABTxtError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        abe::abtxt::parse_abtxt_upto_max(s, 65000).map(AB)
    }
}
impl<const N: usize> FromStr for AB<[u8; N]> {
    type Err = ABTxtError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_abtxt(s)
    }
}

impl<X> Display for AB<X>
where
    Self: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&as_abtxt(self.as_ref()))
    }
}
impl<X> Debug for AB<X>
where
    Self: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&as_abtxt(self.as_ref()))
    }
}

impl<N: AsRef<[u8]>> AsRef<[u8]> for B64<N> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl<N> AsRef<N> for B64<N> {
    #[inline(always)]
    fn as_ref(&self) -> &N {
        &self.0
    }
}

impl AsRef<[u8; 32]> for B64<[u128; 2]> {
    fn as_ref(&self) -> &[u8; 32] {
        unsafe { &*std::ptr::from_ref(self).cast()}
    }
}
impl<N> ToABE for B64<N>
where
    Self: AsRef<[u8]>{

    fn write_abe(&self, out: &mut dyn FnMut(ABE)) {
        abe::abe!( { "b" : (self.b64()) } ).for_each(out)
    }

    fn to_abe_str(&self) -> String {
        format!("[b:{}]",self)
    }
}

impl<N> B64<N> {
    #[inline(always)]
    pub fn into_bytes(self) -> [u8; size_of::<Self>()] {
        unsafe { *(ptr::from_ref(&self).cast::<[u8;size_of::<Self>()]>()) }
    }
    pub fn inner(self) -> N {
        self.0
    }
    pub fn from_ref(b: &N) -> &B64<N> {
        unsafe { &*ptr::from_ref(b).cast() }
    }
}

impl<const L: usize> TryFrom<ABList> for B64<[u8; L]> {
    type Error = DecodeError;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        let bytes = value.as_exact_bytes().map_err(|_| {
            let (l, b) = value.lst.first().unwrap();
            DecodeError::InvalidByte(l.len() + 1, b.unwrap() as u8)
        })?;
        Self::try_fit_bytes_or_b64(bytes)
    }
}
impl<N> From<B64<N>> for ABList
where
    B64<N>: AsRef<[u8]>,
{
    fn from(val: B64<N>) -> Self {
        val.as_ref().to_vec().into()
    }
}

impl<const L: usize> ABEValidator for B64<[u8; L]> {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        no_ctrs(b)?;
        Ok(())
    }
}


impl<N> B64<N>
where
    Self: AsRef<[u8]>,
{
    pub fn b64(&self) -> String {
        base64(self.as_ref())
    }
    pub fn b64_mini(&self) -> String {
        mini_b64(self.as_ref())
    }
    pub fn b64_into(&self, output_buf:&mut String){
        BASE64_URL_SAFE_NO_PAD.encode_string(self.as_ref(), output_buf)
    }
}


impl<N> Borrow<N> for B64<N> {
    fn borrow(&self) -> &N {
        &self.0
    }
}
impl<N> From<N> for B64<N> {
    fn from(v: N) -> Self {
        B64(v)
    }
}

impl<N> Deref for B64<N> {
    type Target = N;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<N> DerefMut for B64<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl B64<Vec<u8>> {
    pub fn parse_str(v: impl AsRef<[u8]>) -> Result<Self, B64Error> {
        base64_decode(v).map(B64)
    }
}

pub use base64::DecodeError as B64Error;
impl<const N: usize> B64<[u8; N]> {
    pub fn try_fit_bytes_or_b64(slice: &[u8]) -> Result<Self, B64Error> {
        match Self::parse_str(slice) {
            Ok(e) => Ok(e),
            Err(e) => Self::try_fit_slice(slice).map_err(|_| e),
        }
    }

    pub fn try_fit_slice(slice: &[u8]) -> Result<Self, FitSliceErr> {
        abe::fit(slice).map(B64)
    }
    pub fn parse_str(st: impl AsRef<[u8]>) -> Result<Self, base64::DecodeError> {
        let s = st.as_ref();
        if s.len() != (N * 4).div_ceil(3) {
            return Err(base64::DecodeError::InvalidLength);
        }
        let mut this: [u8; N] = [0; N];
        let r = BASE64_URL_SAFE_NO_PAD.decode_slice_unchecked(s,&mut this)?;
        debug_assert!(r == N);
        Ok(B64(this))
    }
}
impl<const L: usize> TryFrom<&[u8]> for B64<[u8; L]> {
    type Error = FitSliceErr;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::try_fit_slice(value)
    }
}


impl<const N: usize> FromStr for B64<[u8; N]> {
    type Err = base64::DecodeError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse_str(s)
    }
}
impl<N> Display for B64<N>
where
    Self: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        base64_crate::display::Base64Display::new(&self.as_ref(), &BASE64_URL_SAFE_NO_PAD).fmt(f)
    }
}
impl<N> std::fmt::Debug for B64<N>
where
    Self: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}



impl AB<[u8;16]> {
    pub const fn to_u128(self) -> u128 {
        u128::from_be_bytes(self.0)
        
    }
    pub const fn from_u128(value:u128) -> Self {
        AB(value.to_be_bytes())
        
    }
}
impl From<AB<[u8; 16]>> for u128 {
    fn from(val: AB<[u8; 16]>) -> Self {
        val.to_u128()
    }
}
impl From<u128> for AB<[u8; 16]> {
    fn from(value: u128) -> Self {
        Self::from_u128(value)
    }
}

impl B64<[u8;32]>{
    pub fn to_u256(self) -> U256{
        U256::from_be_bytes(self.0)
    }
    pub fn from_u256(value:U256) -> Self {
        B64(value.to_be_bytes())
    }
}

pub use ruint;
pub use ruint::aliases::{U256, U512};
impl From<B64<[u8; 32]>> for U256 {
    fn from(val: B64<[u8; 32]>) -> Self {
        val.to_u256()
    }
}
impl From<U256> for B64<[u8; 32]> {
    fn from(val: U256) -> Self {
        Self::from_u256(val)
    }
}

impl From<B64<[u8; 64]>> for U512 {
    fn from(val: B64<[u8; 64]>) -> Self {
        U512::from_be_bytes(val.0)
    }
}
impl From<U512> for B64<[u8; 64]> {
    fn from(val: U512) -> Self {
        B64(val.to_be_bytes())
    }
}

#[test]
fn casts(){
    let s = ab_slice(&[b"abc",b"123"]);
    assert_eq!(s[0].0 , b"abc");
    assert_eq!(s[1].0 , b"123");
    assert_eq!("abc",AB::from_ref(b"abc").to_string());
    assert_eq!(B64([1;32]) ^ B64([3;32]),B64([2;32]));
    assert_eq!(B64([1;32]) ^ B64([3;32]),B64([2;32]));
    assert_eq!(B64([1u8;32]).into_bytes(), [1;32]);
    assert_eq!(&[32;32],&B64::from_ref(&[32;32]).0);


    let o : &[u8;32] = B64([1u128;2]).as_ref();
    let i = u128::from_ne_bytes(o[0..16].try_into().unwrap());
    assert_eq!(i,1);
    panic!("ok")
}
