// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_core::prelude::*;
/// Various ABE eval Scope's with database access.
use std::time::Duration;

use crate::{
    protocols::lns::{local::LocalLNS, LNS},
    runtime::Linkspace,
};

pub type RTScope<GT> = (
    EScope<LNS<GT>>,
    EScope<LocalLNS<GT>>,
    (EScope<Conf<GT>>, EScope<ReadHash<GT>>),
);
pub type RTCtx<GT> = EvalCtx<(EvalStd, RTScope<GT>)>;

pub const fn rt_scope<'o, GT>(rt: GT) -> RTScope<GT>
where
    GT: 'o + Fn() -> std::io::Result<Linkspace> + Copy,
{
    let conf = EScope(Conf(rt));
    let readhash = EScope(ReadHash(rt));
    let local_lns = EScope(LocalLNS { rt });
    let lns = EScope(LNS {
        rt,
        timeout: Duration::from_secs(1),
    });
    (lns, local_lns, (conf, readhash))
}
pub fn rt_ctx<'o, GT>(
    ctx: EvalCtx<impl Scope + 'o>,
    rt: GT,
) -> EvalCtx<(impl Scope + 'o, RTScope<GT>)>
where
    GT: 'o + Fn() -> std::io::Result<Linkspace> + Copy,
{
    ctx.scope(rt_scope(rt))
}

pub const fn std_ctx_v<'o, GT>(rt: GT, version: &str) -> RTCtx<GT>
where
    GT: 'o + Fn() -> std::io::Result<Linkspace> + Copy,
{
    EvalCtx {
        scope: (linkspace_core::eval::std_ctx_v(version).scope, rt_scope(rt)),
    }
}

#[derive(Copy, Clone)]
pub struct Conf<R>(R);

impl<R: Fn() -> std::io::Result<Linkspace>> EvalScopeImpl for Conf<R> {
    fn about(&self) -> (String, String) {
        (
            "config".into(),
            format!(
                "read files from {:?} ",
                (self.0)().map(|v| v.env().location().to_owned())
            ),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |this: &Self, inp: &[&[u8]], _, _scope| {
                let p = std::str::from_utf8(inp[0]).map_err(|_| "exptected utf8")?;
                Ok((this.0)()?.env().conf_data(p)?).into()
            },
            info: ScopeFuncInfo {
                id: "conf",
                init_eq: None,
                argc: 1..=1,
                to_abe: false,
                help: "read a file from the conf directory",
            },
            to_abe: none,
        }]
    }
}

#[derive(Copy, Clone)]
pub struct ReadHash<GT>(GT);
impl<R: Fn() -> std::io::Result<Linkspace>> ReadHash<R> {
    // TODO this allows access to private
    fn read_dgpk(&self, inp: &[&[u8]], scope: &dyn Scope) -> Result<Vec<u8>, ApplyErr> {
        let domain = Domain::try_fit_byte_slice(inp[0])?;
        let group = inp
            .get(1)
            .map(|v| GroupID::try_fit_bytes_or_b64(v))
            .transpose()?
            .unwrap_or(PUBLIC);
        if group == PRIVATE {
            // TODO replace with ctx.options
            if !std::env::var("LK_PRIVATE")?.parse::<bool>()? {
                return Err(
                    "prevent reading [#:0] group or set LK_PRIVATE=true to enable ( dangerous if you're evaluating external abe )"
                        .into());
            }
        }
        let path = SPath::from_slice(inp.get(2).copied().unwrap_or(&[]))?.try_idx()?;
        let key = inp
            .get(3)
            .map(|v| PubKey::try_fit_bytes_or_b64(v))
            .transpose()?
            .unwrap_or(B64([0; 32]));
        let reader = (self.0)()?.get_reader();

        let predicates = Query::dgpk(domain, group, path, key).predicates;
        let pkt = reader
            .query_tree(query_mode::Order::Desc, &predicates)
            .next()
            .ok_or_else(|| "no matching packet")?;
        let id = inp.get(4).copied().unwrap_or(b"pkt");
        let args = inp.get(5..).unwrap_or(&[]);
        let r = pkt_scope(&*pkt).lookup_apply(id, args, true, scope);
        drop(pkt);
        drop(reader);
        match r.into_opt() {
            Some(o) => o,
            None => Err(EvalError::NoSuchFunc(id.to_vec()).into()),
        }
    }
}
impl<R: Fn() -> std::io::Result<Linkspace>> EvalScopeImpl for ReadHash<R> {
    fn about(&self) -> (String, String) {
        ("database".into(),
         "get packets from the local db.
e-funcs evaluate their args as if in pkt scope.
funcs evaluate as if [/[func + args]:[rest]]. (e.g. [/readhash:HASH:[group:str]] == [readhash:..:group:str])".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[
            ScopeFunc {
                apply : |this:&Self,inp:&[&[u8]],_,scope|{
                    let hash = B64::try_fit_slice(inp[0])?;
                    let reader = (this.0)()?.get_reader();
                    let pkt = reader.read(&hash)?.ok_or_else(||format!("could not find pkt {}",hash))?;
                    let (id, args) = inp[1..].split_first().unwrap_or((&{b"pkt" as &[u8]},&[]));
                    let r = pkt_scope(&*pkt).lookup_apply(id, args, true,scope);
                    drop(pkt); drop(reader);
                    r
                },
                info: ScopeFuncInfo {
                    id:  "readhash", init_eq: None, argc: 1..=16,to_abe:false,
                    help:"open a pkt by hash and use tail args as if calling in a netpkt scope"
                },
                to_abe:none
            },
            ScopeFunc {
                apply : |this:&Self,inp:&[&[u8]],_,scope|{
                    this.read_dgpk(inp, scope).into()
                },
                info: ScopeFuncInfo {
                    id:  "read", init_eq: None, argc: 2..=16,to_abe:false,
                    help:"read but accesses open a pkt by dgpk path and apply args. e.g. [read:mydomain:[#:pub]:[//a/path]:[@:me]::data:str], prefer eval ctx"
                },
                to_abe:none
            },
        ]
    }
    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[ScopeEval {
            apply: |this, abe: &[ABE], scope| {
                let ctx = EvalCtx { scope };
                let mut it = abe.split(|v| v.is_colon());
                let _empty = it.next().ok_or("arg delimited with ':'")?;
                ast::exact::<0>(_empty)?;
                let hash = it.next().ok_or("missing hash")?;
                let expr = it.next().ok_or("missing expr")?;
                let alt = it.next();
                let hash = eval(&ctx, hash)?.concat();
                let hash: Ptr = Ptr::try_fit_slice(&hash)?;
                let reader = (this.0)()?.get_reader();
                match reader.read(&hash)? {
                    None => {
                        let alt = alt.ok_or_else(|| format!("could not find pkt {}", hash))?;
                        it.next().ok_or("to many args?")?;
                        let r = eval(&ctx, alt)?.concat();
                        ApplyResult::Ok(r)
                    }
                    Some(pkt) => {
                        let r = eval(&pkt_ctx(ctx, &pkt), expr)?.concat();
                        //drop(pkt); drop(reader);
                        ApplyResult::Ok(r)
                    }
                }
            },
            info: ScopeEvalInfo {
                id: "readhash",
                help: "HASH ':' expr (':' alt if not found) ",
            },
        }]
    }
}
