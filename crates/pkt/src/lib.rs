// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(incomplete_features)]
#![feature(
    slice_split_at_unchecked,
    doc_notable_trait,
    thread_local,
    array_zip,
    slice_from_ptr_range,
    ptr_metadata,
    alloc_layout_extra,
    const_slice_split_at_not_mut,
    const_option,
    const_try,
    const_option_ext,
    const_slice_index,
    iter_advance_by,
    generic_const_exprs,
    write_all_vectored,
    lazy_cell
)]
pub use byte_fmt::*;
use core::mem::size_of;
use core::ops::Deref;
use core::slice::from_raw_parts;
use std::fmt::Display;
use serde::{Deserialize, Serialize};
pub use spath::*;
pub use spath_fmt::*;
use utils::as_bytes;

pub mod byte_segments;
pub mod eval;
pub mod exprs;
pub mod field_ids;
pub mod ipath;
pub mod link;
pub mod netpkt;
pub mod point;
pub mod point_parts;
pub mod point_ptr;
pub mod repr;
pub mod spath;
pub mod spath_fmt;
pub mod spath_macro;
pub mod utils;
pub mod read;
mod builder;
mod pkt_bytes;
mod stamp;



pub use byte_segments::*;
pub use endian_types::*;
pub use eval::*;
pub use exprs::*;
pub use field_ids::*;
pub use ipath::*;
pub use linkspace_cryptography::SigningKey;
pub use netpkt::*;
pub use point::*;
pub use point_parts::*;
pub use point_ptr::*;
pub use repr::*;
pub use spath::*;
pub use spath_fmt::*;
pub use stamp::*;
pub use builder::*;
pub use pkt_bytes::*;

#[cfg(test)]
pub mod asm_tests;


// ==== pkt field types
/// Blake3 hash of the packet content. Alias for `B64<[u8;32]\>`
pub type LkHash = B64<[u8; 32]>;
/// Alias for `B64<[u8;32]\>`
pub type Ptr = B64<[u8; 32]>;
/// Alias for `B64<[u8;32]\>`
pub type GroupID = B64<[u8; 32]>;
/// Alias for `AB<[u8;16]>`
pub type Domain = AB<[u8; 16]>;
/// Alias for `AB<[u8;16]>`
pub type Tag = AB<[u8; 16]>;

/// A [Tag] and [Ptr] 
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[repr(C, align(4))]
pub struct Link {
    pub tag: Tag,
    /// Usually a [LkHash], sometimes a [PubKey] or [GroupID]
    pub ptr: Ptr,
}
impl Display for Link{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"{}:{}",self.tag,self.ptr)
    }
}
/// Taproot Schnorr publickey. Alias for `B64<[u8;32]>`
pub type PubKey = B64<[u8; 32]>;
/// Taproot Schnorr signature
pub type Signature = B64<[u8; 64]>;
/// A Big endian u64 of microseconds since EPOCH
pub type Stamp = U64;

// === size/len constraints. By Convention '_size' is n bytes . '_len' is number of elements
pub mod consts {
    use super::*;
    use std::mem::size_of;

    pub const MIN_POINT_SIZE: usize = size_of::<LkHash>() + size_of::<PointHeader>();
    pub const MIN_LINKPOINT_SIZE: usize = MIN_POINT_SIZE + size_of::<LinkPointHeader>();
    pub const MIN_NETPKT_SIZE: usize = size_of::<NetPktHeader>() + MIN_POINT_SIZE;
    pub const MAX_NETPKT_SIZE: usize = u16::MAX as usize - 256;
    pub const MAX_POINT_SIZE: usize =
        MAX_NETPKT_SIZE - size_of::<NetPktHeader>() + size_of::<LkHash>();
    pub const MAX_CONTENT_SIZE: usize = MAX_POINT_SIZE - size_of::<PartialNetHeader>();
    pub const MAX_DATA_SIZE: usize = MAX_CONTENT_SIZE;
    pub const MAX_LINKS_LEN: usize = (MAX_POINT_SIZE - MAX_SPATH_SIZE) / size_of::<Link>();
    pub const MAX_SPATH_SIZE: usize = 242;
    pub const MAX_IPATH_SIZE: usize = MAX_SPATH_SIZE + 8;
    pub const MAX_SPATH_COMPONENT_SIZE: usize = 200;
    pub const MAX_PATH_LEN: usize = 8;
}
pub use consts::*;

