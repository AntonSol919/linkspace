
// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{* };
#[test]
fn build() {
    let pkt = datapoint(&[], ()).as_netbox();
    let ok = pkt.check(true);
    assert!(ok.is_ok(), "{:?}", ok);
    let pkt = linkpoint(
        B64([0; 32]),
        AB([0; 16]),
        &rspace_buf(&[b"a"]),
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
            concat_bytes!(b"Hello world",[255])
        );
        let tmp = bytes.clone();
        assert_eq!(std::mem::size_of_val(&tmp), std::mem::size_of_val(&bytes));
        assert_eq!(
            tmp.as_netparts().point_parts,
            bytes.as_netparts().point_parts
        );
        bytes.check(true).unwrap();

    }
    let parts = datablk.as_netparts();
    assert_eq!(parts.hash(), datablk.hash());
    let space = rspace_buf(&[b"hello", b"world"]);
    let linkpoint = builder::linkpoint(
        [4; 32].into(),
        [1; 16].into(),
        &space,
        &[],
        b"datatest",
        Stamp::new(2),
        (),
    );

    assert_eq!(linkpoint.get_rooted_spacename(), &*space);
    assert_eq!(linkpoint.data(), b"datatest");
    let bytes = linkpoint.as_netbox();
    assert_eq!(bytes.get_rooted_spacename(), &*space);
    assert_eq!(bytes.data(), b"datatest");
    bytes.check(true).unwrap();

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
        &space,
        &[],
        b"datatest",
        Stamp::new(2),
        &signkey,
        (),
    );
    assert_eq!(keypoint.get_rooted_spacename(), &*space);
    assert_eq!(keypoint.data(), b"datatest");
    let bytes = keypoint.as_netbox();
    assert_eq!(bytes.get_rooted_spacename(), &*space);
    assert_eq!(bytes.data(), b"datatest");
    bytes.check(true).unwrap();

    use linkspace_cryptography::*;
    
    {
        let mut kpy : NetPktParts<'_> = keypoint.clone();

        // the signature is for the hash of the linkpoint
        let (fields,signed) = match kpy.fields {
            PointFields::KeyPoint(h, s) => (PointFields::LinkPoint(h),s),
            _=> panic!()
        };
        kpy.point_parts.fields = fields;
        let hash = kpy.compute_hash();
        kpy.hash = hash;
        validate_signature(&signed.pubkey.as_slice().try_into().unwrap(), &signed.signature, &hash).expect("HASH OK");

        // removing signature flag and tail creates the same keypoint
        kpy.point_parts.pkt_header.point_type.remove(PointTypeFlags::SIGNATURE);
        kpy.point_parts.pkt_header.uset_bytes = (kpy.pkt_header.uset_bytes.get() - size_of::<Signed>() as u16).into();

        assert_eq!(kpy.pkt_header,linkpoint.point_header());
        let hash = kpy.compute_hash();
        kpy.hash = hash;

        assert_eq!(kpy, linkpoint);
    }

    let bytes = keypoint.as_netbox();
    eprintln!("{:#?}",bytes);
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
    *p.point.get_domain()
}

pub fn _domain_access_2(p:&NetPktPtr) -> Domain{
    let mut ptr : *const u8 = [0;16].as_ptr();
    let point : *const u8 = unsafe { (&p.point as *const PointThinPtr as *const u8).add(size_of::<PointHeader>())};
    if p.point.0.point_type.contains(PointTypeFlags::LINK){ ptr = unsafe{&*(point as *const LinkPointHeader)}.domain.0.as_ptr()  };
    unsafe{*(ptr as *const Domain)}
}

pub fn _domain_access_3(p:&NetPktPtr) -> Domain{
    let mut domain = &AB([0;16]);
    if let Some(header)= p.point.linkpoint_header(){
        domain = &header.domain
    }
    *domain
}
pub fn _check_interal(p:&NetPktPtr) -> bool{
    p.internal_consitent_length().is_ok()
}
pub fn _key_access_1(p: &NetPktPtr) -> PubKey{
    *p.as_point().get_pubkey()
}
#[test]
pub fn access_test() {
    use linkspace_cryptography::public_testkey;
    let key = public_testkey();

    let dp = datapoint(b"hello", ()).as_netbox();
    let lk = linkpoint(B64([1;32]),ab(b"ok"),RootedSpace::empty(),&[],&[],Stamp::ZERO,()).as_netbox();
    let sp = keypoint(B64([1;32]),ab(b"ok"),RootedSpace::empty(),&[],&[],Stamp::ZERO,&key,()).as_netbox();

    assert_eq!(_domain_access_1(&dp),_domain_access_2(&dp));
    assert_eq!(_domain_access_1(&lk),_domain_access_2(&lk));
    assert_eq!(_domain_access_1(&sp),_domain_access_2(&sp));
    assert_eq!(_domain_access_1(&sp),_domain_access_3(&sp));
    assert_eq!(_domain_access_1(&sp),_domain_access_3(&sp));
    assert_eq!(_domain_access_1(&sp),_domain_access_3(&sp));

}

pub fn check_group(group: &GroupID,mut b:&[u8]) -> Result<usize,Error>{
    let it = std::iter::from_fn(|| {
        if b.is_empty() { return None }
        let pkt : &NetPktPtr= unsafe { &*b.as_ptr().cast()};
        b = &b[pkt.size() as usize ..];
        Some(pkt)
    });
    let mut i = 0;
    for  p in it {
        if p.group() == Some(group){
           i+=1; 
        }
    }
    Ok(i)
}
