// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(write_all_vectored)]
pub mod blob;
pub mod blobmap;

use linkspace::{
    anyhow::{self},
    cli::{
        clap::{Parser, Subcommand},
        opts::CommonOpts,
        *,
    },
};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
/// WARN - currently unstable format
/// Import/export a files and folders.
pub struct Opt {
    #[command(flatten)]
    common: CommonOpts,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
pub enum Cmd {
    #[command(alias = "file")]
    Blob {
        #[command(subcommand)]
        cmd: blob::Cmd,
    },
    #[command(alias = "folder")]
    Blobmap {
        #[command(subcommand)]
        cmd: blobmap::Cmd,
    },
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::metadata::LevelFilter::WARN.into())
        .from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
    let Opt { common, cmd } = Opt::parse();
    match cmd {
        Cmd::Blob { cmd } => blob::cmd(common, cmd),
        Cmd::Blobmap { cmd } => blobmap::cmd(common, cmd),
    }
}
