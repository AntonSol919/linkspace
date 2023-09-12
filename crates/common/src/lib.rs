// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![recursion_limit = "64"]
#![feature(
    cell_update,
    trait_alias,
    thread_local,
    array_windows,
    ptr_metadata,
    split_as_slice,
    iterator_try_collect,
    lazy_cell,
    once_cell_try,
    duration_constants,
    control_flow_enum,
    type_alias_impl_trait,
    try_blocks,
    never_type,
    try_trait_v2,
    write_all_vectored,
    extract_if
)]

pub use anyhow;
pub use serde;

pub use abe;
pub use byte_fmt;

pub use linkspace_argon2_identity as identity;
pub use linkspace_core as core;
pub use linkspace_pkt as pkt;

pub mod dgp;
pub mod thread_local;
pub mod pkt_reader;
pub mod pkt_stream_utils;
pub mod prelude;
pub mod protocols;


#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "cli")]
pub mod predicate_aliases;

#[cfg(test)]
pub mod tests;


#[cfg(feature="runtime")]
pub mod runtime;
#[cfg(feature="runtime")]
pub mod static_env;
#[cfg(feature="runtime")]
pub mod eval;



pub fn saturating_cast(val:u32) -> i32{
    val.min(i32::MAX as u32) as i32
}
pub fn saturating_neg_cast(val:u32) -> i32{
    if val > (i32::MAX as u32 + 1) { i32::MIN} else { -(val as i32) }
}
