// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    io::Write,
    sync::{
        mpsc::{channel, RecvTimeoutError, Sender},
        Mutex,
    },
};

use crate::point::PointOpts;
use linkspace_common::{
    cli::{clap, clap::Parser, opts::{CommonOpts, PktIn}, tracing, Reader, WriteDest, WriteDestSpec},
    core::stamp_fmt::DurationStr,
    prelude::*,
};

#[derive(Parser)]
/**
WARNING: ensure the final packet does not exceed packet size with max-links depending on other fields.

Create a linkpoint or keypoint with links to the packets on stdin.
A result is produced after a set interval or a max number of links have been made.

By default the resulting link packet is appended to the stream of packets.
This can be changed by setting --write and --forward.
Whenever a new link result is produced the buffer is flushed to stdout.

This can create different effects.
[Default] Forward everything, and print the collection result.
--forward stdout --write stdout

Print the result and only than append the packets in its link
--forward buffer --write stdout

dont print anything to stdout, save the result in the database
--forward null --write db

does nothing
--forward stdout --write null
**/
pub struct Collect {
    #[clap(flatten)]
    pkt_in: PktIn,
    #[clap(flatten)]
    build: PointOpts,

    /// destination for incoming packets
    #[clap(short, long, default_value = "stdout")]
    forward: Vec<WriteDestSpec>,
    /// destination for newly created collection packet
    #[clap(short, long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,

    #[clap(alias = "ctag", long, value_enum, default_value = "[now]")]
    collect_tag: TagExpr,
    /// Create packet after collecting max_links from incoming packets
    #[clap(long,default_value_t=MAX_LINKS_LEN-16)]
    max_links: usize,
    /// Create a packet after
    #[clap(long)]
    min_interval: Option<DurationStr>,
    #[clap(long)]
    allow_empty: bool,
    /// Link used for the first collect
    #[clap(alias = "il", long)]
    pub init_link: Vec<LinkExpr>,
    /// Add a link pointing to a previous created packet with the tag
    #[clap(long)]
    chain_tag: Option<Tag>,
}

pub struct Collector {
    links: Vec<Link>,
    current_links: usize,
    // stage area for writing complete packets.
    buf: Vec<u8>,
    forward: Vec<WriteDest>,
    write: Vec<WriteDest>,
    reader: Reader,
    c_opts: Collect,
    dgs: DGP,
}
impl Collector {
    pub fn collect(&mut self, common: &CommonOpts) -> anyhow::Result<()> {
        if !self.c_opts.allow_empty && self.links.is_empty() {
            tracing::debug!("Skip empty");
            return Ok(());
        };
        let links = std::mem::take(&mut self.links);
        let mut data = vec![];
        let pkt = crate::point::build_with_reader(
            common,
            &self.c_opts.build,
            &self.dgs,
            &links,
            &mut data,
            &mut self.reader,
        )?;
        tracing::debug!(new_pkt=?pkt,"New collect Pkt");
        common.write_multi_dest(&mut self.write, &pkt, Some(&mut self.buf))?;
        let ctx = pkt_ctx(common.eval_ctx(), &pkt);
        let hash = pkt.hash();
        self.links.extend(
            self.c_opts
                .chain_tag
                .clone()
                .map(|tag| Link { tag, ptr: hash }),
        );
        for l in self.c_opts.build.link.iter() {
            self.links.push(l.eval(&ctx)?);
        }
        std::mem::drop(pkt);

        self.current_links = 0;
        if !self.buf.is_empty() {
            tracing::debug!("Writing Buffer");
            let mut out = std::io::stdout();
            out.write_all(&self.buf)?;
            out.flush()?;
            self.buf.clear();
        }
        anyhow::Ok(())
    }
    pub fn new_pkt(
        &mut self,
        pkt: NetPktBox,
        common: &CommonOpts,
        tik: Option<&Sender<()>>,
    ) -> anyhow::Result<()> {
        let ctx = common.eval_ctx();
        let tag = self.c_opts.collect_tag.eval(&pkt_ctx(ctx, &**pkt))?;

        self.links.push(Link {
            ptr: pkt.hash(),
            tag,
        });
        self.current_links += 1;

        tracing::trace!(hash=%pkt.hash(),outp=self.forward.len(), "writing new pkt ");
        common.write_multi_dest(&mut self.forward, &**pkt, Some(&mut self.buf))?;

        if self.current_links >= self.c_opts.max_links {
            if let Some(tx) = tik {
                let _ = tx.send(());
            }
            self.collect(common)?
        }
        Ok(())
    }
}

pub fn collect(common: &CommonOpts, c_opts: Collect) -> anyhow::Result<()> {
    let eval_ctx = common.eval_ctx();

    let initial_links: Vec<_> = c_opts
        .init_link
        .iter()
        .chain(c_opts.build.link.iter())
        .map(|v| v.eval(&eval_ctx))
        .try_collect()?;
    tracing::debug!(?initial_links, "Initial");
    let inp = common.inp_reader(&c_opts.pkt_in)?;
    if c_opts.build.sign {
        let _ = c_opts.build.key.identity(common, false)?;
    }
    let dgs = c_opts.build.dgs.eval(&eval_ctx)?;
    tracing::debug!(?dgs);
    let mut collector = Collector {
        links: initial_links.clone(),
        forward: common.open(&c_opts.forward)?,
        write: common.open(&c_opts.write)?,
        reader: common.open_read(c_opts.build.data.as_ref())?,
        dgs,
        buf: vec![],
        c_opts,
        current_links: 0,
    };
    let mut collector = match collector.c_opts.min_interval.clone() {
        None => {
            tracing::debug!("Reading packets");
            for p in inp {
                let pkt = match p {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!(e=?e,"skip final");
                        return Err(e)?;
                    }
                };
                collector.new_pkt(pkt, common, None)?;
            }
            collector
        }
        Some(interval) => {
            tracing::debug!("Setup interval");
            let c = Mutex::new(collector);
            let result: anyhow::Result<()> = std::thread::scope(|s| -> anyhow::Result<()> {
                let cr: &Mutex<_> = &c;
                let (tx, rx) = channel();
                let first = Arc::new(std::sync::Once::new());
                let f = first.clone();
                let _joinhandle = s.spawn(move || -> anyhow::Result<()> {
                    loop {
                        match rx.recv_timeout(interval.0) {
                            Ok(()) => {
                                tracing::debug!("Restart timeout ");
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                tracing::debug!("Timeout collect");
                                if f.is_completed() {
                                    if let Ok(mut collector) = cr.try_lock() {
                                        collector.collect(common)?;
                                    }
                                }
                            }
                            _ => return Ok(()),
                        }
                    }
                });
                tracing::debug!("interval Ok");

                for p in inp {
                    first.call_once(|| ());
                    tracing::trace!(?p, "new pkt");
                    let mut collector = c.lock().unwrap();
                    let pkt = p?;
                    collector.new_pkt(pkt, common, Some(&tx))?;
                }
                Ok(())
            });
            if let Err(e) = result {
                tracing::warn!(?e, "spawn return");
            }
            tracing::trace!("spawn done");
            c.into_inner().unwrap()
        }
    };
    tracing::debug!(final_links=?collector.links,"Done ");
    if !collector.links.is_empty() {
        collector.collect(common)?;
    }
    Ok(())
}
