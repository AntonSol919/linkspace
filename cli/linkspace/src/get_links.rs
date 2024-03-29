use std::{cell::RefCell, rc::Rc};

use crate::*;
use anyhow::bail;
use linkspace::{
    lk_process, lk_process_while, lk_query_push, lk_stop, lk_watch, query::lk_hash_query, try_cb,
};
use linkspace_common::cli::{opts::CommonOpts, reader::PktReadOpts};

#[derive(Parser)]
pub struct GetLinks {
    #[command(flatten)]
    pkt_in: PktReadOpts,
    /// write dest of incoming packets
    #[arg(short, long, default_value = "stdout")]
    forward: Vec<WriteDestSpec>,
    /// write dest of linked packets
    #[arg(short, long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,
    /// recurse N times
    #[arg(short, long)]
    recursive: Option<usize>,
    /// recurse forever. Can be expensive!
    #[arg(short = 'R', long, default_value_t, conflicts_with("recursive"))]
    rrecursive: bool,

    #[command(subcommand)]
    mode: GetLinksMode,
}

#[derive(Clone, Parser, Copy)]
pub enum GetLinksMode {
    /// continue if not known
    Skip,
    /// wait for the point to be saved.
    Pause,
    /// return an error
    Error,
    /// continue and output out of order when it is saved.
    Watch,
}

#[allow(clippy::type_complexity)]
pub fn get_links(
    lk: &linkspace::Linkspace,
    pkt: &dyn NetPkt,
    level: usize,
    mode: GetLinksMode,
    out: Rc<RefCell<dyn FnMut(&dyn NetPkt) -> std::io::Result<()>>>,
) -> anyhow::Result<()> {
    for (count, link) in pkt.get_links().iter().enumerate() {
        let count_be = (count as u64).to_be_bytes();
        let qid = [&count_be as &[u8], &*link.ptr].concat();
        let q = lk_query_push(lk_hash_query(link.ptr), "", "qid", &qid)?;
        let print_fnc = out.clone();
        let res = lk_watch(
            lk,
            &q,
            try_cb(move |p, lk| -> anyhow::Result<()> {
                print_fnc.borrow_mut()(p)?;
                if level > 0 {
                    get_links(lk, p, level - 1, mode, print_fnc.clone())?;
                }
                Ok(())
            }),
        )?;
        if res != 0 {
            continue;
        }
        // Mode should probably set more query options like recv:
        match mode {
            GetLinksMode::Skip => {
                tracing::debug!("link {link} in {} not found", pkt.hash());
                lk_stop(lk, &qid, false);
            }
            GetLinksMode::Watch => {
                lk_process(lk);
            }
            GetLinksMode::Pause => {
                tracing::debug!("Pause on {link} in {} not found", pkt.hash());
                if lk_process_while(lk, Some(&*qid), Stamp::ZERO)? == 0 {
                    bail!("link {link} not found (from {})", pkt.hash())
                }
            }
            GetLinksMode::Error => bail!("link {link} not found (from {})", pkt.hash()),
        };
    }
    Ok(())
}

pub fn exec(common: CommonOpts, cmd: GetLinks) -> anyhow::Result<()> {
    let GetLinks {
        forward,
        write,
        pkt_in,
        mode,
        recursive: recurse,
        rrecursive: rrecurse,
    } = cmd;
    let rec = recurse.unwrap_or(if rrecurse { usize::MAX } else { 0 });
    let lk = common.runtime()?;
    let lk = linkspace::Linkspace::from_impl(&lk);

    let inp = common.inp_reader(&pkt_in)?;

    let mut forward = common.open(&forward)?;

    let c = common.clone();
    let mut write = c.open(&write)?;
    let out_fnc = Rc::new(RefCell::new(move |pkt: &dyn NetPkt| {
        c.write_multi_dest(&mut write, pkt, None)
    }));
    for pkt in inp {
        let pkt = pkt?;
        if !pkt.get_links().is_empty() {
            get_links(lk, &pkt, rec, mode, out_fnc.clone())?;
        };

        common.write_multi_dest(&mut forward, &pkt, None)?;
    }
    if matches!(mode, GetLinksMode::Watch) {
        while lk_process_while(lk, None, Stamp::ZERO)? != 1 {}
    }
    Ok(())
}
