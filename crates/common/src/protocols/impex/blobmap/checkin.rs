// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{dgp::DGP, protocols::impex::blobmap::resolve_spath};

use anyhow::Result;
use linkspace_core::prelude::*;

use notify::{
    event::{AccessKind, AccessMode, RemoveKind},
    EventKind, RecommendedWatcher,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tracing::instrument;

#[instrument(skip(for_each))]
pub fn import_blob<F>(mut for_each: F, full_path: &Path, dgs: &DGP) -> Result<LkHash, std::io::Error>
where
    F: FnMut(NetPktParts) -> Result<(), std::io::Error>,
{
    tracing::debug!("Importing");
    let file = std::fs::File::open(full_path)?;
    let ptr = crate::protocols::impex::blob::into_blob::<
        _,
        Result<(), std::io::Error>,
        Result<LkHash, std::io::Error>,
    >(dgs.group, dgs.domain, file, &mut for_each)?;
    let links = [Link {
        tag: ab(b"blob"),
        ptr,
    }];
    let spoint = linkpoint(dgs.group, dgs.domain, &dgs.path, &links, &[], now(), ());
    tracing::debug!(spoint=?spoint,"Ok");
    let hash = spoint.hash();
    for_each(spoint)?;
    Ok(hash)
}

pub type FSState = HashMap<IPathBuf, LkHash>;
pub fn encode_now<F>(mut for_each: F, root: &Path, dgs: &DGP) -> anyhow::Result<FSState>
where
    F: FnMut(NetPktParts) -> Result<(), std::io::Error>,
{
    let mut store_state = HashMap::new();
    let mut dirs = vec![root.to_owned()];
    while let Some(dir) = dirs.pop() {
        for e in dir.read_dir()? {
            let e = e?;
            let path = e.path();
            if path.is_file() {
                let spath = match resolve_spath(root, &dgs.path, &path, &[])? {
                    Some(s) => s,
                    None => continue,
                };
                tracing::debug!(path=?path,current=?e);
                let hash = import_blob(&mut for_each, &path, dgs)?;
                store_state.insert(spath, hash);
                tracing::debug!(import_result=?hash);
            } else if path.is_dir() {
                dirs.push(path);
            }
        }
    }
    Ok(store_state)
}
pub fn checkin_event<F>(
    ev: notify::Event,
    root: &Path,
    mut for_each: F,
    dgs: &DGP,
) -> anyhow::Result<()>
where
    F: FnMut(NetPktParts) -> Result<(), std::io::Error>,
{
    match ev.kind {
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
            for full_path in ev.paths {
                let spath = match resolve_spath(root, &dgs.path, &full_path, &[])? {
                    Some(s) => s,
                    None => return Ok(()),
                };
                tracing::debug!(spath=%spath,full_path=?full_path,"write");
                import_blob(&mut for_each, &full_path, dgs)?;
            }
        }
        EventKind::Remove(RemoveKind::File) => {
            for full_path in ev.paths {
                let spath = match resolve_spath(root, &dgs.path, &full_path, &[])? {
                    Some(s) => s,
                    None => return Ok(()),
                };
                tracing::debug!(spath=%spath.as_ref(),"Removing");
                let spoint = linkpoint(dgs.group, dgs.domain, &dgs.path, &[], &[], now(), ());
                for_each(spoint)?;
            }
        }
        _ => tracing::debug!("Ignored"),
    }
    Ok(())
}

pub fn encode_forever(
    mut for_each: impl FnMut(NetPktParts) -> std::io::Result<()>,
    root: PathBuf,
    base: DGP,
) -> anyhow::Result<()> {
    let root = root.canonicalize()?;
    encode_now(&mut for_each, &root, &base)?;
    use notify::Watcher;
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = RecommendedWatcher::new(tx, Default::default())?;
    watcher.watch(&root, notify::RecursiveMode::Recursive)?;
    tracing::debug!(watch=?root,"Watch OK");
    for ev in rx {
        match ev {
            Ok(ev) => {
                if let Err(e) = checkin_event(ev, &root, &mut for_each, &base) {
                    tracing::error!(error=?e,"Checkin error");
                }
            }
            Err(e) => todo!("{:?}", e),
        }
    }
    Ok(())
}
