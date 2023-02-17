// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_core::prelude::{GroupID, PubKey};

pub mod handshake;
pub mod impex;
pub mod lns;
pub mod miniquery;

pub fn unicast_group(p1: PubKey, p2: PubKey) -> GroupID {
    p1 ^ p2
}
