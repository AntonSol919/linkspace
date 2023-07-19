// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::{ensure, Context};
use clap::Parser;
use linkspace_core::prelude::{IPathBuf, SPath, AB};

use crate::cli::clap;
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};
use tracing::instrument;
pub mod checkin;
pub mod checkout;
#[derive(Parser, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Mode {
    Checkin,
    Checkout,
}
#[derive(Parser, Debug, Clone)]
pub struct FsSyncOpts {
    #[arg(short, long)]
    pub file_first: bool,
    #[arg(long, env = "LK_SYNC", default_value = "./ssync")]
    pub root: PathBuf,
    #[arg(short, long,action=clap::ArgAction::Count)]
    pub clean: u8,
    #[arg(short, long)]
    pub r#async: bool,
    #[command(subcommand)]
    pub mode: Mode,
    /// modify the mapping between paths and spaths.
    #[arg(long)]
    pub modify: Vec<PathMod>,
}

pub fn resolve_spath(
    abs_root: &Path,
    root_sp: &SPath,
    abs_target: &Path,
    modify: &[PathMod],
) -> anyhow::Result<Option<IPathBuf>> {
    let relative = abs_target.strip_prefix(abs_root)?;
    let rest = relative
        .components()
        .filter_map(|v| v.as_os_str().to_str().map(|v| v.as_bytes()));
    let mut segments: Vec<_> = root_sp.iter().chain(rest).collect();
    for op in modify.iter() {
        match op {
            PathMod::PTake(idx, segm) => {
                if *idx < segments.len() {
                    let s = segments.remove(*idx);
                    if s != segm.0 {
                        tracing::trace!("No match");
                        return Ok(None);
                    }
                }
            }
        }
    }
    tracing::trace!(segments=?segments,"Ok");
    Ok(Some(IPathBuf::try_from_iter(segments)?))
}

pub fn resolve_path(
    abs_root: &Path,
    root_sp: &SPath,
    abs_target: &SPath,
    modify: &[PathMod],
) -> anyhow::Result<Option<PathBuf>> {
    let mut p = abs_root.to_owned();
    let relative = abs_target
        .strip_prefix(root_sp)
        .context("Out of prefix scope?")?;
    let mut relative = relative.iter().collect::<Vec<_>>();
    for modify in modify.iter().rev() {
        match modify {
            PathMod::PTake(idx, segm) => {
                relative.insert(*idx, segm);
            }
        }
    }
    for segm in relative {
        let segm: PathBuf = ::std::str::from_utf8(segm)?.try_into()?;
        let parent = segm
            .parent()
            .expect("no special chars should result in Some('') parent");
        ensure!(
            parent.parent().is_none(),
            "Invalid path segm name (Has parent) {:?} {:?}",
            segm,
            parent.parent()
        );
        ensure!(segm.is_relative(), "invalid path segm name {:?}", segm);
        p.push(segm);
    }
    Ok(Some(p))
}

impl FsSyncOpts {
    /// take absolute path, returns  absolute & modify
    #[instrument(skip(self),fields(modify = ?self.modify))]
    pub fn into_spath(
        &self,
        spath_root: &SPath,
        target_absolute: &Path,
    ) -> anyhow::Result<Option<IPathBuf>> {
        resolve_spath(&self.root, spath_root, target_absolute, &self.modify)
    }
    pub fn into_path(
        &self,
        spath_root: &SPath,
        target_absolute: &SPath,
    ) -> anyhow::Result<Option<PathBuf>> {
        resolve_path(&self.root, spath_root, target_absolute, &self.modify)
    }
}

#[derive(Debug, Clone)]
pub enum PathMod {
    PTake(usize, AB<Vec<u8>>),
}
impl FromStr for PathMod {
    type Err = anyhow::Error;
    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}
