// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::consts::B64_HASH_LENGTH;
pub use arrayvec;
use arrayvec::ArrayString;
use base64_crate::prelude::*;
use linkspace_pkt::{base64_crate, LkHash};
use std::{convert::TryFrom, fmt, str::FromStr};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Base 64 Error")]
    B64Decode(#[from] base64_crate::DecodeError),
    #[error("To many chars")]
    MaxLengthError,
    #[error("To few chars")]
    MinLengthError,
}

/*
A partial b64 hash.
Note that this is a string comparison.
b64 decoding rules regarding 1 or 2 characters are ignored
*/

#[derive(Default, Debug, Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord)]

pub struct PartialHash(pub ArrayString<B64_HASH_LENGTH>);
impl PartialHash {
    pub fn min() -> PartialHash {
        PartialHash::from_str("AAAA").unwrap()
    }
    pub fn complete_try_into(&self) -> Option<linkspace_pkt::LkHash> {
        if !self.0.is_full() {
            return None;
        }
        let mut res = [0; 32];
        BASE64_URL_SAFE_NO_PAD
            .decode_slice_unchecked(self.0.as_bytes(), res.as_mut_slice())
            .ok()?;
        Some(res.into())
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub fn str_matches(&self, hash: &LkHash) -> bool {
        let t = self.0.as_str();
        hash.b64().starts_with(t)
    }
    pub fn str_greater_then(&self, hash: &LkHash) -> bool {
        let t = self.0.as_str().as_bytes();
        let b64 = hash.b64();
        let ok = &b64.as_bytes()[0..t.len()];
        t > ok
    }

    // Gives a good starting point for searching in an ordered list of Hashes's
    // Because b64 encodes 6 bits per char we only give an aprox.
    // i.e. while searching dont forget to skip while cursor.b64() < partial.
    pub fn aprox_btree_idx(&self) -> [u8; 32] {
        let mut res = [0; 32];
        let mut st = [b'A'; B64_HASH_LENGTH];
        st[0..self.0.as_bytes().len()].copy_from_slice(self.0.as_bytes());

        let st = unsafe { std::str::from_utf8_unchecked(&st) };
        BASE64_URL_SAFE_NO_PAD
            .decode_slice_unchecked(st, res.as_mut_slice())
            .unwrap();
        res
    }
    pub fn try_from_strlike(v: impl AsRef<[u8]>) -> Result<PartialHash, Error> {
        let b64 = v.as_ref();
        if b64.len() < 4 || b64.len() > B64_HASH_LENGTH {
            return Err(Error::MaxLengthError);
        }
        let mut b = [b'A'; B64_HASH_LENGTH];
        b[..b64.len()].copy_from_slice(b64);
        // TODO , just  check character table
        BASE64_URL_SAFE_NO_PAD.decode(&b[0..b64.len() & !3usize])?;
        let mut arr = ArrayString::from_byte_string(&b).unwrap();
        arr.truncate(b64.len());
        Ok(PartialHash(arr))
    }
}
impl From<LkHash> for PartialHash {
    fn from(hash: LkHash) -> Self {
        PartialHash(ArrayString::from(&hash.b64()).expect("Hash must fit"))
    }
}

impl TryFrom<&str> for PartialHash {
    type Error = Error;
    fn try_from(b64: &str) -> Result<PartialHash, Self::Error> {
        PartialHash::try_from_strlike(b64)
    }
}

impl FromStr for PartialHash {
    type Err = Error;
    fn from_str(s: &str) -> Result<PartialHash, Self::Err> {
        PartialHash::try_from_strlike(s)
    }
}
impl fmt::Display for PartialHash {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}_?", self.0)
    }
}

#[test]
fn comp() {
    let id: LkHash = crate::consts::PUBLIC;
    let b = id.b64();
    println!("ORIGINAL {}", b);
    let tid: LkHash = id;
    assert_eq!(id, tid);
    let it = (4..b.len())
        .rev()
        .map(|i| PartialHash::try_from(&b[0..i]).unwrap());
    for part in it {
        println!("str cmp {} {:?} ", part.str_matches(&id), part.0);
        let bytes = part.aprox_btree_idx();
        println!(
            "u8 cmp {} \n{:?} \n{:?} ",
            id.starts_with(&bytes),
            bytes,
            id
        );
    }
}
