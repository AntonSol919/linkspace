
use crate::{ NetPktBox, MIN_NETPKT_SIZE, PartialNetHeader, NetPktFatPtr, NetPktArc, Error };



// Ok(Err(e)) means the packet requires at least e bytes.
pub fn parse_netpkt(
    bytes: &[u8],
    skip_hash_check:bool,
) -> Result<NetPktBox, Error> {
    if bytes.len() < MIN_NETPKT_SIZE {
        return Err(Error::MissingHeader);
    }
    let partial_header = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const PartialNetHeader) };
    partial_header.point_header.check()?;
    let netpkt_size = partial_header.point_header.net_pkt_size();
    if netpkt_size as usize > bytes.len() { return Err(Error::MissingBytes { netpkt_size })}

    let mut pkt : NetPktBox = unsafe { partial_header.alloc() };
    {
        let s: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut((&mut *pkt) as *mut NetPktFatPtr as *mut u8, netpkt_size as usize)
        };
        s.copy_from_slice(&bytes[..netpkt_size as usize]);
    };

    pkt.check(skip_hash_check)?;
    Ok(pkt)
}

pub fn parse_netarc(bytes:&[u8], skip_hash_check:bool) -> Result<NetPktArc,Error>{
    if bytes.len() < MIN_NETPKT_SIZE {
        return Err(Error::MissingHeader);
    }
    let partial_header = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const PartialNetHeader) };
    partial_header.point_header.check()?;
    let netpkt_size = partial_header.point_header.net_pkt_size();
    if netpkt_size as usize > bytes.len() { return Err(Error::MissingBytes { netpkt_size })}

    let pkt_inner = &bytes[..netpkt_size as usize][std::mem::size_of::<PartialNetHeader>()..];
    let pkt = unsafe{NetPktArc::from_header_and_copy(partial_header,skip_hash_check, |dest|dest.copy_from_slice(pkt_inner) )?};
    Ok(pkt)
}

//This is probably the best way to expose reading.
//However, we need access to &[Link] and .ipath to have a unaligned and aligned version
//pub fn parse_netparts(bytes:&[u8], validate:bool) -> Result<Result<NetPktParts,usize>,crate::Error>{


#[test]
fn parsing() {
    use crate::*;
    let parts = datapoint(b"hello", ());
    let boxed_parts = parts.as_netbox();
    let arc_box_parts = boxed_parts.as_netarc();
    let arc_parts = parts.as_netarc();
    let parts_arc_box = arc_box_parts.as_netparts();
    let parts_box_parts = boxed_parts.as_netparts();
    let box_arc_box_parts = arc_box_parts.as_netbox();


    let data : Vec<u8>= parts.byte_segments().collect();
    let box_parse = parse_netpkt(&data, true).unwrap().unwrap();
    let arc_parse = parse_netarc(&data, true).unwrap().unwrap();

    let list : Vec<&dyn NetPkt>= vec![&parts,&boxed_parts,&arc_box_parts,&arc_parts,&parts_arc_box,&parts_box_parts,&box_arc_box_parts,&box_parse,&arc_parse];

    for el in &list{
        let vec : Vec<u8> = el.byte_segments().collect();
        assert_eq!(vec,data);
    }
}
