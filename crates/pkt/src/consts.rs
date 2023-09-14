
// === size/len constraints. By Convention '_size' is n bytes . '_len' is number of elements
use super::*;
use std::{mem::size_of, sync::LazyLock};

pub const MAX_POINT_SIZE: usize = (u16::MAX as usize+1) - 512;

pub const MIN_POINT_SIZE: usize = size_of::<LkHash>() + size_of::<PointHeader>();
pub const MIN_LINKPOINT_SIZE: usize = MIN_POINT_SIZE + size_of::<LinkPointHeader>();
pub const MIN_NETPKT_SIZE: usize = size_of::<NetPktHeader>() + MIN_POINT_SIZE;

// ensure compiler error if it exceeds u16::MAX
pub const MAX_NETPKT_U16SIZE: u16 = MAX_POINT_SIZE as u16 + size_of::<NetPktHeader>() as u16 + size_of::<LkHash>() as u16; 
pub const MAX_NETPKT_SIZE: usize = MAX_NETPKT_U16SIZE as usize;

pub const MAX_CONTENT_SIZE: usize = MAX_POINT_SIZE - size_of::<PointHeader>();


pub const MAX_DATA_SIZE: usize = MAX_CONTENT_SIZE;
pub const MAX_LINKPOINT_DATA_SIZE: usize = MAX_CONTENT_SIZE - size_of::<LinkPointHeader>();
pub const MAX_KEYPOINT_DATA_SIZE: usize = MAX_CONTENT_SIZE - size_of::<Signed>();
pub const MAX_LINKS_LEN: usize = (MAX_POINT_SIZE - MAX_SPACENAME_SIZE) / size_of::<Link>();
pub const MAX_SPACENAME_SIZE: usize = 242;
pub const MAX_ROOTED_SPACENAME_SIZE: usize = MAX_SPACENAME_SIZE + 8;
pub const MAX_SPACENAME_COMPONENT_SIZE: usize = 200;
pub const MAX_SPACE_DEPTH: usize = 8;

pub static TEST_GROUP_PKT: LazyLock<NetPktBox> =
    LazyLock::new(|| datapoint(b"Test Group", NetPktHeader::EMPTY).as_netbox());
pub static TEST_GROUP: LazyLock<LkHash> = LazyLock::new(|| TEST_GROUP_PKT.hash());
pub static PUBLIC_GROUP_PKT: LazyLock<NetPktBox> =
    LazyLock::new(|| datapoint(b"Hello, Sol!\n", NetPktHeader::EMPTY).as_netbox());
pub static SINGLE_LINK_PKT: LazyLock<NetPktBox> = LazyLock::new(|| {
    linkpoint(
        PRIVATE,
        ab(b""),
        RootedSpace::empty(),
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

pub const PUBLIC_GROUP_B64: &str = "Yrs7iz3VznXh-ogv4aM62VmMNxXFiT4P24tIfVz9sTk";
pub const PUBLIC: LkHash = B64([98, 187, 59, 139, 61, 213, 206, 117, 225, 250, 136, 47, 225, 163, 58, 217, 89, 140, 55, 21, 197, 137, 62, 15, 219, 139, 72, 125, 92, 253, 177, 57]);

//static consistency check
const _EQ_ASSERT_SIZE: fn() = || {
    let _ = core::mem::transmute::<[u8;MAX_DATA_SIZE], [u8;MAX_NETPKT_SIZE - size_of::<PartialNetHeader>()]>;
    let _ = core::mem::transmute::<[u8;MAX_DATA_SIZE], [u8;65020]>;
};
#[test]
fn correct_public_ids() {
    use linkspace_cryptography::blake3_hash;
    let bytes = PUBLIC_GROUP_PKT.as_point().pkt_segments();
    assert_eq!(&bytes.0[0],&[0,1,0,16,72,101,108,108,111,44,32,83,111,108,33,10]);
    assert_eq!(blake3_hash(&bytes.0[0]), PUBLIC_GROUP_PKT.hash().0);
    
    assert_eq!(PUBLIC, PUBLIC_GROUP_B64.parse().unwrap(),"{:?}",PUBLIC_GROUP_PKT.hash().0);
    assert_eq!(PUBLIC, PUBLIC_GROUP_PKT.hash());
    assert_eq!(PUBLIC, PUBLIC_GROUP_PKT.as_point().compute_hash());
    assert_eq!(PUBLIC, PUBLIC_GROUP_PKT.as_point().parts().compute_hash());
    assert_eq!(PUBLIC_GROUP_B64, PUBLIC_GROUP_PKT.hash().b64());
    let p = PUBLIC_GROUP_PKT.as_netparts().fields;
    match p {
        PointFields::DataPoint(p) => assert_eq!(p.len(), b"Hello, Sol!\n".len()),
        _ => panic!(),
    }
}
