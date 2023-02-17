// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;


/// Rust enum repr of a [NetPkt]
#[derive(Clone, Copy, Debug)]
#[repr(C, align(4))]
pub struct NetPktParts<'a> {
    pub net_header: NetPktHeader,
    pub(crate) hash: LkHash,
    // we dont give out &mut links to pkt parts, this invalidates the hash
    pub(crate) point_parts: PointParts<'a>,
}
impl<'a> NetPktParts<'a> {
    /// The caller must ensure PktHash matches the Pkt
    pub fn from_unchecked(net_header: NetPktHeader, hash: LkHash, pkt_parts: PointParts<'a>) -> Self {
        debug_assert_eq!(pkt_parts.compute_hash(), hash);
        NetPktParts {
            net_header,
            hash,
            point_parts: pkt_parts,
        }
    }
    pub fn from(pkt: &'a dyn NetPkt) -> NetPktParts {
        Self::from_unchecked(pkt.net_header(), pkt.hash(), pkt.as_point().parts())
    }
}

impl<'a> Deref for NetPktParts<'a> {
    type Target = PointParts<'a>;
    fn deref(&self) -> &Self::Target {
        &self.point_parts
    }
}

impl<'a> NetPkt for NetPktParts<'a> {
    #[inline(always)]
    fn hash_ref(&self) -> &LkHash {
        &self.hash
    }

    fn net_header_ref(&self) -> &NetPktHeader {
        &self.net_header
    }
    #[inline(always)]
    fn byte_segments(&self) -> ByteSegments {
        let segments = self.point_parts.pkt_segments();
        let head = unsafe {
            let ptr: *const u8 = self as *const Self as *const u8;
            core::slice::from_raw_parts(ptr, size_of::<NetPktHeader>() + size_of::<LkHash>())
        };
        segments.push_front(head)
    }

    fn as_point(&self) -> &dyn Point {
        &self.point_parts
    }

    fn recv(&self) -> Option<Stamp> {
        None
    }
}

/*
impl<'o> Into<NetPktBox> for NetPktParts<'o>{
    fn into(self) -> NetPktBox {
        self.as_netbox()
    }
}
*/
impl<'o> From<NetPktParts<'o>> for NetPktBox {
    fn from(value: NetPktParts<'o>) -> Self {
        value.as_netbox()
    }
}
impl<'o> From<NetPktParts<'o>> for NetPktArc {
    fn from(value: NetPktParts<'o>) -> Self {
        value.as_netarc()
    }
}
