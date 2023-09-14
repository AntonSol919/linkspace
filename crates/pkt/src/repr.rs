// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{sync::LazyLock, fmt::{Formatter, Result, self, Display}};

use bstr::{BStr };
use byte_fmt::{abe::{ABE, ToABE, ast::parse_abe_strict_b}, B64, AB};

use crate::{NetPkt, PointExt, Point, PRIVATE, PUBLIC, TEST_GROUP };

/// default fmt in many cases and output for `[pkt]`
// Could be made shorter by using parse_abe_with_unencoded_b
pub static DEFAULT_PKT: &str = "\
type\\t[type:str]\\n\
hash\\t[hash:str]\\n\
group\\t[/~?:[group]/#/b]\\n\
domain\\t[domain:str]\\n\
spacename\\t[spacename:str]\\n\
pubkey\\t[/~?:[pubkey]/@/b]\\n\
create\\t[create:str]\\n\
links\\t[links_len:str]\\n\
[/links:\\t[tag:str] [ptr:str]\\n]\\n\
data\\t[data_size:str]\\n\
[data/~utf8]\\n\
";


pub static DEFAULT_FMT: LazyLock<Vec<ABE>> = LazyLock::new(|| parse_abe_strict_b(DEFAULT_PKT.as_bytes()).unwrap());
pub static DEFAULT_POINT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe_strict_b(DEFAULT_PKT.as_bytes()).unwrap());
pub static DEFAULT_NETPKT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe_strict_b(DEFAULT_PKT.as_bytes()).unwrap());

pub static PYTHON_REPR_PKT_FMT: LazyLock<Vec<ABE>> =
    LazyLock::new(|| parse_abe_strict_b(PYTHON_PKT.as_bytes()).unwrap());

pub static PYTHON_PKT: &str = "todo - PYTHON_PKT";

pub static JSON_PKT: &str = "todo";


/// A static packet formatter similar to DEFAULT_PKT without using ABE expressions.
pub struct PktFmt<'o>(pub &'o dyn NetPkt);


impl<'o> core::fmt::Debug for PktFmt<'o>{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.to_str(f,false,usize::MAX)
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
    let ccode = bytes.0[14] >> 4;
    let dcode = bytes.0[15] >> 4;
    write!(f,"<span lk-{class}='{b64}' class='lk-b64 lk-c{ccode} lk-d{dcode}'>{}</span>",
            if use_mini{bytes.b64_mini()} else {group_abe})
}
impl<'o> PktFmt<'o>{
    pub fn to_str<F: fmt::Write>(&self, f: &mut F,add_recv:bool,data_limit:usize) -> Result{
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
        let space = pkt.get_spacename().to_string();
        let pubkey = pkt.get_pubkey().to_abe_str();
        let create = pkt.get_create_stamp().get();
    
        let links_len = pkt.get_links().len();
        if add_recv {
            match pkt.recv(){
                Some(r) => write!(f,"recv\t{}\n",r),
                None => write!(f,"recv\t???\n")
            }?;
        }
        write!(f,"type\t{ptype}
hash\t{hash}
group\t{group}
domain\t{domain}
space\t{space}
pubkey\t{pubkey}
create\t{create}
links\t{links_len}
")?;
        for crate::Link{ptr,tag}in pkt.get_links(){
            write!(f,"\t{} {ptr}\n",tag.as_str(true))?;
        }
    let data = pkt.data();
    let data = &data[0..data.len().min(data_limit)];
    
        let data = BStr::new(data);
        let data_size = pkt.data().len();
        write!(f,"data\t{data_size}\n{data}")

    }

    /** create a html fragment describing the packet

    The format is a bit arbitrary. Its designed to be safe to paste into an html document and usable for most usecases. But it is verbose and somewhat opinionated.
    If it doesn't fit your usecase, just build your own template.
    Note: the lk-c[0..31] and lk-d[0..31] class's are derived from the hash and should be used for color coding when appropriate.
    */
    pub fn to_html<F: fmt::Write>(&self, f: &mut F,
                   write_escaped_lossy_data: bool,
                   include: Option<&mut dyn FnMut(&dyn NetPkt, &mut F) -> Result>
    ) -> Result{

        let pkt = self.0;

        let hash = pkt.hash_ref().to_string();
        let code = pkt.hash_ref().0[0] >> 4;

        let data = pkt.data();
        let size = data.len();

        let links_len = pkt.get_links().len();
        let point = pkt.as_point();
        let ptype = point.point_header_ref().point_type.bits();
        let depth = pkt.get_depth();
        let with_pubkey = pkt.pubkey().map(|e| format!("lk-pubkey='{e}'")).unwrap_or(String::new());
        write!(f,"<div lk-point='{hash}' lk-ptype='{ptype}' class='lk-c{code}' {with_pubkey}
lk-data-size='{size}' lk-links-len='{links_len}' lk-depth='{depth}'>")?;
        fmt_b64(pkt.hash_ref(), "hash", f)?;

        if let Some(lh) = point.linkpoint_header(){
            if let Some(kp) = point.signed(){
                fmt_b64(&kp.pubkey, "pubkey", f)?;
            }
            let domain64 = B64(lh.domain.0);
            let domain = EscapeHTML(lh.domain.as_str(true));
            write!(f,"<span lk-domain='{domain64}'>{domain}</span>")?;
            fmt_b64(&lh.group, "group", f)?;

            let create = pkt.get_create_stamp();
            write!(f,"<span lk-create='{create}'>{create}</span>")?;

            let space = pkt.get_spacename();
            write!(f,"<span lk-depth='{depth}' >{depth}</span>")?;
            let spaceb = B64(space.space_bytes());
            writeln!(f,"<ol lk-space='{spaceb}' lk-depth='{depth}'>")?;
            for (i,p) in space.iter().enumerate(){
                let pb64= B64(p);
                let spacec = EscapeHTML( AB(p));
                write!(f,"<li lk-space{i}='{pb64}'>{spacec}</li>")?;
            }
            writeln!(f,"</ol>")?;

            let links_len = pkt.get_links().len();
            write!(f,"<span lk-links-len='{links_len}'>{links_len}</span>")?;

            writeln!(f,"<ol lk-links='{links_len}'>")?;
            for crate::Link{ptr,tag}in pkt.get_links(){
                let tagb64 = B64(tag.0); // 
                let tag = EscapeHTML(tag.as_str(true));
                write!(f,"<li lk-link-tag='{tagb64}'><span lk-tag='{tagb64}'>{tag}</span>")?;
                fmt_b64(&ptr,"ptr",f)?;
                writeln!(f,"</li>")?;
            }
            write!(f,"</ol>")?;
        }

        writeln!(f,"<span lk-data-size='{size}'>{size}</span>")?;
        if write_escaped_lossy_data {
            let data = EscapeHTML(BStr::new(data));
            write!(f,"<pre lk-data='{size}'>{data}</pre>")?;
        }
        if let Some(incf) = include {
            incf(self.0,f)?;
        }
        write!(f,"</div>")
    }
}

struct EscapeHTML<X>(X);
impl<X:Display>  Display for EscapeHTML<X>{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for ch in self.0.to_string().chars(){
            match ch{
                '&' =>  f.write_str("&amp")?,
                '<' =>  f.write_str("&lt")?,
                '>' =>  f.write_str("&gt")?,
                o => write!(f,"{o}")?
            }
        }
        Ok(())
    }
}

// quick hack 
pub struct PktFmtDebug<'o>(pub &'o dyn NetPkt);
impl<'o> core::fmt::Display for PktFmtDebug<'o> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        PktFmt(self.0).to_str(f,true,160)
    }
}
