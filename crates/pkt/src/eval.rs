// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
use anyhow::{anyhow, bail, Context};
use byte_fmt::abe::ast::Ctr;
use byte_fmt::abe::scope::core_ctx;
use byte_fmt::abe::{eval::*, ToABE, ABE};
use byte_fmt::abe::{scope_macro, fncs};

pub fn pkt_scope(pkt: &dyn NetPkt) -> impl Scope + '_ {
    let pkt_env = EScope(NetPktFieldsEval(pkt));
    let pkt_def = EScope(NetPktPrintDefault(pkt));
    let link_select = EScope(SelectLink(pkt.as_point().get_links()));
    let recv = EScope(RecvStamp { pkt });
    (pkt_env, pkt_def, (link_select, recv))
}

pub fn pkt_ctx<'o>(ctx: EvalCtx<impl Scope + 'o>, pkt: &'o dyn NetPkt) -> EvalCtx<impl Scope + 'o> {
    ctx.scope(pkt_scope(pkt))
}
pub fn opt_pkt_ctx<'o>(
    ctx: EvalCtx<impl Scope + 'o>,
    pkt: Option<&'o dyn NetPkt>,
) -> EvalCtx<impl Scope + 'o> {
    let pkt_scope = pkt.map(pkt_scope);
    ctx.scope(pkt_scope)
}

#[track_caller]
pub fn pkt_fmt(pkt: &dyn NetPkt) -> String {
    let ctx = pkt_ctx(core_ctx(), pkt);
    String::from_utf8(abe::eval::eval(&ctx, &DEFAULT_FMT).unwrap().concat()).unwrap()
}
#[track_caller]
pub fn netpkt_fmt(pkt: &dyn NetPkt) -> String {
    let ctx = pkt_ctx(core_ctx(), pkt);
    String::from_utf8(abe::eval::eval(&ctx, &DEFAULT_NETPKT_FMT).unwrap().concat()).unwrap()
}
#[track_caller]
pub fn point_fmt(pkt: &dyn NetPkt) -> String {
    let ctx = pkt_ctx(core_ctx(), pkt);
    String::from_utf8(abe::eval::eval(&ctx, &DEFAULT_POINT_FMT).unwrap().concat()).unwrap()
}
macro_rules! as_scopefn {
    ($el:tt , $name:expr) => {
        ScopeFunc {
            apply: |pkt, i: &[&[u8]], _, _| {
                let field = $el::ENUM;
                let mut out = vec![];
                match i {
                    &[b"abe"] => field.abe(&pkt.0, &mut out)?,
                    &[b"str"] => field.display(&pkt.0, &mut out)?,
                    &[] | &[b""] | &[b"bytes"] => field.bytes(&pkt.0, &mut out)?,
                    _ => return ApplyResult::Err(anyhow!("expect ?(str|abe)")),
                }
                Ok(out).into()
            },
            to_abe: abe::eval::none,
            info: ScopeFuncInfo {
                id: $name,
                init_eq: Some(true),
                argc: 0..=1,
                help: concat!("?(str|abe) - netpkt.", $name ),
                to_abe: false,
            },
        }
    };
}

pub struct NetPktFieldsEval<'o>(&'o dyn NetPkt);
impl<'o> EvalScopeImpl for NetPktFieldsEval<'o> {
    fn about(&self) -> (String, String) {
        (
            "netpkt field".into(),
            r#"get a field of a netpkt. also used in watch predicates."#.into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &fixed_fields_arr!(as_scopefn)
    }
}
pub struct NetPktPrintDefault<'o>(&'o dyn NetPkt);
impl<'o> EvalScopeImpl for NetPktPrintDefault<'o> {
    fn about(&self) -> (String, String) {
        ("print pkt default".into(), String::new())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( @C "pkt",0..=0,Some(true),"default pk fmt",|pkt:&Self,_,_,scope| {
                let ctx = EvalCtx{scope}.scope(pkt_scope(pkt.0));
                Ok(abe::eval::eval(&ctx,&DEFAULT_FMT).unwrap().concat())
            }, none),
            ( @C "netpkt",0..=0,Some(true),"TODO default netpkt fmt",|pkt:&Self,_,_,scope| {
                let ctx = EvalCtx{scope}.pre_scope(pkt_scope(pkt.0));
                Ok(abe::eval::eval(&ctx,&DEFAULT_FMT).unwrap().concat())
            }, none),
            ( @C "point",0..=0,Some(true),"TODO default point fmt",|pkt:&Self,_,_,scope| {
                let ctx = EvalCtx{scope}.pre_scope(pkt_scope(pkt.0));
                Ok(abe::eval::eval(&ctx,&DEFAULT_FMT).unwrap().concat())
            }, none),
            ( @C "pkt-quick",0..=2,Some(true),"[add recv? =false , data_limit = max] same as pkt but without dynamic lookup",|pkt:&Self,arg:&[&[u8]],_,_| {
                let mut buf = String::new();
                let add_recv_field = !matches!(arg.get(0).copied(), None | Some(b"false") | Some(b""));
                let data_limit = arg.get(1).map(|o| Ok::<usize,anyhow::Error>(std::str::from_utf8(o)?.parse()?)).transpose()?.unwrap_or(usize::MAX);
                PktFmt(pkt.0).to_str(&mut buf,add_recv_field,data_limit)?;
                Ok(buf.into_bytes())
            }, none),


            ( @C "html-quick",0..=0,Some(true),"same as html but without dynamic lookup",|pkt:&Self,_,_,_| {
                let mut buf = String::new();
                PktFmt(pkt.0).to_html(&mut buf,true,None)?;
                Ok(buf.into_bytes())
            }, none),

            ( "netbytes", 0..=0, Some(true),"raw netpkt bytes",|pkt:&Self,_| Ok(pkt.0.byte_segments().to_bytes().into_vec()))
        ])
    }
}

