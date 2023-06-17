// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub mod header;
pub mod partial;
pub mod reroute;

pub mod netpkt_arc;
pub mod netpkt_parts;
pub mod netpkt_ptr;
//pub mod slot;

pub use eval::*;
pub use header::*;
pub use netpkt_arc::*;
pub use netpkt_parts::*;
pub use netpkt_ptr::*;
pub use partial::*;
use std::fmt::Debug;
use triomphe::Arc;


/// Heap allocated repr of a [NetPkt].
pub type NetPktBox = Box<NetPktFatPtr>;
impl From<&dyn NetPkt> for NetPktBox {
    fn from(value: &dyn NetPkt) -> Self {
        value.as_netbox()
    }
}

use crate::*;
use auto_impl::auto_impl;

/** A trait to access fields of a net pkt. Auto impls [NetPktExt], [Point], and [PointExt].

A NetPkt is the combination of [Point], [LkHash], [NetPktHeader].
the trait is impl for various layouts such as [NetPktBox], [NetPktArc].

There are wrapping structs in as [reroute] that provide extended options.
**/
#[auto_impl(&mut,Box)]
#[doc(notable_trait)]
pub trait NetPkt: Debug {
    fn as_point(&self) -> &dyn Point;
    fn hash_ref(&self) -> &LkHash;
    fn net_header_ref(&self) -> &NetPktHeader;
    fn net_header_mut(&mut self) -> Option<&mut NetPktHeader> {
        None
    }
    /**
    recv is somewhat special.
    It depends on the context. Reading directly from the database it should return the stamp at which it was inserted.
    NOTE: Do not rely on this value being unique - in the db or otherwise.
    */
    fn recv(&self) -> Option<Stamp>;

    fn byte_segments(&self) -> ByteSegments;
    #[inline(always)]
    fn as_netbox(&self) -> NetPktBox {
        use std::alloc;
        let segm = self.byte_segments();
        let b = unsafe {
            let (layout, metadata) = netpktbox_layout(self.as_point().point_header_ref());
            let ptr: *mut u8 = alloc::alloc(layout);
            if ptr.is_null() {
                alloc::handle_alloc_error(layout);
            }
            let ptr: *mut NetPktFatPtr =std::ptr::from_raw_parts_mut::<NetPktFatPtr>(ptr.cast(), metadata);

            {
                let s = std::slice::from_raw_parts_mut(ptr.cast::<u8>(), layout.size());
                segm.write_segments_unchecked(s.as_mut_ptr());
            }
            Box::from_raw(ptr)
        };
        if cfg!(debug_assertions) {
            b.thin_netpkt().check(true).unwrap();
        }
        b
    }
    fn as_netarc(&self) -> NetPktArc {
        let h = NetPktPtr {
            net_header: self.net_header(),
            hash: self.hash(),
            point: PointThinPtr(self.as_point().point_header()),
        };
        // TODO. we can avoid this copy
        let byte_segments = self.byte_segments();
        let arc = Arc::from_header_and_iter(h, byte_segments.skip(size_of::<NetPktPtr>()));
        NetPktArc(arc)
    }
}

impl<T: NetPkt + ?Sized> NetPkt for &T {
    fn as_point(&self) -> &dyn Point {
        (**self).as_point()
    }

    fn hash_ref(&self) -> &LkHash {
        (**self).hash_ref()
    }
    fn net_header_ref(&self) -> &NetPktHeader {
        (**self).net_header_ref()
    }
    fn byte_segments(&self) -> ByteSegments {
        (**self).byte_segments()
    }

    fn as_netbox(&self) -> NetPktBox {
        (**self).as_netbox()
    }

    fn as_netarc(&self) -> NetPktArc {
        (**self).as_netarc()
    }
    fn net_header_mut(&mut self) -> Option<&mut NetPktHeader> {
        None
    }
    fn recv(&self) -> Option<Stamp> {
        None
    }
}

impl<T: NetPkt + ?Sized> NetPktExt for T {}

#[doc(notable_trait)]
/// Utilities for [NetPkt]
pub trait NetPktExt
where
    Self: NetPkt,
{
    /// see [NetPkt::recv]
    fn get_recv(&self) -> Stamp {
        self.recv().unwrap_or_else(now)
    }
    fn hash(&self) -> LkHash {
        *self.hash_ref()
    }
    fn net_header(&self) -> NetPktHeader {
        *self.net_header_ref()
    }
    fn size(&self) -> u16 {
        self.as_point().point_header_ref().net_pkt_size()
    }
    fn as_netparts(&self) -> NetPktParts
    where
        Self: Sized,
    {
        NetPktParts::from(self)
    }
    fn to_default_str(&self) -> String{ PktFmt(&self).to_string()}
}

impl<T> Point for T where T: NetPktExt{
    
    fn parts(&self) -> PointParts {
        self.as_point().parts()
    }
    fn data(&self) -> &[u8] {
        self.as_point().data()
    }

    fn tail(&self) -> Option<Tail> {
        self.as_point().tail()
    }

    fn linkpoint_header(&self) -> Option<&LinkPointHeader> {
        self.as_point().linkpoint_header()
    }

    fn keypoint_header(&self) -> Option<&KeyPointHeader> {
        self.as_point().keypoint_header()
    }

    fn pkt_segments(&self) -> ByteSegments {
        self.as_point().pkt_segments()
    }

    fn point_header_ref(&self) -> &PointHeader {
        self.as_point().point_header_ref()
    }
}

#[test]
pub fn basic() {
    use crate::NetPkt;
    let _pkt = crate::datapoint(&[], ()).as_netbox();
}
#[test]
pub fn calc_len() {
    use crate::NetPkt;
    let spath = crate::ipath_buf(&[b"hello", b"world"]);
    let links = [crate::Link::new("test", [2u8; 32])];
    let sp = crate::linkpoint(
        [0; 32].into(),
        [0; 16].into(),
        &spath,
        &links,
        b"ok",
        crate::now(),
        (),
    )
    .as_netbox();
    let upto_header_bytes = sp.as_netpkt_bytes()[0..MIN_NETPKT_SIZE].try_into().unwrap();
    let calculated = PartialNetHeader::from(upto_header_bytes)
        .point_header
        .net_pkt_size();
    assert_eq!(sp.size(), calculated)
}
