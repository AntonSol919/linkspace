
use crate::{ NetPktBox, MIN_NETPKT_SIZE, PartialNetHeader, NetPktFatPtr};



// Ok(Err(e)) means the packet requires at least e bytes.
pub fn parse_netpkt(
    bytes: &[u8],
    validate: bool,
) -> Result<Result<NetPktBox,usize>, crate::Error> {
    if bytes.len() < MIN_NETPKT_SIZE {
        return Ok(Err(MIN_NETPKT_SIZE));
    }
    let partial_header = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const PartialNetHeader) };
    partial_header.point_header.check()?;
    let pktsize = partial_header.point_header.net_pkt_size();
    if pktsize > bytes.len() { return Ok(Err(pktsize))}

    let mut pkt : NetPktBox = unsafe { partial_header.alloc() };
    {
        let s: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut((&mut *pkt) as *mut NetPktFatPtr as *mut u8, pktsize)
        };
        s.copy_from_slice(&bytes[..pktsize]);
    };
    
    if validate {
        pkt.check::<true>()?
    } else {
        pkt.check::<false>()?
    };
    Ok(Ok(pkt))
}

