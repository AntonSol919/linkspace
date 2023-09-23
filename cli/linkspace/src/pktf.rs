// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_common::{
    cli::{
        clap::{self, Parser},
        opts::{CommonOpts,},reader::PktReadOpts,
        tracing, WriteDest, WriteDestSpec,
    },
    prelude::{TypedABE,* },
};
use std::io::{stderr, stdout, Write};

/**
Format packets from stdin according to a template.
**/
#[derive(Parser, Clone)]
pub struct PktFmtOpts {
    #[command(flatten)]
    pkt_in: PktReadOpts,
    /// set forward to stdout and print to stderr. Similar to `| tee >( lk pktf >&2 )`
    #[arg(long)]
    inspect: bool,
    /// fallback on eval error
    #[arg(short, long)]
    error: Option<TypedABE<Vec<u8>>>,
    /// dont warn on failure
    #[arg(short, long)]
    silent: bool,
    /// forward packets to dest
    #[arg(short, long, default_value = "null")]
    forward: Vec<WriteDestSpec>,
    /// Don't print delimiter for last packet.
    #[arg(short, long)]
    join_delimiter: bool,

    /// delimiter to print between packets.
    #[arg(short, long, default_value = "\\n")]
    delimiter: TypedABE<Vec<u8>>,
    /// read non-abe bytes from the fmt as-is - i.e. allow newlines and utf8 in the format.
    #[arg(alias="strict",long)]
    no_parse_unencoded: bool,
    /// ABE expression to evaluate per packet - use a second and third expression to use differrent formats for [datapoint, [linkpoint [keypoint]]]
    #[arg(action = clap::ArgAction::Append, env="LK_PKTF")]
    fmt: Vec<String>,

    /// Use the default formatting function without evaluation.
    #[arg(long,conflicts_with("fmt"))]
    fast: bool
}

pub fn pkt_info(mut common: CommonOpts, popts: PktFmtOpts) -> anyhow::Result<()> {
    let PktFmtOpts {
        inspect,
        error,
        silent,
        mut forward,
        delimiter,
        fmt,
        join_delimiter,
        no_parse_unencoded,
        pkt_in,
        fast,
    } = popts;
    let parse_unencoded = !no_parse_unencoded;
    let write_private = common.write_private().unwrap_or(true);
    common.mut_read_private().get_or_insert(true);
    let datap_fmt = fmt.get(0).map(|o| {
        parse_abe(o, parse_unencoded)
    }).transpose()?.unwrap_or(DEFAULT_FMT.clone());
    let linkp_fmt = fmt.get(1).map(|o| parse_abe(o, parse_unencoded)).transpose()?.unwrap_or_else(||datap_fmt.clone());
    let keyp_fmt = fmt.get(2).map(|o| parse_abe(o, parse_unencoded)).transpose()?.unwrap_or_else(||linkp_fmt.clone());

    let ctx = common.eval_ctx();
    if error.is_none() && !silent {
        let data_test = eval(&(&ctx,pkt_scope(&***PUBLIC_GROUP_PKT)), &datap_fmt);
        let link_test = eval(&(&ctx,pkt_scope(&***SINGLE_LINK_PKT)), &linkp_fmt);
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
    #[allow(dropping_copy_types)]
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

        let mut write = |bytes:&[u8]| -> std::io::Result<()> {
            if join_delimiter && !first {
                out.write_all(&delimiter)?
            };
            out.write_all(&bytes)?;
            if !join_delimiter {
                out.write_all(&delimiter)?;
            }
            out.flush()?;
            tracing::trace!(hash=?pkt.hash(),"Flush OK");
            common.write_multi_dest(&mut forward, &**pkt, None)?;
            first = false;
            Ok(())
        };

        if fast {
            write(PktFmt(&**pkt).to_string().as_bytes())?;
        }else {
            let abe = match pkt.as_point().point_header_ref().point_type {
                PointTypeFlags::DATA_POINT => &datap_fmt,
                PointTypeFlags::LINK_POINT => &linkp_fmt,
                PointTypeFlags::KEY_POINT => &keyp_fmt,
                _ => todo!(),
            };
            let ctx = common.eval_pkt_ctx(&**pkt);
            match eval(&ctx, abe).with_context(|| print_abe(abe)) {
                Ok(b) =>  write(&b.concat())?,
                Err(e) => {
                    if let Some(err_fmt) = &error {
                        write(&err_fmt)?;
                    } else {
                        Err(e)?
                    }
                }
            }
        }

        
    }
    Ok(())
}
