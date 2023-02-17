// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{clap, clap::Parser, keys::KeyOpts, opts::CommonOpts, ReadSource, Reader, WriteDest},
    prelude::*,
};

#[derive(Parser, Debug)]
pub struct PointOpts {
    #[clap(long, alias = "u")]
    pub create: Option<StampExpr>,
    #[clap(long, conflicts_with = "create")]
    pub create_int: Option<Stamp>,

    #[clap(short, long)]
    pub link: Vec<LinkExpr>,

    #[clap(long)]
    pub sign: bool,
    #[clap(flatten)]
    pub key: KeyOpts,

    /// read input from 'stdin' 'file:filename' 'abe:expr'
    #[clap(long)]
    pub data: Option<ReadSource>,
    pub dgs: DGPExpr,
}

pub fn build<'o>(
    common: &CommonOpts,
    build_opts: &'o PointOpts,
    dgs: &'o DGP,
    links: &'o [Link],
    data_buf: &'o mut Vec<u8>,
    data_source: &mut Reader,
) -> anyhow::Result<NetPktParts<'o>> {
    let ctx = common.eval_ctx();
    let data = (data_source)(ctx.dynr(), data_buf)?;
    let stamp = build_opts
        .create
        .as_ref()
        .map(|s| s.eval(&ctx))
        .transpose()?
        .or(build_opts.create_int)
        .unwrap_or_else(now);
    let key = if build_opts.sign {
        Some(build_opts.key.identity(&common, true)?)
    } else {
        None
    };
    Ok(point(
        dgs.group,
        dgs.domain,
        &dgs.path,
        links,
        data,
        stamp,
        key,
        (),
    ))
}

pub fn linkpoint(
    mut common: CommonOpts,
    opts: PointOpts,
    dest: &mut [WriteDest],
) -> anyhow::Result<()> {
    let ctx = common.eval_ctx();
    let mut reader = common.open_read(&opts.data)?;
    let links: Vec<_> = opts.link.iter().map(|v| v.eval(&ctx)).try_collect()?;
    let dgs = opts.dgs.eval(&ctx)?;
    let mut buf = vec![];
    let pkt = build(&common, &opts, &dgs, &links, &mut buf, &mut reader)?;
    common.mut_write_private().get_or_insert(true);
    common.write_multi_dest(dest, &pkt, None)?;
    Ok(())
}
