// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{prelude::*, protocols::impex::blobmap::resolve_path};
use anyhow::Context;
use linkspace_core::prelude::pkt_predicates::PktPredicates;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use tracing::instrument;
type SyncState = BTreeMap<PathBuf, (Stamp, Option<LkHash>)>;
#[instrument(skip(reader))]
pub fn checkout_now(
    reader: ReadTxn,
    dgp: DGP,
    mut watch: PktPredicates,
    root: &Path,
    destroy: u8,
    file_first: bool,
) -> anyhow::Result<SyncState> {
    dgp.as_predicates()
        .try_for_each(|p| watch.add_predicate(&p))?;
    let root = root
        .canonicalize()
        .with_context(|| root.to_string_lossy().into_owned())?;
    let mut state: SyncState = SyncState::default();
    if destroy > 0 {
        let meta = std::fs::metadata(&root);
        if let Ok(m) = meta {
            if destroy > 1 {
                tracing::debug!("Removing {:?}", root);
                if m.is_dir() {
                    std::fs::remove_dir_all(&root)?
                } else if m.is_file() {
                    std::fs::remove_file(&root)?
                }
            } else {
                anyhow::bail!("nothing to do ")
            }
        }
    }
    std::fs::create_dir_all(root.parent().unwrap())?;
    let sp = dgp.path;
    for pkt in reader.query_tree(query_mode::Order::Desc, &watch) {
        let p = match resolve_path(&root, &sp, pkt.get_spath(), &[])? {
            Some(p) => p,
            None => {
                tracing::warn!("Ignoring {}", pkt.get_spath());
                continue;
            }
        };
        let e = state.entry(p).or_insert_with(|| (Stamp::ZERO, None));
        if e.0.get() < pkt.get_create_stamp().get() {
            *e = (
                *pkt.get_create_stamp(),
                pkt.get_links().first().map(|r| r.ptr),
            );
        }
    }
    tracing::info!("Found {} files", state.len());
    let mut iter = state.iter();
    let mut tmp;
    let iter: &mut dyn Iterator<Item = _> = if file_first {
        &mut iter
    } else {
        tmp = iter.rev();
        &mut tmp
    };
    for (path, (_stamp, chash)) in iter {
        match chash {
            Some(head) => {
                let file = std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .write(true)
                    .open(path)
                    .with_context(|| path.to_string_lossy().into_owned())?;
                file.set_len(0)?;
                crate::protocols::impex::blob::checkout(&reader, file, *head)?
            }
            None => {
                if destroy > 0 {
                    let _ = std::fs::remove_file(path);
                } else {
                    tracing::info!(path=?path,"Destory = 0 , not removing")
                }
            }
        }
    }
    Ok(state)
}
