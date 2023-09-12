// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![allow(incomplete_features)]
#![feature(
    iterator_try_collect,
    int_roundings,
    concat_bytes,
    exact_size_is_empty,
    pointer_is_aligned,
    ptr_from_ref,
    try_blocks,
    slice_split_at_unchecked,
    doc_notable_trait,
    thread_local,
    slice_from_ptr_range,
    ptr_metadata,
    alloc_layout_extra,
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
use byte_fmt::abe::FitSliceErr;
use core::mem::size_of;
use core::ops::Deref;
use core::slice::from_raw_parts;
use core::fmt::Display;
use serde::{Deserialize, Serialize};
pub use spath::*;
pub use spath_fmt::*;

pub mod byte_segments;
pub mod eval;
pub mod consts;
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
mod stamp;
pub mod build_info;



pub use consts::*;
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

pub mod asm_tests;


// ==== pkt field types
/// Blake3 hash of the packet content. Alias for `B64<[u8;32]\>`
pub type LkHash = B64<[u8; 32]>;
/// Alias for `B64<[u8;32]\>`
pub type GroupID = B64<[u8; 32]>;
/// Alias for `AB<[u8;16]>`
pub type Domain = AB<[u8; 16]>;
/// Alias for `AB<[u8;16]>`
pub type Tag = AB<[u8; 16]>;

/// A [Tag] and [LkHash] 
#[derive(Copy, Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[repr(C, align(4))]
pub struct Link {
    pub tag: Tag,
    /// Usually a [LkHash], sometimes a [PubKey] or [GroupID]
    pub ptr: LkHash,
}
impl From<(Tag,LkHash)> for Link {
    fn from((tag,ptr): (Tag,LkHash)) -> Self {
        Link{tag,ptr}
    }
}
impl TryFrom<(&str,LkHash)> for Link {
    type Error = FitSliceErr;

    fn try_from((tag,ptr): (&str,LkHash)) -> Result<Self, Self::Error> {
        Ok(Link{tag: Tag::try_fit_byte_slice(tag.as_bytes())?,ptr})
    }
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
    /// Points are padded with upto 7 \xFF bytes and are u64 aligned - this is accessible here for completeness sake.
    fn padding(&self) -> &[u8];
    /// Return a LinkPointHeader, works for both key and link points.
    fn linkpoint_header(&self) -> Option<&LinkPointHeader>;
    fn signed(&self) -> Option<&Signed> ;
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
    fn as_keypoint(&self) -> Option<(LinkPoint,Signed)> {
        if let PointFields::KeyPoint(a,b) = self.parts().fields {
            return Some((a,b));
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
        unwrap_or(self.domain(), &AB([0;16]))
    }
    fn create_stamp(&self) -> Option<&Stamp> {
        self.linkpoint_header().map(|v| &v.create_stamp)
    }
    fn get_create_stamp(&self) -> &Stamp {
        self.create_stamp().unwrap_or(&Stamp::ZERO)
    }
    fn signature(&self) -> Option<&Signature> {
        self.signed().map(|h| &h.signature)
    }

    fn get_signature(&self) -> &Signature {
        self.signature().unwrap_or(&B64([0; 64]))
    }

    fn pubkey(&self) -> Option<&PubKey> {
        self.signed().map(|h| &h.pubkey)
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

    fn get_path(&self) -> &SPath {
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
    

    fn check_private(&self) -> Result<(),crate::Error>{
        if self.group().copied() == Some(PRIVATE) { Err(crate::Error::PrivateGroup)}
        else {Ok(())}
    }
}

use bitflags::bitflags;
bitflags! {
    #[derive(Serialize,Deserialize,Copy,Clone,Debug,PartialEq,Eq,Ord,PartialOrd)]
    /// Pkt flag indicating its type.
    ///
    /// Only the _POINT combinations are valid in a packet.
    pub struct PointTypeFlags: u8 {
        /// Indicate that the chances of anybody interested in this packet are zero.
        /// Implementations can ignore this, mostly useful for importing many datablocks.
        const EMPTY = 0b0000_0000;
        const DATA = 0b000_00001;
        const LINK = 0b0000_0010;
        const SIGNATURE = 0b0000_0100;
        const ERROR = 0b1000_0000;

        const DATA_POINT = Self::DATA.bits();
        const LINK_POINT = Self::DATA.bits() | Self::LINK.bits();
        const KEY_POINT = Self::DATA.bits() | Self::LINK.bits() | Self::SIGNATURE.bits();
        const ERROR_POINT = Self::ERROR.bits();
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
        PointTypeFlags::from_bits(b).unwrap()
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
    #[error("an unknown pkt type {0} was encountered")]
    UnknownPktType(u8),
    #[error("the packet length does not agree with tail lengths")]
    TailLength,
    #[error("invalid path")]
    SPath(#[from] crate::spath::PathError),
    #[error("signed invalid pkt type")]
    SignedInvalidPkt,
    #[error("the signed and unsigned header do not agree on length")]
    KeyPointLength,
    #[error("hash does not match pkt")]
    HashMismatch,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("packet has trailing data")]
    InvalidPktDataLength,
    #[error("content len exceeds max")] // TODO ambigues use
    ContentLen,
    #[error("missing bytes {MIN_NETPKT_SIZE} required to read pkt size")]
    MissingHeader,
    #[error("reserved bits not null")]
    ReservedBitsSet,
    #[error("The padding bits aren't set right")]
    PaddingBitsNotU8Max,
    #[error("bad link size")]
    IndivisableLinkbytes,
    #[error("data offset should be between spi_offset and pkt_size")]
    DataOffsetIncompatible,
    #[error("offset should be after header")]
    ISPOffsetIncompatible,
    #[error("pointheader has reserved bit set {0}")]
    HeaderReservedSet(u8),
    #[error("missing bytes - pkt is {netpkt_size} long")]
    MissingBytes{netpkt_size:u16},
    #[error("the [#:0] group can't be used in this context")]
    PrivateGroup
}
impl Error {
    pub fn requires_more(self) -> Option<usize>{
        match self {
            Error::MissingHeader => Some(MIN_NETPKT_SIZE),
            Error::MissingBytes{netpkt_size} => Some(netpkt_size.into()),
            _ => None
        }
    }
    pub fn io(self) -> std::io::Error { std::io::Error::other(self)}
}
impl From<Error> for std::io::Error {
    fn from(val: Error) -> Self {
        val.io()
    }
}


pub trait SigningExt {
    fn pubkey(&self) -> PubKey;
}
impl SigningExt for SigningKey {
    fn pubkey(&self) -> PubKey {
        self.pubkey_bytes().into()
    }
}


// Seems to be better at generating cmov instructions
#[inline(always)]
pub const fn unwrap_or<'o,T>(opt:Option<&'o T>, mut default:&'o T) -> &'o T{
    if let Some(val) = opt {
        default = val;
    }
    default
}





