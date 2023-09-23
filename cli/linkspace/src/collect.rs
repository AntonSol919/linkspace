// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    io::Write,
    time::Instant,
};

use crossbeam_channel::{Sender,bounded};
use crate::{point::PointOpts };
use linkspace_common::{
    cli::{clap, clap::Parser, opts::{CommonOpts }, tracing, WriteDest, WriteDestSpec, reader::{Reader, PktReadOpts, check_stdin}},
    core::stamp_fmt::DurationStr,
    prelude::*, dgs::DGS,
};

#[derive(Parser)]
/**
Create a linkpoint or keypoint with links to the packets received.
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
    /// tag for each link
    #[arg(alias = "ctag", long, value_enum, default_value = "[now]")]
    collect_tag: TagExpr,
    /// if set - adds a link to the previously created packet
    #[arg(long)]
    chain_tag: Option<TagExpr>,
    #[arg(long)]
    allow_empty: bool,
    /// link used for the first collect
    #[arg(alias = "il", long)]
    pub init_link: Vec<LinkExpr>,

    /// create packet after collecting max_links from incoming packets
    #[arg(long,default_value_t=MAX_LINKS_LEN-16)]
    max_links: usize,
    /// create a packet after [DurationStr]
    #[arg(long)]
    min_interval: Option<DurationStr>,

    /// destination for incoming packets
    #[arg(short, long, default_value = "stdout")]
    forward: Vec<WriteDestSpec>,
    /// destination for newly created collection packet
    #[arg(short, long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,

    #[command(flatten)]
    pkt_in: PktReadOpts,
    #[command(flatten)]
    build: PointOpts,
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
    dgs: DGS,
}
impl Collector {
    pub fn collect(&mut self, common: &CommonOpts) -> anyhow::Result<Option<()>> {
        if !self.c_opts.allow_empty && self.links.is_empty() {
            tracing::debug!("Skip empty");
            return Ok(Some(()));
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
        let pkt = match pkt {
            Some(p) => p,
            None => return Ok(None),
        };
        tracing::debug!(new_pkt=?pkt,"New collect Pkt");
        common.write_multi_dest(&mut self.write, &pkt, Some(&mut self.buf))?;
        let ctx = (common.eval_ctx(),pkt_scope(&pkt));
        let hash = pkt.hash();
        self.links.extend(
            self.c_opts
                .chain_tag
                .as_ref()
                .map(|tag| tag.eval(&ctx)).transpose()?
                .map(|tag| Link { tag, ptr: hash }),
        );
        for l in self.c_opts.build.link.iter() {
            self.links.push(l.eval(&ctx)?);
        }
        #[allow(dropping_copy_types)]
        std::mem::drop(pkt);

        self.current_links = 0;
        if !self.buf.is_empty() {
            tracing::debug!("Writing Buffer");
            let mut out = std::io::stdout();
            out.write_all(&self.buf)?;
            out.flush()?;
            self.buf.clear();
        }
        anyhow::Ok(Some(()))
    }
    pub fn new_pkt(
        &mut self,
        pkt: NetPktBox,
        common: &CommonOpts,
    ) -> anyhow::Result<bool> {
        let ctx = common.eval_ctx();
        let tag = self.c_opts.collect_tag.eval(&(ctx,pkt_scope(&**pkt)))?;

        self.links.push(Link {
            ptr: pkt.hash(),
            tag,
        });
        self.current_links += 1;

        tracing::trace!(hash=%pkt.hash(),outp=self.forward.len(), "writing new pkt ");
        common.write_multi_dest(&mut self.forward, &**pkt, Some(&mut self.buf))?;

        Ok(self.current_links >= self.c_opts.max_links)
    }
}

pub fn collect(common: &CommonOpts, c_opts: Collect) -> anyhow::Result<()> {
    check_stdin(&c_opts.pkt_in, &c_opts.build.read, false)?;
    let eval_ctx = common.eval_ctx();
    let initial_links: Vec<_> = c_opts
        .init_link
        .iter()
        .chain(c_opts.build.link.iter())
        .map(|v| v.eval(&eval_ctx))
        .try_collect()?;
    tracing::debug!(?initial_links, "Initial");
    if c_opts.build.sign {
        let _ = c_opts.build.key.identity(common, false)?;
    }
    let dgs = c_opts.build.dgs.eval(&eval_ctx)?;
    tracing::debug!(?dgs);
    let mut collector = Collector {
        links: initial_links,
        forward: common.open(&c_opts.forward)?,
        write: common.open(&c_opts.write)?,
        reader: c_opts.build.read.open_reader(false,&eval_ctx)?,
        dgs,
        buf: vec![],
        c_opts,
        current_links: 0,
    };
    match collector.c_opts.min_interval {
        None => {
            
            let inp = common.inp_reader(&collector.c_opts.pkt_in)?;
            tracing::debug!("Reading packets");
            for p in inp {
                let pkt = match p {
                    Ok(p) => p,
                    Err(e) => {
                        tracing::error!(e=?e,"skip final");
                        return Err(e)?;
                    }
                };
                let try_collect = collector.new_pkt(pkt, common)?;
                if try_collect{
                    let collect = collector.collect(common)?;
                    if collect.is_none(){
                        return Ok(());
                    }
                }
            }
            if !collector.links.is_empty() {
                collector.collect(common)?;
            }
            Ok(())
        }
        Some(interval) => {
            tracing::debug!("Setup interval");
            let result: anyhow::Result<()> = std::thread::scope(|s| -> anyhow::Result<()> {
                let (tx, rx) : (Sender<NetPktBox>,_)= bounded(0);
                let first = Arc::new(std::sync::Once::new());
                let f = first.clone();
                let pkt_in = collector.c_opts.pkt_in.clone();
                let _joinhandle = s.spawn(move || -> anyhow::Result<()> {
                    let inp = common.inp_reader(&pkt_in)?;
                    for pkt in inp {
                        first.call_once(|| ());
                        tracing::trace!(?pkt, "new pkt");
                        if let Err(e) = tx.send(pkt?){
                            tracing::info!(?e,"packet dropped");
                            Err(e)?
                        };
                    }
                    tracing::debug!("closed stdin");
                    Ok(())
                });
                let mut next = Instant::now() + interval.0;
                loop {
                    match rx.recv_deadline(next) {
                        Ok(pkt) => {
                            if collector.new_pkt(pkt, common)?{
                                if collector.collect(common)?.is_none(){
                                    return Ok(())
                                }else {
                                    next = Instant::now() + interval.0;
                                    tracing::debug!("Restart timeout ");
                                }
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            tracing::debug!("Timeout collect");
                            next = Instant::now() + interval.0;
                            if f.is_completed() {
                                let collect = collector.collect(common)?;
                                if collect.is_none(){
                                    return Ok(())
                                }
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            if !collector.links.is_empty() {
                                collector.collect(common)?;
                            }
                            return Ok(())
                        },
                    }
                }
            });
            result.map_err(|e|{
                tracing::warn!(?e, "spawn return");
                anyhow::anyhow!("spawn err?")
            })
        }
    }
}
