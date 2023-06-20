use either::Either;
// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{
        clap,
        clap::{Parser },
        keys::KeyOpts,
        opts::{CommonOpts} ,
        WriteDestSpec, reader::{DataReadOpts, Reader,PktReadOpts, check_stdin},
    },
    core::eval::Scope,
    prelude::*,
};
use anyhow::{Context,  ensure};


/** rewrite link and key points with alternative fields.

Note that options are expressions with the current packet in scope.
rewrite --path "[hash]/[group]"
rewrite --create "[create:+1D]"

**/

// TODO add Vec<linkmut { filter, add, map, }>
#[derive(Parser)]
pub struct Rewrite {
    #[clap(flatten)]
    pkt_in: PktReadOpts,

    #[clap(long, default_value = "stdout")]
    pub write: Vec<WriteDestSpec>,
    #[clap(long, default_value = "null")]
    pub forward: Vec<WriteDestSpec>,

    #[clap(short, long)]
    pub group: Option<HashExpr>,
    #[clap(short, long)]
    pub domain: Option<DomainExpr>,
    #[clap(short, long)]
    pub path: Option<IPathExpr>,
    #[clap(long, alias = "u")]
    pub create: Option<StampExpr>,

    /// Sign all spoints/asserts or only sign already signed
    #[clap(value_enum)]
    pub sign_mode: SignMode,
    #[clap(flatten)]
    pub key: KeyOpts,
    #[clap(short,long)]
    /// If set, use --data* options - --data-eval has pkt is in scope
    pub interpret: bool,
    #[clap(flatten)]
    pub data_read: DataReadOpts
}
pub fn rewrite_pkt(
    h: &LinkPointHeader,
    t: &Tail,
    opts: &Rewrite,
    key: Option<&SigningKey>,
    data: Either<&[u8],&mut Reader>,
    ctx: &EvalCtx<impl Scope>,
) -> anyhow::Result<NetPktBox> {
    let group = opts
        .group
        .as_ref()
        .map(|v| v.eval(ctx))
        .transpose()?
        .unwrap_or(h.group);
    let domain = opts
        .domain
        .as_ref()
        .map(|v| v.eval(ctx))
        .transpose()?
        .unwrap_or(h.domain);
    let path = opts.path.as_ref().map(|v| v.eval(ctx)).transpose()?;
    let path = path.as_deref().unwrap_or(t.ipath);
    let mut buf = vec![];
    let data :&[u8]= match data {
        Either::Left(d) => d,
        Either::Right(reader) => {
            let freespace : usize = calc_free_space(path, t.links, &[], key.is_some()).try_into()?;
            reader.read_next_data(&ctx.dynr(),freespace, &mut buf)?.context("No data provided")?;
            &buf
        },
    };

    let pkt = try_point(
        group,
        domain,
        path,
        t.links,
        data,
        opts.create
            .as_ref()
            .map(|o| o.eval(ctx))
            .transpose()?
            .unwrap_or(h.create_stamp),
        key,
        (),
    )?
    .as_netbox();
    Ok(pkt)
}

#[derive(clap::ValueEnum, PartialEq, Clone, Debug, Copy)]
pub enum SignMode {
    SignAll,
    Unsign,
    Skip,
    Resign,
}
pub fn rewrite(common: &CommonOpts, ropts: Rewrite) -> anyhow::Result<()> {
    check_stdin(&ropts.pkt_in, &ropts.data_read, false)?;
    ensure!( ropts.interpret || ropts.data_read == Default::default(),"read options are ignored if --interpret is not set");
    let Rewrite {
        write,
        forward,
        sign_mode,
        key,
        data_read,
        pkt_in,
        interpret,
        ..
    } = &ropts;
    let ctx = common.eval_ctx();
    if matches!(sign_mode, SignMode::SignAll | SignMode::Resign) {
        key.identity(common, true)?;
    }
    let mut reader = data_read.open_reader(false,&ctx)?;
    let inp = common.inp_reader(pkt_in)?;
    let mut write = common.open(write)?;
    let mut forward = common.open(forward)?;
    for p in inp {
        let pkt = p?;
        let data = if *interpret { Either::Right(&mut reader)} else { Either::Left(pkt.data())};
        match pkt.parts().fields {
            PointFields::Unknown(_) => todo!(),
            PointFields::DataPoint(_) => common.write_multi_dest(&mut write, &**pkt, None)?,
            PointFields::LinkPoint(s) => {
                let pctx = pkt_ctx(ctx.reref(), &**pkt);
                let key = if *sign_mode == SignMode::SignAll {
                    key.identity(common, false).ok()
                } else {
                    None
                };
                let pkt = rewrite_pkt(&s.head, &s.tail, &ropts, key, data, &pctx)?;
                common.write_multi_dest(&mut write, &**pkt, None)?;
            }
            PointFields::KeyPoint(lp,_s) => {
                match sign_mode {
                    SignMode::Skip => {
                        common.write_multi_dest(&mut write, &**pkt, None)?;
                    }
                    SignMode::Resign | SignMode::SignAll => {
                        let key = key.identity(common, false)?;
                        let pctx = pkt_ctx(ctx.reref(), &**pkt);
                        let pkt = rewrite_pkt(
                            &lp.head,
                            &lp.tail,
                            &ropts,
                            Some(key),
                            data,
                            &pctx,
                        )?;
                        common.write_multi_dest(&mut write, &**pkt, None)?;
                    }
                    SignMode::Unsign => {
                        let pctx = pkt_ctx(ctx.reref(), &**pkt);
                        let pkt = rewrite_pkt(
                            &lp.head,
                            &lp.tail,
                            &ropts,
                            None,
                            data,
                            &pctx,
                        )?;
                        common.write_multi_dest(&mut write, &**pkt, None)?;
                    }
                };
            }
            PointFields::Error(_) => todo!(),
            _ => todo!(),
        }

        common.write_multi_dest(&mut forward, &**pkt, None)?;
    }
    Ok(())
}
