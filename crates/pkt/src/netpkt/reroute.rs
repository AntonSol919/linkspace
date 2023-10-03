// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{cell::OnceCell, mem::size_of, ops::Deref};

use crate::{ByteSegments, NetPktArc, NetPktArcPtr, NetPktBox, NetPktExt};

use super::{NetPkt, NetPktHeader};

/// Wrapper around a netpkt with a mutable NetHeader
#[derive(Debug, Clone)]
pub struct ReroutePkt<T: ?Sized> {
    pub net_header: NetPktHeader,
    pub pkt: T,
}
impl<T: NetPkt> ReroutePkt<T> {
    pub fn new(pkt: T) -> Self {
        ReroutePkt {
            net_header: pkt.net_header(),
            pkt,
        }
    }
}
impl<T: ?Sized> Deref for ReroutePkt<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.pkt
    }
}

impl<T: NetPkt + ?Sized> NetPkt for ReroutePkt<T> {
    fn net_header_mut(&mut self) -> Option<&mut NetPktHeader> {
        Some(&mut self.net_header)
    }

    fn hash_ref(&self) -> &crate::LkHash {
        self.pkt.hash_ref()
    }

    fn net_header_ref(&self) -> &NetPktHeader {
        &self.net_header
    }
    fn byte_segments(&self) -> ByteSegments {
        let mut segm = self.pkt.byte_segments();
        segm.0[0] = &segm.0[0][size_of::<NetPktHeader>()..];
        segm.push_front(self.net_header.as_bytes())
    }
    fn as_netbox(&self) -> crate::NetPktBox {
        let mut v = self.pkt.as_netbox();
        v._net_header = self.net_header;
        v
    }

    fn as_point(&self) -> &dyn crate::Point {
        self.pkt.as_point()
    }

    fn recv(&self) -> Option<crate::Stamp> {
        self.pkt.recv()
    }
}

/// Wrapper around a NetPkt that sets its recv field
///
/// For [NetPkt::recv] and during abe evaluation
#[derive(Debug, Clone, Copy)]
pub struct RecvPkt<T: ?Sized = NetPktBox> {
    pub recv: crate::Stamp,
    pub pkt: T,
}
impl<A> From<&dyn NetPkt> for RecvPkt<A>
where
    A: for<'o> From<&'o dyn NetPkt>,
{
    fn from(value: &dyn NetPkt) -> Self {
        Self::from_dyn(value)
    }
}
impl<A> RecvPkt<A> {
    pub fn from_dyn(pkt: &dyn NetPkt) -> RecvPkt<A>
    where
        A: for<'o> From<&'o dyn NetPkt>,
    {
        RecvPkt {
            recv: pkt.get_recv(),
            pkt,
        }
        .map(A::from)
    }
    pub fn map<B>(self, fnc: impl FnOnce(A) -> B) -> RecvPkt<B> {
        RecvPkt {
            pkt: fnc(self.pkt),
            recv: self.recv,
        }
    }
    pub fn owned(self) -> RecvPkt
    where
        A: NetPkt,
    {
        self.map(|v| v.as_netbox())
    }
}
impl<T: ?Sized> Deref for RecvPkt<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.pkt
    }
}

// TODO  add auto methods
impl<T: NetPkt + ?Sized> NetPkt for RecvPkt<T> {
    fn hash_ref(&self) -> &crate::LkHash {
        self.pkt.hash_ref()
    }
    fn recv(&self) -> Option<crate::Stamp> {
        Some(self.recv)
    }

    fn net_header_ref(&self) -> &NetPktHeader {
        self.pkt.net_header_ref()
    }

    fn byte_segments(&self) -> ByteSegments {
        self.pkt.byte_segments()
    }

    fn as_point(&self) -> &dyn crate::Point {
        self.pkt.as_point()
    }
}

/// Wrapper around a NetPkt that ensures .as_netarc() is only called once.
#[derive(Debug, Clone)]
pub struct ShareArcPkt<T: ?Sized> {
    pub arc: OnceCell<NetPktArc>,
    pub pkt: T,
}
impl<T: NetPkt> ShareArcPkt<T> {
    pub fn new(pkt: T) -> Self {
        ShareArcPkt {
            pkt,
            arc: OnceCell::new(),
        }
    }
}
impl<T: NetPkt + ?Sized> ShareArcPkt<T> {
    pub fn borrow_arc(&self) -> &NetPktArcPtr {
        self.arc.get_or_init(|| self.pkt.as_netarc())
    }
}
impl<T: ?Sized> Deref for ShareArcPkt<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.pkt
    }
}

// TODO  add auto methods
impl<T: NetPkt + ?Sized> NetPkt for ShareArcPkt<T> {
    fn hash_ref(&self) -> &crate::LkHash {
        self.pkt.hash_ref()
    }
    fn recv(&self) -> Option<crate::Stamp> {
        self.pkt.recv()
    }

    fn net_header_ref(&self) -> &NetPktHeader {
        self.pkt.net_header_ref()
    }

    fn as_netarc(&self) -> NetPktArc {
        self.arc.get_or_init(|| self.pkt.as_netarc()).clone()
    }

    fn byte_segments(&self) -> ByteSegments {
        self.pkt.byte_segments()
    }

    fn as_point(&self) -> &dyn crate::Point {
        self.pkt.as_point()
    }
}
