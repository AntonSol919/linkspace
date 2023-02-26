// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    io::{self, stdin, Stdin},
    path::PathBuf,
};

use crate::{
    pkt_reader::NetPktDecoder,
    prelude::*,
    runtime::{handlers::PktStreamHandler, Linkspace},
};
use anyhow::Context;
use clap::Parser;

use super::{write_pkt2, ReadSource, Reader, WriteDest, WriteDestSpec};
#[derive(Parser, Debug, Clone)]
pub struct CommonOpts {
    #[clap(flatten)]
    pub linkspace: LinkspaceOpts,
    #[clap(flatten)]
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
    #[clap(
        short,
        long,
        env = "LINKSPACE",
        help = "linkspace root - defaults to $HOME/linkspace"
    )]
    pub root: Option<PathBuf>,
    #[clap(
        long,
        env = "LINKSPACE_INIT",
        help = "create root if it does not exists"
    )]
    pub init: bool,
}
impl LinkspaceOpts {
    pub fn root(&self) -> io::Result<PathBuf> {
        crate::static_env::find_linkspace(self.root.as_deref())
    }
    pub fn eval(&self, v: &str) -> anyhow::Result<ABList> {
        Ok(eval(&self.eval_ctx(), &parse_abe(v)?)?)
    }
    pub fn fake_eval_ctx(
    ) -> crate::eval::RTCtx<impl Fn() -> std::io::Result<Linkspace> + Copy + 'static> {
        crate::eval::std_ctx_v(|| Err(std::io::Error::other("no linkspace set")), EVAL0_1)
    }
    pub fn eval_ctx<'o>(
        &'o self,
    ) -> crate::eval::RTCtx<impl Fn() -> std::io::Result<Linkspace> + Copy + 'o> {
        crate::eval::std_ctx_v(|| self.runtime_io(), EVAL0_1)
    }
    pub fn keys_dir(&self) -> io::Result<PathBuf> {
        Ok(self.root()?.join("keys"))
    }
    pub fn runtime_io(&self) -> io::Result<Linkspace> {
        crate::static_env::open_linkspace_root(self.root.as_deref(), self.init)
    }
    pub fn runtime(&self) -> anyhow::Result<Linkspace> {
        self.runtime_io().context("error opening runtime")
    }
    pub fn env_io(&self) -> io::Result<&BTreeEnv> {
        crate::static_env::get_env(&self.root()?, self.init)
    }
    pub fn env(&self) -> anyhow::Result<&BTreeEnv> {
        self.env_io()
            .context("missing env. try opening with --init or set the --root")
    }
}

#[derive(Parser, Debug, Clone)]
pub struct IOOpts {
    #[clap(
        global = true,
        alias = "private_group",
        long,
        env = "PRIVATE_GROUP",
        help = "enable io of linkpoints in {#:0}"
    )]
    private: bool,
    #[clap(flatten)]
    pub inp: InOpts,
    #[clap(flatten)]
    pub out: OutOpts,
}

#[derive(Parser, Debug, Clone)]
pub struct OutOpts {
    #[clap(
        long,
        env = "WRITE_PRIVATE",
        help = "enable output of linkpoints in {#:0}"
    )]
    write_private: Option<bool>,
}
#[derive(Parser, Debug, Clone, Copy)]
pub struct InOpts {
    #[clap(
        long,
        env = "READ_PRIVATE",
        help = "enable input of linkpoints in {#:0}"
    )]
    pub(crate) read_private: Option<bool>,
    #[clap(
        long,
        env = "LINKSPACE_HOP",
        help = "toggle hop netheader incr. true for commands unless stated otherwise"
    )]
    pub(crate) hop: Option<bool>,
    #[clap(
        long,
        env = "LINKSPACE_NO_CHECK",
        help = "skip validating hashes and signatures"
    )]
    pub no_check: bool,
}
impl InOpts {
    pub fn pkt_reader<P: std::io::Read>(self, reader: P) -> NetPktDecoder<P> {
        NetPktDecoder {
            allow_private: self.read_private.unwrap_or(false),
            reader,
            hop: self.hop.unwrap_or(true),
            validate: !self.no_check,
        }
    }
}

impl CommonOpts {
    pub fn default_hop(&mut self) {
        self.io.inp.hop.get_or_insert(true);
    }
    pub fn enable_private(&mut self) {
        self.io.private = true;
        self.io.inp.read_private = Some(true);
        self.io.out.write_private = Some(true)
    }
    pub fn mut_write_private(&mut self) -> &mut Option<bool> {
        &mut self.io.out.write_private
    }
    pub fn mut_read_private(&mut self) -> &mut Option<bool> {
        &mut self.io.inp.read_private
    }
    pub fn write_private(&self) -> Option<bool> {
        if self.io.private {
            Some(true)
        } else {
            self.io.out.write_private
        }
    }
    pub fn read_private(&self) -> Option<bool> {
        if self.io.private {
            Some(true)
        } else {
            self.io.inp.read_private
        }
    }
    pub fn check_private(&self, pkt: impl NetPkt) -> Option<impl NetPkt> {
        let write_private = self.write_private().unwrap_or(false);
        if !write_private && pkt.as_point().group() == Some(&PRIVATE) {
            tracing::warn!(pkt=%pkt_fmt(&pkt),"Skip writing private (null) group");
            return None;
        }
        Some(pkt)
    }
    pub fn open(&self, lst: &[WriteDestSpec]) -> std::io::Result<Vec<WriteDest>> {
        let ctx = self.eval_ctx();
        lst.iter()
            .filter_map(|v| v.open(&ctx).transpose())
            .try_collect()
    }
    pub fn open_read(&self, r: Option<&ReadSource>) -> anyhow::Result<Reader> {
        ReadSource::into_reader(r, self.io.inp, &self.eval_ctx())
    }
    pub fn write_dest(
        &self,
        dest: &mut WriteDest,
        pkt: &dyn NetPkt,
        buffer: &mut Option<&mut dyn std::io::Write>,
    ) -> std::io::Result<()> {
        let pkt = match self.check_private(pkt) {
            Some(p) => p,
            None => return Ok(()),
        };
        let out: &mut dyn std::io::Write = match &mut dest.out {
            super::Out::Db => {
                return save_pkt(&mut self.linkspace.env_io()?.get_writer()?, pkt).map(|_| ())
            }
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
        let _ = tracing::debug_span!("Writing",pkt=%pkt_fmt(pkt)).entered();
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
        tracing::trace!(allow_private);
        let mut out = std::io::stdout();
        let ctx = self.clone();
        move |p: &dyn NetPkt, _rx: &Linkspace| -> std::io::Result<()> {
            write_pkt2(&None, p, &ctx.eval_ctx(), &mut out)
        }
    }
    pub fn inp_reader(&self) -> io::Result<NetPktDecoder<Stdin>> {
        let inp = stdin(); // Do not buffer. cli like handshake must not buffer partial packets and have  subsequent programs fail
        Ok(NetPktDecoder {
            allow_private: self.read_private().unwrap_or(false),
            reader: inp,
            hop: self.io.inp.hop.unwrap_or_default(),
            validate: !self.io.inp.no_check,
        })
    }
}
