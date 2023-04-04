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
        as_bytes(self)
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
        as_bytes(self)
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
    pub const ERROR: Self = PointHeader {
        point_type: PointTypeFlags::ERROR_POINT,
        reserved: 0,
        point_size: U16::new(4),
    };
    pub const fn new(kind: PointTypeFlags, content_len: usize) -> Result<Self, Error> {
        PointHeader {
            point_type: kind,
            reserved: 0,
            point_size: U16::new(content_len as u16 + 4),
        }
        .check()
    }
    pub const fn check(self) -> Result<Self, Error> {
        if self.reserved != 0 {
            return Err(Error::HeaderReservedSet(self.reserved));
        }
        let len = self.content_size();
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
        as_bytes(self)
    }
    /// Size of a point (type,length,content). This is without the hash.
    pub const fn point_size(&self) -> usize {
        self.point_size.get() as usize
    }
    /// size of a point's content. If it is a datapoint, this is the size of the data.
    pub const fn content_size(&self) -> usize {
        self.point_size() - size_of::<PointHeader>()
    }
    /// Size of a netpkt. The pointsize plus pointhash and netpktheader
    pub const fn net_pkt_size(&self) -> usize {
        self.point_size() + size_of::<LkHash>() + size_of::<NetPktHeader>()
    }
}
