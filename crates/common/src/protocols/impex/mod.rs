// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/// WARN - currently in development.
/// A maping between a filesystem's state and linkspace using mostly datapoints
use std::ops::{FromResidual, Try};
pub mod blob;
#[cfg(feature = "fs")]
pub mod blobmap;

pub fn chunk_reader_try_fold<B, F, R, const LEN: usize>(
    mut r: impl std::io::Read,
    init: B,
    mut f: F,
) -> R
where
    F: FnMut(B, &[u8]) -> R,
    R: Try<Output = B> + FromResidual<Result<std::convert::Infallible, std::io::Error>>,
{
    let mut buf = [0; LEN];
    let mut at = 0;
    let mut acc = init;
    loop {
        let i = r.read(&mut buf[at..])?;
        at += i;
        if at == LEN || i == 0 {
            acc = f(acc, &buf[..at])?;
        }
        if i == 0 {
            return try { acc };
        }
        if at == LEN {
            at = 0
        }
    }
}
