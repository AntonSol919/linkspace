// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    ptr_from_ref,
    extract_if,
    thread_local,
    file_create_new,
    let_chains,
    try_blocks,
    const_option_ext,
    const_bigint_helper_methods,
    concat_idents,
    exact_size_is_empty,
    split_array,
    option_get_or_insert_default,
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
    lazy_cell
)]
pub use parse_display;

pub use linkspace_cryptography as crypto;
pub use linkspace_pkt as pkt;
use pkt::NetPktPtr;
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

#[repr(align(32))]
pub struct StaticPkts<const N: usize>(pub [u8;N]);
pub static LNS_ROOTS: StaticPkts<520> = StaticPkts(*include_bytes!("./lnsroots.pkt"));
impl<const N:usize> StaticPkts<N>{
    pub fn iter<'o>(&'o self) -> impl Iterator<Item= &'o NetPktPtr> + 'o {
        let mut bytes = &self.0 as &[u8];
        std::iter::from_fn(move ||{
            if bytes.is_empty() { return None;}
            let pkt = crate::pkt::read::read_pkt(bytes, true).unwrap();
            match pkt{
                std::borrow::Cow::Borrowed(o) => {
                    bytes = &bytes[pkt::NetPktExt::size(&o) as usize..];
                    Some(o)
                },
                std::borrow::Cow::Owned(_) => panic!(),
            }
        })
    }
}
