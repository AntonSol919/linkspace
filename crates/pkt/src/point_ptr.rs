// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    fmt::{self, Debug},
    ptr,
};

use super::*;
use crate::utils::none;

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
                ptr::from_ref(self).cast::<()>(),
                usize::from(self.0.padded_content_size()),
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
        unsafe { &*(ptr::from_ref(&self.pkt_header).cast::<PointThinPtr>()) }
    }
    /// # Safety
    ///
    /// should be well aligned valid netpktbytes.
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &PointPtr {
        PointThinPtr::from_bytes_unchecked(b).as_sized()
    }
    pub fn internal_consitent_length(&self) -> Result<(), Error> {
        self.thin_point().internal_consitent_length()
    }
}
impl PointThinPtr {
    /// # Safety
    ///
    /// This is a unchecked cast, meaning accessing fields is dangerous
    /// to validate, first ensure you have enough bytes to read a PktHeader,
    /// Then call internal_consistent_length and compare the lengths
    /// finally call check_signature to ensure if its signed that its valid.
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &PointThinPtr {
        assert!(b.len() >= size_of::<PointHeader>(), "Never gone work");
        assert!(b.as_ptr().align_offset(8) == 0, "Unaligned cast");
        assert!(b.len() % 8 == 0, "missing padding");
        &*(b.as_ptr().cast::<PointThinPtr>())
    }
    #[inline(always)]
    pub (crate) fn linkpoint_header(&self) -> Option<&LinkPointHeader>{
        let mut v = None;
        if self.0.point_type.contains(PointTypeFlags::LINK) {
            v = Some(unsafe {&*self.pkt_bytes().as_ptr().add(size_of::<PointHeader>()).cast::<LinkPointHeader>()});
        }
        v
    }
    /// This must be called with a verified point type.
    #[inline(always)]
    pub(crate)fn fixed(&self) -> BarePointFields {
        let (_point, rest) = unsafe {
            self.pkt_bytes()
                .split_at_unchecked(size_of::<PointHeader>())
        };
        let padding_len: usize = self.0.padding().into();

        if self.0.point_type.contains(PointTypeFlags::LINK) {
            unsafe {
                let (lp, mut rest) = rest.split_at_unchecked(size_of::<LinkPointHeader>());
                let header = &*lp.as_ptr().cast::<LinkPointHeader>();
                let mut signed: Option<&Signed> = None;
                if self.0.point_type.contains(PointTypeFlags::SIGNATURE) {
                    let (tail, signed_b) =
                        rest.split_at_unchecked(rest.len() - size_of::<Signed>());
                    rest = tail;
                    signed = Some(&*signed_b.as_ptr().cast::<Signed>());
                }
                let (_tail, padding) =
                    rest.split_at_unchecked(rest.len() - self.0.padding() as usize);

                return BarePointFields::LinkPoint {
                    header,
                    padding,
                    signed,
                };
            }
        }
        let (mem, padding) = unsafe { rest.split_at_unchecked(rest.len() - padding_len) };
        match self.0.point_type {
            PointTypeFlags::DATA_POINT => BarePointFields::DataPoint { data: mem, padding },
            PointTypeFlags::ERROR_POINT => BarePointFields::Error { msg: mem, padding },
            _ => BarePointFields::Unknown(rest),
        }
    }

    /// Check the header length fields and return the point size. This is the length without the hash.
    pub fn internal_consitent_length(&self) -> Result<(), Error> {
        self.point_header().check()?;
        let mut point_size = self.point_header().upoint_size();
        let padding = match self.fixed() {
            BarePointFields::DataPoint { padding, .. } => padding,
            BarePointFields::LinkPoint {
                padding,
                header,
                signed: _,
            } => {
                if self.0.point_type.contains(PointTypeFlags::SIGNATURE) {
                    // signed.is_some(
                    point_size -= size_of::<Signed>() as u16;
                }
                let isp_offset = header.info.offset_ipath.get();
                let data_offset = header.info.offset_data.get();
                if data_offset > point_size {
                    return Err(Error::DataOffsetIncompatible);
                }
                if isp_offset > data_offset {
                    return Err(Error::DataOffsetIncompatible);
                }
                let link_size = isp_offset
                    .checked_sub(
                        (size_of::<PointHeader>() + size_of::<LinkPointHeader>())
                            .try_into()
                            .unwrap(),
                    )
                    .ok_or(Error::ISPOffsetIncompatible)?;
                if link_size % u16::try_from(size_of::<Link>()).unwrap() != 0 {
                    return Err(Error::IndivisableLinkbytes);
                }
                unsafe { self.unchecked_tail() }.ipath.check_components()?;
                padding
            }
            BarePointFields::Error { padding, .. } => padding,
            BarePointFields::Unknown(_) => &[],
        };
        if padding.len() > 7 {return Err(Error::PaddingBitsNotU8Max);}
        if !padding.iter().all(|o|*o ==255){return Err(Error::PaddingBitsNotU8Max);}
        Ok(())
    }

