// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    slice_as_chunks,
    split_as_slice,
    split_array,
    try_trait_v2,
    iterator_try_collect,
    array_chunks,
    associated_type_bounds,
    type_alias_impl_trait,
    bigint_helper_methods
)]
pub mod abe_macro;
pub mod abtxt;
pub mod ast;
pub mod convert;
pub mod eval;
pub mod scope;
pub use thiserror;

use std::error::Error;
use std::fmt::{Debug, Display};

pub use ast::{parse_abe, parse_abe_b, print_abe, ABE};
pub use convert::{ABEValidator, ToABE, TypedABE};

pub fn cut_ending_nulls2(b: &[u8]) -> &[u8] {
    match b.iter().rposition(|v| *v != 0) {
        None => &[],
        Some(i) => &b[0..=i],
    }
}
pub fn cut_prefix_nulls(b: &[u8]) -> &[u8] {
    cut_prefixeq::<0>(b)
}
pub fn cut_prefixeq<const BYTE: u8>(b: &[u8]) -> &[u8] {
    match b.iter().position(|v| *v != BYTE) {
        None => &[],
        Some(i) => &b[i..],
    }
}

pub const fn fit<const L: usize>(slice: &[u8]) -> Result<[u8; L], FitSliceErr> {
    if slice.len() != L {
        return Err(FitSliceErr {
            size: Some(L),
            got: Ok(slice.len()),
        });
    }
    fit_front(slice)
}
pub const fn fit_back<const FILL: u8, const L: usize>(
    slice: &[u8],
) -> Result<[u8; L], FitSliceErr> {
    if slice.len() > L {
        return Err(FitSliceErr {
            size: Some(L),
            got: Ok(slice.len()),
        });
    }
    let mut v = [FILL; L];
    let d = L - slice.len();
    let mut i = 0;
    while i < L - d {
        v[d + i] = slice[i];
        i += 1;
    }
    Ok(v)
}
pub const fn fit_front<const L: usize>(slice: &[u8]) -> Result<[u8; L], FitSliceErr> {
    if slice.len() > L {
        return Err(FitSliceErr {
            size: Some(L),
            got: Ok(slice.len()),
        });
    }
    let mut v = [0u8; L];
    let mut i = 0;
    while i < slice.len() {
        v[i] = slice[i];
        i += 1;
    }
    Ok(v)
}
#[derive(Debug, Copy, Clone)]
pub struct FitSliceErr {
    pub size: Option<usize>,
    // FIXME: This is a mistype. Ok(v) means got, Err(e) is an alternative description
    pub got: Result<usize, &'static str>,
}

impl Error for FitSliceErr {}
impl Display for FitSliceErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wrong number of bytes.")?;
        if let Some(v) = self.size {
            write!(f, " Expected {v}.")?;
        }
        match self.got {
            Ok(e) => write!(f, " Got {e}."),
            Err(e) => f.write_str(e),
        }
    }
}
