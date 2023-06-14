// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;


/// The rusty enum repr of a point.
/// Constructed through datapoint, linkpoint, and keypoint
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct PointParts<'a> {
    pub pkt_header: PointHeader,
    pub fields: PointFields<'a>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum PointFields<'a> {
    DataPoint(&'a [u8]),
    LinkPoint(LinkPoint<'a>),
    KeyPoint(KeyPoint<'a>),
    Error(&'a [u8]),
    Unknown(&'a [u8]),
}
impl<'a> PointFields<'a> {
    pub fn common_idx(&self) -> Option<(&LinkPointHeader, &IPath, Option<&PubKey>)> {
        match self {
            PointFields::LinkPoint(sp) => Some((&sp.head, sp.tail.ipath, None)),
            PointFields::KeyPoint(a) => {
                Some((&a.head.linkpoint, a.tail.ipath, Some(&a.head.signed.pubkey)))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[repr(C, align(4))]
pub struct LinkPoint<'a> {
    pub head: LinkPointHeader,
    pub tail: Tail<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tail<'a> {
    pub links: &'a [Link],
    pub data: &'a [u8],
    pub ipath: &'a IPath,
}

/// A signed [[LinkPoint]]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct KeyPoint<'a> {
    pub head: KeyPointHeader,
    pub tail: Tail<'a>,
}

impl<'a> Tail<'a> {
    pub fn links_as_bytes(&'a self) -> &'a [u8] {
        unsafe {
            from_raw_parts(
                self.links.as_ptr().cast::<u8>(),
                std::mem::size_of_val(self.links),
            )
        }
    }
    pub fn byte_len(&self) -> usize {
        self.data.len() + self.ipath.ipath_bytes().len() + self.links_as_bytes().len()
    }
}

impl<'tail> PointParts<'tail> {
    pub fn data_ptr(&self) -> &'tail [u8] {
        match self.fields {
            PointFields::DataPoint(b) => b,
            PointFields::LinkPoint(LinkPoint {  tail, .. }) => tail.data,
            PointFields::KeyPoint(KeyPoint {  tail,.. }) => tail.data,
            PointFields::Error(b) => b,
            PointFields::Unknown(o) => o,
        }
    }
}

impl<'tail> Point for PointParts<'tail> {
    #[inline(always)]
    fn parts(&self) -> PointParts<'tail> {
        *self
    }
    #[inline(always)]
    fn pkt_segments(&self) -> ByteSegments {
        let pkt_head = self.pkt_header.as_bytes();
        match &self.fields {
            PointFields::Unknown(b) => ByteSegments::from_array([pkt_head, b]),
            PointFields::DataPoint(b) => ByteSegments::from_array([pkt_head, b]),
            PointFields::Error(b) => ByteSegments::from_array([pkt_head, b]),
            PointFields::LinkPoint(LinkPoint { head, tail }) => ByteSegments::from_array([
                pkt_head,
                head.as_bytes(),
                tail.links_as_bytes(),
                tail.ipath.ipath_bytes(),
                tail.data,
            ]),
            PointFields::KeyPoint(KeyPoint { head, tail }) => ByteSegments::from_array([
                pkt_head,
                head.as_bytes(),
                tail.links_as_bytes(),
                tail.ipath.ipath_bytes(),
                tail.data,
            ]),
        }
    }

    #[inline(always)]
    fn data(&self) -> &[u8] {
        self.data_ptr()
    }

    #[inline(always)]
    fn tail(&self) -> Option<Tail> {
        match self.fields {
            PointFields::LinkPoint(LinkPoint {  tail ,.. }) => Some(tail),
            PointFields::KeyPoint(KeyPoint {  tail , .. }) => Some(tail),
            _ => None,
        }
    }
    #[inline(always)]
    fn point_header_ref(&self) -> &PointHeader {
        &self.pkt_header
    }
    #[inline(always)]
    fn linkpoint_header(&self) -> Option<&LinkPointHeader> {
        match &self.fields {
            PointFields::LinkPoint(LinkPoint { head, ..}) => Some(head),
            PointFields::KeyPoint(KeyPoint { head, ..}) => Some(&head.linkpoint),
            _ => None,
        }
    }

    fn keypoint_header(&self) -> Option<&KeyPointHeader> {
        match &self.fields {
            PointFields::KeyPoint(h) => Some(&h.head),
            _ => None,
        }
    }
}

impl<'o> core::fmt::Debug for PointFields<'o> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown(arg0) => f.debug_tuple("Unknown").field(arg0).finish(),
            Self::DataPoint(arg0) => {
                let name = if arg0.len() > 400 {
                    format!("DataPoint[0..{}][..400]", arg0.len())
                } else {
                    "DataPoint".to_string()
                };
                f.debug_tuple(&name)
                    .field(&AB(&arg0[0..arg0.len().min(400)]))
                    .finish()
            }
            Self::LinkPoint(arg0) => f.debug_tuple("LinkPoint").field(arg0).finish(),
            Self::KeyPoint(arg0) => f.debug_tuple("KeyPoint").field(arg0).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(&AB(&arg0)).finish(),
        }
    }
}