/** General trait for accessing point field.

Fields are included in [LkHash].

Various fields are accessed through [[PointExt]]

Impl'ed by [[PointPtr]] and [[PointParts]]
[[PointPtr]] impls is a byte layout
[[PointParts]] impl is a rusty enum repr

**/
#[doc(notable_trait)]
pub trait Point: core::fmt::Debug {
    /// The rusty enum repr of a point.
    fn parts(&self) -> PointParts;
    fn fields(&self) -> PointFields {
        self.parts().fields
    }
    
    fn data(&self) -> &[u8];
    fn tail(&self) -> Option<Tail>;
    /// Return a LinkPointHeader, works for both key and link points.
    fn linkpoint_header(&self) -> Option<&LinkPointHeader>;
    fn keypoint_header(&self) -> Option<&KeyPointHeader>;
    /// A utility function to translate this format into bytes for hashing & io
    fn pkt_segments(&self) -> ByteSegments;
    fn point_header_ref(&self) -> &PointHeader;
}

impl<T: Point + ?Sized> PointExt for T {}

#[doc(notable_trait)]
/// Directly access a [Point]'s domain, group, links, publickey, etc.
pub trait PointExt
where
    Self: Point,
{
    fn fields(&self) -> PointFields {
        self.parts().fields
    }
    fn point_header(&self) -> PointHeader {
        *self.point_header_ref()
    }
    fn is_datapoint(&self) -> bool {
        self.as_datapoint().is_some()
    }
    fn is_linkpoint(&self) -> bool {
        self.as_linkpoint().is_some()
    }
    fn is_keypoint(&self) -> bool {
        self.as_keypoint().is_some()
    }
    fn as_datapoint(&self) -> Option<&[u8]> {
        if let PointFields::DataPoint(a) = self.parts().fields {
            return Some(a);
        }
        None
    }
    fn as_linkpoint(&self) -> Option<LinkPoint> {
        if let PointFields::LinkPoint(a) = self.parts().fields {
            return Some(a);
        }
        None
    }
    fn as_keypoint(&self) -> Option<KeyPoint> {
        if let PointFields::KeyPoint(a) = self.parts().fields {
            return Some(a);
        }
        None
    }

    fn group(&self) -> Option<&GroupID> {
        self.linkpoint_header().map(|v| &v.group)
    }
    fn get_group(&self) -> &GroupID {
        self.group().unwrap_or(&B64([0; 32]))
    }
    fn domain(&self) -> Option<&Domain> {
        self.linkpoint_header().map(|v| &v.domain)
    }
    fn get_domain(&self) -> &Domain {
        self.domain().unwrap_or(&AB([0; 16]))
    }
    fn create_stamp(&self) -> Option<&Stamp> {
        self.linkpoint_header().map(|v| &v.create_stamp)
    }
    fn get_create_stamp(&self) -> &Stamp {
        self.create_stamp().unwrap_or(&Stamp::ZERO)
    }
    fn signature(&self) -> Option<&Signature> {
        self.keypoint_header().map(|h| &h.signed.signature)
    }

    fn get_signature(&self) -> &Signature {
        self.signature().unwrap_or(&B64([0; 64]))
    }

    fn pubkey(&self) -> Option<&PubKey> {
        self.keypoint_header().map(|h| &h.signed.pubkey)
    }

    fn get_pubkey(&self) -> &PubKey {
        self.pubkey().unwrap_or(&B64([0; 32]))
    }

    fn ipath(&self) -> Option<&IPath> {
        self.tail().map(|v| v.ipath)
    }

    fn path(&self) -> Option<&SPath> {
        self.tail().map(|v| v.ipath.spath())
    }

    fn links(&self) -> Option<&[Link]> {
        self.tail().map(|v| v.links)
    }

    fn path_len(&self) -> Option<&u8> {
        self.tail().map(|t| t.ipath.path_len())
    }

    fn get_path_len(&self) -> &u8 {
        self.path_len().unwrap_or(&0)
    }

    fn get_ipath(&self) -> &IPath {
        self.ipath().unwrap_or(IPath::EMPTY)
    }

    fn get_spath(&self) -> &SPath {
        self.path().unwrap_or_else(|| SPath::empty())
    }

    fn get_data_str(&self) -> Result<&str, core::str::Utf8Error> {
        std::str::from_utf8(self.data())
    }
    fn get_links(&self) -> &[Link] {
        self.links().unwrap_or_default()
    }
    fn select(&self) -> SelectLink{ SelectLink(self.get_links())}
    fn compute_hash(&self) -> LkHash {
        linkspace_cryptography::hash_segments(&self.pkt_segments().0).into()
    }
    fn net_pkt_size(&self) -> usize {
        self.point_header_ref().net_pkt_size()
    }
}

