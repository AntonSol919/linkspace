// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
#[test]
fn build() {
    let pkt = datapoint(&[], ()).as_netbox();
    let ok = pkt.check::<true>();
    assert!(ok.is_ok(), "{:?}", ok);
    let pkt = linkpoint(
        B64([0; 32]),
        AB([0; 16]),
        &ipath_buf(&[b"a"]),
        &[],
        b"ok",
        Stamp::ZERO,
        (),
    )
    .as_netbox();
    let ok = pkt.check::<true>();
    assert!(ok.is_ok(), "{:?}", ok);
}

#[test]
fn sanity() {
    let datablk = builder::datapoint(b"Hello world", ());
    let bytes: NetPktBox = datablk.as_netbox();
    {
        assert_eq!(
            &bytes.as_netpkt_bytes()[size_of::<PartialNetHeader>()..],
            b"Hello world"
        );
        let tmp = bytes.clone();
        assert_eq!(std::mem::size_of_val(&tmp), std::mem::size_of_val(&bytes));
        assert_eq!(
            tmp.as_netparts().point_parts,
            bytes.as_netparts().point_parts
        )
    }
    let parts = datablk.as_netparts();
    assert_eq!(parts.hash(), datablk.hash());
    let spath = ipath_buf(&[b"hello", b"world"]);
    let linkpoint = builder::linkpoint(
        [4; 32].into(),
        [1; 16].into(),
        &spath,
        &[],
        b"datatest",
        Stamp::new(2),
        (),
    );
    assert_eq!(linkpoint.get_ipath(), &*spath);
    assert_eq!(linkpoint.data(), b"datatest");
    let bytes = linkpoint.as_netbox();
    assert_eq!(bytes.get_ipath(), &*spath);
    bytes.check::<true>().unwrap();
    assert_eq!(bytes.data(), b"datatest");

    let parts = bytes.as_netparts();
    assert_eq!(parts.hash(), linkpoint.hash());
    assert_eq!(linkpoint.point_parts, parts.point_parts);
    let b2 = parts.as_netbox();
    assert_eq!(b2.pkt_bytes(), bytes.pkt_bytes());
    println!("Hash {:?}", parts);

    use linkspace_crypto::*;
    let signkey = linkspace_crypto::public_testkey();

    let keypoint = builder::keypoint(
        [4; 32].into(),
        [1; 16].into(),
        &spath,
        &[],
        b"datatest",
        Stamp::new(2),
        &signkey,
        (),
    );

    let ahead = keypoint.as_keypoint().unwrap().head;
    let hash = linkpoint.hash();
    assert_eq!(ahead.signed.linkpoint_hash, hash);
    let signature = sign_hash(&signkey, &hash);
    assert_eq!(*ahead.signed.signature, signature);
    assert_eq!(*ahead.signed.pubkey, *signkey.pubkey());
    let pubkey = signkey.0.verifying_key().to_bytes();
    validate_signature(&pubkey.as_slice().try_into().unwrap(), &signature, &hash).expect("HASH OK");

    let bytes = keypoint.as_netbox();
    bytes.check::<true>().expect("check ok");
    let parts = bytes.as_netparts();
    assert_eq!(parts.hash(), keypoint.hash());
}

pub fn access(p: &NetPktPtr) -> Domain {
    *p.as_point().get_domain()
}
