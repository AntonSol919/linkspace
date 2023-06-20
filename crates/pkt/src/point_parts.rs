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


#[derive(Clone, Copy, Eq, PartialEq,Debug)]
#[repr(C,align(4))]
pub struct Signed{
    pub pubkey:PubKey,
    pub signature:Signature
}
impl Signed {
    fn as_bytes(&self) -> &[u8] {
        unsafe{&*std::slice::from_raw_parts(std::ptr::from_ref(self).cast(),size_of::<Self>())}
    }
    pub(crate) fn validate(&self,hash:&[u8;32]) -> Result<(), linkspace_cryptography::Error> {
        linkspace_cryptography::validate_signature(
            &self.pubkey.0,
            &self.signature.0,
            &hash,
        )
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
#[non_exhaustive]
pub enum PointFields<'a> {
    DataPoint(&'a [u8]),
    LinkPoint(LinkPoint<'a>),
    KeyPoint(LinkPoint<'a>, Signed),
    Error(&'a [u8]),
    Unknown(&'a [u8]),
}
impl<'a> PointFields<'a> {
    pub fn common_idx(&self) -> Option<(&LinkPointHeader, &IPath, Option<&PubKey>)> {
        match self {
            PointFields::LinkPoint(sp) => Some((&sp.head, sp.tail.ipath, None)),
            PointFields::KeyPoint(sp,signed) => {
                Some((&sp.head, sp.tail.ipath, Some(&signed.pubkey)))
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
            PointFields::KeyPoint(LinkPoint{tail,..}, _) => tail.data,
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

        let padding = self.padding();
        match &self.fields {
            PointFields::Unknown(b) => ByteSegments::from_array([pkt_head, b,padding]),
            PointFields::DataPoint(b) => ByteSegments::from_array([pkt_head, b,padding]),
            PointFields::Error(b) => ByteSegments::from_array([pkt_head, b,padding]),
            PointFields::LinkPoint(LinkPoint { head, tail }) => ByteSegments::from_array([
                pkt_head,
                head.as_bytes(),
                tail.links_as_bytes(),
                tail.ipath.ipath_bytes(),
                tail.data,
                padding
            ]),
            PointFields::KeyPoint(LinkPoint{head, tail}, signed) => ByteSegments::from_array([
                pkt_head,
                head.as_bytes(),
                tail.links_as_bytes(),
                tail.ipath.ipath_bytes(),
                tail.data,
                padding,
                signed.as_bytes()
            ]),
        }
    }

    #[inline(always)]
    fn data(&self) -> &[u8] {
        self.data_ptr()
    }
    fn padding(&self) -> &[u8]{
        let pad_len = self.pkt_header.padded_point_size() - self.pkt_header.upoint_size();
        static PAD : [u8;8] = [255;8];
        &PAD[..pad_len as usize]
    }

    #[inline(always)]
    fn tail(&self) -> Option<Tail> {
        match self.fields {
            PointFields::LinkPoint(LinkPoint {  tail ,.. }) => Some(tail),
            PointFields::KeyPoint(LinkPoint{  tail , .. },_) => Some(tail),
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
            PointFields::KeyPoint(LinkPoint{ head, ..},_) => Some(head),
            _ => None,
        }
    }

    fn signed(&self) -> Option<&Signed> {
        match &self.fields {
            PointFields::KeyPoint(_,t) => Some(&t),
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
            Self::KeyPoint(arg0,arg1) => f.debug_tuple("NewKeyPoint").field(arg0).field(arg1).finish(),
            Self::Error(arg0) => f.debug_tuple("Error").field(&AB(&arg0)).finish(),
        }
    }
}
