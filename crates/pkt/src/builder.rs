// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_cryptography::SigningKey;

use crate::*;

pub fn datapoint(data: &[u8], netopts: impl Into<NetOpts>) -> NetPktParts<'_> {
    try_datapoint_ref(data, netopts.into()).unwrap()
}
pub fn try_datapoint_ref(data: &[u8], netopts: NetOpts) -> Result<NetPktParts<'_>, Error> {
    let pkt_parts = PointParts {
        pkt_header: PointHeader::new(PointTypeFlags::DATA_POINT, data.len())?,
        fields: PointFields::DataPoint(data),
    };
    let hash = pkt_parts.compute_hash();
    Ok(NetPktParts {
        net_header: netopts.into(),
        hash,
        point_parts: pkt_parts,
    })
}

// TODO : decide if all default arguments should produce a try_datapoint and ifso, what the defaults are.
#[allow(clippy::too_many_arguments)]
pub fn try_point<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
    signkey: Option<&SigningKey>,
    netopts: impl Into<NetOpts>,
) -> Result<NetPktParts<'t>, Error> {
    let netopts = netopts.into();
    match signkey {
        Some(key) => try_keypoint_ref(group, domain, ipath, links, data, stamp, key, netopts),
        None => try_linkpoint_ref(group, domain, ipath, links, data, stamp, netopts),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn point<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
    signkey: Option<&SigningKey>,
    netopts: impl Into<NetOpts>,
) -> NetPktParts<'t> {
    try_point(
        group,
        domain,
        ipath,
        links,
        data,
        stamp,
        signkey,
        netopts.into(),
    )
    .unwrap()
}
pub fn linkpoint<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
    netopts: impl Into<NetOpts>,
) -> NetPktParts<'t> {
    let netopts = netopts.into();
    try_linkpoint_ref(group, domain, ipath, links, data, stamp, netopts).unwrap()
}
#[allow(clippy::too_many_arguments)]
pub fn keypoint<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
    signkey: &SigningKey,
    netopts: impl Into<NetOpts>,
) -> NetPktParts<'t> {
    let netopts = netopts.into();
    try_keypoint_ref(group, domain, ipath, links, data, stamp, signkey, netopts).unwrap()
}

fn linkp<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
) -> Result<(PointHeader, LinkPoint<'t>), Error> {
    ipath.check_components().unwrap();
    let tail = Tail { links, data, ipath };
    let ipath_size = ipath.ipath_bytes().len();
    let offset_ipathu = (size_of::<PointHeader>() + size_of::<LinkPointHeader>()).saturating_add(std::mem::size_of_val(links));
    let offset_ipath = U16::new( offset_ipathu.try_into().map_err(|_| Error::ContentLen)?);
    let offset_data = U16::new((offset_ipathu + ipath_size).try_into().map_err(|_| Error::ContentLen)?);
    let pkt_header = PointHeader::new(
        PointTypeFlags::LINK_POINT,
        size_of::<LinkPointHeader>() + tail.byte_len(),
    )?;
    let sp = LinkPoint {
        head: LinkPointHeader {
            info: LinkPointInfo {
                offset_ipath,
                offset_data,
            },
            group,
            domain,
            create_stamp: stamp,
        },
        tail,
    };
    Ok((pkt_header, sp))
}

pub fn try_linkpoint_ref<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
    netopts: NetOpts,
) -> Result<NetPktParts<'t>, Error> {
    let (pkt_header, linkpoint) = linkp(group, domain, ipath, links, data, stamp)?;
    let pkt_parts = PointParts {
        pkt_header,
        fields: PointFields::LinkPoint(linkpoint),
    };
    let hash = pkt_parts.compute_hash();
    Ok(NetPktParts {
        net_header: netopts.into(),
        hash,
        point_parts: pkt_parts,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn try_keypoint_ref<'t>(
    group: GroupID,
    domain: Domain,
    ipath: &'t IPath,
    links: &'t [Link],
    data: &'t [u8],
    stamp: Stamp,
    signkey: &SigningKey,
    netopts: NetOpts,
) -> Result<NetPktParts<'t>, Error> {
    let (linkpoint_pkt_header, sp) = linkp(group, domain, ipath, links, data, stamp)?;
    let linkpoint_hash = PointParts {
        pkt_header: linkpoint_pkt_header,
        fields: PointFields::LinkPoint(sp),
    }
    .compute_hash();
    let signature = linkspace_cryptography::sign_hash(signkey, &linkpoint_hash.0).into();
    let pkt_parts = PointParts {
        pkt_header: PointHeader::new(
            PointTypeFlags::KEY_POINT,
            size_of::<KeyPointHeader>() + sp.tail.byte_len(),
        )?,
        fields: PointFields::KeyPoint(KeyPoint {
            head: KeyPointHeader {
                reserved: KeyPointPadding::default(),
                signed: Signed {
                    pubkey: signkey.pubkey(),
                    signature,
                    linkpoint_hash,
                },
                inner_point: linkpoint_pkt_header,
                linkpoint: sp.head,
            },
            tail: sp.tail,
        }),
    };
    let hash = pkt_parts.compute_hash();
    Ok(NetPktParts {
        net_header: netopts.into(),
        hash,
        point_parts: pkt_parts,
    })
}

pub fn errorpoint(error: &[u8], netopts: impl Into<NetOpts>) -> NetPktParts<'_> {
    __error_blk_ref(error, netopts.into())
}

fn __error_blk_ref(error: &[u8], netopts: NetOpts) -> NetPktParts<'_> {
    let pkt_parts = PointParts {
        pkt_header: PointHeader::new(PointTypeFlags::ERROR_POINT, error.len()).unwrap(),
        fields: PointFields::Error(error),
    };
    let hash = pkt_parts.compute_hash();
    NetPktParts {
        net_header: netopts.into(),
        hash,
        point_parts: pkt_parts,
    }
}

/// Calculate the free space left in bytes. Negative values means it will not fit.
#[inline]
#[allow(clippy::as_conversions)]
pub const fn calc_free_space(
    path: &SPath,
    links: &[Link],
    data : &[u8],
    signed:bool
) -> isize {
    let mut size = if signed { MAX_KEYPOINT_DATA_SIZE } else { MAX_LINKPOINT_DATA_SIZE} as isize;
    size = size.saturating_sub_unsigned(links.len() * std::mem::size_of::<Link>());
    if !path.spath_bytes().is_empty(){
        size = size.saturating_sub_unsigned(path.spath_bytes().len() + 8);
    }
    size = size.saturating_sub_unsigned(data.len());
    size
}
