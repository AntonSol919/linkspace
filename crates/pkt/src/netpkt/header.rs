// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use byte_fmt::{
    abe::ToABE,
    endian_types::{U32, U8},
    AB, B64,
};
use core::fmt;
use serde::{Deserialize, Serialize};
use std::ptr;

use crate::Stamp;
use std::cell::Cell;

/// A thread local default net header value when creating new netpackets
#[thread_local]
pub static DEFAULT_ROUTING_BITS: Cell<NetPktHeader> = Cell::new(NetPktHeader::EMPTY);
//use zerocopy::{AsBytes, FromBytes};

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(packed)]
/// Variable fields in a [crate::NetPkt] used in point exchange
pub struct NetPktHeader {
    pub prefix: AB<[u8; 3]>,
    pub flags: NetFlags,
    pub hop: U32,
    pub stamp: Stamp,
    pub ubits: [U32; 4],
}

static_assertions::assert_eq_size!(NetPktHeader, [u8; 32]);
impl From<NetPktHeader> for B64<[u8; 32]> {
    fn from(val: NetPktHeader) -> Self {
        B64(val.cinto())
    }
}
impl From<B64<[u8; 32]>> for NetPktHeader {
    fn from(v: B64<[u8; 32]>) -> Self {
        NetPktHeader::cfrom(v.0)
    }
}

impl ToABE for NetPktHeader {
    fn to_abe(&self) -> Vec<byte_fmt::abe::ABE> {
        let NetPktHeader {
            prefix,
            flags,
            hop,
            stamp,
            ubits,
        } = self;
        byte_fmt::abe::abev!( +(prefix.to_abe())
                : +(U8::new(flags.bits()).abe_bits())
                : +(hop.to_abe())
                : +(stamp.to_abe())
                : +(ubits[0].to_abe())
                : +(ubits[1].to_abe())
                : +(ubits[2].to_abe())
                : +(ubits[3].to_abe())
        )
    }
}

impl fmt::Display for NetPktHeader {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "{}", self.to_abe_str())
    }
}

impl Default for NetPktHeader {
    /// DEFAULT_ROUTING_BITS
    fn default() -> Self {
        DEFAULT_ROUTING_BITS.get()
    }
}
impl NetPktHeader {
    pub const EMPTY: Self = NetPktHeader {
        prefix: AB(*b"LK1"),
        flags: NetFlags::empty(),
        stamp: Stamp::MAX,
        hop: U32::ZERO,
        ubits: [U32::ZERO; 4],
    };

    #[inline(always)]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        unsafe { &*ptr::from_ref(self).cast::<[u8; 32]>() }
    }
    #[inline(always)]
    pub const fn cfrom(b: [u8; 32]) -> Self {
        unsafe { b.as_ptr().cast::<Self>().read_unaligned() }
    }
    #[inline(always)]
    pub const fn cinto(self) -> [u8; 32] {
        unsafe { *ptr::from_ref(&self).cast::<[u8; 32]>() }
    }

    #[must_use]
    pub fn hop(mut self) -> Self {
        self.hop = self.hop.incr();
        self.flags.remove(NetFlags::ALWAYS_ZERO);
        self
    }

    #[must_use]
    pub fn with_flags(mut self, flags: NetFlags) -> Self {
        self.flags = flags;
        self
    }
    #[must_use]
    pub fn and_flags(mut self, remove: NetFlags, insert: NetFlags) -> Self {
        self.flags.remove(remove);
        self.flags.insert(insert);
        self
    }
    pub fn flags_u8(&self) -> &u8 {
        unsafe { &*(ptr::from_ref(&self.flags).cast::<u8>()) }
    }
    pub fn mut_flags_u8(&mut self) -> &mut u8 {
        unsafe { &mut *(ptr::from_mut(&mut self.flags).cast::<u8>()) }
    }
}

use bitflags::bitflags;

bitflags! {
    /// Variable flags used in transit
    #[derive(Serialize,Deserialize,Copy,Clone,Debug,Eq,PartialEq,Ord,PartialOrd)]
    pub struct NetFlags: u8 {
        /// Indicate that the chances of anybody interested in this packet are zero.
        /// Implementations can ignore this, mostly useful for importing many datablocks.
        const SILENT = 0b0000_0001;
        const LINKED_IN_FUTURE_PKT = 0b0000_0010;
        const LINKED_IN_PREVIOUS_PKT = 0b0000_0100;
        /// Request that this packet is not forwarded
        const DONT_FORWARD = 0b0000_1000;
        const ALWAYS_ZERO = 0b1000_0000;
    }
}
impl From<NetFlags> for byte_fmt::endian_types::U8 {
    fn from(val: NetFlags) -> Self {
        byte_fmt::endian_types::U8::new(val.bits())
    }
}
/// [NetPktHeader] builder.
#[derive(Copy, Clone, Debug)]
pub enum NetOpts {
    Default,
    Flags(NetFlags),
    Stamp(Stamp),
    Advanced(NetPktHeader),
}
impl From<()> for NetOpts {
    fn from(_: ()) -> Self {
        NetOpts::Default
    }
}

impl From<NetFlags> for NetOpts {
    fn from(f: NetFlags) -> Self {
        NetOpts::Flags(f)
    }
}

impl From<NetPktHeader> for NetOpts {
    fn from(h: NetPktHeader) -> Self {
        NetOpts::Advanced(h)
    }
}

impl From<NetOpts> for NetPktHeader {
    fn from(val: NetOpts) -> Self {
        match val {
            NetOpts::Default => NetPktHeader::default(),
            NetOpts::Flags(f) => NetPktHeader::default().with_flags(f),
            NetOpts::Advanced(h) => h,
            NetOpts::Stamp(ttl) => NetPktHeader {
                stamp: ttl,
                ..NetPktHeader::EMPTY
            },
        }
    }
}
