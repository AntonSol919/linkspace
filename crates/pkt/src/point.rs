// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{netpkt::NetPktHeader, *};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct PointHeader {
    pub reserved: u8,
    pub point_type: PointTypeFlags,
    /// You don't want this raw value - the size of all fields - without the padding between the space and optional signature. 
    pub uset_bytes: U16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct LinkPointInfo {
    pub offset_rspace: U16,
    pub offset_data: U16,
}
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct LinkPointHeader {
    pub info: LinkPointInfo,
    pub create_stamp: Stamp,
    pub group: GroupID,
    pub domain: Domain,
}
impl LinkPointHeader {
    pub fn as_bytes(&self) -> &[u8; size_of::<Self>()] {
        unsafe {&*std::ptr::from_ref(self).cast()}
    }
}

impl PointHeader {
    pub const ERROR: Self = match PointHeader::new_content_len(PointTypeFlags::ERROR_POINT,0){
        Ok(o) => o,
        Err(_) => panic!(),
    };

    #[allow(clippy::as_conversions)]
    pub (crate) const fn new_content_len(kind: PointTypeFlags, content_len: usize) -> Result<Self, Error> {
        let point_size = content_len.saturating_add(size_of::<PointHeader>());
        Self::new_point_size(kind, point_size)
    }
    pub (crate) const fn new_point_size(kind: PointTypeFlags, point_size: usize) -> Result<Self, Error> {
        if point_size > MAX_POINT_SIZE { return Err(Error::ContentLen)};
        PointHeader {
            point_type: kind,
            reserved: 0,
            uset_bytes: U16::new(point_size as u16) 
        }
        .check()
    }
    #[allow(clippy::as_conversions)]
    pub const fn check(self) -> Result<Self, Error> {
        if self.reserved != 0 {
            return Err(Error::HeaderReservedSet(self.reserved));
        }
        let len = self.ucontent_size() as usize ;
        if len > MAX_CONTENT_SIZE {
            return Err(Error::ContentLen);
        }
        let ok = match self.point_type {
            PointTypeFlags::DATA_POINT => true,
            PointTypeFlags::LINK_POINT => len >= size_of::<LinkPointHeader>(),
            PointTypeFlags::KEY_POINT => len >= size_of::<LinkPointHeader>()+size_of::<Signed>(),
            PointTypeFlags::ERROR_POINT => true,
            e => return Err(Error::UnknownPktType(e.bits())),
        };
        if ok {
            Ok(self)
        } else {
            Err(Error::ContentLen)
        }
    }
    pub fn as_bytes(&self) -> &[u8; 4] {
        unsafe {&*std::ptr::from_ref(self).cast()}
    }
    /// Size of a point (type,length,content,?signature). This is without the hash or padding between content and signature.
    pub (crate) const fn upoint_size(&self) -> u16{
        self.uset_bytes.get() 
    }
    pub (crate) const fn padding(&self) -> u16 {
        self.padded_point_size() - self.upoint_size()
    }
    pub (crate)const fn padded_point_size(&self) -> u16 {
        assert!(std::mem::size_of::<usize>()<= 8, "some code depends on this assumption");
        self.upoint_size().div_ceil(8)*8
    }
    #[allow(clippy::as_conversions)]
    pub (crate) const fn ucontent_size(&self) -> u16{
        self.upoint_size().saturating_sub( size_of::<PointHeader>() as u16)
    }
    pub (crate) const fn padded_content_size(&self) -> u16 {
        self.padded_point_size().saturating_sub( size_of::<PointHeader>() as u16)
    }
    /// Size of a netpkt. The pointsize plus pointhash and netpktheader + padding 
    #[allow(clippy::as_conversions)]
    pub const fn size(&self) -> u16{
        self.padded_point_size() + size_of::<LkHash>() as u16 + size_of::<NetPktHeader>() as u16
    }
}
