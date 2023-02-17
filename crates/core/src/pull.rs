// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_pkt::abe::eval::Scope;

use crate::prelude::*;

pub fn encode_view(_ev: &Query) -> NetPktBox {
    todo!()
}

pub fn read_pull_pkt(
    query: &mut Query,
    pkt: &NetPktPtr,
    reader: &ReadTxn,
    ctx: &EvalCtx<impl Scope>,
) -> anyhow::Result<()> {
    for l in pkt.get_links() {
        let inner = reader
            .read(&l.ptr)?
            .context("Missing pointer")?;
        read_pull_pkt(query, &inner, reader, ctx)?;
    }
    let pctx = pkt_ctx(ctx.reref(), &pkt);
    query.parse(pkt.data(), &pctx)?;
    Ok(())
}
