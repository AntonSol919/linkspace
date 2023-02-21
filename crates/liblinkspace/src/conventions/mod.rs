// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod status;
pub use status::{LkStatus,lk_status_poll,lk_status_set};

use anyhow::Context;
use linkspace_common::prelude::EXCHANGE_DOMAIN;

use super::*;
/** pull requests create a linkpoint in {f:exchange}:{#:0}:/pull/{query.group}/{query.domain}/{query.id}

It is up to an exchange process to fullfill the query.
You can use [lk_status_poll] to determine if a exchange is active
**/
pub fn lk_pull(lk: &Linkspace, query: &Query, ttl: Stamp) -> LkResult<LkHash> {
    let req = lk_pull_req(query, ttl)?;
    lk_save(lk, &req)?;
    Ok(req.hash())
}
#[doc(hidden)]
pub fn lk_pull_req(query: &Query, duration: Stamp) -> LkResult<NetPktBox> {
    let group: GroupID = query
        .0
        .predicates
        .group
        .as_eq()
        .context("requires exact group predicate")?
        .into();
    let domain: Domain = query
        .0
        .predicates
        .domain
        .as_eq()
        .context("requires exact domain predicate")?
        .into();
    let id = query.0.watch_id().transpose()?.context("missing :watch option")?;
    let data = query.0.to_string();
    tracing::trace!(data);
    let pull_path = ipath_buf(&[b"pull", &*group, &*domain, &id]);
    let mut pkt = lk_linkpoint(
        EXCHANGE_DOMAIN,
        LOCAL_ONLY_GROUP,
        &pull_path,
        &[],
        data.as_bytes(),
        None,
    )?;
    pkt.net_header_mut().unwrap().until = now().saturating_add(duration);
    Ok(pkt.as_netbox())
}
