use std::sync::LazyLock;

// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_cryptography::public_testkey;
use linkspace_pkt::*;
pub const B64_HASH_LENGTH: usize = 43;
pub use linkspace_pkt::consts::*;
pub use linkspace_pkt::consts as pkt_consts;

pub static TEST_GROUP_PKT : LazyLock<NetPktBox> = LazyLock::new(|| datapoint(b"Test Group", NetPktHeader::EMPTY).as_netbox());
pub static TEST_GROUP_ID : LazyLock<LkHash> = LazyLock::new(|| TEST_GROUP_PKT.hash());
pub static PUBLIC_GROUP_PKT : LazyLock<NetPktBox> = LazyLock::new(|| datapoint(b"Hello, Sol", NetPktHeader::EMPTY).as_netbox());
pub static SINGLE_LINK_PKT: LazyLock<NetPktBox> = LazyLock::new(|| linkpoint(
        PRIVATE,
        ab(b""),
        IPath::empty(),
        &[Link {
            tag: ab(b""),
            ptr: B64([0; 32])
        }],
        &[0],
        Stamp::new(0),
        NetPktHeader::EMPTY
    )
    .as_netbox());

pub const PRIVATE: LkHash = B64([0; 32]);
pub const PUBLIC_GROUP_B64: &str = "RD3ltOheG4CrBurUMntnhZ8PtZ6yAYF_C1urKGZ0BB0";
pub const PUBLIC: LkHash = B64([
    68, 61, 229, 180, 232, 94, 27, 128, 171, 6, 234, 212, 50, 123, 103, 133, 159, 15, 181, 158,
    178, 1, 129, 127, 11, 91, 171, 40, 102, 116, 4, 29,
]);

/// pull requests are saved here.
pub static EXCHANGE_DOMAIN: Domain = abx(b"exchange");

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
pub fn static_pkts() -> Vec<NetPktBox> {
    let links = [
        Link::new("pub", PUBLIC),
        Link::new("test", *TEST_GROUP_ID),
    ];
    let mut list = vec![];
    // This is mostly to ensure code never depends on stamps being uniq or within some range.
    for sp_segm in ["hello", "sol"] {
        let spath = ipath_buf(&[b"staticpkt", sp_segm.as_bytes()]);
        for stamp in [Stamp::ZERO, Stamp::MAX] {
            list.push(
                linkpoint(PUBLIC, ab(b"test"), &spath, &links, &[], stamp, ()).as_netbox(),
            );
            list.push(
                keypoint(
                    PUBLIC,
                    ab(b"test"),
                    &spath,
                    &links,
                    &[],
                    stamp,
                    &public_testkey(),
                    (),
                )
                .as_netbox(),
            );
        }
        for stamp in [Stamp::new(1), Stamp::new(u64::MAX - 1)] {
            list.push(
                linkpoint(PUBLIC, ab(b"test"), &spath, &links, &[], stamp, ()).as_netbox(),
            );
        }
    }
    list.push(PUBLIC_GROUP_PKT.as_netbox());
    list.push(TEST_GROUP_PKT.as_netbox());
    list
    //vec![]
}