pub fn lptr(l:&Link)->&LkHash{&l.ptr}
pub fn ptrv(l:&Link)->Vec<u8>{l.ptr.to_vec()}
#[derive(Copy,Clone)]
pub struct SelectLink<'o>(pub &'o [Link]);
// TODO . this should be done with ExtendedTestOp 
impl<'o> SelectLink<'o> {
    pub fn first_eq(self,tag:Tag) -> Option<&'o Link>{
        self.0.iter().find(|l| l.tag == tag)
    }
    pub fn first_tailmask(self,tail:&[u8]) -> Option<&'o Link>{
        if tail.len() > 16 { return None};
        self.0.iter().find(|l| l.tag.ends_with(tail))
    }
}

impl<'o> EvalScopeImpl for SelectLink<'o> {
    fn about(&self) -> (String, String) {
        ("select link".into(), String::new())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([(
            "*=",
            1..=1,
            "[suffix] get first link with tag ending in suffix",
            |links: &Self, i: &[&[u8]]| {
                links.first_tailmask(i[0]).map(ptrv).context("no such link")
            }
        )])
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[scope_macro!(
            "links",
            ":{EXPR} where expr is repeated for each link binding 'ptr' and 'tag'",
            |links: &Self, abe: &[ABE], scope| {
                let abe = match &abe {
                    &[ABE::Ctr(Ctr::Colon), r @ ..] => r,
                    _ => anyhow::bail!("links expects")
                };
                let mut out = vec![];
                for link in links.0 {
                    let ctx = EvalCtx { scope }.scope(EScope(LinkEnv { link }));
                    match eval(&ctx, abe) {
                        Ok(ablist) => match ablist.into_exact_bytes() {
                            Ok(o) => {
                                out.extend_from_slice(&o);
                            }
                            Err(_e) => bail!("links expects result to be undelimited bytes (fixme)")
                        },
                        Err(e) => return Err(e.into()),
                    }
                }
                Ok(out)
            }
        )]
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LinkEnv<'o> {
    link: &'o Link,
}
impl<'o> EvalScopeImpl for LinkEnv<'o> {
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            (
                "ptr",
                0..=1,
                "[?(str|abe)] - 32 byte pointer",
                |lk: &Self, i: &[&[u8]]| {
                    match i.get(0).copied().unwrap_or(b"") {
                        b"abe" => Ok(lk.link.ptr.to_abe_str().into_bytes()),
                        b"str" => Ok(lk.link.ptr.to_string().into_bytes()),
                        b"" => Ok(lk.link.ptr.0.to_vec()),
                        _ => bail!("unexpected fmt expect ?(str|abe)"),
                    }
                }
            ),
            (
                "tag",
                0..=1,
                "[?(str|abe)] - 16 byte tag ",
                |lk: &Self, i: &[&[u8]]| {
                    match i.get(0).copied().unwrap_or(b"") {
                        b"abe" => Ok(lk.link.tag.to_abe_str().into_bytes()),
                        b"str" => Ok(lk.link.tag.as_str(true).into_owned().into_bytes()),
                        b"" => Ok(lk.link.tag.0.to_vec()),
                        _ => bail!("unexpected fmt expect ?(str|abe)"),
                    }
                }
            )
        ])
    }
}

fn eval_recv(b: Stamp, args:&[&[u8]]) -> anyhow::Result<Vec<u8>>{
    let ok = match args.get(0).copied().unwrap_or(b"") {
        b"abe" => b.to_abe_str().into_bytes(),
        b"str" => b.to_string().into_bytes(),
        b"" => b.0.to_vec(),
        _ => bail!("unexpected fmt expect ?(str|abe)"),
    };
    Ok(ok)
}


#[derive(Copy, Clone, Debug)]
pub struct RecvStamp<'o> {
    pkt: &'o dyn NetPkt,
}
impl<'o> EvalScopeImpl for RecvStamp<'o> {
    fn about(&self) -> (String, String) {
        (
            "recv".into(),
            "recv stamp for packet. value depends on the context".into(),
        )
    }

    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            (
                "recv",
                0..=1,
                Some(true),
                "recv stamp - returns now if unavailable in context",
                |t: &Self, args: &[&[u8]]| eval_recv(t.pkt.get_recv(),args)
            ),
            (
                "recv_now",
                0..=1,
                Some(true),
                "recv stamp - returns an error if not available in context",
                |t: &Self, args: &[&[u8]]|
                eval_recv(
                    t.pkt.recv().context("no recv available in context")?,
                    args
                )
            )
        ])
    }
}

#[test]
fn pktfmt() {
    let pkt = datapoint(b"hello", ());
    let ctx = core_ctx();
    let ctx = pkt_ctx(ctx, &pkt);
    let abe = abe::parse_abe("[pkt] [data]").unwrap();
    let st = eval(&ctx,&abe).unwrap().concat();
    let _v = std::str::from_utf8(&st).unwrap();
}
