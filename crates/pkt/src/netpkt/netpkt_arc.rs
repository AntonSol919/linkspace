// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{NetPkt, NetPktPtr, PartialNetHeader, Error };
use std::{
    borrow::Borrow, fmt::Debug, mem::size_of, ops::Deref, ptr::{ self}, sync::atomic::AtomicUsize,
};
use triomphe::{Arc, HeaderSlice};

#[derive(Clone)]
/// Arc around the byte repr [NetPkt]
/// 
/// Send + Sync + cheap Clone, [super::NetPktHeader] is immutable
// see [Reroute] for mutating netheader.
pub struct NetPktArc(pub(crate) Arc<HeaderSlice<NetPktPtr, [u8]>>);


impl From<&dyn NetPkt> for NetPktArc {
    fn from(value: &dyn NetPkt) -> Self {
        value.as_netarc()
    }
}
/// A thin pointer to the inner NetPktPtr of a NetPktArc
#[repr(transparent)]
pub struct NetPktArcPtr(NetPktPtr);

impl ToOwned for NetPktArcPtr {
    type Owned = NetPktArc;
    fn to_owned(&self) -> Self::Owned {
        self.as_netarc()
    }
}
impl Borrow<NetPktArcPtr> for NetPktArc {
    fn borrow(&self) -> &NetPktArcPtr {
        self.thin_arc()
    }
}

// Copy of triomphe InnerArc
#[repr(C)]
struct ArcInner {
    count: std::sync::atomic::AtomicUsize,
    data: HeaderSlice<NetPktPtr, [u8]>,
}

impl NetPktArcPtr {
    pub fn netpktptr(&self) -> &NetPktPtr {
        &self.0
    }
    /// Get a look at the outer [NetPktArc] 
    #[inline]
    pub fn with_arc<U>(&self, f: impl FnOnce(&NetPktArc) -> U) -> U {
        let size : usize = self.as_point().point_header_ref().padded_content_size().into();
        let ptr: *const NetPktPtr = ptr::from_ref(self.netpktptr());
        let inner_arc : *const () =unsafe{ ptr.cast::<u8>().sub(size_of::<AtomicUsize>()).cast::<()>()};

        let inner: *const ArcInner = std::ptr::from_raw_parts(inner_arc, size);
        let ptr : &(*const ArcInner,()) = &(inner,()); // Yes we're casting a reference to a pointer 
        let npa : &NetPktArc = unsafe { &*std::ptr::from_ref(ptr).cast::<NetPktArc>()};
        f(npa)
    }
}

impl Debug for NetPktArcPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.with_arc(|a| Debug::fmt(a, f))
    }
}
impl Debug for NetPktArc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NetPktArc")
            .field(&Arc::count(&self.0))
            .field(&self.thin_arc().0)
            .finish()
    }
}
impl NetPktArc {
    pub fn thin_arc(&self) -> &NetPktArcPtr {
        unsafe { &*(ptr::from_ref(&self.0.header).cast::<NetPktArcPtr>())}
    }
    pub fn into_raw_arc(self) -> *const NetPktArcPtr {
        Arc::into_raw(self.0).cast() 
    }
    /// # Safety
    ///
    /// must be created through [Self::into_raw_arc]
    pub unsafe fn from_raw_arc(ptr: *const NetPktArcPtr) -> Self {
        assert!(!ptr.is_null());
        let byte_size : usize = unsafe { (*ptr).as_point().point_header_ref().padded_content_size() }.into();
        let inner_arc: *const () = ptr.cast::<u8>().sub(size_of::<AtomicUsize>()).cast::<()>();
        let inner: *const ArcInner = std::ptr::from_raw_parts(inner_arc, byte_size);
        let arc: Arc<HeaderSlice<NetPktPtr, [u8]>> = unsafe { std::mem::transmute(inner) };
        NetPktArc(arc)
    }
    /// # Safety
    ///
    /// copy_from must input the correct bytes.
    pub unsafe fn from_header_and_copy(partial: PartialNetHeader,skip_hash:bool, copy_from:impl FnOnce(&mut [u8])) -> Result<Self,Error>{
        let h = crate::NetPktPtr {
            net_header: partial.net_header,
            hash: partial.hash,
            point: crate::PointThinPtr(partial.point_header),
        };
        let byte_size : usize= h.point.point_header().padded_content_size().into() ;
        let arc = triomphe::Arc::from_header_and_fn(
            h,byte_size,copy_from
        );
        let pkt= NetPktArc(arc);
        pkt.check(skip_hash)?;
        Ok(pkt)
    }
}
impl Deref for NetPktArc {
    type Target = NetPktArcPtr;
    fn deref(&self) -> &Self::Target {
        self.thin_arc()
    }
}
impl Deref for NetPktArcPtr {
    type Target = NetPktPtr;
    fn deref(&self) -> &Self::Target {
        self.netpktptr()
    }
}
impl NetPkt for NetPktArcPtr {
    fn hash_ref(&self) -> &crate::LkHash {
        self.netpktptr().hash_ref()
    }

