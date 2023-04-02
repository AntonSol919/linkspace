// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use abe::{ast::no_ctrs, thiserror, ABEValidator, ToABE};
use serde::{Deserialize, Serialize};

use std::{
    array::TryFromSliceError,
    cmp::Ordering,
    fmt::{Binary, Display},
    num::ParseIntError,
    ops::Deref,
    str::FromStr,
};

macro_rules! endian_number{
	  ($name:ident,$native:ident,$size:expr, $to_bytes:ident, $from_bytes:ident , $alt_endian:ident) => {
        #[derive(Copy, Clone, PartialEq,Eq, Serialize,Deserialize,Default,Hash)]
        #[repr(transparent)]
        pub struct $name(pub [u8;$size]);
        impl std::fmt::Debug for $name{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let id = stringify!($name).to_ascii_lowercase(); // FIXME: this be part of the template string
                if *self == Self::MAX { f.write_str(concat!("MAX",stringify!($ident)))}
                else { write!(f,"{}{}",self.get(),id)}
                //else { write!(f,concat!("{}",stringify!($ident)),self.get())}
            }
}
        impl From<$name> for $alt_endian {
            fn from(value: $name) -> Self{
                $alt_endian::new(value.get())
            }
        }
		    impl $name {
            #[inline(always)] pub const fn new(val: $native) -> $name{ $name(val.$to_bytes())}
            #[inline(always)] pub const fn get(self) -> $native{ $native::$from_bytes(self.0)}
            pub const MAX : $name = $name::new($native::MAX);
            pub const ZERO : $name = $name::new(0);
            #[must_use]
            #[inline(always)] pub const fn incr(self) -> $name { $name::new(self.get().saturating_add(1))}
            #[inline(always)] pub const fn align(self) -> [$native;1] { [$native::from_ne_bytes(self.0)]}
            #[inline(always)] pub const fn to_bytes(self) -> [u8;$size] { self.0}
            pub const fn saturating_sub(self, rhs:Self) -> Self {
                $name::new(self.get().saturating_sub(rhs.get()))
            }
            pub const fn saturating_add(self, rhs:Self) -> Self {
                $name::new(self.get().saturating_add(rhs.get()))
            }
        }

        impl Ord for $name{
            #[inline(always)]
            fn cmp(&self, other: &Self) -> Ordering {
                self.get().cmp(&other.get())
            }
        }
        impl PartialOrd for $name {
            #[inline(always)]
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        impl From<$native> for $name {
            #[inline(always)]
            fn from(v: $native) -> $name { $name::new(v)}
        }
        #[allow(clippy::from_over_into)]
        impl Into<$native> for $name {
            #[inline(always)]
            fn into(self) -> $native { self.get()}
        }
        impl From<[u8;$size]> for $name {
            #[inline(always)]
            fn from(v:[u8;$size]) -> $name { $name(v)}
        }
        impl TryFrom<&[u8]> for $name {
            type Error = TryFromSliceError;
            fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
                <[u8;$size]>::try_from(value).map($name::from)
            }
        }

        #[allow(clippy::from_over_into)]
        impl Into<[u8;$size]> for $name {
            fn into(self) -> [u8;$size] { self.0}
        }

        impl Binary for $name{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                binary_fmt_slice(&self.0,f)
            }
        }
        impl Display for $name{

            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                Display::fmt(&self.get(),f)
            }
        }
        impl FromStr for $name{
            type Err = ParseIntError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                <$native>::from_str(s).map($name::from)
            }
        }
        
        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] { &self.0}
        }

        impl std::ops::BitAnd for $name {
            type Output = Self;
            #[inline(always)]
            fn bitand(self, rhs: Self) -> Self::Output {
                $name($native::from_ne_bytes(self.0).bitand($native::from_ne_bytes(rhs.0)).to_ne_bytes())
            }
        }
        impl std::ops::BitOr for $name {
            type Output = Self;
            #[inline(always)]
            fn bitor(self, rhs: Self) -> Self::Output {
                $name($native::from_ne_bytes(self.0).bitor($native::from_ne_bytes(rhs.0)).to_ne_bytes())
            }
        }
        impl std::ops::Not for $name {
            type Output = Self;
            #[inline(always)]
            fn not(self) -> Self::Output {
                $name($native::from_ne_bytes(self.0).not().to_ne_bytes())
            }
        }
        impl Deref for $name{
            type Target = [u8;$size];
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        impl ToABE for $name{
            fn to_abe(&self) -> Vec<abe::ABE> {
                let type_name = stringify!($name).to_ascii_lowercase();
                abe::abev!( { type_name : (self.get().to_string())  } )
            }
        }

        impl ABEValidator for $name{
            fn check(b: &[abe::ABE]) -> Result<(),abe::ast::MatchError> {
                abe::ast::no_ctrs(b)?;
                Ok(())
            }
        }

	  };
}

pub fn binary_str(slice: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = "0b".to_string();
    for byte in slice {
        write!(&mut out, "_{byte:0>8b}").unwrap();
    }
    out
}
pub fn binary_fmt_slice(slice: &[u8], f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("0b")?;
    for byte in slice {
        write!(f, "_{byte:0>8b}")?;
    }
    Ok(())
}

