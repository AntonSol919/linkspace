// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{ffi::OsStr, os::unix::prelude::OsStrExt};

use anyhow::Context;

use linkspace_core::prelude::*;
/// Various ABE eval Scope's with database access.

use crate::{
    runtime::Linkspace, protocols::lns::eval::{NetLNS, PrivateLNS}, 
};
pub trait LKS  where Self: Fn() -> anyhow::Result<Linkspace> + Sized + Copy{
    fn lk(self) -> anyhow::Result<Linkspace>{ (self)()}
}
impl<F> LKS for F where F: Fn() -> anyhow::Result<Linkspace>+Copy{
    fn lk(self) -> anyhow::Result<Linkspace> {
        (self)()
    }
}

pub type RTScope<GT> = (
    EScope<NetLNS<GT>>,
    EScope<PrivateLNS<GT>>,
    (EScope<FileEnv<GT>>, EScope<ReadHash<GT>>,Option<EScope<OSEnv>>),
);
pub type RTCtx<GT> = EvalCtx<(EvalStd, RTScope<GT>)>;

pub const fn rt_scope<'o, GT>(rt: GT,enable_env:bool) -> RTScope<GT>
where
    GT: 'o + LKS
{
    let files= EScope(FileEnv(rt));
    let readhash = EScope(ReadHash(rt));
    let local_lns = EScope(PrivateLNS { rt });
    let env = if enable_env{ Some(EScope(OSEnv))} else {None};
    let lns = EScope(NetLNS {
        rt,
        timeout: std::time::Duration::from_secs(1),
    });
    (lns, local_lns, (files, readhash,env))
}
pub fn rt_ctx<'o, GT>(
    ctx: EvalCtx<impl Scope + 'o>,
    rt: GT,
    enable_env:bool
) -> EvalCtx<(impl Scope + 'o, RTScope<GT>)>
where
    GT: 'o + LKS
{
    ctx.scope(rt_scope(rt,enable_env))
}

pub const fn std_ctx_v<'o, GT>(rt: GT, version: &str,enable_env:bool) -> RTCtx<GT>
where
    GT: 'o + LKS
{
    EvalCtx {
        scope : (linkspace_core::eval::std_ctx_v(version).scope, rt_scope(rt,enable_env)),
    }
}

#[derive(Copy, Clone)]
pub struct FileEnv<R>(R);

impl<R: LKS> EvalScopeImpl for FileEnv<R> {
    fn about(&self) -> (String, String) {
        (
            "filesystem env".into(),
            format!(
                "read files from {:?}/files ",
                self.0.lk().map(|v| v.env().dir().to_owned())
            ),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |this: &Self, inp: &[&[u8]], _, _scope| {
                let p = std::str::from_utf8(inp[0])?;
                Ok(this.0.lk()?.env().files_data(p,true)?.unwrap()).into()
            },
            info: ScopeFuncInfo {
                id: "files",
                init_eq: None,
                argc: 1..=1,
                to_abe: false,
                help: "read a file from the LK_DIR/files directory",
            },
            to_abe: none,
        }]
    }
}

#[derive(Copy, Clone)]
pub struct ReadHash<GT>(GT);
impl<R:LKS> ReadHash<R> {
    fn read_dgpk(&self, inp: &[&[u8]], scope: &dyn Scope) -> Result<Vec<u8>, ApplyErr> {
        let domain = Domain::try_fit_byte_slice(inp[0])?;
        let group = inp
            .get(1)
            .map(|v| GroupID::try_fit_bytes_or_b64(v))
            .transpose()?
            .unwrap_or(PUBLIC);
        if group == PRIVATE {
            // TODO decide on thread model
            if !std::env::var("LK_PRIVATE")?.parse::<bool>()? {
                return Err(anyhow::anyhow!(
                    "prevent reading [#:0] group or set LK_PRIVATE=true to enable ( dangerous if you're evaluating external abe )"
                ));
            }
        }
        let path = SPath::from_slice(inp.get(2).copied().unwrap_or(&[]))?.try_ipath()?;
        let key = inp
            .get(3)
            .map(|v| PubKey::try_fit_bytes_or_b64(v))
            .transpose()?
            .unwrap_or(B64([0; 32]));
        let reader =self.0.lk()?.get_reader();

        let predicates = Query::dgpk(domain, group, path, key).predicates;
        let pkt = reader
            .query_tree(query_mode::Order::Desc, &predicates)
            .next()
            .context("no matching packet")?;
        let id = inp.get(4).copied().unwrap_or(b"pkt");
        let args = inp.get(5..).unwrap_or(&[]);
        let r = pkt_scope(&*pkt).lookup_apply(id, args, true, scope);
        drop(reader);
        match r.into_opt() {
            Some(o) => o,
            None => Err(EvalError::NoSuchFunc(id.to_vec()).into()),
        }
    }
}
impl<R: LKS> EvalScopeImpl for ReadHash<R> {
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
                    let reader = this.0.lk()?.get_reader();
                    let pkt = reader.read(&hash)?.with_context(||format!("could not find pkt {}",hash))?;
                    let (id, args) = inp[1..].split_first().unwrap_or((&{b"pkt" as &[u8]},&[]));
                    let r = pkt_scope(&*pkt).lookup_apply(id, args, true,scope);
                    drop(reader);
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
                    help:"read but accesses open a pkt by dgpk path and apply args. e.g. [read:mydomain:[#:pub]:[//a/path]:[@:me]::data:str] - does not use default group/domain - prefer eval ctx"
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
                let _empty = it.next().context("arg delimited with ':'")?;
                ast::exact::<0>(_empty)?;
                let hash = it.next().context("missing hash")?;
                let expr = it.next().context("missing expr")?;
                let alt = it.next();
                let hash = eval(&ctx, hash)?.concat();
                let hash: Ptr = Ptr::try_fit_slice(&hash)?;
                let reader = this.0.lk()?.get_reader();
                match reader.read(&hash)? {
                    None => {
                        let alt = alt.with_context(|| format!("could not find pkt {}", hash))?;
                        it.next().context("to many args?")?;
                        let r = eval(&ctx, alt)?.concat();
                        ApplyResult::Value(r)
                    }
                    Some(pkt) => {
                        let r = eval(&pkt_ctx(ctx, &pkt), expr)?.concat();
                        //drop(pkt); drop(reader);
                        ApplyResult::Value(r)
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

#[derive(Copy, Clone)]
pub struct OSEnv;

impl EvalScopeImpl for OSEnv{
    fn about(&self) -> (String, String) {
        (
            "env".into(),
            "os environment variables".into()
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |_this: &Self, inp: &[&[u8]], _, _scope| {
                let st : &OsStr = OsStr::from_bytes(inp[0]); // TODO this might be wrong
                std::env::var_os(st)
                    .with_context(|| format!("{st:?} env variable not set"))
                    .map(|o| o.as_bytes().to_vec())
                    .into()
            },
            info: ScopeFuncInfo {
                id: "env",
                init_eq: None,
                argc: 1..=1,
                to_abe: false,
                help: "read the raw OS environment variables as bytes",
            },
            to_abe: none,
        }]
    }
}
