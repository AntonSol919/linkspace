// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_common::{
    cli::{
        clap,
        clap::Parser,
        keys::KeyOpts,
        opts::CommonOpts,
        reader::{DataReadOpts, Reader},
        WriteDest,
    },
    dgs::{DGSExpr, DGS},
    prelude::*,
};

#[derive(Parser, Debug, Default)]
pub struct MultiOpts {
    /// produce copies while reading data  (defaults is to read once and truncate)
    #[arg(short, long)]
    multi: bool,
    /// add link to the previous point
    #[arg(long, requires("multi"))]
    multi_link: Option<TagExpr>,
    // #[arg(long,requires("multiple"))] multi_fixed_stamp: bool
}

#[derive(Parser, Debug)]
pub struct PointOpts {
    /// 8 byte abe expression - e.g. [now:-1D] or [u64:0]  (defaults to now)
    #[arg(long, alias = "u", help = "")]
    pub create: Option<StampExpr>,
    /// decimal create stamp - eqv to [u64:{create_int}]
    #[arg(long, conflicts_with = "create")]
    pub create_int: Option<Stamp>,

    #[arg(long)]
    pub sign: bool,

    #[command(flatten, next_help_heading = "Data Options")]
    pub read: DataReadOpts,

    #[command(flatten, next_help_heading = "Sign Options")]
    pub key: KeyOpts,

    pub dgs: DGSExpr,

    #[arg(last = true)]
    pub link: Vec<LinkExpr>,
}

pub fn build<'o>(
    common: &CommonOpts,
    build_opts: &'o PointOpts,
    dgs: &'o DGS,
    links: &'o [Link],
    data: &'o [u8],
) -> anyhow::Result<NetPktParts<'o>> {
    let scope = common.eval_scope();
    let stamp = build_opts
        .create
        .as_ref()
        .map(|s| s.eval(&scope))
        .transpose()?
        .or(build_opts.create_int)
        .unwrap_or_else(now);
    let key = if build_opts.sign {
        Some(build_opts.key.identity(common, true)?)
    } else {
        None
    };
    Ok(point(
        dgs.group,
        dgs.domain,
        &dgs.space,
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
    dgs: &'o DGS,
    links: &'o [Link],
    data_buf: &'o mut Vec<u8>,
    reader: &mut Reader,
) -> anyhow::Result<Option<NetPktParts<'o>>> {
    let scope = common.eval_scope();
    let freespace: usize = calc_free_space(&dgs.space, links, &[], build_opts.sign).try_into()?;
    match reader.read_next_data(&scope, freespace, data_buf)? {
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
    let scope = common.eval_scope();
    let mut reader = opts.read.open_reader(false, &scope)?;
    let mut links: Vec<_> = opts.link.iter().map(|v| v.eval(&scope)).try_collect()?;
    let dgs = opts.dgs.eval(&scope)?;
    let mut buf = vec![];
    let pkt = build_with_reader(&common, &opts, &dgs, &links, &mut buf, &mut reader)?
        .context("read EOF?")?;
    common.write_multi_dest(dest, &pkt, None)?;
    if !multi.multi {
        return Ok(());
    }
    let mut ptr = pkt.hash();
    #[allow(dropping_copy_types)]
    std::mem::drop(pkt);

    loop {
        if let Some(e) = &multi.multi_link {
            links.push(Link {
                tag: e.eval(&scope)?,
                ptr,
            });
        }
        buf.clear();
        match build_with_reader(&common, &opts, &dgs, &links, &mut buf, &mut reader)? {
            Some(p) => {
                common.write_multi_dest(dest, &p, None)?;
                ptr = p.hash();
            }
            None => return Ok(()),
        };
        if multi.multi_link.is_some() {
            links.pop();
        }
    }
}

// this is unfortunate as it is PointOpts but with the only difference being Option<dgs>
#[derive(Parser, Debug)]
pub struct GenPointOpts {
    #[arg(long, alias = "u")]
    pub create: Option<StampExpr>,
    #[arg(long, conflicts_with = "create")]
    pub create_int: Option<Stamp>,
    #[arg(long)]
    pub sign: bool,

    #[command(flatten, next_help_heading = "Data Options")]
    pub read: DataReadOpts,
    #[command(flatten, next_help_heading = "Sign Options")]
    pub key: KeyOpts,
    pub dgs: Option<DGSExpr>,
    #[arg(last = true)]
    pub link: Vec<LinkExpr>,
}
