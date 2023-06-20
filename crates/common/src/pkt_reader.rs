// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_core::{prelude::*, try_opt};
use std::io::{Error as IoError, ErrorKind, Read};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Pkt Error")]
    Pkt(#[from] linkspace_core::pkt::Error),
    #[error("Io Error")]
    IO(#[from] IoError),
}

#[derive(Debug)]
pub struct NetPktDecoder<T> {
    pub allow_private: bool,
    pub reader: T,
    pub hop: bool,
    pub skip_hash: bool,
}
impl<T> NetPktDecoder<T> {
    pub fn new(reader: T) -> Self {
        NetPktDecoder {
            allow_private: false,
            reader,
            hop: true,
            skip_hash: false,
        }
    }
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
        try_opt!(partial_header.point_header.check());
        let len = partial_header.point_header.size();
        let mut pkt = unsafe { partial_header.alloc() };
        {
            let s: &mut [u8] = unsafe {
                std::slice::from_raw_parts_mut((&mut *pkt) as *mut NetPktFatPtr as *mut u8, len as usize)
            };
            tracing::trace!("Read rest");
            let r = self
                .reader
                .read_exact(&mut s[std::mem::size_of::<PartialNetHeader>()..]);
            try_opt!(r);
        };
        try_opt!(pkt.check(self.skip_hash));
        if !self.allow_private{
            if let Err(e) = pkt.check_private(){
                return Some(Err(Error::Pkt(e)));
            }
        }
        if self.hop {
            let head = &mut pkt._net_header;
            *head = head.hop();
        }
        Some(Ok(pkt))
    }
}

