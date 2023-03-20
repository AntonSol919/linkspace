// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::fmt::{self, Debug};

use super::*;
use crate::utils::none;

// starting from the first pkt_byte, if the type is keypoint, this is the offset for the inner linkpoint pkt
pub(crate) const LINKPOINT_IN_KEYPOINT_OFFSET: usize = size_of::<PointHeader>()
    + size_of::<KeyPointHeader>()
    - size_of::<PointHeader>()
    - size_of::<LinkPointHeader>();

/// Flat [u8] repr of a [Point]
#[repr(C, align(4))]
pub struct PointPtr {
    pub(crate) pkt_header: PointHeader,
    content: [u8],
}
#[repr(C)]
pub struct PointThinPtr(pub(crate) PointHeader);

impl Debug for PointThinPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PointThin").field(&self.as_sized()).finish()
    }
}
impl PointThinPtr {
    //pub(crate) const EMPTY : Self = PointThinPtr(PointHeader::ERROR);
    #[inline(always)]
    pub fn point_header(&self) -> &PointHeader {
        &self.0
    }
    #[inline(always)]
    pub fn as_sized(&self) -> &PointPtr {
        unsafe {
            &*(std::ptr::from_raw_parts(
                self as *const PointThinPtr as *const (),
                self.0.content_size(),
            ))
        }
    }
}

impl fmt::Debug for PointPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let PointParts { pkt_header, fields } = self.parts();
        f.debug_struct("PktPtr")
            .field("pkt_header", &pkt_header)
            .field("fields", &fields)
            .finish()
    }
}

impl PointPtr {
    #[inline(always)]
    pub fn thin_point(&self) -> &PointThinPtr {
        unsafe { &*((&self.pkt_header) as *const PointHeader as *const PointThinPtr) }
    }
    /// # Safety
    ///
    /// should be well aligned valid netpktbytes.
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &PointPtr {
        PointThinPtr::from_bytes_unchecked(b).as_sized()
    }
    pub fn internal_consitent_length(&self) -> Result<usize, Error> {
        self.thin_point().internal_consitent_length()
    }
    #[inline(always)]
    pub fn pkt_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                self.pkt_header.point_size(),
            )
        }
    }
}
impl PointThinPtr {
    /// # Safety
    ///
    /// This is a unchecked cast, meaning accessing fields is dangerous
    /// to validate, first ensure you have enough bytes to read a PktHeader,
    /// Then call internal_consistent_length and compare the lengths
    /// finally call check_signature to ensure if its an assert pkt that its valid.
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &PointThinPtr {
        assert!(b.len() >= size_of::<PointHeader>(), "Never gone work");
        assert!(b.as_ptr().align_offset(4) == 0, "Unaligned cast");
        &*(b.as_ptr() as *const PointThinPtr)
    }
    #[inline(always)]
    fn fixed(&self) -> BarePointFields {
        let mem = &self.content_memory();
        match self.0.point_type {
            PointTypeFlags::DATA_POINT => BarePointFields::DataPoint(mem),
            PointTypeFlags::LINK_POINT => {
                let (linkpoint, tail) = mem.split_at(size_of::<LinkPointHeader>());
                let linkpoint = unsafe { &*(linkpoint.as_ptr() as *const LinkPointHeader) };
                BarePointFields::LinkPoint {
                    lp_header: linkpoint,
                    tail,
                }
            }
            PointTypeFlags::KEY_POINT => {
                let (assert, tail) = mem.split_at(size_of::<KeyPointHeader>());
                let assert = unsafe { &*(assert.as_ptr() as *const KeyPointHeader) };
                BarePointFields::KeyPoint {
                    a_header: assert,
                    tail,
                }
            }
            PointTypeFlags::ERROR_POINT => BarePointFields::Error(mem),
            _ => panic!("Working with invalid packets."),
        }
    }

