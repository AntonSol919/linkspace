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
/// pull requests are saved here.
pub static EXCHANGE_DOMAIN: Domain = abx(b"exchange");



pub fn static_pkts() -> Vec<NetPktBox> {
    let links = [
        Link::new("pub", PUBLIC),
        Link::new("test", *TEST_GROUP),
    ];
    let mut list = vec![];
    // This is mostly to ensure code never depends on stamps being uniq or within some range.
    for sp_segm in ["hello", "sol"] {
        let rspace = rspace_buf(&[b"staticpkt", sp_segm.as_bytes()]);
        for stamp in [Stamp::ZERO, Stamp::MAX] {
            list.push(
                linkpoint(PUBLIC, ab(b"test"), &rspace, &links, &[], stamp, ()).as_netbox(),
            );
            list.push(
                keypoint(
                    PUBLIC,
                    ab(b"test"),
                    &rspace,
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
                linkpoint(PUBLIC, ab(b"test"), &rspace, &links, &[], stamp, ()).as_netbox(),
            );
        }
    }
    list.push(PUBLIC_GROUP_PKT.as_netbox());
    list.push(TEST_GROUP_PKT.as_netbox());
    list
    //vec![]
}
