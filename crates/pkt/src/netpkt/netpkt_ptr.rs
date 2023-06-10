// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::*;
use crate::{netpkt::reroute::ReroutePkt, *};
use core::fmt;
use std::borrow::Borrow;


#[doc(hidden)]
/// A (fat) pointer to valid netpkt bytes
/// 
/// A fat pointer to it can only be constructed through a valid NetPktBytes
/// WARN: it is likely that this will be removed. NetPktPtr does everything but better except please miri
#[derive(Debug)]
#[repr(C, align(4))]
pub struct NetPktFatPtr {
    pub _net_header: NetPktHeader,
    pub(crate) hash: LkHash,
    pub pkt: PointPtr,
}

/// Byte repr of a [NetPkt]
#[repr(C, align(4))]
pub struct NetPktPtr {
    pub net_header: NetPktHeader,
    pub(crate) hash: LkHash,
    pub(crate) point: PointThinPtr,
}

impl ToOwned for NetPktPtr {
    type Owned = NetPktBox;

    fn to_owned(&self) -> Self::Owned {
        self.as_netbox()
    }
}
impl Borrow<NetPktPtr> for NetPktBox {
    fn borrow(&self) -> &NetPktPtr {
        self.thin_netpkt()
    }
}

impl std::fmt::Debug for NetPktPtr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        std::fmt::Debug::fmt(self.as_sized(), f)
    }
}
impl NetPktPtr {
    //pub(crate) const EMPTY : Self = NetPktThin { net_header: NetPktHeader::EMPTY, hash: B64([0;32]), point: PointThinPtr::EMPTY };

    #[inline(always)]
    pub fn as_sized(&self) -> &NetPktFatPtr {
        let (_layout, metadata) = netpktbox_layout(self.point.point_header());
        unsafe { &*std::ptr::from_raw_parts(self as *const NetPktPtr as *const (), metadata) }
    }
    /// # Safety
    ///
    /// Must be aligned valid netpkt bytes.
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &Self {
        assert!(
            b.len() >= MIN_NETPKT_SIZE,
            "Never gone work {} < {} with b= {:?}",
            b.len(),
            MIN_NETPKT_SIZE,
            &b
        );
        assert!(b.as_ptr().align_offset(4) == 0, "Unaligned cast");
        let netpkt = &*{ b.as_ptr() as *const Self };
        debug_assert!(netpkt.check(true).is_ok());
        netpkt
    }
    pub fn reroute(&self, route: NetPktHeader) -> ReroutePkt<&Self> {
        ReroutePkt {
            net_header: route,
            pkt: self,
        }
    }

    pub fn check(&self, skip_hash:bool) -> Result<&[u8], Error> {
        let _ = self.point.internal_consitent_length()?;
        if skip_hash{
            self.point.check_signature()?;
            if self.hash() != self.point.compute_hash() {
                return Err(Error::HashMismatch);
            }
        }
        Ok(self.as_netpkt_bytes())
    }
    pub fn as_netpkt_bytes(&self) -> &[u8] {
        unsafe { from_raw_parts(self as *const Self as *const u8, self.size() as usize ) }
    }
}

impl Clone for NetPktBox {
    fn clone(&self) -> Self {
        self.thin_netpkt().as_netbox()
    }
}

impl NetPktFatPtr {
    pub fn thin_netpkt(&self) -> &NetPktPtr {
        unsafe { &*(self as *const Self as *const NetPktPtr) }
    }
    /// # Safety
    ///
    /// Must be a aligned buffer of at least self.pkt_header.net_pkt_size len with the correct hash
    pub unsafe fn from_bytes_unchecked(b: &[u8]) -> &Self {
        NetPktPtr::from_bytes_unchecked(b).as_sized()
    }
    pub fn into_raw_box(this: Box<Self>) -> *const NetPktPtr {
        Box::into_raw(this) as *const NetPktPtr
    }
    /// # Safety
    ///
    /// Must be constructed with [Self::into_raw_box]
    pub unsafe fn from_raw_box(ptr: *const NetPktPtr) -> Box<Self> {
        Box::from_raw(
            (*ptr).as_sized() as *const NetPktFatPtr as *mut NetPktFatPtr as *mut NetPktFatPtr,
        )
    }
}

impl NetPkt for NetPktPtr {
    fn as_netarc(&self) -> NetPktArc {
        let header = NetPktPtr {
            net_header: self.net_header,
            hash: self.hash,
            point: PointThinPtr(self.point.0)
        };
        let bytes = self.as_netpkt_bytes();
        NetPktArc(Arc::from_header_and_slice(header, &bytes[std::mem::size_of::<PartialNetHeader>()..]))
    }

    #[inline(always)]
    fn net_header_mut(&mut self) -> Option<&mut NetPktHeader> {
        Some(&mut self.net_header)
    }

    #[inline(always)]
    fn byte_segments(&self) -> ByteSegments {
        ByteSegments::from_array([self.as_netpkt_bytes()])
    }

    #[inline(always)]
    fn hash_ref(&self) -> &LkHash {
        &self.hash
    }

    #[inline(always)]
    fn net_header_ref(&self) -> &NetPktHeader {
        &self.net_header
    }

    fn as_point(&self) -> &dyn Point {
        &self.point
    }

    fn recv(&self) -> Option<Stamp> {
        None
    }
}

impl Deref for NetPktPtr {
    type Target = PointThinPtr;
    fn deref(&self) -> &Self::Target {
        &self.point
    }
}
impl Deref for NetPktFatPtr {
    type Target = NetPktPtr;
    fn deref(&self) -> &Self::Target {
        self.thin_netpkt()
    }
}

impl NetPkt for NetPktFatPtr {
    fn hash_ref(&self) -> &LkHash {
        &self.hash
    }
    fn net_header_ref(&self) -> &NetPktHeader {
        &self._net_header
    }

    fn byte_segments(&self) -> ByteSegments {
        self.thin_netpkt().byte_segments()
    }

    fn as_point(&self) -> &dyn Point {
        self.thin_netpkt().as_point()
    }

    fn recv(&self) -> Option<Stamp> {
        None
    }

    fn net_header_mut(&mut self) -> Option<&mut NetPktHeader> {
        Some(&mut self._net_header)
    }
}

pub fn netpktbox_layout(
    pkt_header: &PointHeader,
) -> (
    std::alloc::Layout,
    <NetPktFatPtr as std::ptr::Pointee>::Metadata,
) {
    use std::alloc::Layout;
    let clen = pkt_header.content_size() as usize;
    (
        Layout::new::<PartialNetHeader>()
            .extend(Layout::new::<u8>().repeat(clen).unwrap().0)
            .unwrap()
            .0
            .pad_to_align(),
        clen,
    )
}

#[test]
pub fn build() {
    let sp = linkpoint(
        B64([0; 32]),
        AB([0; 16]),
        &ipath_buf(&[b"yo"]),
        &[],
        b"ok",
        now(),
        (),
    )
    .as_netbox();
    let sp_bytes = sp.clone().byte_segments().0.concat().into_boxed_slice();
    let b = unsafe { NetPktPtr::from_bytes_unchecked(&*sp_bytes) };
    assert_eq!(sp.tail(), b.tail());
    let h = sp.hash();

    let raw = NetPktFatPtr::into_raw_box(sp);
    let h2 = unsafe { &*raw }.hash();
    assert_eq!(h, h2);
    let b = unsafe{NetPktFatPtr::from_raw_box(raw)};
    assert_eq!(h, b.hash())
}
