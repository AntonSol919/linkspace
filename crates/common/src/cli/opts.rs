// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    io::{self },
    path::{PathBuf, Path},  sync::OnceLock, 
};

use crate::{
    pkt_reader::NetPktDecoder,
    prelude::*,
    runtime::{handlers::PktStreamHandler, Linkspace},
};
use anyhow::Context;
use clap::Parser;

use super::{write_pkt2, WriteDest, WriteDestSpec, reader::PktReadOpts};
#[derive(Parser, Debug, Clone)]
pub struct CommonOpts {
    #[command(flatten,next_help_heading="Instance Options")]
    pub linkspace: LinkspaceOpts,
    #[command(flatten,next_help_heading="General Pkt IO Options")]
    pub io: IOOpts,
}
impl std::ops::Deref for CommonOpts {
    type Target = LinkspaceOpts;
    fn deref(&self) -> &Self::Target {
        &self.linkspace
    }
}
#[derive(Parser, Debug, Clone)]
pub struct LinkspaceOpts {
    #[arg(
        short,
        long,
        env = "LK_DIR",
        help = "location of the linkspace instance - defaults to $HOME/linkspace"
    )]
    pub dir: Option<PathBuf>,
    #[arg(
        long,
        env = "LK_INIT",
        help = "create dir if it does not exists"
    )]
    pub init: bool,
    #[arg(long,help="enable [env:OS_VAR] abe scope",default_value_t)]
    pub env: bool,


    #[arg(skip)]
    pub open_dir: OnceLock<PathBuf>,
}
impl LinkspaceOpts {
    pub fn dir(&self) -> io::Result<&Path> {
        self.open_dir.get_or_try_init(|| crate::static_env::lk_dir(self.dir.as_deref()))
            .map(|o| o.as_path())
    }
    

    pub fn runtime_io(&self) -> io::Result<Linkspace> {
        crate::static_env::get_lk(self.dir.as_deref(), self.init)    
    }



    
    pub fn fake_eval_ctx(
    ) -> impl Scope {
        crate::eval::lk_scope(|| anyhow::bail!("no linkspace instance "), false)
    }
    pub fn eval_ctx(
        &self,
    ) -> impl Scope + '_{
        crate::eval::lk_scope(
            || self.runtime_io().context("could not open linkspace instance"),
            self.env)
    }
    pub fn eval_pkt_ctx<'o>(&'o self,pkt:&'o impl NetPkt) -> impl Scope+'o{
        (self.eval_ctx(),pkt_scope(pkt))
    }
    pub fn keys_dir(&self) -> io::Result<PathBuf> {
        Ok(self.dir()?.join("keys"))
    }
    
    pub fn runtime(&self) -> anyhow::Result<Linkspace> {
        self.runtime_io()
            .with_context(||format!(
                "Error opening runtime {:?}",
                crate::static_env::lk_dir(self.dir.as_deref())
            ))
    }
    
}

#[derive(Parser, Debug, Clone,Copy)]
pub struct IOOpts {
    #[arg(
        global = true,
        alias = "private_group",
        long,
        env = "LK_PRIVATE",
        help = "enable io of linkpoints in [#:0]"
    )]
    private: bool,
    #[command(flatten)]
    pub out: OutOpts,
    #[command(flatten)]
    pub inp: InOpts,
}

#[derive(Parser, Debug, Clone,Copy)]
pub struct OutOpts {
    #[arg(
        long,
        env = "LK_PRIVATE_WRITE",
        help = "enable output of linkpoints in [#:0]"
    )]
    private_write: Option<bool>,
}
#[derive(Parser, Debug, Clone,Copy )]
pub struct InOpts {
    #[arg(
        long,
        env = "LK_PRIVATE_READ",
        help = "enable input of linkpoints in [#:0]"
    )]
    pub(crate) private_read: Option<bool>,
    #[arg(
        long,
        env = "LK_HOP",
        help = "toggle hop netheader incr. true for commands unless stated otherwise"
    )]
    pub(crate) hop: Option<bool>,
    #[arg(
        long,
        env = "LK_SKIP_HASH",
        help = "skip validating hashes and signatures"
    )]
    pub skip_hash: bool,
}
impl InOpts {
    pub fn pkt_reader<P: std::io::Read>(self, reader: P) -> NetPktDecoder<P> {
        NetPktDecoder {
            allow_private: self.private_read.unwrap_or(false),
            reader,
            hop: self.hop.unwrap_or(true),
            skip_hash: self.skip_hash,
        }
    }
}

