// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    thread_local,
    file_create_new,
    let_chains,
    try_blocks,
    io_error_other,
    const_option_ext,
    const_ptr_read,
    const_bigint_helper_methods,
    concat_idents,
    exact_size_is_empty,
    split_array,
    option_get_or_insert_default,
    array_zip,
    const_slice_index,
    const_try,
    const_option,
    cell_update,
    concat_bytes,
    bigint_helper_methods,
    control_flow_enum,
    try_trait_v2,
    type_alias_impl_trait,
    duration_constants,
    div_duration,
    never_type,
    drain_filter,
    hash_drain_filter,
    once_cell
)]
pub use parse_display;

pub use linkspace_cryptography as crypto;
pub use linkspace_pkt as pkt;
pub mod consts;
pub mod env;
pub mod eval;
pub mod matcher;
pub mod mut_header;
pub mod partial_hash;
pub mod predicate;
pub mod prelude;
pub mod pull;
pub mod query;
pub mod stamp_fmt;
pub mod stamp_range;


#[macro_export]
macro_rules! try_opt {
    ($expr:expr $(,)?) => {
        match $expr {
            core::result::Result::Ok(val) => val,
            core::result::Result::Err(err) => {
                return core::option::Option::Some(core::result::Result::Err(
                    core::convert::From::from(err),
                ));
            }
        }
    };
}

pub static LNS_ROOTS:&[u8] = include_bytes!("./lnsroots.pkt");
