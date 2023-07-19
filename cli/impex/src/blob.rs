// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace::{
    anyhow,
    cli::{clap, clap::Parser, opts::CommonOpts},
    prelude::*,
};
use std::io::{stdin, Write};

#[derive(Parser, Debug)]
pub enum Cmd {
    Encode(Encode),
    Checkin(Encode),
    Checkout { head: HashExpr },
}

#[derive(Parser, Clone, Debug)]
pub struct Encode {
    #[arg(short, long, default_value_t)]
    group: HashExpr,
    #[arg(short, long, default_value_t)]
    domain: Domain,
}
pub fn cmd(common: CommonOpts, cmd: Cmd) -> anyhow::Result<()> {
    match cmd {
        Cmd::Encode(o) => encode(common, o),
        Cmd::Checkout { head } => checkout(common, head),
        Cmd::Checkin(_) => todo!(),
    }
}

pub(crate) fn encode(common: CommonOpts, opts: Encode) -> anyhow::Result<()> {
    let group = opts.group.eval(&common.eval_ctx())?;
    let inp = stdin();
    let mut out = std::io::stdout().lock();
    linkspace::protocols::impex::blob::into_blob::<_, _, std::io::Result<LkHash>>(
        group,
        opts.domain,
        inp,
        |pkt| {
            let mut bytes = pkt.as_bytes_segments();
            bytes.write(&mut out)
        },
    )?;
    out.flush()?;
    Ok(())
}

pub(crate) fn checkout(common: CommonOpts, hexpr: HashExpr) -> anyhow::Result<()> {
    let env = common.env()?;
    let head = hexpr.eval(&common.eval_ctx())?;
    let reader = env.get_reader()?;
    linkspace::protocols::impex::blob::checkout(&reader, std::io::stdout().lock(), head)?;
    Ok(())
}