use bitflags::bitflags;
bitflags! {
    #[derive(Serialize,Deserialize)]
    /// Pkt flag indicating its type.
    ///
    /// Only the _POINT combinations are valid in a packet.
    pub struct PointTypeFlags: u8 {
        /// Indicate that the chances of anybody interested in this packet are zero.
        /// Implementations can ignore this, mostly useful for importing many datablocks.
        const EMPTY = 0b00000000;
        const ANY_PKT = 0b00000001;
        const DATA = 0b00000001;
        const LINK = 0b00000010;
        const SIGNATURE = 0b00000100;
        const ERROR = 0b1000_0000;

        const DATA_POINT = Self::DATA.bits;
        const LINK_POINT = Self::DATA.bits | Self::LINK.bits;
        const KEY_POINT = Self::DATA.bits | Self::LINK.bits | Self::SIGNATURE.bits;
        const ERROR_POINT = Self::ERROR.bits;
    }
}
impl std::fmt::Display for PointTypeFlags {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl PointTypeFlags {
    pub const fn as_str(self) -> &'static str {
        match self {
            PointTypeFlags::DATA_POINT => "DataPoint",
            PointTypeFlags::LINK_POINT => "LinkPoint",
            PointTypeFlags::KEY_POINT => "KeyPoint",
            PointTypeFlags::ERROR_POINT => "ErrorPoint",
            _ => "UnknownPointType",
        }
    }
    pub fn unchecked_from(b: u8) -> Self {
        unsafe { std::mem::transmute(b) }
    }
}

#[track_caller]
#[deprecated]
pub const fn as_domain(b: &[u8]) -> Domain {
    ab(b)
}
#[track_caller]
#[deprecated]
pub const fn as_tag(b: &[u8]) -> Tag {
    match AB::try_fit_byte_slice(b) {
        Ok(o) => o,
        Err(_e) => panic!("cant fit into 16 bytes"),
    }
}

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("SPath segments are incorrectly set in the header")]
    SPathSegmentMismatch,
    #[error("An unknown pkt type {0} was encountered")]
    UnknownPktType(u8),
    #[error("The packet length does not agree with tail lengths")]
    TailLength,
    #[error("Invalid SPath")]
    SPath(#[from] crate::spath::PathError),
    #[error("Signed Invalid Pkt type")]
    SignedInvalidPkt,
    #[error("Assert and inner LinkPoint do not agree on length")]
    KeyPointLength,
    #[error("Hash does not match pkt")]
    HashMismatch,
    #[error("Assert holds invalid signature")]
    InvalidSignature,
    #[error("The memory for this packet has trailing data")]
    InvalidPktDataLength,
    #[error("Content len exceeds max")] // TODO ambigues use
    ContentLen,
    #[error("Too little data to repr a pkt")]
    MissingHeader,
    #[error("Reserved bits not null")]
    ReservedBitsSet,
    #[error("Links have a size of 48 bytes. The offset must be wrong")]
    IndivisableLinkbytes,
    #[error("Data offset should be between spi_offset and pkt_size")]
    DataOffsetIncompatible,
    #[error("ISPOffset should be after the header")]
    ISPOffsetIncompatible,
    #[error("The pointheader has its reserved bit set {0}")]
    HeaderReservedSet(u8),
}

/*
pub fn xor<const N:usize>(mut a: [u8;N],b:&[u8;N]) -> [u8;N]{
a.iter_mut().zip(b.iter()).for_each(|(a,b)| *a ^= b);
a
}


pub fn nbytes(nf: &impl NetFields) -> usize {
nf.pkt_header().pkt_size() + size_of::<PktHash>() + size_of::<RoutingHeader>()
}
pub fn nwords(nf: &impl NetFields) -> usize {
    (nbytes(nf) + 3) / 4
}
*/

pub trait SigningExt {
    fn pubkey(&self) -> PubKey;
}
impl SigningExt for SigningKey {
    fn pubkey(&self) -> PubKey {
        self.pubkey_bytes().into()
    }
}
