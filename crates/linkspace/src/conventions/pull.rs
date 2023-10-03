
/** pull requests create a linkpoint in \[f:exchange\]:\[#:0\]:/pull/\[query.group\]/\[query.domain\]/\[query.id\]

Pull queries must have the predicates 'domain:=:..' and 'group:=:..'.
It is up to an exchange process to fulfill the query.
The domain should be conservative with its query.
Requesting too much can add significant overhead.

You can use [lk_status_watch] to determine if a exchange is active
 **/
use anyhow::Context;
use linkspace_common::prelude::EXCHANGE_DOMAIN;

use crate::*;

#[cfg(feature="runtime")]
/// Save a query in linkspace using the point format compatible with the pull convention
pub fn lk_pull(lk: &Linkspace, query: &Query) -> LkResult<LkHash> {
    let req = lk_pull_point(query)?;
    lk_save(lk, &req)?;
    Ok(req.hash())
}
/// Prefer using [lk_pull] - creates a pullpoint from a query
pub fn lk_pull_point(query: &Query ) -> LkResult<NetPktBox> {
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
    let id = query.0.qid()?.flatten().context("missing :qid:... option")?;
    let data = query.0.to_string();
    tracing::trace!(data);
    let pull_space = rspace_buf(&[b"pull", &*group, &*domain, id]);
    let pkt = lk_linkpoint(
        data.as_bytes(),
        EXCHANGE_DOMAIN,
        PRIVATE,
        &pull_space,
        &[],
        None,
    )?;
    Ok(pkt.as_netbox())
}
