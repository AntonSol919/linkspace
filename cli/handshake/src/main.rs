// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(unix_sigpipe)]
use linkspace_common::{
    anyhow::{self, Context},
    cli::{clap::Parser, keys::KeyOpts, opts::CommonOpts, reader::PktReadOpts, *},
    pkt_reader,
    prelude::NetPktFatPtr,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
/**
Exchange keypoints to prove each side holds a key.
**/
pub struct Opts {
    #[command(flatten)]
    common: CommonOpts,
    #[arg(long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,
    #[arg(long, default_value = "null")]
    /// copy input to
    forward: Vec<WriteDestSpec>,
    #[arg(long)]
    max_diff_secs: Option<usize>,
    #[command(flatten)]
    key: KeyOpts,
    #[command(subcommand)]
    mode: Handshake,
    #[command(flatten)]
    inp: PktReadOpts,
}

#[derive(Parser, Debug)]
pub enum Handshake {
    /// phase0 and phase1
    Connect,
    /// phase 1 and 3
    Serve,
    /// new keypoint
    Phase0,
    /// verify phase0 and output new keypoint linking it
    Phase1,
    /// verify phase1 and output new keypoint linking it
    Phase2,
    /// verify phase 2
    Phase3,
}

#[unix_sigpipe = "sig_dfl"]
fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::metadata::LevelFilter::WARN.into())
        .from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
    let Opts {
        mut common,
        write,
        forward,
        max_diff_secs,
        key,
        mode,
        inp,
    } = Opts::parse();
    common.default_hop();
    let mut forward = common.open(&forward)?;
    let mut write = common.open(&write)?;
    let id = key.identity(&common, false).context("Decrypting pass")?;
    let c2 = common.clone();
    let mut writer = |pkt: &NetPktFatPtr| c2.write_multi_dest(&mut write, &**pkt, None);
    use linkspace_common::protocols::handshake::*;
    let mut pkt_inp = common.inp_reader(&inp)?;
    let mut inp = std::iter::from_fn(move || {
        Some(match pkt_inp.next()? {
            Ok(p) => common
                .write_multi_dest(&mut forward, &**p, None)
                .map(move |()| p)
                .map_err(pkt_reader::Error::from),
            Err(e) => Err(e),
        })
    });
    match mode {
        Handshake::Phase0 => writer(&phase0_client_init(id).0)?,
        Handshake::Phase1 => writer(
            &phase1_server_signs(
                &Phase0(inp.next().context("Expected phase0")??),
                id,
                max_diff_secs,
            )?
            .0,
        )?,
        Handshake::Phase2 => {
            let (phase2, _server_key) = phase2_client_signs(
                &Phase0(inp.next().context("Missing phase0")??),
                &Phase1(inp.next().context("Missing phase1")??),
                id,
                max_diff_secs,
            )?;
            writer(&phase2.0)?
        }
        Handshake::Phase3 => {
            phase3_server_verify(
                &Phase0(inp.next().context("Missing phase0")??),
                &Phase1(inp.next().context("Missing phase1")??),
                &Phase2(inp.next().context("Missing phase2")??),
                id,
            )?;
        }
        Handshake::Connect => {
            let phase0 = phase0_client_init(id);
            writer(&phase0.0)?;
            let phase1 = Phase1(inp.next().context("Missing phase1")??);
            let (phase2, _key) = phase2_client_signs(&phase0, &phase1, id, max_diff_secs)?;
            writer(&phase2.0)?;
        }
        Handshake::Serve => {
            tracing::trace!("Init server");
            let phase0 = match inp.next() {
                Some(p) => Phase0(p.context("Reading phase0")?),
                None => anyhow::bail!("client hung up immediately"),
            };
            let phase1 = phase1_server_signs(&phase0, id, max_diff_secs)?;
            writer(&phase1.0)?;
            let phase2 = Phase2(inp.next().context("Missing phase2")??);
            let _key = phase3_server_verify(&phase0, &phase1, &phase2, id)?;
        }
    }
    Ok(())
}
