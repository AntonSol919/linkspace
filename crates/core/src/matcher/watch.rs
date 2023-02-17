// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use serde::{Deserialize, Serialize};
use linkspace_pkt::{PktHash };
use crate::{ env::tree_query::{TreeQuery}, prelude::logquery::LogQuery};


#[derive(Serialize,Deserialize, Debug, Clone)]
pub enum WatchKind<H=(),T=(),R=()>{
    Hash(H),
    Tree(T),
    Log(R),
}
pub type WatchQuery=WatchKind<PktHash,TreeQuery,LogQuery>;
pub type WatchQueryCtx<HCtx=(),TCtx=(),RCtx=()>=WatchKind<(PktHash,HCtx),(TreeQuery,TCtx),(LogQuery,RCtx)>;
