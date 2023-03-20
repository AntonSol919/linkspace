// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_core::{
    prelude::{query_mode::Order, *}
};
use crate::{runtime::Linkspace, protocols::lns::{ admin::save_private_claim }};

use super::{*, name::Name, claim::Claim};

pub fn get_private_claim<'o>(
    reader: &'o ReadTxn,
    name: &Name,
    admin: Option<PubKey>
) -> ApplyResult<RecvPktPtr<'o>> {
    ApplyResult::Value(get_private_claims(reader, name,admin)?.next()?)
}
pub fn get_private_claims<'o>(
    reader: &'o ReadTxn,
    name: &Name,
    admin: Option<PubKey>
) -> anyhow::Result<impl Iterator<Item = RecvPktPtr<'o>>> {
    let path = name.claim_ipath();
    let mut preds = PktPredicates::from_gdp(PRIVATE, LNS, &path).create_before(now())?;
    if let Some(v) = admin {
        preds.pubkey.add(TestOp::Equal, v.into())
    }
    Ok(reader.query_tree(Order::Desc, &preds))
}








pub (crate) fn setup_local_keyclaim(
    lk: &Linkspace,
    claim: Claim,
    admin: Option<&SigningKey>,
) -> anyhow::Result<()> {
    // We fake a claim chain and just insert it into the admin tree.
    save_private_claim(lk, &claim, admin, &[],true)?;
    Ok(())
}
