use fxhash::FxHashMap;
use linkspace_core::prelude::lmdb::BTreeEnv;

// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::prelude::*;
use std::cell::OnceCell;
/// static btreeenv, shares one receiver thread and database session.
/// With thread local linkspace's
/// TODO: allow multiple
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

static ENVS: RwLock<Option<FxHashMap<same_file::Handle, BTreeEnv>>> = RwLock::new(None);

#[thread_local]
pub static LINKSPACE: OnceCell<Linkspace> = OnceCell::new();

pub fn get_env(path: &Path, mkdir: bool) -> io::Result<BTreeEnv> {
    // this is just a basic dedup. This isn't protection against moving stuff about.
    let handle = match same_file::Handle::from_path(path) {
        Ok(h) => h,
        Err(_e) if mkdir => {
            std::fs::create_dir_all(path)?;
            same_file::Handle::from_path(path)?
        }
        Err(e) => return Err(e),
    };
    if let Some(v) = ENVS
        .read()
        .unwrap()
        .as_ref()
        .and_then(|o| o.get(&handle).cloned())
    {
        return Ok(v);
    }
    let env = BTreeEnv::open(path.to_owned(), mkdir)?;
    ENVS.write()
        .unwrap()
        .get_or_insert_default()
        .insert(handle, env.clone());
    Ok(env)
}

pub fn lk_dir(root: Option<&Path>) -> io::Result<PathBuf> {
    let path = root
        .map(|v| v.to_path_buf())
        .or_else(|| std::env::var_os("LK_DIR").map(PathBuf::from))
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|v| v.join("linkspace"))
        })
        .ok_or_else(|| io::Error::other("unknown linkspace fs entry"))?;
    Ok(path)
}

/// With a path it will ALWAYS open a new instance - without it will clone the first opened
pub fn open_linkspace_dir(path: Option<&Path>, create_env: bool) -> io::Result<Linkspace> {
    let buf;
    let path = match path {
        Some(p) => p,
        None => {
            buf = lk_dir(None)?;
            &buf
        }
    };
    let env = get_env(path, create_env)?;
    let lk = Linkspace::new_opt_rt(env.clone(), Default::default());
    LINKSPACE.get_or_init(|| lk.clone());
    Ok(lk)
}

/// Defaults to using already open linkspace runtime - then tries to open the path.
pub fn get_lk(path: Option<&Path>, create_env: bool) -> io::Result<Linkspace> {
    if let Some(o) = LINKSPACE.get() {
        Ok(o.clone())
    } else {
        open_linkspace_dir(path, create_env)
    }
}
