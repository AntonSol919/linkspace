// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::sync::LazyLock;

use bstr::BStr;
use byte_fmt::abe::{parse_abe, ABE, ToABE};

use crate::{NetPkt, PointExt, Point, PRIVATE, PUBLIC, TEST_GROUP};
/// default fmt in many cases and output for `[pkt]`
pub static DEFAULT_PKT: &str = "\
type\\t[type:str]\\n\
hash\\t[hash:str]\\n\
group\\t[/~?:[group]/#/b]\\n\
domain\\t[domain:str]\\n\
path\\t[path:str]\\n\
pubkey\\t[/~?:[pubkey]/@/b]\\n\
create\\t[create:str]\\n\
links\\t[links_len:str]\\n\
[/links:\\t[tag:str] [ptr:str]\\n]\\n\
data\\t[data_size:str]\\n\
[data/~utf8]\\n\
";
/// A static equivalent to [pkt_fmt] without using abe.


pub static DEFAULT_FMT: LazyLock<Vec<ABE>> = LazyLock::new(|| parse_abe(DEFAULT_PKT).unwrap());
pub static DEFAULT_POINT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe(DEFAULT_PKT).unwrap());
pub static DEFAULT_NETPKT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe(DEFAULT_PKT).unwrap());

pub static PYTHON_REPR_PKT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe(PYTHON_PKT).unwrap());

pub static PYTHON_PKT: &str = "todo - PYTHON_PKT";

pub static JSON_PKT: &str = "todo";


/// A static packet formatter similar to DEFAULT_PKT without using ABE expressions.
pub struct PktFmt<'o>(pub &'o dyn NetPkt);

impl<'o> core::fmt::Debug for PktFmt<'o>{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pkt = self.0;
        let ptype = pkt.as_point().point_header_ref().point_type.as_str();
        let hash = pkt.hash_ref().to_string();

        let group = match *pkt.get_group(){
            e if e == PRIVATE=> "[#:0]".into(),
            e if e == PUBLIC=> "[#:pub]".into(),
            e if e == *TEST_GROUP => "[#:test]".into(),
            e => e.to_abe_str()
        };
        let domain = pkt.get_domain().to_string();
        let path = pkt.get_path().to_string();
        let pubkey = pkt.get_pubkey().to_abe_str();
        let create = pkt.get_create_stamp().get();

        let links_len = pkt.get_links().len();
        
        f.write_fmt(format_args!("type\t{ptype}
hash\t{hash}
group\t{group}
domain\t{domain}
path\t{path}
pubkey\t{pubkey}
create\t{create}
links\t{links_len}
"))?;
        for crate::Link{ptr,tag}in pkt.get_links(){
            f.write_fmt(format_args!("\t{tag} {ptr}\n"))?;
        }
        let data = BStr::new(pkt.data());
        let data_len = pkt.data().len();
        f.write_fmt(format_args!("data\t{data_len}\n{data}"))
    }
}
impl<'o> core::fmt::Display for PktFmt<'o> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Debug::fmt(self,f)
    }
}
