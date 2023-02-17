// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::thread::JoinHandle;

use anyhow::Context;
use linkspace_common::{
    cli::{clap, clap::Args, opts::CommonOpts, tracing},
    core::pull::read_pull_pkt,
    prelude::*,
    runtime::{handlers::EchoOpt, threads::run_untill_spawn_thread},
};

use crate::print_query;

/**
Read multiple queries from pkts on stdin.
**/
#[derive(Debug, Args, Clone)]
#[group(skip)]
pub struct MultiView {
    #[clap(long,short,action = clap::ArgAction::Count)]
    print: u8,
    /// by default evaluation in ctx is limited to static functions. enable 'live' queries.
    #[clap(short, long)]
    full_ctx: bool,

    /// Continue after closing stdin
    #[clap(short, long)]
    linger: bool,
    //adato:PathBuf
}

pub fn multi_view(common: CommonOpts, multi_view: MultiView) -> anyhow::Result<()> {
    let linger = multi_view.linger;
    let rx = common.runtime()?.clone();
    let ctx = Arc::new((common, multi_view));
    let handle: JoinHandle<anyhow::Result<()>> =
        run_untill_spawn_thread(rx.clone(), move |spawner| -> anyhow::Result<()> {
            let inp = ctx.0.inp_reader().context("reader open failed")?;
            for pkt in inp {
                tracing::debug!(?pkt, "inp packet");
                let pkt = pkt?;
                let c = ctx.clone();
                spawner.unbounded_send(Box::new(move |rx| {
                    if let Err(e) = setup_watch(&*pkt, &rx, &c) {
                        eprintln!("{:#?}", e);
                    }
                }))?;
            }
            Ok(())
        })?;
    handle
        .join()
        .map_err(|_| anyhow::anyhow!("Thread failed?"))??;
    if linger {
        tracing::info!("stdin closed, run while work");
        let _ = rx.run_while(None, None);
    }
    Ok(())
}

pub fn setup_watch(
    pkt: &NetPktPtr,
    rx: &Linkspace,
    (common, mv): &(CommonOpts, MultiView),
) -> anyhow::Result<()> {
    let mut query = Query::default();
    if mv.full_ctx {
        let _ = read_pull_pkt(&mut query, pkt, &*rx.get_reader(), &common.eval_ctx())?;
    } else {
        let _ = read_pull_pkt(&mut query, pkt, &*rx.get_reader(), &core_ctx())?;
    }
    if mv.print > 0 {
        print_query(mv.print, &query);
        return Ok(());
    }
    let span = debug_span!("watch", origin = pkt_fmt(pkt));
    let cb = common.stdout_writer();

    match EchoOpt::new(cb, &query, pkt) {
        Ok(c) => rx.view_query(&query, c, span)?,
        Err(c) => rx.view_query(&query, c, span)?,
    };
    Ok(())
}