impl CommonOpts {
    pub fn default_hop(&mut self) {
        self.io.inp.hop.get_or_insert(true);
    }
    pub fn enable_private_group(&mut self) {
        self.io.private = true;
        self.io.inp.private_read = Some(true);
        self.io.out.private_write = Some(true)
    }
    pub fn mut_write_private(&mut self) -> &mut Option<bool> {
        &mut self.io.out.private_write
    }
    pub fn mut_read_private(&mut self) -> &mut Option<bool> {
        &mut self.io.inp.private_read
    }
    pub fn write_private(&self) -> Option<bool> {
        if self.io.private {
            Some(true)
        } else {
            self.io.out.private_write
        }
    }
    pub fn read_private(&self) -> Option<bool> {
        if self.io.private {
            Some(true)
        } else {
            self.io.inp.private_read
        }
    }
    pub fn check_private<S:NetPkt>(&self, pkt:S ) -> Result<Option<S>,linkspace_pkt::Error> {
        match self.write_private(){
            Some(false) => pkt.check_private().map_err(|e| {tracing::error!(?e,pkt=%PktFmtDebug(&pkt),"trying to write private");e})?,
            Some(true) => {},
            None => {
                if let Err(e) = pkt.check_private(){
                    tracing::info!(?e,pkt=%PktFmtDebug(&pkt),"trying to write private - but no --private or --private-write true/false set - ignoring packet");
                    return Ok(None)
                }
            }
        }
        Ok(Some(pkt))
    }
    pub fn open(&self, lst: &[WriteDestSpec]) -> std::io::Result<Vec<WriteDest>> {
        let ctx = self.eval_ctx();
        lst.iter()
            .filter_map(|v| v.open(&ctx).transpose())
            .try_collect()
    }
    pub fn write_dest(
        &self,
        dest: &mut WriteDest,
        pkt: &dyn NetPkt,
        buffer: &mut Option<&mut dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let pkt = match self.check_private(pkt).map_err(linkspace_pkt::Error::io)?{
            Some(p) => p,
            None => return Ok(()),
        };
        let out: &mut dyn std::io::Write = match &mut dest.out {
            super::Out::Db => {
                self.linkspace.runtime_io()?.env().save_dyn_one(pkt)?;
                return Ok(())
            },
            super::Out::Fd(f) => f,
            super::Out::Buffer => buffer
                .as_mut()
                .ok_or_else(|| io::Error::other("no buffer in this context"))?,
        };
        write_pkt2(&dest.prep, pkt, &self.eval_ctx(), out)
    }

    pub fn write_multi_dest(
        &self,
        mdest: &mut [WriteDest],
        pkt: &dyn NetPkt,
        mut buffer: Option<&mut dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let _ = tracing::debug_span!("Writing",pkt=%PktFmtDebug(pkt),recv=?pkt.recv() ).entered();
        for dest in mdest.iter_mut() {
            self.write_dest(dest, pkt, &mut buffer)?;
        }
        tracing::debug!("finish writing");
        Ok(())
    }

    pub fn multi_writer(self, mut mdest: Vec<WriteDest>) -> impl PktStreamHandler {
        let this = self;
        move |p: &dyn NetPkt, _rx: &Linkspace| -> std::io::Result<()> {
            this.write_multi_dest(&mut mdest, p, None)?;
            Ok(())
        }
    }
    pub fn stdout_writer(&self) -> impl PktStreamHandler {
        let allow_private = self.write_private().unwrap_or(false);
        let mut out = std::io::stdout();
        let ctx = self.clone();
        move |p: &dyn NetPkt, _rx: &Linkspace| -> std::io::Result<()> {
            if!allow_private{p.check_private().map_err(|e|e.io())?}
            write_pkt2(&None, p, &ctx.eval_ctx(), &mut out)
        }
    }
    pub fn inp_reader(&self,inp: &PktReadOpts) -> io::Result<NetPktDecoder<Box<dyn std::io::Read>>> {
        // Do not buffer. cli like handshake must not buffer partial packets and have  subsequent programs fail
        let reader = inp.open()?.expect("bug: - should be empty reader").into_read();
        Ok(NetPktDecoder {
            reader,
            allow_private: self.read_private().unwrap_or(false),
            hop: self.io.inp.hop.unwrap_or_default(),
            skip_hash: self.io.inp.skip_hash,
        })
    }
}
