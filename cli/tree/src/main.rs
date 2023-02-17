// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use liblinkspace::misc::RecvPkt;
use liblinkspace::runtime::lk_get_all;
use liblinkspace::{abe::lk_split_abe, lk_query, lk_query_append};
use liblinkspace::{lk_encode, lk_eval, lk_open, prelude::*};

use clap::Parser;

#[derive(Parser)]
#[clap(author, version = "0.1", about, long_about = None)]
/**
Print a domain:group:path tree.
Using fmt changes the printing of the latest packets, use "" to not print any extra information
path_encode is used to print the path name component, by default this will use lns lookup.
Use --path-encode "b:32" to only encode 32 byte long components as base64
**/
struct Cli {
    #[clap(short, long, env = "LINKSPACE")]
    linkspace: Option<PathBuf>,
    dgpe: String,
    #[clap(
        default_value = "{hash:str}\\n{/links:> {ptr/2mini} \\: {tag:str}\\n}\\n{data_size:str}     {data/trim<:40/?a}\\n"
    )]
    /// eval for printing packets
    fmt: String,
    #[clap(short, long, default_value = "local@/local#/@/#/b:32")]
    path_encode: String,
    #[clap(last = true)]
    statement: Vec<String>,
}

#[derive(Default)]
struct PathNode {
    last: Cell<bool>,
    path: IPathBuf,
    packets: Vec<Rc<PathNodeEntry>>,
    children: BTreeMap<Vec<u8>, PathNode>,
}
impl PathNode {
    pub fn get(&mut self, path: &[&[u8]]) -> &mut PathNode {
        match path.split_first() {
            Some((p, rest)) => self
                .children
                .entry(p.to_vec())
                .or_insert_with(|| PathNode {
                    path: self.path.clone().append(p),
                    ..Default::default()
                })
                .get(rest),
            None => self,
        }
    }
}
#[allow(dead_code)]
struct PathNodeEntry {
    index: Cell<usize>,
    pkt: RecvPkt<NetPktBox>,
}

fn main() -> LkResult<()> {
    let cli = Cli::parse();
    let Cli {
        linkspace, dgpe, ..
    } = &cli;

    let mut query = lk_query();
    let mut ok = Ok(true);
    let mut i = 0;
    // arguments ould have been typed DGPExpr. This shows one way to parse manually while only importing liblinkspace
    lk_split_abe(&dgpe, b"/", |expr: &str, _ctr: u8| {
        i += 1;
        ok = match i {
            1 => lk_query_append(&mut query, &format!("domain:=:{}", expr)),
            2 => lk_query_append(&mut query, &format!("group:=:{}", expr)),
            3 => lk_query_append(&mut query, &format!("prefix:=:{}", expr)),
            _ => Err("expected domain:group:prefix".into()),
        };
        ok.is_ok()
    })?;
    if i < 2 {
        lk_query_append(&mut query, &format!("group:=:{{#:pub}}"))?;
    }
    for stmt in &cli.statement {
        lk_query_append(&mut query, &stmt)?;
    }

    let lk = lk_open(linkspace.as_deref(), false)?;
    let mut root = PathNode::default();
    root.last.set(true);
    let mut pkt_list = vec![];
    let mut cb = |pkt: &dyn NetPkt| {
        let p = pkt.get_ipath();
        let node = Rc::new(PathNodeEntry {
            index: Cell::new(0),
            pkt: pkt.into(),
        });
        pkt_list.push(node.clone());
        root.get(&p.comps_bytes()[..*p.path_len() as usize])
            .packets
            .push(node);
        true
    };
    lk_get_all(&lk, &query, &mut cb)?;
    /*
    pkt_list.sort_by_cached_key(|entry| lk_eval(&cli.sort, Some(&entry.pkt)).unwrap());
    pkt_list.iter().enumerate().for_each(|(i,n)| n.index.set(i));
    */
    fn fmt_node<'n: 'x, 'x>(
        path: &'x mut Vec<&'n PathNode>,
        node: &'n PathNode,
        opts: &Cli,
        out: &mut dyn Write,
    ) -> LkResult<()> {
        let mut prefix = String::new();
        for n in path.iter() {
            prefix.push_str(if n.last.get() { "   " } else { "│  " });
        }
        let hook = if node.last.get() { "└" } else { "├" };
        let name = if !path.is_empty() {
            lk_encode(node.path.last(), &opts.path_encode)
        } else {
            opts.dgpe.clone()
        };
        let mut line = format!("{prefix}{hook}─ {name}");
        if !node.packets.is_empty() {
            use std::fmt::Write;
            write!(line, " ─ ({})", node.packets.len())?;
        }
        writeln!(out, "{}", line)?;

        if !opts.fmt.is_empty() {
            let mut prefix = prefix.clone();
            prefix.push_str(if node.last.get() { "   " } else { "│  " });
            if let Some(p) = node.packets.first() {
                let st = lk_eval(&opts.fmt, Some(&*p.pkt))?;
                for v in st.split_inclusive(|v| *v == b'\n') {
                    out.write_all(prefix.as_bytes())?;
                    out.write_all(v)?;
                }
            }
        }

        if let Some((_, v)) = node.children.last_key_value() {
            v.last.set(true);
        }

        path.push(node);
        for v in node.children.values() {
            fmt_node(path, v, opts, out)?;
        }
        path.pop();
        Ok(())
    }
    let p = liblinkspace::runtime::lk_info(&lk).path;
    println!("{p}");
    fmt_node(&mut vec![], &root, &cli, &mut std::io::stdout())?;
    Ok(())
}