use abe::eval::ABList;
use abe::FitSliceErr;

macro_rules! big_endian {
    ($name:ident,$native:ident,$size:expr,$alt_endian:ident) => {
        endian_number!(
            $name,
            $native,
            $size,
            to_be_bytes,
            from_be_bytes,
            $alt_endian
        );
        impl BigEndianUInt for $name {
            type Native = $native;
        }

        impl TryFrom<ABList> for $name {
            type Error = FitSliceErr;
            fn try_from(value: ABList) -> Result<Self, Self::Error> {
                let bytes = value.as_exact_bytes().map_err(|_| FitSliceErr {
                    size: Some($size),
                    got: Err("Delimited bytes"),
                })?;
                abe::fit(bytes).map($name)
            }
        }
        impl From<$name> for ABList {
            fn from(value: $name) -> ABList {
                value.0.to_vec().into()
            }
        }
        impl crate::AB<[u8; $size]> {
            pub fn uint(self) -> $name {
                $name(self.0)
            }
        }
    };
}

big_endian!(U16, u16, 2, LU16);
big_endian!(U32, u32, 4, LU32);
big_endian!(U64, u64, 8, LU64);
big_endian!(U128, u128, 16, LU128);

macro_rules! little_endian {
	  ($name:ident,$native:ident,$size:expr,$alt_endian:ident) => {
        endian_number!($name,$native,$size,to_le_bytes,from_le_bytes,$alt_endian);
        impl $name {
            #[inline(always)]
            pub const fn try_fit_slice(slice:&[u8]) -> Result<$name,FitSliceErr> {
                match abe::fit_front(slice){
                    Ok(o) => Ok($name(o)),
                    Err(e) => Err(e)
                }
            }

            pub fn lu_abe(&self) -> Vec<abe::ABE>{
                abe::abev!( { "lu" : (self.get().to_string())})
            }
        }
            impl TryFrom<ABList> for $name{
                type Error = FitSliceErr;
                fn try_from(value: ABList) -> Result<Self, Self::Error> {
                    let bytes = value.as_exact_bytes().map_err(|_| FitSliceErr{size: Some($size), got:Err("Delimited bytes")})?;
                    $name::try_fit_slice(bytes)
                }
            }

        impl From<$name> for ABList{
            fn from(value : $name) -> ABList {
                abe::cut_ending_nulls2(&value.0).to_vec().into()
            }
        }

    }
}

little_endian!(LU16, u16, 2, U16);
little_endian!(LU32, u32, 4, U32);
little_endian!(LU64, u64, 8, U64);
little_endian!(LU128, u128, 16, U128);

#[derive(thiserror::Error, Debug, Copy, Clone)]
pub enum TryFitSliceError {
    #[error("can't fit {got} bytes into value of {max} bytes")]
    Overflow { max: usize, got: usize },
}

pub trait BigEndianUInt
where
    Self: Copy
        + PartialEq
        + Eq
        + Ord
        + PartialOrd
        + std::fmt::Debug
        + Send
        + Sync
        + 'static
        + From<Self::Native>,
{
    type Native: Into<Self>;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct U8(pub u8);
impl U8 {
    pub const fn new(b: u8) -> U8 {
        U8(b)
    }
    pub fn abe_bits(self) -> Vec<abe::ABE> {
        abe::abev!({ "b2": (format!("{:0>8b}", self.0)) })
    }
}
impl TryFrom<ABList> for U8 {
    type Error = TryFitSliceError;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        let bytes = value.as_exact_bytes().unwrap_or_default();
        if bytes.len() != 1 {
            Err(TryFitSliceError::Overflow {
                max: 1,
                got: bytes.len(),
            })
        } else {
            Ok(U8(bytes[0]))
        }
    }
}
impl From<U8> for u8 {
    fn from(val: U8) -> Self {
        val.0
    }
}
impl From<U8> for ABList {
    fn from(val: U8) -> Self {
        vec![val.0].into()
    }
}
impl ABEValidator for U8 {
    fn check(b: &[abe::ABE]) -> Result<(), abe::ast::MatchError> {
        no_ctrs(b)?;
        Ok(())
    }
}
impl ToABE for U8 {
    fn to_abe(&self) -> Vec<abe::ABE> {
        abe::abev!( { "u8" : (self.0.to_string()) } )
    }
}

#[test]
fn fits() {
    assert_eq!(LU64::try_fit_slice(&[12]).unwrap().get(), 12)
}

#[test]
fn abe() {
    fn io<T: ABEValidator + ToABE + PartialEq + std::fmt::Debug>(val: T) {
        let ctx = crate::core_ctx();
        let abe = val.to_abe();
        println!("{:?}", abe);
        let evals = crate::abe::TypedABE::<T>::from_unchecked(abe)
            .eval(&ctx)
            .map_err(|_| "err")
            .unwrap();
        assert_eq!(evals, val);
        println!("{:?}", evals);
    }
    io(U8(24));
    io(LU16::from(24));
    io(LU32::from(24));
    io(LU64::from(24));
    io(LU128::from(24));
    io(U16::from(24));
    io(U32::from(24));
    io(U64::from(24));
    io(U128::from(24));
}