    /// Check the header length fields and return the point size. This is the length without the hash.
    #[inline(always)]
    pub fn internal_consitent_length(&self) -> Result<usize, Error> {
        self.point_header().check()?;
        let point_size = self.point_header().point_size();
        match self.fixed() {
            BarePointFields::DataPoint(_) => (),
            BarePointFields::LinkPoint { lp_header, tail: _ } => {
                let isp_offset = lp_header.info.offset_ipath.get();
                let data_offset = lp_header.info.offset_data.get();
                if data_offset as usize > point_size {
                    return Err(Error::DataOffsetIncompatible);
                }
                if isp_offset > data_offset {
                    return Err(Error::DataOffsetIncompatible);
                }
                let link_size = isp_offset
                    .checked_sub((size_of::<PointHeader>() + size_of::<LinkPointHeader>()) as u16)
                    .ok_or(Error::ISPOffsetIncompatible)?;
                if link_size % size_of::<Link>() as u16 != 0 {
                    return Err(Error::IndivisableLinkbytes);
                }
                unsafe { self.unchecked_linkpoint_tail() }
                    .ipath
                    .check_components()?;
            }
            BarePointFields::KeyPoint {
                a_header: assert,
                tail: _,
            } => {
                let inner_linkpoint_size = self.point_header().content_size()
                    - size_of::<KeyPointPadding>()
                    - size_of::<PubKey>()
                    - size_of::<Signature>();
                if inner_linkpoint_size < MIN_LINKPOINT_SIZE {
                    return Err(Error::KeyPointLength);
                }
                if assert.inner_point.point_type != PointTypeFlags::LINK_POINT {
                    return Err(Error::SignedInvalidPkt);
                }
                let inner_lp_bytes = &self.pkt_bytes()[LINKPOINT_IN_KEYPOINT_OFFSET..];
                let linkpoint = unsafe { PointThinPtr::from_bytes_unchecked(inner_lp_bytes) };
                let lp_size = linkpoint.internal_consitent_length()?;
                if inner_linkpoint_size != lp_size + size_of::<LkHash>() {
                    return Err(Error::KeyPointLength);
                }
            }
            BarePointFields::Error(_) => (),
            BarePointFields::Unknown(_) => (),
        };
        Ok(point_size)
    }