    pub fn check_signature(&self) -> Result<(), Error> {
        match self.fixed() {
            BarePointFields::LinkPoint {
                header: _,
                padding: _,
                signed: Some(signed),
            } => {
                let pbytes = self.pkt_bytes();
                let sans_signature = &pbytes[..pbytes.len() - size_of::<Signed>()];
                let hash = linkspace_cryptography::blake3_hash(sans_signature);
                signed
                    .validate(hash.as_bytes())
                    .map_err(|_| Error::InvalidSignature)?;
            }
            _ => (),
        };
        Ok(())
    }
    fn self_ptr(&self) -> *const u8 {
        ptr::from_ref(self).cast()
    }
    /// include padding
    pub fn pkt_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                ptr::from_ref(self).cast(),
                usize::from(self.point_header().padded_point_size()),
            )
        }
    }

    #[inline(always)]
    fn map_fixed<'a, A: 'a>(
        &'a self,
        onunknown: impl FnOnce(&'a ()) -> A,
        ondata: impl FnOnce(&'a ()) -> A,
        onlinkpoint: impl FnOnce(&'a LinkPointHeader) -> A,
        onkeypoint: impl FnOnce(&'a LinkPointHeader, &'a Signed) -> A,
        onerror: impl FnOnce(&'a ()) -> A,
    ) -> A {
        match self.fixed() {
            BarePointFields::DataPoint { .. } => ondata(&()),
            BarePointFields::LinkPoint {
                header,
                signed: None,
                ..
            } => onlinkpoint(header),
            BarePointFields::LinkPoint {
                header,
                signed: Some(signed),
                ..
            } => onkeypoint(header, signed),
            BarePointFields::Error { .. } => onerror(&()),
            BarePointFields::Unknown(_) => onunknown(&()),
        }
    }

    #[inline(always)]
    unsafe fn unchecked_tail(&self) -> Tail {
        let ptr = self.self_ptr();
        let p = &*{ ptr.add(size_of::<PointHeader>()).cast::<LinkPointHeader>() };
        let start = size_of::<PointHeader>() + size_of::<LinkPointHeader>();
        let is_offset = usize::from(p.info.offset_ipath.get());
        let data_offset = usize::from(p.info.offset_data.get());
        let mut size = self.point_header().upoint_size();
        if self.0.point_type.contains(PointTypeFlags::SIGNATURE) {
            size -= size_of::<Signed>() as u16;
        }
        //assert!(start <= is_offset && is_offset <= data_offset && data_offset <= size , "bad sizes");
        let start = ptr.add(start).cast::<Link>();
        let end = ptr.add(is_offset).cast::<Link>();
        assert!(start.is_aligned());
        let links: &[Link] = core::slice::from_ptr_range(start..end);
        let isp_bytes = core::slice::from_ptr_range(ptr.add(is_offset)..ptr.add(data_offset));
        let data = core::slice::from_ptr_range(ptr.add(data_offset)..ptr.add(size.into()));
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
            PointFields::LinkPoint(LinkPoint { tail, .. }) => Some(tail),
            PointFields::KeyPoint(LinkPoint { tail, .. }, _) => Some(tail),
            _ => None,
        }
    }
    #[inline(always)]
    fn signed(&self) -> Option<&Signed> {
        self.map_fixed(none, none, none, |_, a| Some(a), none)
    }
    #[inline(always)]
    fn linkpoint_header(&self) -> Option<&LinkPointHeader> {
        self.map_fixed(none, none, Some, |a, _| Some(a), none)
    }
    #[inline(always)]
    fn parts(&self) -> PointParts {
        let fields = match self.fixed() {
            BarePointFields::DataPoint { data, padding: _ } => PointFields::DataPoint(data),
            BarePointFields::LinkPoint {
                header,
                signed: None,
                padding: _,
            } => {
                let tail = unsafe { self.unchecked_tail() };
                PointFields::LinkPoint(LinkPoint {
                    head: *header,
                    tail,
                })
            }
            BarePointFields::LinkPoint {
                header,
                signed: Some(signed),
                padding: _,
            } => {
                let tail = unsafe { self.unchecked_tail() };
                PointFields::KeyPoint(
                    LinkPoint {
                        head: *header,
                        tail,
                    },
                    *signed,
                )
            }
            BarePointFields::Error { msg, padding: _ } => PointFields::Error(msg),
            BarePointFields::Unknown(o) => PointFields::Unknown(o),
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

    fn padding(&self) -> &[u8] {
        match self.fixed() {
            BarePointFields::DataPoint { padding, .. }
            | BarePointFields::LinkPoint { padding, .. }
            | BarePointFields::Error { padding, .. }
            | BarePointFields::Unknown(padding) => padding,
        }
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
    fn padding(&self) -> &[u8] {
        self.thin_point().padding()
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
        self.thin_point().pkt_segments()
    }

    fn signed(&self) -> Option<&Signed> {
        self.thin_point()
            .map_fixed(none, none, none, |_, o| Some(o), none)
    }
}

// This layout can produce cmov instructions
#[derive(Debug)]
pub(crate) enum BarePointFields<'o> {
    DataPoint {
        data: &'o [u8],
        #[allow(unused)]
        padding: &'o [u8],
    },
    LinkPoint {
        header: &'o LinkPointHeader,
        signed: Option<&'o Signed>,
        #[allow(unused)]
        padding: &'o [u8],
    },
    Error {
        msg: &'o [u8],
        #[allow(unused)]
        padding: &'o [u8],
    },
    Unknown(&'o [u8]),
}
