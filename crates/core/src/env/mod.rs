// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// the db is duck type compatible between inmem and lmdb

use linkspace_pkt::{NetPkt, reroute::RecvPkt, NetPktPtr};

pub mod tree_key;
pub mod write_result;

pub mod lmdb;
pub mod query_mode;

pub type RecvPktPtr<'o> = RecvPkt<&'o NetPktPtr>;


pub trait Env {
    type RecvPkt<'o> : NetPkt;
}


impl Env for lmdb::BTreeEnv{
    type RecvPkt<'o> = RecvPktPtr<'o>;
}
