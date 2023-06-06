
// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
#[test]
fn build() {
    let pkt = datapoint(&[], ()).as_netbox();
    let ok = pkt.check(true);
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
    let ok = pkt.check(true);
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
    bytes.check(true).unwrap();
    assert_eq!(bytes.data(), b"datatest");

    let parts = bytes.as_netparts();
    assert_eq!(parts.hash(), linkpoint.hash());
    assert_eq!(linkpoint.point_parts, parts.point_parts);
    let b2 = parts.as_netbox();
    assert_eq!(b2.pkt_bytes(), bytes.pkt_bytes());
    println!("Hash {:?}", parts);

    let signkey = public_testkey();

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

    use linkspace_cryptography::*;
    let ahead = keypoint.as_keypoint().unwrap().head;
    let hash = linkpoint.hash();
    assert_eq!(ahead.signed.linkpoint_hash, hash);
    let signature = sign_hash(&signkey, &hash);
    assert_eq!(*ahead.signed.signature, signature);
    assert_eq!(*ahead.signed.pubkey, *signkey.pubkey());
    let pubkey = signkey.0.verifying_key().to_bytes();
    validate_signature(&pubkey.as_slice().try_into().unwrap(), &signature, &hash).expect("HASH OK");

    let bytes = keypoint.as_netbox();
    bytes.check(true).expect("check ok");
    let parts = bytes.as_netparts();
    assert_eq!(parts.hash(), keypoint.hash());
}

pub fn _domain_access_0(p:&dyn NetPkt) -> Domain{
    *p.as_point().get_domain()
}


// ideally these two functions result in the same assembly
// at last check its not perfect, but _1 is close enough, no panics and cmove instead of branches.
pub fn _domain_access_1(p: &NetPktPtr) -> Domain {
    *p.as_point().get_domain()
}
pub fn _domain_access_2(p:&NetPktPtr) -> Domain{
    let mut ptr : *const u8 = [0;32].as_ptr();
    let point : *const u8 = unsafe { (&p.point as *const PointThinPtr as *const u8).add(size_of::<PointHeader>())};
    if p.point.0.point_type.contains(PointTypeFlags::LINK){ ptr = unsafe{&*(point as *const LinkPointHeader)}.domain.0.as_ptr()  };
    if p.point.0.point_type.contains(PointTypeFlags::SIGNATURE){ ptr = unsafe{&*(point as *const KeyPointHeader)}.linkpoint.domain.0.as_ptr()  };
    unsafe{*(ptr as *const Domain)}
}


// ideally these two functions result in the same assembly
pub fn _key_access_1(p: &NetPktPtr) -> PubKey{
    *p.as_point().get_pubkey()
}
pub fn _key_access_2(p:&NetPktPtr) -> PubKey{
    let mut ptr : *const u8 = [0;32].as_ptr();
    let point : *const u8 = unsafe { (&p.point as *const PointThinPtr as *const u8).add(size_of::<PointHeader>())};
    if p.point.0.point_type.contains(PointTypeFlags::SIGNATURE){ ptr = unsafe{&*(point as *const KeyPointHeader)}.signed.pubkey.as_ptr()};
    unsafe{*(ptr as *const PubKey)}
}

#[test]
pub fn access_test() {
    use linkspace_cryptography::public_testkey;
    let key = public_testkey();

    let dp = datapoint(b"hello", ()).as_netbox();
    let lk = linkpoint(B64([1;32]),ab(b"ok"),IPath::empty(),&[],&[],Stamp::ZERO,()).as_netbox();
    let sp = keypoint(B64([1;32]),ab(b"ok"),IPath::empty(),&[],&[],Stamp::ZERO,&key,()).as_netbox();

    assert_eq!(_domain_access_1(&dp),_domain_access_2(&dp));
    assert_eq!(_domain_access_1(&lk),_domain_access_2(&lk));
    assert_eq!(_domain_access_1(&sp),_domain_access_2(&sp));

    assert_eq!(_key_access_1(&dp),_key_access_2(&dp));
    assert_eq!(_key_access_1(&lk),_key_access_2(&lk));
    assert_eq!(_key_access_1(&sp),_key_access_2(&sp));
}
