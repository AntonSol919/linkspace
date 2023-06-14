// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/*

Experiment with a common enum layout for FFI instead of using &dyn NetPkt

*/

use crate::{ NetPktPtr, NetPktHeader, reroute::{Reroute, RecvPkt, PktShareArc}, NetFields, NetPktArc, NetPktParts, NetPktBox, Point };

#[derive(Debug,Clone)]
pub struct IPkt<'o>(InnerIPkt<'o>);
#[derive(Debug,Clone)]
enum InnerIPkt<'o>{
    Constr(PktShareArc<NetPktParts<'o>>),
    Ref(Reroute<RecvPkt<PktShareArc<&'o NetPktPtr>>>),
    Box(RecvPkt<PktShareArc<NetPktBox>>),
    Arc(Reroute<RecvPkt<NetPktArc>>)
}
pub fn from_arc(_pkt:NetPktArc,_recv:Option<crate::Stamp>) -> IPkt<'static>{
    todo!()
}
pub fn from_ptr<'o>(_pkt: &'o NetPktPtr, _recv:Option<crate::Stamp>) -> IPkt<'o>{
    todo!()
}
pub fn from_pkt<'o>(_pkt:&dyn NetFields, _recv:Option<crate::Stamp>) -> IPkt<'static>{
    todo!()
}
pub fn from_cstr<'o>(_pkt:NetPktParts<'o>, _recv:Option<crate::Stamp>) -> IPkt<'o>{
    todo!()
}

impl<'o> IPkt<'o> {
    pub fn arc(&self) -> IPkt<'static>{
        todo!()
    }
    pub fn boxed(&self) -> IPkt<'static>{
        todo!()
    }
    
    fn mapn<A:'o>(&'o self, fnc: impl FnOnce(&'o dyn NetFields) -> A) -> A{
        match &self.0 {
            InnerIPkt::Constr(v) => fnc(v),
            InnerIPkt::Ref(v) => fnc(v),
            InnerIPkt::Box(v) => fnc(v),
            InnerIPkt::Arc(v) => fnc(v),
        }
    }
}

impl<'o> NetFields for IPkt<'o>{
    fn hash_ref(&self) ->  &crate::Hash {
        self.mapn(NetFields::hash_ref)
    }
    fn net_header_ref(&self) ->  &NetPktHeader {
        self.mapn(NetFields::net_header_ref)
    }

    fn point_header_ref(&self) ->  &crate::PointHeader {
        self.mapn(NetFields::point_header_ref)
    }

    fn as_netparts(&self) -> NetPktParts {
        self.mapn(NetFields::as_netparts)
    }

    fn byte_segments(&self) -> crate::ByteSegments {
        self.mapn(NetFields::byte_segments)
    }

    fn as_point(&self) ->  &dyn Point {
        match &self.0 {
            InnerIPkt::Constr(v) => v.as_point(),
            InnerIPkt::Ref(v) => v.pkt.pkt.pkt.as_point(),
            InnerIPkt::Box(v) => v.as_point(),
            InnerIPkt::Arc(v) => v.as_point(),
        }
    }
}
impl<'o> Point for IPkt<'o> {
    fn parts(&self) -> crate::PointParts {
        match &self.0{
            InnerIPkt::Constr(p) => p.parts(),
            InnerIPkt::Ref(p) => p.parts(),
            InnerIPkt::Box(p) => p.parts(),
            InnerIPkt::Arc(p) => p.parts(),
        }
    }

    fn data(&self) ->  &[u8] {
        todo!()
    }

    fn tail(&self) -> Option<crate::Tail>  {
        todo!()
    }

    fn linkpoint_header(&self) -> Option< &crate::LinkPointHeader>  {
        todo!()
    }

    fn keypoint_header(&self) -> Option< &crate::KeyPointHeader>  {
        todo!()
    }

    fn pkt_segments(&self) -> crate::ByteSegments {
        todo!()
    }

    fn point_header_ref(&self) ->  &crate::PointHeader {
        todo!()
    }
}


