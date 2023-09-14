// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use serde::{Serialize,Deserialize};

use crate::B64;

impl<N: AsRef<[u8]> + Serialize> Serialize for B64<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            base64(self.0.as_ref()).serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}
impl<'de, N:AsRef<[u8]>> Deserialize<'de> for B64<N> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable(){
            let v = <&str>::deserialize(deserializer)?;
            Ok(v.parse().map_err(serde::de::Error::custom)?)
        }else {
            Ok(B64( <[u8;N]>::deserialize(deserializer)?))
        }
    }
}
