// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::thread::JoinHandle;

use anyhow::Context;
use linkspace_common::{
    cli::{clap, clap::Args, opts::CommonOpts, reader::PktReadOpts, tracing},
    prelude::*,
    runtime::{handlers::NotifyClose, threads::run_until_spawn_thread},
};

use crate::watch::PrintABE;

/**
Read multiple queries from pkts on stdin.
**/
#[derive(Args, Clone)]
#[group(skip)]
pub struct MultiWatch {
    #[command(flatten)]
    inp: PktReadOpts,
    #[command(flatten)]
    print: PrintABE,
    /// by default evaluation in scope is limited to static functions. enable 'live' queries.
    #[arg(short, long)]
    full_scope: bool,

    /// Continue after closing stdin
    #[arg(short, long)]
    linger: bool,

    #[command(flatten)]
    constraint: OrConstrait,
}

pub fn multi_watch(common: CommonOpts, multi_watch: MultiWatch) -> anyhow::Result<()> {
    let linger = multi_watch.linger;
    let rx = common.runtime()?;

    let ctx = Arc::new((common, multi_watch));
    let handle: JoinHandle<anyhow::Result<()>> =
        run_until_spawn_thread(rx.clone(), move |spawner| -> anyhow::Result<()> {
            let inp = ctx.0.inp_reader(&ctx.1.inp).context("reader open failed")?;
            for pkt in inp {
                tracing::debug!(?pkt, "inp packet");
                let pkt = pkt?;
                let c = ctx.clone();
                spawner.unbounded_send(Box::new(move |rx| {
                    if let Err(e) = setup_watch(&pkt, &rx, &c) {
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
    (common, mv): &(CommonOpts, MultiWatch),
) -> anyhow::Result<()> {
    let mut query = Query::default();
    let cli_scope = common.eval_scope();
    query.parse(
        pkt.data(),
        if mv.full_scope {
            &cli_scope
        } else {
            &CORE_SCOPE
        },
    )?;

    let mut ok = mv.constraint.or.is_empty();
    for opt in mv.constraint.or.iter() {
        if query.parse(opt.as_bytes(), &cli_scope).is_ok() {
            ok = true;
            break;
        }
    }
    anyhow::ensure!(ok, "cant find valid set");

    if mv.print.do_print() {
        mv.print.print_query(&query, &mut std::io::stdout())?;
        return Ok(());
    }
    let span = debug_span!("multi-watch", origin=%pkt.hash());
    let cb = common.stdout_writer();
    let cb = NotifyClose::new(cb, &query, pkt);
    rx.watch_query(&query, cb, span)?;
    Ok(())
}

#[derive(Args, Clone)]
#[group(skip)]
pub struct OrConstrait {
    /** Add one or more query constraints. e.g. --or 'group:=:[#:pub]\ndomain:=:example' --or 'domain:=:hello'

    Queries will have the additional predicate/options added and will be ignored if they result in the empty set.
    NOTE: This means a query without any group or domain would imply the first option

    **/
    #[arg(long)]
    pub or: Vec<String>,
}
