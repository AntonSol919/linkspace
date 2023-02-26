// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_core::{prelude::*, try_opt};
use std::io::{Error as IoError, ErrorKind, Read};

#[derive(Debug)]
pub struct NetPktDecoder<T> {
    pub allow_private: bool,
    pub reader: T,
    pub hop: bool,
    pub validate: bool,
}
impl<T> NetPktDecoder<T> {
    pub fn new(reader: T) -> Self {
        NetPktDecoder {
            allow_private: false,
            reader,
            hop: true,
            validate: true,
        }
    }
}

#[derive(Copy, Clone)]
pub struct UnalignedPkt<'o>(&'o NetPktPtr);
impl<'o> UnalignedPkt<'o> {
    /// Currently fine but if alignment is enforced this will be trouble.
    pub unsafe fn get(self) -> &'o NetPktPtr {
        self.0
    }
    pub fn as_netbox(self) -> NetPktBox {
        self.0.as_netbox()
    }
    pub fn as_netarc(self) -> NetPktArc {
        self.0.as_netarc()
    }
}

pub fn parse_netpkt(
    bytes: &[u8],
    validate: bool,
    allow_private: bool,
) -> Result<Option<UnalignedPkt>, Error> {
    if bytes.len() < MIN_NETPKT_SIZE {
        return Ok(None);
    }
    let header = unsafe { std::ptr::read_unaligned(bytes.as_ptr() as *const PartialNetHeader) };
    header.pkt_header.check()?;
    if bytes.len() < header.pkt_header.net_pkt_size() {
        return Ok(None);
    }
    let pkt = unsafe { &*(bytes.as_ptr() as *const NetPktPtr) };
    if validate {
        pkt.check::<true>()?
    } else {
        pkt.check::<false>()?
    };

    if !allow_private && pkt.group() == Some(&PRIVATE) {
        return Err(Error::PrivateGroup);
    }
    Ok(Some(UnalignedPkt(pkt)))
}

impl<T: Read> Iterator for NetPktDecoder<T> {
    type Item = Result<NetPktBox, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut partial_header = PartialNetHeader::EMPTY;
        {
            let mut buf: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut(
                    &mut partial_header as *mut PartialNetHeader as *mut u8,
                    std::mem::size_of::<PartialNetHeader>(),
                )
            };
            let mut zeros = 0;
            while !buf.is_empty() {
                let r = self.reader.read(buf);
                tracing::trace!(?r, "read");
                match r {
                    Ok(0) if zeros > 3 => return None,
                    Ok(0) => zeros += 1,
                    Ok(n) => {
                        let tmp = buf;
                        buf = &mut tmp[n..];
                    }
                    Err(ref e) if e.kind() == ErrorKind::UnexpectedEof => return None,
                    //Err(ref e) if e.kind() == ErrorKind::Interrupted && buf.len() == UPTO_PKT_SIZE => {return None}
                    Err(e) => return Some(Err(Error::IO(e))),
                }
            }
        }
        try_opt!(partial_header.pkt_header.check());
        let len = partial_header.pkt_header.net_pkt_size();
        let mut pkt = unsafe { partial_header.alloc() };
        {
            let s: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut((&mut *pkt) as *mut NetPktFatPtr as *mut u8, len)
            };
            tracing::trace!("Read rest");
            let r = self
                .reader
                .read_exact(&mut s[std::mem::size_of::<PartialNetHeader>()..]);
            try_opt!(r);
        };

        let check = if self.validate {
            pkt.check::<true>()
        } else {
            pkt.check::<false>()
        };
        try_opt!(check);
        if !self.allow_private && pkt.group() == Some(&PRIVATE) {
            return Some(Err(Error::PrivateGroup));
        }
        if self.hop {
            let head = &mut pkt._net_header;
            *head = head.hop();
        }
        Some(Ok(pkt))
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Pkt Error")]
    Pkt(#[from] linkspace_core::pkt::Error),
    #[error("Io Error")]
    IO(#[from] IoError),
    #[error("private null group not allowed in this context")]
    PrivateGroup,
}