    fn as_netarc(&self) -> NetPktArc {
        self.with_arc(|v| v.clone())
    }
    fn net_header_ref(&self) -> &crate::NetPktHeader {
        self.netpktptr().net_header_ref()
    }

    fn byte_segments(&self) -> crate::ByteSegments {
        self.netpktptr().byte_segments()
    }

    fn as_point(&self) -> &dyn crate::Point {
        self.netpktptr().as_point()
    }

    fn recv(&self) -> Option<crate::Stamp> {
        None
    }
}

impl NetPkt for NetPktArc {
    fn hash_ref(&self) -> &crate::LkHash {
        self.thin_arc().hash_ref()
    }
    fn net_header_ref(&self) -> &crate::NetPktHeader {
        self.thin_arc().net_header_ref()
    }

    fn as_netarc(&self) -> NetPktArc {
        self.clone()
    }

    fn byte_segments(&self) -> crate::ByteSegments {
        self.thin_arc().byte_segments()
    }

    fn as_point(&self) -> &dyn crate::Point {
        self.thin_arc().as_point()
    }

    fn recv(&self) -> Option<crate::Stamp> {
        None
    }

    fn as_netbox(&self) -> crate::NetPktBox {
        self.thin_arc().0.as_netbox()
    }
}

#[test]
pub fn build() {
    use crate::*;
    let parts = datapoint(b"hello", ());
    let boxed = parts.as_netbox();
    let arced = parts.as_netarc();
    let p2 = boxed.as_netparts();
    let p3 = arced.as_netparts();
    assert_eq!(p2.point_parts, p3.point_parts);
    let a2 = boxed.as_netarc();
    use std::mem::size_of_val;
    assert_eq!(size_of_val(&*a2.0), 32 + 32 + 4 + 5 + 7); // 3 is padding
    assert_eq!(size_of_val(&*a2.0), size_of_val(&*a2.0));

    println!("Outer {:p}", arced.0.heap_ptr());
    println!("Arced {:p}", arced.0);
    println!("arced data {:p}", arced.net_header_ref());

    let raw = arced.into_raw_arc();
    println!("Raw {:p}", raw);
    let href = unsafe { &*(raw) }.net_header_ref();
    println!("raw data {:p}", href);
    let arced = unsafe{NetPktArc::from_raw_arc(raw)};
    println!("arced back {:p}", arced.0);
    println!("arced back data {:p}", arced.net_header_ref());

    let h2 = arced.net_header_ref();
    assert_eq!(href, h2);
    assert_eq!(href, parts.net_header_ref());

    let mut tmp = None;
    unsafe { &*raw }.with_arc(|i| tmp = Some(i.clone()));
    assert_eq!(Arc::count(&arced.0), 2);
    std::mem::drop(tmp);
    assert_eq!(Arc::count(&arced.0), 1);
}
