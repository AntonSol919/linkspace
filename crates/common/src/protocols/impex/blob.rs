// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::{self};
use linkspace_core::{crypto::blake3, prelude::*};

pub static LK_MMAP_FILE_SIZE : std::sync::OnceLock<u64> = OnceLock::new();
pub fn should_mmap(file_size:u64) -> bool {
    let min_mmap_size = *LK_MMAP_FILE_SIZE.get_or_init(|| {
        std::env::var("LK_MMAP_FILE_SIZE").ok().and_then(|v|v.parse().ok()).unwrap_or(u64::MAX)
    });
    min_mmap_size <= file_size
}
pub static EMPTY_DATA_PKT : LazyLock<NetPktBox> = LazyLock::new(||datapoint(b"", NetPktHeader::EMPTY).as_netbox());
pub static EMPTY_DATA_HASH : LazyLock<LkHash> = LazyLock::new(|| EMPTY_DATA_PKT.hash());
pub const BLOB_SP: StaticSpace<6> = StaticSpace::from_raw(*b"\x05\0blob");

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Incomplete")]
    Incomplete(Vec<LkHash>),
    #[error("Io Error {0}")]
    IO(#[from] ::std::io::Error),
    #[error("Wrong context")]
    WrongContext,
    #[error("Other Err {0}")]
    Other(anyhow::Error),
}

pub use std::path::Path;
use std::{
    io::IoSlice,
    ops::{FromResidual, Try}, sync::{LazyLock, OnceLock}, 
};

pub fn datapkt_path_reader<F, R>(path: &Path, netopts: impl Into<NetOpts>, f: F) -> R
where
    F: FnMut(NetPktParts) -> R,
    R: Try<Output = ()> + FromResidual<Result<std::convert::Infallible, std::io::Error>>,
{
    let file = std::fs::File::open(path)?;
    let meta = file.metadata()?;
    if !should_mmap(meta.len()){
        datapkt_io_reader(file, netopts, f)
    }else {
        let mmap = unsafe { memmap2::Mmap::map(&file)? };
        let bytes = mmap.as_ref();
        datapkt_buf_reader(bytes, netopts, f)
    }
}

pub fn datapkt_buf_reader<R: Try<Output = ()>>(
    buf: &[u8],
    netopts: impl Into<NetOpts>,
    for_each: impl FnMut(NetPktParts) -> R,
) -> R {
    let netopts = netopts.into();
    buf.chunks(MAX_DATA_SIZE)
        .map(|v| datapoint(v, netopts))
        .try_for_each(for_each)
}
pub fn datapkt_io_reader<F, R>(
    reader: impl std::io::Read,
    netopts: impl Into<NetOpts>,
    mut f: F,
) -> R
where
    F: FnMut(NetPktParts) -> R,
    R: Try<Output = ()> + FromResidual<Result<std::convert::Infallible, std::io::Error>>,
{
    let netopts = netopts.into();
    super::chunk_reader_try_fold::<_, _, _, MAX_DATA_SIZE>(reader, (), |(), buf| {
        f(datapoint(buf, netopts))
    })
}

pub fn into_blob<F, R, O>(
    group: GroupID,
    domain: Domain,
    reader: impl std::io::Read,
    mut for_each: F,
) -> O
where
    F: FnMut(NetPktParts) -> R,
    R: Try<Output = ()> + FromResidual<Result<std::convert::Infallible, std::io::Error>>,
    O: Try<Output = LkHash> + FromResidual<<R as Try>::Residual>,
{
    let mut hasher = blake3::Hasher::new();
    let mut links = vec![];
    super::chunk_reader_try_fold::<_, _, _, MAX_DATA_SIZE>(reader, (), |(), buf| {
        hasher.update(buf);
        let pkt = datapoint(buf, ());
        links.push(Link {
            tag: ab(b"bytes"),
            ptr: pkt.hash(),
        });
        for_each(pkt)
    })?;
    if links.is_empty() {
        return O::from_output(*EMPTY_DATA_HASH);
    }
    if links.len() == 1 {
        return O::from_output(links[0].ptr);
    }
    let blob_hash = hasher.finalize();
    let mut space = BLOB_SP.to_owned().try_into_rooted().unwrap();
    space
        .extend_from_iter([blob_hash.as_bytes() as &[u8], b"part"])
        .unwrap();
    let mut links: &mut [Link] = &mut links;
    while links.len() > MAX_LINKS_LEN {
        let part_hash = {
            let part = linkpoint(
                group,
                domain,
                &space,
                &links[..MAX_LINKS_LEN],
                &[],
                Stamp::ZERO,
                (),
            );
            for_each(part)?;
            part.hash()
        };
        links = &mut links[MAX_LINKS_LEN - 1..];
        links[0] = Link {
            tag: ab(b"list"),
            ptr: part_hash,
        };
    }
    let bloblist = linkpoint(group, domain, &space, links, &[], Stamp::ZERO, ());
    let hash = bloblist.hash();
    for_each(bloblist)?;
    O::from_output(hash)
}

pub fn checkout_from(
    reader: &ReadTxn,
    mut out: impl std::io::Write,
    pkt: impl NetPkt,
) -> Result<(), Error> {
    let mut incomplete = vec![];
    if pkt.as_point().is_datapoint() {
        out.write_all(pkt.as_point().data())?;
        return Ok(());
    }
    let mut data_ptrs = vec![];
    open(
        &mut data_ptrs,
        reader,
        pkt.as_point().get_links(),
        &mut incomplete,
    )?;
    if !incomplete.is_empty() {
        return Err(Error::Incomplete(incomplete));
    }
    let mut datablks = reader
        .get_pkts_by_logidx(data_ptrs.into_iter())
        .map(|pkt| IoSlice::new(pkt.data()))
        .collect::<Vec<_>>();
    out.write_all_vectored(&mut datablks)?;
    out.flush()?;
    Ok(())
}

pub fn checkout(reader: &ReadTxn, out: impl std::io::Write, hash: LkHash) -> Result<(), Error> {
    let outer = reader.read(&hash)?.ok_or(Error::Incomplete(vec![hash]))?;
    checkout_from(reader, out, &*outer)
}

fn open(
    lst: &mut Vec<Stamp>,
    reader: &ReadTxn,
    links: &[Link],
    missing: &mut Vec<LkHash>,
) -> Result<(), Error> {
    for r in links {
        if r.tag == ab(b"bytes") {
            match reader.read_ptr(&r.ptr)? {
                Some(ptr) => lst.push(ptr),
                None => missing.push(r.ptr),
            }
        } else if r.tag == ab(b"list") {
            match reader.read(&r.ptr)? {
                Some(pkt) => open(lst, reader, pkt.get_links(), missing)?,
                None => missing.push(r.ptr),
            }
        } else {
            return Err(Error::Other(anyhow::anyhow!("unsupported link {:?}", r)));
        }
    }
    Ok(())
}
