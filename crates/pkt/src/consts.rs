#![allow(clippy::as_conversions)]
// === size/len constraints. By Convention '_size' is n bytes . '_len' is number of elements
use super::*;
use std::{mem::size_of, sync::LazyLock};

pub const MIN_POINT_SIZE: usize = size_of::<LkHash>() + size_of::<PointHeader>();
pub const MIN_LINKPOINT_SIZE: usize = MIN_POINT_SIZE + size_of::<LinkPointHeader>();
pub const MIN_NETPKT_SIZE: usize = size_of::<NetPktHeader>() + MIN_POINT_SIZE;

pub const MAX_POINT_SIZE: usize = u16::MAX as usize - 512;
// ensure compiler error if it exceeds u16::MAX
pub const MAX_NETPKT_U16SIZE: u16 = MAX_POINT_SIZE as u16 + size_of::<NetPktHeader>() as u16 + size_of::<LkHash>() as u16; 
pub const MAX_NETPKT_SIZE: usize = MAX_NETPKT_U16SIZE as usize;

pub const MAX_CONTENT_SIZE: usize = MAX_POINT_SIZE - size_of::<PointHeader>();


pub const MAX_DATA_SIZE: usize = MAX_CONTENT_SIZE;
pub const MAX_LINKPOINT_DATA_SIZE: usize = MAX_CONTENT_SIZE - size_of::<LinkPointHeader>();
pub const MAX_KEYPOINT_DATA_SIZE: usize = MAX_CONTENT_SIZE - size_of::<KeyPointHeader>();
pub const MAX_LINKS_LEN: usize = (MAX_POINT_SIZE - MAX_SPATH_SIZE) / size_of::<Link>();
pub const MAX_SPATH_SIZE: usize = 242;
pub const MAX_IPATH_SIZE: usize = MAX_SPATH_SIZE + 8;
pub const MAX_SPATH_COMPONENT_SIZE: usize = 200;
pub const MAX_PATH_LEN: usize = 8;

pub static TEST_GROUP_PKT: LazyLock<NetPktBox> =
    LazyLock::new(|| datapoint(b"Test Group", NetPktHeader::EMPTY).as_netbox());
pub static TEST_GROUP: LazyLock<LkHash> = LazyLock::new(|| TEST_GROUP_PKT.hash());
pub static PUBLIC_GROUP_PKT: LazyLock<NetPktBox> =
    LazyLock::new(|| datapoint(b"Hello, Sol", NetPktHeader::EMPTY).as_netbox());
pub static SINGLE_LINK_PKT: LazyLock<NetPktBox> = LazyLock::new(|| {
    linkpoint(
        PRIVATE,
        ab(b""),
        IPath::empty(),
        &[Link {
            tag: ab(b""),
            ptr: B64([0; 32]),
        }],
        &[0],
        Stamp::new(0),
        NetPktHeader::EMPTY,
    )
    .as_netbox()
});

pub const PRIVATE: LkHash = B64([0; 32]);
pub const PUBLIC_GROUP_B64: &str = "RD3ltOheG4CrBurUMntnhZ8PtZ6yAYF_C1urKGZ0BB0";
pub const PUBLIC: LkHash = B64([
    68, 61, 229, 180, 232, 94, 27, 128, 171, 6, 234, 212, 50, 123, 103, 133, 159, 15, 181, 158,
    178, 1, 129, 127, 11, 91, 171, 40, 102, 116, 4, 29,
]);

//static consistency check
const _EQ_ASSERT_SIZE: fn() = || {
    let _ = core::mem::transmute::<[u8;MAX_DATA_SIZE], [u8;MAX_NETPKT_SIZE - size_of::<PartialNetHeader>()]>;
};
#[test]
fn correct_public_ids() {
    assert_eq!(PUBLIC, PUBLIC_GROUP_PKT.hash());
    assert_eq!(PUBLIC_GROUP_B64, PUBLIC_GROUP_PKT.hash().b64());
    let p = PUBLIC_GROUP_PKT.as_netparts().fields;
    match p {
        PointFields::DataPoint(p) => assert_eq!(p.len(), b"Hello, Sol".len()),
        _ => panic!(),
    }
}