    pub fn check_signature(&self) -> Result<(), Error> {
        if let BarePointFields::KeyPoint {
            a_header: assert,
            tail: _,
        } = self.fixed()
        {
            if assert.reserved != KeyPointPadding::default() {
                return Err(Error::ReservedBitsSet);
            }
            let inner_ptr = &assert.inner_point as *const PointHeader;
            let inner_linkpoint: &PointPtr = unsafe {
                &*std::ptr::from_raw_parts(
                    inner_ptr as *const (),
                    assert.inner_point.content_size(),
                )
            };
            let hash = inner_linkpoint.compute_hash();
            if hash != assert.signed.linkpoint_hash {
                return Err(Error::HashMismatch);
            }
            assert
                .signed
                .validate()
                .map_err(|_| Error::InvalidSignature)?;
        };
        Ok(())
    }
    fn self_ptr(&self) -> *const u8 {
        self as *const Self as *const u8
    }
    pub fn pkt_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                self.point_header().point_size(),
            )
        }
    }

    fn content_memory(&self) -> &[u8] {
        &self.pkt_bytes()[size_of::<PointHeader>()..]
    }

    #[inline(always)]
    fn map_fixed<'a, A: 'a>(
        &'a self,
        onunknown: impl FnOnce(&'a ()) -> A,
        ondata: impl FnOnce(&'a ()) -> A,
        onlinkpoint: impl FnOnce(&'a LinkPointHeader) -> A,
        onassert: impl FnOnce(&'a KeyPointHeader) -> A,
        onerror: impl FnOnce(&'a ()) -> A,
    ) -> A {
        match self.fixed() {
            BarePointFields::DataPoint(_) => ondata(&()),
            BarePointFields::LinkPoint {
                lp_header: linkpoint,
                tail: _,
            } => onlinkpoint(linkpoint),
            BarePointFields::KeyPoint {
                a_header: assert,
                tail: _,
            } => onassert(assert),
            BarePointFields::Error(_) => onerror(&()),
            BarePointFields::Unknown(_) => onunknown(&()),
        }
    }

    #[inline(always)]
    unsafe fn unchecked_assert_tail(&self) -> Tail {
        let inner_linkpoint =
            self.self_ptr().add(LINKPOINT_IN_KEYPOINT_OFFSET) as *const PointThinPtr;
        (*inner_linkpoint).unchecked_linkpoint_tail()
    }
    #[inline(always)]
    unsafe fn unchecked_linkpoint_tail(&self) -> Tail {
        let ptr = self.self_ptr();
        let p = &*{ ptr.add(size_of::<PointHeader>()) as *const LinkPointHeader };
        let start = size_of::<PointHeader>() + size_of::<LinkPointHeader>();
        let is_offset = p.info.offset_ipath.get() as usize;
        let data_offset = p.info.offset_data.get() as usize;
        let size = self.point_header().point_size();
        //assert!(start <= is_offset && is_offset <= data_offset && data_offset <= size , "bad sizes");
        let links: &[Link] = core::slice::from_ptr_range(
            ptr.add(start) as *const Link..ptr.add(is_offset) as *const Link,
        );
        let isp_bytes = core::slice::from_ptr_range(ptr.add(is_offset)..ptr.add(data_offset));
        let data = core::slice::from_ptr_range(ptr.add(data_offset)..ptr.add(size));
        let spath_idx = IPath::from_unchecked(isp_bytes);
        debug_assert!(spath_idx.check_components().is_ok(), "{spath_idx:?}");
        Tail {
            ipath: spath_idx,
            data,
            links,
        }
    }
}

impl Point for PointThinPtr {
    #[inline(always)]
    fn data(&self) -> &[u8] {
        self.parts().data_ptr()
    }
    #[inline(always)]
    fn tail(&self) -> Option<Tail> {
        match self.parts().fields {
            PointFields::LinkPoint(LinkPoint { head: _, tail }) => Some(tail),
            PointFields::KeyPoint(KeyPoint { head: _, tail }) => Some(tail),
            _ => None,
        }
    }
    #[inline(always)]
    fn keypoint_header(&self) -> Option<&KeyPointHeader> {
        self.map_fixed(none, none, none, Some, none)
    }
    #[inline(always)]
    fn linkpoint_header(&self) -> Option<&LinkPointHeader> {
        self.map_fixed(none, none, Some, |a| Some(&a.linkpoint), none)
    }
    #[inline(always)]
    fn parts(&self) -> PointParts {
        let fields = match self.point_header().point_type {
            PointTypeFlags::DATA_POINT => PointFields::DataPoint(self.content_memory()),
            PointTypeFlags::LINK_POINT => {
                let head = unsafe { &*(self.content_memory().as_ptr() as *const LinkPointHeader) };
                let tail = unsafe { self.unchecked_linkpoint_tail() };
                PointFields::LinkPoint(LinkPoint { head: *head, tail })
            }
            PointTypeFlags::KEY_POINT => {
                let head = unsafe { &*(self.content_memory().as_ptr() as *const KeyPointHeader) };
                let tail = unsafe { self.unchecked_assert_tail() };
                PointFields::KeyPoint(KeyPoint { head: *head, tail })
            }
            _ => PointFields::Error(self.content_memory()),
        };
        PointParts {
            pkt_header: *self.point_header(),
            fields,
        }
    }

    #[inline(always)]
    fn point_header_ref(&self) -> &PointHeader {
        self.point_header()
    }

    #[inline(always)]
    fn pkt_segments(&self) -> ByteSegments {
        ByteSegments::from_array([self.pkt_bytes()])
    }
}

impl Point for PointPtr {
    #[inline(always)]
    fn data(&self) -> &[u8] {
        self.thin_point().data()
    }
    #[inline(always)]
    fn tail(&self) -> Option<Tail> {
        self.thin_point().tail()
    }
    #[inline(always)]
    fn keypoint_header(&self) -> Option<&KeyPointHeader> {
        self.thin_point().map_fixed(none, none, none, Some, none)
    }
    #[inline(always)]
    fn linkpoint_header(&self) -> Option<&LinkPointHeader> {
        self.thin_point().linkpoint_header()
    }
    #[inline(always)]
    fn parts(&self) -> PointParts {
        self.thin_point().parts()
    }

    #[inline(always)]
    fn point_header_ref(&self) -> &PointHeader {
        &self.pkt_header
    }

    #[inline(always)]
    fn pkt_segments(&self) -> ByteSegments {
        ByteSegments::from_array([self.pkt_bytes()])
    }
}

#[allow(dead_code)]
#[repr(align(4))]
enum BarePointFields<'o> {
    DataPoint(&'o [u8]),
    LinkPoint {
        lp_header: &'o LinkPointHeader,
        tail: &'o [u8],
    },
    KeyPoint {
        a_header: &'o KeyPointHeader,
        tail: &'o [u8],
    },
    Error(&'o [u8]),
    Unknown(&'o [u8]),
}
