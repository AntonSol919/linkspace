// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_common::{
    cli::{clap, clap::Parser, keys::KeyOpts, opts::CommonOpts,  WriteDest, reader::{DataReadOpts, Reader}},
    prelude::*,
};

#[derive(Parser, Debug)]
pub struct MultiOpts {
    /// produce copies while reading data ( defaults is to read once and truncate. )
    #[clap(short,long)]
    multi: bool,
    /// Add link to the previous point 
    #[clap(long,requires("multi"))]
    multi_link: Option<TagExpr>,
    // #[clap(long,requires("multiple"))] multi_fixed_stamp: bool
}

#[derive(Parser, Debug)]
pub struct PointOpts {
    #[clap(long, alias = "u")]
    pub create: Option<StampExpr>,
    #[clap(long, conflicts_with = "create")]
    pub create_int: Option<Stamp>,

    //pub bare:bool,
    #[clap(long)]
    pub sign: bool,
    #[clap(flatten)]
    pub key: KeyOpts,

    #[clap(flatten)]
    pub read: DataReadOpts,
    pub dgs: DGPExpr,

    #[clap(last=true)]
    pub link: Vec<LinkExpr>,
}

pub fn build<'o>(
    common: &CommonOpts,
    build_opts: &'o PointOpts,
    dgs: &'o DGP,
    links: &'o [Link],
    data: &'o [u8],
) -> anyhow::Result<NetPktParts<'o>> {
    let ctx = common.eval_ctx();
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

pub fn build_with_reader<'o>(
    common: &CommonOpts,
    build_opts: &'o PointOpts,
    dgs: &'o DGP,
    links: &'o [Link],
    data_buf: &'o mut Vec<u8>,
    reader: &mut Reader,
) -> anyhow::Result<Option<NetPktParts<'o>>> {
    let ctx = common.eval_ctx();
    let freespace : usize = calc_free_space(&dgs.path, &links, &[], build_opts.sign).try_into()?;
    match reader.read_next_data(&ctx.dynr(),freespace, data_buf)?{
        Some(_) => Ok(Some(build(common, build_opts, dgs, links, data_buf)?)),
        None => Ok(None),
    }
}

pub fn linkpoint(
    mut common: CommonOpts,
    opts: PointOpts,
    multi: MultiOpts,
    dest: &mut [WriteDest],
) -> anyhow::Result<()> {
    common.mut_write_private().get_or_insert(true);
    common.mut_write_private().get_or_insert(true);
    let ctx = common.eval_ctx();
    let mut reader = opts.read.open_reader(false, &ctx)?;
    let mut links: Vec<_> = opts.link.iter().map(|v| v.eval(&ctx)).try_collect()?;
    let dgs = opts.dgs.eval(&ctx)?;
    let mut buf = vec![];
    let pkt = build_with_reader(&common, &opts, &dgs, &links, &mut buf, &mut reader)?.context("read EOF?")?;
    common.write_multi_dest(dest, &pkt, None)?;
    if !multi.multi{ return Ok(()) }
    let mut ptr = pkt.hash();
    #[allow(dropping_copy_types)]
    let _ = std::mem::drop(pkt);

    loop {
        if let Some(e) = &multi.multi_link{
            links.push(Link{tag: e.eval(&ctx)?,ptr});
        }
        buf.clear();
        match build_with_reader(&common, &opts, &dgs, &links, &mut buf, &mut reader)?{
            Some(p) => {
                common.write_multi_dest(dest, &p, None)?;
                ptr = p.hash();
            },
            None => return Ok(()),
        };
        if multi.multi_link.is_some() { links.pop();}
    }
}
