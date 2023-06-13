// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{opts::{CommonOpts }, clap,tracing,  WriteDestSpec, clap::Parser, reader::PktReadOpts},
    prelude::*,
};

use crate::{watch::{ DGPDWatchCLIOpts, statements2query} };
#[derive(Parser)]
pub struct Filter {
    /// Don't filter datapoints
    #[clap(long)]
    allow_datapoint: bool,
    #[clap(flatten)]
    query: DGPDWatchCLIOpts,
    #[clap(long, short, default_value = "stdout")]
    write: Vec<WriteDestSpec>,
    /// destination for filtered packets
    #[clap(short = 'f', long, default_value = "null")]
    write_false: Vec<WriteDestSpec>,
    #[clap(flatten)]
    pkt_in: PktReadOpts,
    /// re-evaluate the query after every packet.
    #[clap(short,long)]
    live: bool,
}
pub fn select(
    common: CommonOpts,
    filter:Filter
) -> anyhow::Result<()> {
    let Filter { allow_datapoint, query, write, write_false, pkt_in, live } = filter;
    let mut write = common.open(&write)?;
    let mut write_false = common.open(&write_false)?;
    let stmnts :Vec<_> = query.iter_statments()?;
    let query= statements2query(&stmnts, &common.eval_ctx())?;
    tracing::trace!(?query, "Query");
    let mut e = WatchEntry::new(Default::default(), query, 0, (), debug_span!("Select"))?;
    tracing::trace!(?e, "Watching");
    let inp = common.inp_reader(&pkt_in)?;
    for pkt in inp {
        tracing::trace!(?pkt, "recv");
        let pkt = pkt?;
        if allow_datapoint && pkt.is_datapoint(){
            common.write_multi_dest(&mut write, &**pkt, None)?;
            continue;
        }
        let recv_pkt = RecvPktPtr {
            recv: now(),
            pkt: &pkt,
        };
        let (test_ok, cnt) = e.test(recv_pkt);
        tracing::trace!(test_ok, ?cnt, ?pkt, "Test pkt");
        if test_ok {
            common.write_multi_dest(&mut write, &**pkt, None)?;
            if live {
                let ctx = pkt_ctx(common.eval_ctx(), &**pkt);
                let query= statements2query(&stmnts, &ctx)?;
                e.query = Box::new(query);
                if let Err(e) =  e.update_tests(){
                    tracing::info!(?e,"new test is empty");
                    break;
                }
            }

        } else {
            common.write_multi_dest(&mut write_false, &**pkt, None)?;
        }
        if cnt.is_break() {
            break;
        }
    }
    Ok(())
}
