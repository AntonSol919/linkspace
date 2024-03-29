// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::ensure;
use linkspace_common::{
    cli::{
        clap, clap::Parser, opts::CommonOpts, reader::PktReadOpts, tracing, Out, WriteDest,
        WriteDestSpec,
    },
    prelude::*,
};

#[derive(Parser, Clone)]
pub struct SaveForward {
    /// Only new packets are saved to dest
    #[arg(long, default_value = "db")]
    new: Vec<WriteDestSpec>,
    #[arg(long, default_value = "null")]
    old: Vec<WriteDestSpec>,
    /// add stdout to both --old and --dest
    #[arg(short, long)]
    forward_stdout: bool,
    #[command(flatten)]
    pkt_in: PktReadOpts,
}

pub fn save(opts: SaveForward, mut common: CommonOpts) -> anyhow::Result<()> {
    common.default_hop();
    let SaveForward {
        mut new,
        mut old,
        forward_stdout,
        pkt_in,
    } = opts;
    if forward_stdout {
        new.push(WriteDest::stdout());
        old.push(WriteDest::stdout());
    }
    let lk = common.runtime()?;
    let env = lk.env();
    let inp = common.inp_reader(&pkt_in)?;
    let mut new = common.open(&new)?;
    let mut old = common.open(&old)?;
    ensure!(
        new.iter().any(|v| matches!(v.out, Out::Db)),
        "currently not possible to skip saving new packets add a --new-only db"
    );
    new.retain(|v| !matches!(v.out, Out::Db));
    old.retain(|v| !matches!(v.out, Out::Db));
    tracing::trace!("Start await");
    for pkt in inp {
        let pkt = pkt?;
        // TODO: It might be better to spin a thread that will batch writes in a single transaction.
        // Depends on the speed of writing vs checking
        let is_new = env.save_ptr_one(&pkt)?.is_written();
        let dest = if is_new { &mut new } else { &mut old };
        common.write_multi_dest(dest, &pkt, None)?;
        tracing::debug!(hash=?pkt.hash(),is_new,"Flush OK");
    }
    Ok(())
}
