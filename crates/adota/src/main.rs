// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(try_trait_v2, control_flow_enum, exit_status_error, write_all_vectored)]

use std::{ffi::OsString, io::stdout, path::PathBuf};

use adoto::*;
use anyhow::Context;
fn main() -> anyhow::Result<()> {
    let p: OsString = std::env::args_os().nth(1).context("missing arg")?;
    let v = Expr::read_dir(PathBuf::from(p))?
        .simplify()
        .context("Empty dir")?;
    eprintln!("{:#?}", v);
    let v = v.map(&mut |_, _p| "hello\nworld");
    {
        //eprintln!("{}",v.to_doc().pretty(3));
    }

    v.as_ref().consume(&mut |v, _| eprintln!("{:?}", v));
    let v = v.serdify();
    serde_json::to_writer_pretty(stdout(), &v)?;
    Ok(())
}
