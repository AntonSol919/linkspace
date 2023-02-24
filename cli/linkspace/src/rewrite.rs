// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{
        clap,
        clap::{Parser, ValueEnum},
        keys::KeyOpts,
        opts::CommonOpts,
        ReadSource, Reader, WriteDestSpec,
    },
    core::eval::Scope,
    prelude::*,
};

/** rewrite link and key points with alternative fields.

Note that options are expressions with the current packet in scope.
rewrite --path "{hash}/{group}"
rewrite --create "{create:+1D}"

**/

// TODO add Vec<linkmut { filter, add, map, }>
#[derive(Parser)]
pub struct Rewrite {
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

    #[clap(long,default_value="abe-live:{data}")]
    pub data: ReadSource,
    #[clap(long, default_value_t, value_enum)]
    pub error_mode: ErrorMode,
}
#[derive(ValueEnum, Default, Clone, Copy, Debug)]
pub enum ErrorMode {
    #[default]
    Quit,
}

pub fn rewrite_pkt(
    h: &LinkPointHeader,
    t: &Tail,
    opts: &Rewrite,
    key: Option<&SigningKey>,
    data_reader: &mut Reader,
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
    let path = path.as_deref().unwrap_or(&t.ipath);
    let mut data = vec![];
    let pkt_data = (data_reader)(&ctx.dynr(), &mut data)?;

    let pkt = try_point(
        group,
        domain,
        path,
        t.links,
        &pkt_data,
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
    let Rewrite {
        write,
        forward,
        sign_mode,
        key,
        data,
        ..
    } = &ropts;
    let ctx = common.eval_ctx();
    if matches!(sign_mode, SignMode::SignAll | SignMode::Resign) {
        key.identity(&common, true)?;
    }
    let inp = common.inp_reader()?;
    let data = Some(data.clone());
    let mut reader = common.open_read(data.as_ref())?;
    let mut write = common.open(&write)?;
    let mut forward = common.open(&forward)?;
    for p in inp {
        let pkt = p?;
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
                let pkt = rewrite_pkt(&s.head, &s.tail, &ropts, key, &mut reader, &pctx)?;
                common.write_multi_dest(&mut write, &**pkt, None)?;
            }
            PointFields::KeyPoint(s) => {
                match sign_mode {
                    SignMode::Skip => {
                        common.write_multi_dest(&mut write, &**pkt, None)?;
                    }
                    SignMode::Resign | SignMode::SignAll => {
                        let key = key.identity(common, false)?;
                        let pctx = pkt_ctx(ctx.reref(), &**pkt);
                        let pkt = rewrite_pkt(
                            &s.head.linkpoint,
                            &s.tail,
                            &ropts,
                            Some(key),
                            &mut reader,
                            &pctx,
                        )?;
                        common.write_multi_dest(&mut write, &**pkt, None)?;
                    }
                    SignMode::Unsign => {
                        let pctx = pkt_ctx(ctx.reref(), &**pkt);
                        let pkt = rewrite_pkt(
                            &s.head.linkpoint,
                            &s.tail,
                            &ropts,
                            None,
                            &mut reader,
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
