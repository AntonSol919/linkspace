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
use liblinkspace::linkspace::lk_get_all;
use liblinkspace::{abe::lk_split_abe, lk_query };
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
        default_value = "[hash:str]\\n[/links:\\t[ptr/2mini]\\t[tag:str]\\n]\\n[data_size:str]\\n[data/ltrim:40/?a]"
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

    let mut query = lk_query(None);
    let mut ok = Ok(true);
    let mut i = 0;
    // arguments ould have been typed DGPExpr. This shows one way to parse manually while only importing liblinkspace
    lk_split_abe(&dgpe, b"/", |expr: &str, _ctr: u8| {
        i += 1;
        ok = match i {
            1 => lk_query_parse(&mut query, &format!("domain:=:[{expr}]"),()),
            2 => lk_query_parse(&mut query, &format!("group:=:[{expr}]"),()),
            3 => lk_query_parse(&mut query, &format!("prefix:=:[{expr}]"),()),
            _ => Err(anyhow::anyhow!("expected domain:group:prefix")),
        };
        ok.is_ok()
    })?;
    if i < 2 {
        lk_query_parse(&mut query, "group:=:[#:pub]",())?;
    }
    for stmt in &cli.statement {
        lk_query_parse(&mut query, &stmt,())?;
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

    // horizontal order
    //pkt_list.sort_by_cached_key(|entry| lk_eval(&cli.sort, Some(&entry.pkt)).unwrap());
    //pkt_list.iter().enumerate().for_each(|(i,n)| n.index.set(i));


    fn fmt_node<'n: 'x, 'x>(
        prefix: &mut Vec<&'static str>,
        path: &'x mut Vec<&'n PathNode>,
        node: &'n PathNode,
        opts: &Cli,
        out: &mut dyn Write,
    ) -> LkResult<()> {
        use std::fmt::Write;

        let name = if !path.is_empty() {
            lk_encode(node.path.last(), &opts.path_encode)
        } else {
            opts.dgpe.clone()
        };
        let mut line = format!("{}-+{name}",prefix.concat());
        if !node.packets.is_empty() {
            write!(line, " ─ ({})", node.packets.len())?;
        }
        writeln!(out, "{}", line)?;

        
        if node.last.get(){
            prefix.push("   ");
        }else {
            prefix.push("  |");
        }

        if !opts.fmt.is_empty() {
            let mut pre = prefix.concat().into_bytes();
            pre.push(b'>');
            for p in node.packets.iter() {
                let st = lk_eval(&opts.fmt, &*p.pkt as &dyn NetPkt)?;
                for line in st.split_inclusive(|v| *v == b'\n') {
                    out.write_all(&pre)?;
                    out.write_all(line)?;
                }
            }
            pre.pop();
            out.write(&pre)?;
        }

        if let Some((_, v)) = node.children.last_key_value() {
            v.last.set(true);
        }

        path.push(node);
        for v in node.children.values() {
            fmt_node(prefix,path, v, opts, out)?;
        }
        prefix.pop();
        path.pop();
        Ok(())
    }
    let p = liblinkspace::linkspace::lk_info(&lk).path;
    println!("fixme");
    println!("{:?}",p);
    fmt_node(&mut vec![],&mut vec![], &root, &cli, &mut std::io::stdout())?;
    Ok(())
}
