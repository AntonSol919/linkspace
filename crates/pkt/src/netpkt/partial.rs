// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::*;
static_assertions::assert_eq_size!([u8; MIN_NETPKT_SIZE], PartialNetHeader);
/// Utility struct used for decoding from stream
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub struct PartialNetHeader {
    pub net_header: crate::NetPktHeader,
    pub hash: crate::LkHash,
    pub point_header: crate::PointHeader,
}
impl PartialNetHeader {
    pub const EMPTY: Self = PartialNetHeader {
        net_header: NetPktHeader::EMPTY,
        hash: B64([0; 32]),
        point_header: PointHeader::ERROR,
    };
    pub fn from(bytes: &[u8; MIN_NETPKT_SIZE]) -> PartialNetHeader {
        unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const Self) }
    }
    /// # Safety
    ///
    /// This must be a reference to a buffer of at least self.pkt_header.net_pkt_size len with the correct hash
    pub unsafe fn alloc(self) -> NetPktBox {
        let (layout, metadata) = crate::netpktbox_layout(&self.point_header);
        let ptr: *mut u8 = std::alloc::alloc(layout);
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        let ptr: *mut NetPktFatPtr =
            std::ptr::from_raw_parts_mut::<NetPktFatPtr>(ptr as *mut (), metadata);
        let mut val = Box::from_raw(ptr);
        val._net_header = self.net_header;
        val.hash = self.hash;
        val.pkt.pkt_header = self.point_header;
        val
    }
}
