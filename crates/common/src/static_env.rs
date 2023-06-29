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
use std::sync::OnceLock;
pub static ROOT_PATH: OnceLock<PathBuf> = OnceLock::new();
static ENV: OnceLock<BTreeEnv> = OnceLock::new();

#[thread_local]
static LINKSPACE: OnceCell<Linkspace> = OnceCell::new();
pub fn get_env(root: &Path, mkdir: bool) -> io::Result<&'static BTreeEnv> {
    ENV.get_or_try_init(|| -> io::Result<BTreeEnv> {
        let mut env = BTreeEnv::open(root.to_owned(), mkdir)?;
        Arc::get_mut(&mut env.0).unwrap().log_head.init();
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
    LINKSPACE
        .get_or_try_init(|| {
            let env = get_env(&path, new)?;
            let rt = Linkspace::new_opt_rt(env.clone(), Default::default());
            rt.env().0.log_head.init();
            Ok(rt)
        })
        .map(|r| r.clone())
}

#[thread_local]
static GROUP: OnceCell<GroupID> = OnceCell::new();
pub fn set_group(group: GroupID) {
    assert_eq!(
        *GROUP.get_or_init(|| group),
        group,
        "user bug: the default group can only be set once per thread"
    );
}
/** [Thread Local]: get the 'default' group. from [set_group] || $LK_GROUP || [#:pub]

If the LK_GROUP expression requires LNS evaluation this will use the thread local linkspace or open the default.
**/
pub fn group() -> GroupID {
    use std::env::*;
    *GROUP.get_or_init(|| match std::env::var("LK_GROUP") {
        Err(VarError::NotPresent) => PUBLIC,
        Ok(o) => {
            let expr: GroupExpr = o.parse().expect("cant parse LK_GROUP");
            let ctx = std_ctx_v(
                || {
                    if let Some(o) = LINKSPACE.get() {
                        return Ok(o.clone());
                    }
                    tracing::info!("opening default linkspace to read evaluate LK_GROUP variable");
                    Ok(open_linkspace_dir(None, false)?)
                },
                EVAL0_1,
                true,
            );
            expr.eval(&ctx).expect("can't eval LK_GROUP")
        }
        _ => panic!("can't read LK_DOMAIN as utf8"),
    })
}

#[thread_local]
static DOMAIN: OnceCell<Domain> = OnceCell::new();

/// set the result for [domain]
pub fn set_domain(domain: Domain) {
    assert_eq!(
        *DOMAIN.get_or_init(|| domain),
        domain,
        "user bug: the standard domain can only be set once per thread"
    );
}
/** [Thread Local]: get the 'default' domain. from [set_domain] || $LK_DOMAIN || [0;16]

If the LK_DOMAIN expression requires LNS evaluation this will use the thread local linkspace or open the default.
**/
pub fn domain() -> Domain {
    use std::env::*;
    *DOMAIN.get_or_init(|| match std::env::var("LK_DOMAIN") {
        Err(VarError::NotPresent) => ab(b""),
        Ok(o) => {
            let expr: DomainExpr = o.parse().expect("cant parse LK_DOMAIN");
            let ctx = std_ctx_v(
                || {
                    if let Some(o) = LINKSPACE.get() {
                        return Ok(o.clone());
                    }
                    tracing::info!("opening default linkspace to read evaluate LK_DOMAIN variable");
                    Ok(open_linkspace_dir(None, false)?)
                },
                EVAL0_1,
                true,
            );
            expr.eval(&ctx).expect("can't eval LK_DOMAIN")
        }
        _ => panic!("can't read LK_DOMAIN as utf8"),
    })
}
