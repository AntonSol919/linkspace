// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::prelude::*;

use super::{claim::Claim, name::{Name }, LNS};

pub fn list_all_potential_claims_with_prefix<'o>(reader:&'o ReadTxn,name: &Name) -> impl Iterator<Item=anyhow::Result<Claim>> +'o{
    let path = name.claim_ipath();
    let now = now();
    let mut preds = PktPredicates::from_gd(name.claim_group().expect("'file' names don't do claims"), LNS).create_before(now).unwrap();
    let _ = preds.prefix(&**path);
    //preds.state.i_branch.add(TestOp::Equal, 0);
    reader.query_tree(query_mode::Order::Desc, &preds).flat_map(move |pkt| -> Option<anyhow::Result<Claim>> {
        match Claim::from(pkt){
            Ok(c) => if c.until() > now {Some(Ok(c))} else {None}
            Err(e) => Some(Err(e)),
        }
    })
}


pub type TaggedClaim = ((Stamp,[u8;8]),anyhow::Result<Option<Claim>>);
pub fn list_all_reverse_lookups(_reader: &ReadTxn, _tag: &[u8],_ptr:Option<LkHash>) -> Vec<Vec<TaggedClaim>> {
    todo!()
}


