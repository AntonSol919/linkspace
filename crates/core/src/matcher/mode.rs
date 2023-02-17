// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.



use std::str::FromStr;

use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq,Debug)]
pub enum Mode{
    
    Local,
    Stream,
    Rx,
}

impl FromStr for Mode {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "l" | "local" => Ok(Mode::Local),
            "s" | "stream" => Ok(Mode::Stream),
            "r" | "rx" => Ok(Mode::Rx),
            _ => anyhow::bail!("could not parse (local|stream|rx)")
        }

    }
}
impl Default for Mode {
    fn default() -> Self {
        Mode::Stream
    }
}
impl Mode {
    pub fn read_local(&self) -> bool { matches!(self, Mode::Local | Mode::Stream)}
}
