// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{protocols::lns::admin::save_private_claim, runtime::Linkspace};
use linkspace_core::prelude::{query_mode::Order, RecvPktPtr, *};

use super::{claim::Claim, name::Name, *};

pub fn get_private_claim<'o>(
    reader: &'o ReadTxn,
    name: &Name,
    admin: Option<PubKey>,
) -> ApplyResult<RecvPktPtr<'o>> {
    ApplyResult::Value(get_private_claims(reader, name, true, admin)?.next()?)
}

pub fn get_private_claims<'o>(
    reader: &'o ReadTxn,
    name: &Name,
    exact: bool,
    admin: Option<PubKey>,
) -> anyhow::Result<impl Iterator<Item = RecvPktPtr<'o>>> {
    let path = name.claim_space();
    let mut preds = PktPredicates::from_gdp(PRIVATE, LNS, &path, exact).create_before(now())?;
    preds.state.i_branch = TestSet::new_eq(0);
    if let Some(v) = admin {
        preds.pubkey.add(TestOp::Equal, v.into())
    }
    Ok(reader.query_tree(Order::Desc, &preds))
}

pub(crate) fn setup_local_keyclaim(
    lk: &Linkspace,
    claim: Claim,
    admin: Option<&SigningKey>,
) -> anyhow::Result<()> {
    // We fake a claim chain and insert it into the admin tree.
    save_private_claim(lk, &claim, admin, &[], true)?;
    Ok(())
}
