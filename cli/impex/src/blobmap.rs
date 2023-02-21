// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use liblinkspace::{
    anyhow,
    cli::{clap, clap::Parser, opts::CommonOpts, WriteDest},
    prelude::*,
    test_exprs::*,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub enum Cmd {
    Encode(Encode),
    Checkout(Checkout),
}
#[derive(Parser, Clone, Debug)]
pub struct Checkout {
    #[clap(short, long)]
    pub r#async: bool,
    #[clap(flatten)]
    q: ExtWatchCLIOpts,
    #[clap(long)]
    dest: PathBuf,
}

#[derive(Parser, Clone, Debug)]
pub struct Encode {
    #[clap(short, long)]
    pub r#async: bool,
    #[clap(long, default_value = "stdout")]
    dest: Vec<WriteDest>,
    source: PathBuf,
    dgs: DGPExpr,
}
pub fn cmd(common: CommonOpts, cmd: Cmd) -> anyhow::Result<()> {
    match cmd {
        Cmd::Encode(o) => encode(common, o),
        Cmd::Checkout(c) => checkout(common, c),
    }
}

fn checkout(common: CommonOpts, opts: Checkout) -> anyhow::Result<()> {
    let env = common.env()?;
    let reader = env.get_reader()?;
    let tq = opts.q.try_into_tq(&common.eval_ctx())?;
    if opts.r#async {
        todo!()
    } else {
        liblinkspace::protocols::impex::blobmap::checkout::checkout_now(
            reader, tq, &opts.dest, 0, false,
        )?;
    }
    Ok(())
}

pub(crate) fn encode(common: CommonOpts, opts: Encode) -> anyhow::Result<()> {
    let dgs = opts.dgs.eval(&common.eval_ctx())?;
    let for_each = move |pkt: NetPktParts| common.write_multi_dest(&opts.dest, &pkt, None);
    if opts.r#async {
        liblinkspace::protocols::impex::blobmap::checkin::encode_forever(
            for_each,
            opts.source,
            dgs,
        )?;
    } else {
        liblinkspace::protocols::impex::blobmap::checkin::encode_now(for_each, &opts.source, &dgs)?;
    }
    Ok(())
}
