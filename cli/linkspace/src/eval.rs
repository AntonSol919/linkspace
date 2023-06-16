// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{
        clap::{self, Parser, Subcommand},
        opts::CommonOpts,
        reader::{ DataReadOpts},
    },
    prelude::{eval,scope::{ argv::ArgList},  *},
};
use std::io::{ Write};

#[derive(Parser, Clone)]
pub struct EvalOpts {
    /// output json ABList format
    #[clap(long)]
    json: bool,
    abe: String,
    /// add argv context from a data source - (i.e. [0] [1] ... [7])
    #[command(subcommand)]
    data: Option<WithData>
}
#[derive(Subcommand,Clone,Debug)]
pub enum WithData{
    Argv(DataReadOpts),
}

pub fn eval_cmd(common: CommonOpts, opts: EvalOpts) -> anyhow::Result<()> {
    let EvalOpts { json, abe, data } = opts;

    let abe = parse_abe(&abe)?;

    let mut arglist = vec![];
    let ctx = common.eval_ctx();
    if let Some(WithData::Argv(read_opts)) = data {

        let mut reader = read_opts.open_reader(true, &ctx)?;
        loop {
            let tmp = ctx.scope(ArgList::new(arglist.as_slice()));
            let ctx = tmp.dynr();
            let mut bytes = vec![];
            let cont = reader.read_next_data(&ctx, usize::MAX, &mut bytes) ?;
            if cont.is_none() {break};
            arglist.push(bytes);
        }
    }
    
    let ctx = ctx.scope(ArgList::new(arglist.as_slice()));
    let val = eval(&ctx, &abe)?;
    let mut out = std::io::stdout();
    if json {
        use serde_json::{to_value, value::Value};
        let mut lst = val
            .inner()
            .iter()
            .map(|(b, v)| {
                (
                    String::from_utf8(b.clone())
                        .map(Value::String)
                        .unwrap_or_else(|_| to_value(b).unwrap()),
                    v,
                )
            })
            .map(to_value);
        let vec = Value::Array(lst.try_collect()?);
        println!("{vec}");
    } else {
        out.write_all(&val.concat())?;
    }
    out.flush()?;
    Ok(())
}

