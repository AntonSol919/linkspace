// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_common::{
    cli::{
        clap::{self, Parser},
        opts::{CommonOpts, PktIn},
        tracing, WriteDest, WriteDestSpec,
    },
    prelude::{TypedABE, *},
};
use std::io::{stderr, stdout, Write};

/**
Format packets from stdin according to a template.
**/
#[derive(Parser, Clone)]
pub struct PrintFmtOpts {
    #[clap(flatten)]
    pkt_in: PktIn,
    /// set forward to stdout and print to stderr. Similar to `| tee >( linkspace printf >&2 )`
    #[clap(long)]
    inspect: bool,
    /// fallback on eval error
    #[clap(short, long)]
    error: Option<TypedABE<Vec<u8>>>,
    /// dont warn on failure
    #[clap(short, long)]
    silent: bool,
    /// forward packets to dest
    #[clap(short, long, default_value = "null")]
    forward: Vec<WriteDestSpec>,
    /// Don't print delimiter for last packet.
    #[clap(short, long)]
    join_delimiter: bool,

    /// delimiter to print between packets.
    #[clap(short, long, default_value = "\\n")]
    delimiter: TypedABE<Vec<u8>>,
    #[clap(value_parser=parse_abe,default_value=&DEFAULT_PKT, action = clap::ArgAction::Append, env="LK_PRINTF")]
    fmt: Vec<Vec<ABE>>,
}

pub fn pkt_info(mut common: CommonOpts, popts: PrintFmtOpts) -> anyhow::Result<()> {
    let PrintFmtOpts {
        inspect,
        error,
        silent,
        mut forward,
        delimiter,
        fmt,
        join_delimiter,
        pkt_in,
    } = popts;
    let write_private = common.write_private().unwrap_or(true);
    common.mut_read_private().get_or_insert(true);

    let datap_fmt = fmt.get(0).context("Missing fmt")?.clone();
    let linkp_fmt = fmt.get(1).unwrap_or(&datap_fmt).clone();
    let keyp_fmt = fmt.get(2).unwrap_or(&linkp_fmt).clone();

    let ctx = common.eval_ctx();
    if error.is_none() && !silent {
        let data_test = eval(&pkt_ctx(ctx, &***PUBLIC_GROUP_PKT), &datap_fmt);
        let link_test = eval(&pkt_ctx(ctx, &***SINGLE_LINK_PKT), &linkp_fmt);
        if data_test.is_err() || link_test.is_err() {
            tracing::warn!(
                ?data_test,
                ?link_test,
                "chance of failure. set --error or --silent flag "
            );
        }
    }
    let error = error.map(|b| b.eval(&ctx)).transpose()?;
    let delimiter = delimiter.eval(&ctx)?;
    std::mem::drop(ctx);
    let out: &mut dyn Write;
    let mut stdo;
    let mut stde;
    if inspect {
        forward.push(WriteDest::stdout());
        stde = stderr().lock();
        out = &mut stde;
    } else {
        stdo = stdout().lock();
        out = &mut stdo;
    }
    let mut forward = common.open(&forward)?;
    let inp = common.inp_reader(&pkt_in)?;
    let mut first = true;
    for p in inp {
        let pkt = p?;
        if !write_private && pkt.group() == Some(&PRIVATE) {
            continue;
        }

        let abe = match pkt.as_point().point_header_ref().point_type {
            PointTypeFlags::DATA_POINT => &datap_fmt,
            PointTypeFlags::LINK_POINT => &linkp_fmt,
            PointTypeFlags::KEY_POINT => &keyp_fmt,
            _ => todo!(),
        };
        let ctx = pkt_ctx(common.eval_ctx(), &**pkt);
        match eval(&ctx, abe).with_context(|| print_abe(abe)) {
            Ok(b) => {
                if join_delimiter && !first {
                    out.write_all(&delimiter)?
                };
                out.write_all(&b.concat())?;
            }
            Err(e) => {
                if let Some(err_fmt) = &error {
                    if join_delimiter && !first {
                        out.write_all(&delimiter)?
                    };
                    out.write_all(&err_fmt)?;
                } else {
                    Err(e)?
                }
            }
        }
        if !join_delimiter {
            out.write_all(&delimiter)?;
        }
        out.flush()?;
        tracing::trace!(hash=?pkt.hash(),"Flush OK");
        common.write_multi_dest(&mut forward, &**pkt, None)?;
        first = false;
    }
    Ok(())
}
