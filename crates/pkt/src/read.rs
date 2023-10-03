use std::{ptr, borrow::Cow};

use crate::{Error, NetPktFatPtr, PartialNetHeader, MIN_NETPKT_SIZE, NetPktPtr};

pub fn read_pkt(bytes: &[u8], skip_hash_check: bool) -> Result<Cow<NetPktPtr>, Error> {
    if bytes.len() < MIN_NETPKT_SIZE {
        return Err(Error::MissingHeader);
    }
    let partial_header = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<PartialNetHeader>()) };
    partial_header.point_header.check()?;
    let netpkt_size = partial_header.point_header.size();
    if usize::from(netpkt_size) > bytes.len() {
        return Err(Error::MissingBytes { netpkt_size });
    }
    if bytes.as_ptr().is_aligned_to(8){
        let pkt = unsafe { &*bytes.as_ptr().cast::<NetPktPtr>()};
        pkt.check(skip_hash_check)?;
        return Ok(Cow::Borrowed(pkt));
    }
    let mut pkt: Box<NetPktFatPtr>= unsafe { partial_header.alloc() };
    {
        let npf : &mut NetPktFatPtr = &mut pkt;
        let s: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut(
                ptr::from_mut(npf).cast::<u8>(),
                usize::from(netpkt_size) ,
            )
        };
        s.copy_from_slice(&bytes[.. netpkt_size.into()]);
    };
    pkt.check(skip_hash_check)?;
    Ok(Cow::Owned(pkt))
}

#[test]
fn parsing() {
    use crate::*;
    fn test(parts: NetPktParts) {
    let boxed_parts = parts.as_netbox();
    let arc_parts = parts.as_netarc();
    let arc_box_parts = boxed_parts.as_netarc();
    let parts_arc_box = arc_box_parts.as_netparts();
    let parts_box_parts = boxed_parts.as_netparts();
    let box_arc_box_parts = arc_box_parts.as_netbox();

    let mut data: Vec<u8> = parts.byte_segments().collect();
    let extra = data.as_slice().as_ptr().align_offset(8);
    let _ = data.splice(0..0, 0..extra as u8).collect::<Vec<_>>();
    let data = &data[extra..];
    let cow_read = read_pkt(data,true).unwrap();
    assert!(matches!(cow_read,Cow::Borrowed(_)));
    let box_parse = cow_read.as_netbox();
    let arc_parse = cow_read.as_netarc();
    let parts_parse = cow_read.as_netparts();


    let mut d2 = data.to_vec();
    d2.insert(0,0);
    let cow_read2 = read_pkt(&d2[1..],true).unwrap();
    assert!(matches!(cow_read2,Cow::Owned(_)));
    let box_parse2 = cow_read2.as_netbox();
    let arc_parse2 = cow_read2.as_netarc();
    let parts_parse2 = cow_read2.as_netparts();
    let cw = cow_read.as_ref();
    let cw2 = cow_read2.as_ref();
    macro_rules! lst {
        ($($x:expr),+ $(,)?) => (
            vec![ $( ( stringify!($x),$x) ),* ]
        )
    }
    let list: Vec<(&str,&dyn NetPkt)> = lst![
        &parts,
        &boxed_parts,
        &arc_box_parts,
        &arc_parts,
        &parts_arc_box,
        &parts_box_parts,
        &box_arc_box_parts,
        &cw,
        &box_parse,
        &arc_parse,
        &parts_parse,

        &cw2,
        &box_parse2,
        &arc_parse2,
        &parts_parse2,
    ];

    for (name,el) in &list {
        eprintln!("{name}");
        let vec: Vec<u8> = el.byte_segments().collect();
        assert_eq!(vec, data,"{name}");
    }
    }
    let space = rspace_buf(&[b"hello", b"world"]);

    test(datapoint(b"hello", ()));
    let lp = builder::linkpoint(
        [4; 32].into(),
        [1; 16].into(),
        &space,
        &[],
        b"datatest",
        Stamp::new(2),
        (),
    );

    test(lp);
    let signkey = linkspace_cryptography::public_testkey();
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
    test(keypoint);
}
