// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![recursion_limit = "64"]
#![feature(
    trait_alias,
    thread_local,
    array_windows,
    io_error_other,
    ptr_metadata,
    split_as_slice,
    iterator_try_collect,
    btree_drain_filter,
    hash_drain_filter,
    cell_update,
    duration_constants,
    control_flow_enum,
    type_alias_impl_trait,
    try_blocks,
    never_type,
    try_trait_v2,
    once_cell,
    write_all_vectored,
    drain_filter
)]

pub use anyhow;
pub use serde;

pub use abe;
pub use byte_fmt;

pub use linkspace_argon2_identity as identity;
pub use linkspace_core as core;
pub use linkspace_pkt as pkt;

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(test)]
pub mod tests;

pub mod pkt_reader;
pub mod pkt_stream_utils;
pub mod prelude;
pub mod protocols;
pub mod runtime;
pub mod static_env;

pub mod dgp;
pub mod eval;
pub mod predicate_aliases;
