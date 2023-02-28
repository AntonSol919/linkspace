// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_pkt::abe::eval::Scope;

use crate::prelude::*;

pub fn encode_watch(_ev: &Query) -> NetPktBox {
    todo!()
}

/// Read data & links as a query. Ctx(including 'now') is set to the parent pkt for each link.
pub fn read_pull_pkt(
    query: &mut Query,
    pkt: &NetPktPtr,
    reader: &ReadTxn,
    ctx: EvalCtx<&dyn Scope>,
) -> anyhow::Result<()> {
    if !pkt.is_datapoint(){
        let stamp = pkt.create_stamp().copied();
        let subc= ctx.pre_scope(EScope(StampEF{fixed_now:stamp}));
        let subc= pkt_ctx(subc.reref(), &pkt);
        tracing::debug!(ctx=pkt_fmt(pkt), "set pkt ctx");
        
        for l in pkt.get_links() {
            let inner = reader
                .read(&l.ptr)?
                .context("Missing pointer")?;
            read_pull_pkt(query, &inner, reader, subc.dynr())?;
        }
    }
    tracing::debug!(data=pkt.get_data_str()?, "constructing query");
    query.parse(pkt.data(), &ctx)?;
    Ok(())
}
