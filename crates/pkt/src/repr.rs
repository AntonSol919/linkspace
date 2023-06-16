// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{sync::LazyLock, fmt::{Formatter, Result, self}};

use bstr::BStr;
use byte_fmt::{abe::{parse_abe, ABE, ToABE}, B64};

use crate::{NetPkt, PointExt, Point, PRIVATE, PUBLIC, TEST_GROUP };
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
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let pkt = self.0;
        let ptype = pkt.as_point().point_header_ref().point_type.as_str();
        let hash = pkt.hash_ref().to_string();

        let group = match *pkt.get_group(){
            e if e == PRIVATE=> "[#:0]".into(),
            e if e == PUBLIC=> "[#:pub]".into(),
            e if e == *TEST_GROUP => "[#:test]".into(),
            e => e.to_abe_str()
        };
        let domain = pkt.get_domain().as_str(true);
        let path = pkt.get_path().to_string();
        let pubkey = pkt.get_pubkey().to_abe_str();
        let create = pkt.get_create_stamp().get();

        let links_len = pkt.get_links().len();
        
        write!(f,"type\t{ptype}
hash\t{hash}
group\t{group}
domain\t{domain}
path\t{path}
pubkey\t{pubkey}
create\t{create}
links\t{links_len}
")?;
        for crate::Link{ptr,tag}in pkt.get_links(){
            write!(f,"\t{} {ptr}\n",tag.as_str(true))?;
        }
        let data = BStr::new(pkt.data());
        let data_size = pkt.data().len();
        write!(f,"data\t{data_size}\n{data}")
    }
}
impl<'o> core::fmt::Display for PktFmt<'o> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        core::fmt::Debug::fmt(self,f)
    }
}
pub fn fmt_b64(bytes:&B64, class:&'static str , f:&mut impl fmt::Write) -> Result{
    let b64 = bytes.b64();
    let mut use_mini = false;
    let group_abe = match *bytes{
        e if e == PRIVATE=> "[#:0]".into(),
        e if e == PUBLIC=> "[#:pub]".into(),
        e if e == *TEST_GROUP => "[#:test]".into(),
        e => {use_mini = true; e.to_abe_str()}
    };
    let code = bytes.0[0] >> 4;
    write!(f,"<div class=\"{class} lk-c{code}\" lk-b64=\"{b64}\">{}</div> ",
            if use_mini{bytes.b64_mini()} else {group_abe})
 
}
impl<'o> PktFmt<'o>{
    pub fn to_html<F: fmt::Write>(&self, f: &mut F,
                   data_el: impl FnOnce(&dyn NetPkt, &mut F) -> Result
    ) -> Result{

        let pkt = self.0;

        let hash = pkt.hash_ref().to_string();
        let code = pkt.hash_ref().0[0] >> 4;

        let point = pkt.as_point();
        let ptype = point.point_header_ref().point_type.as_str();

        write!(f,"<div lk-point=\"{hash}\" class=\"lk-point {ptype} lk-c{code}\">")?;
        fmt_b64(pkt.hash_ref(), "lk-hash", f)?;

        if let Some(lh) = point.linkpoint_header(){
            if let Some(kp) = point.keypoint_header(){
                fmt_b64(&kp.signed.pubkey, "lk-pubkey", f)?;
            }
            fmt_b64(&lh.group, "lk-group", f)?;
            write!(f,"<div class=\"lk-domain\">{}</div>",lh.domain.as_str(true))?;
            
            let _path = pkt.get_path().to_string();
            let _create = pkt.get_create_stamp().get();

            let links_len = pkt.get_links().len();
            write!(f,"<div class=\"lk-links-len\">{links_len}</div>")?;

            write!(f,"<ol class=\"lk-links\">")?;
            for crate::Link{ptr,tag}in pkt.get_links(){
                write!(f,"<li><div class=\"lk-tag\">{}</div>",tag.as_str(true))?;
                fmt_b64(&ptr,"lk-ptr",f)?;
                write!(f,"</li>")?;
            }
            write!(f,"</ol>")?;
        }

        let data = pkt.data();
        write!(f,"<div class=\"lk-data-size\">{}</div>",data.len())?;
        data_el(self.0,f)?;
        write!(f,"</dvi>")
    }
}
