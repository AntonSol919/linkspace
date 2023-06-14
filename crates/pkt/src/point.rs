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
    pub point_size: U16,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct LinkPointInfo {
    pub offset_ipath: U16,
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

pub type KeyPointPadding = [u8; 4];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct KeyPointHeader {
    pub reserved: KeyPointPadding,
    pub signed: Signed,
    pub inner_point: PointHeader,
    pub linkpoint: LinkPointHeader,
}
impl KeyPointHeader {
    pub fn as_bytes(&self) -> &[u8; size_of::<Self>()] {
        unsafe {&*std::ptr::from_ref(self).cast()}
    }
}
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct Signed {
    pub pubkey: PubKey,
    pub signature: Signature,
    pub linkpoint_hash: LkHash,
}
impl Signed {
    pub fn validate(&self) -> Result<(), linkspace_cryptography::Error> {
        linkspace_cryptography::validate_signature(
            &self.pubkey.0,
            &self.signature.0,
            &self.linkpoint_hash.0,
        )
    }
}

impl PointHeader {
    pub const ERROR: Self = match PointHeader::new(PointTypeFlags::ERROR_POINT,0){
        Ok(o) => o,
        Err(_) => panic!(),
    };
    #[allow(clippy::as_conversions)]
    pub const fn new(kind: PointTypeFlags, content_len: usize) -> Result<Self, Error> {
        let point_size = content_len.saturating_add(size_of::<PointHeader>());
        if point_size > MAX_CONTENT_SIZE { return Err(Error::ContentLen)};
        PointHeader {
            point_type: kind,
            reserved: 0,
            point_size: U16::new(point_size as u16) 
        }
        .check()
    }
    #[allow(clippy::as_conversions)]
    pub const fn check(self) -> Result<Self, Error> {
        if self.reserved != 0 {
            return Err(Error::HeaderReservedSet(self.reserved));
        }

        let len = self.content_size() as usize ;
        if len > MAX_CONTENT_SIZE {
            return Err(Error::ContentLen);
        }
        let ok = match self.point_type {
            PointTypeFlags::DATA_POINT => true,
            PointTypeFlags::LINK_POINT => len >= size_of::<LinkPointHeader>(),
            PointTypeFlags::KEY_POINT => len >= size_of::<KeyPointHeader>(),
            PointTypeFlags::ERROR_POINT => true,
            e => return Err(Error::UnknownPktType(e.bits)),
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
    /// Size of a point (type,length,content). This is without the hash.
    pub const fn point_size(&self) -> u16{
        self.point_size.get() 
    }
    /// size of a point's content. If it is a datapoint, this is the size of the data.
    #[allow(clippy::as_conversions)]
    pub const fn content_size(&self) -> u16{
        self.point_size().saturating_sub( size_of::<PointHeader>() as u16)
    }
    /// Size of a netpkt. The pointsize plus pointhash and netpktheader
    #[allow(clippy::as_conversions)]
    pub const fn net_pkt_size(&self) -> u16{
        self.point_size() + size_of::<LkHash>() as u16 + size_of::<NetPktHeader>() as u16
    }
}
