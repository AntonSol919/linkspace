// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(incomplete_features)]
#![feature(
    array_windows,
    slice_flatten,
    int_roundings,
    const_ptr_read,
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
    str::FromStr,
};


pub use abe;
pub use abe::eval;
use abe::{
    abtxt::{ABTxtError, MAX_STR, as_abtxt},
    ast::{as_bytes, no_ctrs, MatchError},
    cut_prefix_nulls, cut_prefixeq,
    eval::{ABList, ApplyResult, EScope, EvalScopeImpl, ScopeFunc, Comment},
    fit_back, fncs, thiserror, ABEValidator, FitSliceErr, ToABE, ABE,
};

use std::fmt::{self, Debug, Display};

pub fn as_abtxt_c(mut b: &[u8], cut: bool) -> std::borrow::Cow<'_, str> {
    if cut {
        b = cut_prefix_nulls(b)
    }
    abe::abtxt::as_abtxt(b)
}

pub fn ab_slice<X>(i: &[X]) -> &[AB<X>] {
    unsafe { std::mem::transmute(i) }
}

use base64_crate::prelude::*;
pub fn base64(b: impl AsRef<[u8]>) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(b.as_ref())
}
pub fn base64_decode(st: impl AsRef<[u8]>) -> Result<Vec<u8>, DecodeError> {
    BASE64_URL_SAFE_NO_PAD.decode(st.as_ref())
}

pub fn b64(b: &[u8], mini: bool) -> String {
    if mini{
        mini_b64(b)
    } else {
        base64(b)
    }
}
pub fn mini_b64(v: &[u8]) -> String {
    let mut r = String::with_capacity(10);
    let len = v.len();
    let padc = len / 8;
    let st = base64(v);
    if v.len() <= 12 {
        return st;
    }
    r.push_str(&st[0..6]);
    r.push_str(&":".repeat(padc / 2));
    if padc % 2 != 0 {
        r.push('.');
    }
    r.push_str(&st[st.len() - 2..]);
    r
}

#[derive(Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// newtype around bytes to print/parse [[abe]] text
pub struct AB<N: ?Sized = Vec<u8>>(pub N);
#[derive(Default, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
/// newtype around bytes to print/parse b64 (url-safe no-padding)
pub struct B64<N = [u8; 32]>(pub N);

impl<'o> From<&'o [u8]> for &'o AB<[u8]> {
    fn from(d: &'o [u8]) -> Self {
        unsafe { *(d.as_ptr() as *const Self) }
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
        unsafe { std::mem::transmute(self) }
    }
}
impl<N> ToABE for B64<N>
where
    Self: AsRef<[u8]>{

    fn write_abe(&self, out: &mut dyn FnMut(ABE)) {
        abe::abe!( { "b" : (self.b64()) } ).for_each(out)
    }
}

impl<N> B64<N> {
    #[inline(always)]
    pub fn into_bytes(self) -> [u8; size_of::<Self>()] {
        unsafe { *(&self as *const Self as *const [u8; size_of::<Self>()]) }
    }
    pub fn inner(self) -> N {
        self.0
    }
    pub fn from_ref(b: &N) -> &B64<N> {
        unsafe { std::mem::transmute(b) }
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
        if r != N {
            panic!()
        };
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
        f.write_str(&base64(self.as_ref()))
    }
}
impl<N> std::fmt::Debug for B64<N>
where
    Self: AsRef<[u8]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&base64(self.as_ref()))
    }
}
/// Implements [AAAAAA/b64] and [\0\0\xff/2b64]
#[derive(Copy, Clone, Debug)]
pub struct B64EvalFnc;
impl EvalScopeImpl for B64EvalFnc {
    fn about(&self) -> (String, String) {
        ("b64".into(), "base64 url-safe no-padding".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ("?b",1..=1,"encode base64",|_,i:&[&[u8]]| Ok(base64(i[0]).into_bytes())),
            ("2mini",1..=1,"encode mini",|_,i:&[&[u8]]| Ok(mini_b64(i[0]).into_bytes())),
            ( @C "b", 1..=1, None, "decode base64",
               |_,i:&[&[u8]],_,_| Ok(base64_decode(i[0])?),
               |_,b:&[u8],opts:&[ABE]| -> ApplyResult<String>{
                   if opts.is_empty(){
                       return ApplyResult::Value(format!("[b:{}]",base64(b)));
                   }
                   for len_st in opts.iter().filter_map(|v| as_bytes(v).ok()){
                       let len = std::str::from_utf8(len_st)?.parse::<u32>()?;
                       if len as usize == b.len() {
                           return ApplyResult::Value(format!("[b:{}]",base64(b)));
                       }
                   }
                   ApplyResult::NoValue
               }
             )
        ])
    }
}

use abe::eval::{BytesFE, Encode, EvalCtx, Help, LogicOps, UIntFE};
pub type EvalCore = (
    (EScope<BytesFE>, EScope<UIntFE>, EScope<B64EvalFnc>),
    ((EScope<Comment>,EScope<Help>), EScope<LogicOps>, EScope<Encode>),
);
pub type EvalCoreCtx = EvalCtx<EvalCore>;
pub const EVAL_SCOPE: EvalCore = core_scope();
pub const fn core_scope() -> EvalCore {
    (
        (EScope(BytesFE), EScope(UIntFE), EScope(B64EvalFnc)),
        ((EScope(Comment),EScope(Help)), EScope(LogicOps), EScope(Encode)),
    )
}
pub const fn core_ctx() -> EvalCoreCtx {
    EvalCtx {
        scope: core_scope()
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
