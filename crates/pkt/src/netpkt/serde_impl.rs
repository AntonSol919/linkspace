// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.use serde::Serialize;

use serde::{Deserialize, Deserializer, Serialize};

// TODO: should be feature gated

/**
This is generally considered bad practice.
A packet length is encoded twice.
Serialize/Deserialize packets as raw bytes with lk_read and lk_write.
*/
use crate::{NetPkt, NetPktArc, NetPktFatPtr, NetPktPtr};

impl Serialize for NetPktPtr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_bytes::serialize(self.as_netpkt_bytes(), serializer)
    }
}
impl Serialize for NetPktFatPtr {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_bytes::serialize(self.as_netpkt_bytes(), serializer)
    }
}
impl Serialize for NetPktArc {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serde_bytes::serialize(self.as_netpkt_bytes(), serializer)
    }
}

impl<'de> Deserialize<'de> for crate::NetPktBox {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        let pkt = crate::read::read_pkt(bytes, false).map_err(serde::de::Error::custom)?;
        Ok(pkt.into_owned())
    }
}
impl<'de> Deserialize<'de> for crate::NetPktArc {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = <&[u8]>::deserialize(deserializer)?;
        let pkt = crate::read::read_pkt(bytes, false).map_err(serde::de::Error::custom)?;
        Ok(pkt.as_ref().as_netarc())
    }
}
