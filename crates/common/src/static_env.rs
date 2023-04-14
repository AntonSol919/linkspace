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
use std::sync::OnceLock;
use std::thread::JoinHandle;

pub static ROOT_PATH: OnceLock<PathBuf> = OnceLock::new();
static ENV: OnceLock<BTreeEnv> = OnceLock::new();
static IPC_THREAD: OnceLock<JoinHandle<()>> = OnceLock::new();
thread_local! {
    static LINKSPACE: OnceCell<Linkspace> = OnceCell::new();
}
pub fn get_env(root: &Path, mkdir: bool) -> io::Result<&'static BTreeEnv> {
    ENV.get_or_try_init(|| -> io::Result<BTreeEnv> {
        let mut env = BTreeEnv::open(root.to_owned(), mkdir)?;
        env.log_head.init_udp();
        ROOT_PATH.set(root.canonicalize()?).unwrap();
        Ok(env)
    })
}

pub fn find_linkspace(root: Option<&Path>) -> io::Result<PathBuf> {
    match ROOT_PATH.get() {
        // something already opened
        Some(p) => {
            if let Some(r) = root {
                if &r.canonicalize()? != p {
                    todo!("only one env can be opened ({:?} already open)", p);
                }
            }
            Ok(p.to_owned())
        }
        None => {
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
    }
}

pub fn open_linkspace_dir(root: Option<&Path>, new: bool) -> io::Result<Linkspace> {
    let path = find_linkspace(root)?;
    Ok(LINKSPACE.with(|rt| {
        rt.get_or_try_init(|| {
            // ensure the IPC thread is propery enabled.
            if ENV.get().is_none() != IPC_THREAD.get().is_none() {
                return Err(io::Error::other(
                    "use get_linkspace before get_env if both are required",
                ));
            };
            let env = get_env(&path, new)?;
            let rt = Linkspace::new_opt_rt(env.clone(), Default::default());
            IPC_THREAD.get_or_init(|| rt.env().log_head.setup_ipc_thread());
            Ok(rt)
        })
        .map(|r| r.clone())
    })?)
}
