// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::time::Duration;

use abe::{fncs, eval::{ScopeMacro,  EvalScopeImpl, ScopeFunc, EvalCtx,eval, ApplyResult, ScopeMacroInfo}, ABE, ast};
use anyhow::{anyhow, Context, bail };
use linkspace_pkt::{Tag, Ptr, pkt_ctx, ptrv};

use crate::{ eval::LKS };

use super::{claim::{Claim }, name::Name, GROUP_TAG, PUBKEY_TAG, public_claim::Issue};



#[derive(Debug, Clone, Copy)]
/// LNS but also tries to resolve by asking others.
pub struct NetLNS<R> {
    pub rt: R,
    pub timeout: Duration,
}
#[derive(Debug, Clone, Copy)]
pub struct PrivateLNS<R> {
    pub rt: R,
}

impl<R: LKS> PrivateLNS<R> {
    fn get_claim_link_ptr(&self, name: Name, tag: Tag) -> ApplyResult{
        let claim = self.get_claim(name)??;
        claim.links().first_eq(tag).map(ptrv).into()
    }
    fn get_claim(&self, name: Name) -> anyhow::Result<Option<Claim>>{
        super::lookup_claim(&self.rt.lk()?, &name)
    }
    fn get_by_tag(&self, tag: Tag,ptr:Ptr) -> anyhow::Result<Option<Name>>{
        Ok(super::reverse_lookup(&self.rt.lk()?, tag,ptr).into_ok()?.map(|o| o.name))
    }
    
    fn get_by_tag_abe(&self,tag:Tag,ptr:Ptr) -> ApplyResult<String> {
        let name :Name= self.get_by_tag(tag, ptr)??;
        if tag == GROUP_TAG { Ok(format!("[#:{name}]")).into()}
        else if tag == PUBKEY_TAG { Ok(format!("[@:{name}]")).into()}
        else {Err(anyhow!("bug! weird tag{tag}")).into()}
    }
}


fn name_str(inp: anyhow::Result<Option<Name>>) -> ApplyResult{
        inp?.context("cant find lns entry")?
            .to_string().into_bytes().into()
}

impl<R: LKS> NetLNS<R> {
    fn private(self) -> PrivateLNS<R>{PrivateLNS { rt: self.rt }}
    fn get_claim_link_ptr(&self, name: Name, tag: Tag) -> anyhow::Result<Vec<u8>>{
        let claim = self.get_claim(name)?;
        claim.links().first_eq(tag).map(ptrv).context("tag not set in claim")
    }
    // TODO - this and get by_tag needs to probe the lns resolver instead of doing it themselves.
    fn get_claim(&self, name: Name) -> anyhow::Result<Claim>{
        match self.private().get_claim(name.clone())?{
            Some(c) => Ok(c),
            None => {
                let mut issue :Result<(),Issue>= Ok(());
                match super::lookup_live_chain(&self.rt.lk()?, &name, &mut |i| {issue = Err(i);Ok(())})?{
                    Ok(c) => Ok(c.claim),
                    Err(_e) => bail!("couldn't find claim - last-issue: {issue:?}"), // make udp call
                }
            }
        }
    }
    fn get_by_tag(&self, tag: Tag,ptr:Ptr) -> anyhow::Result<Option<Name>>{
        match self.private().get_by_tag(tag, ptr)?{
            Some(v) => Ok(Some(v)),
            None => Ok(None)
        }
    }
    fn get_by_tag_abe(&self,tag:Tag,ptr:Ptr) -> ApplyResult<String> {
        let name :Name= self.get_by_tag(tag, ptr)??;
        if tag == GROUP_TAG { Ok(format!("[#:{name}]")).into()}
        else if tag == PUBKEY_TAG { Ok(format!("[@:{name}]")).into()}
        else {Err(anyhow!("bug! weird tag{tag}")).into()}
    }
}

impl<R: LKS > EvalScopeImpl for NetLNS<R> {
    fn about(&self) -> (String, String) {
        ("lns".into(), String::new())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            (
                "#",
                1..=7,
                Some(true),
                "(namecomp)* - get the associated lns group",
                // error on get failure
                |this: &Self, args: &[&[u8]]| this.get_claim_link_ptr(Name::from(args)?, GROUP_TAG ),
                // pass on get failure
                |this: &Self, phash: &[u8], _| this.get_by_tag_abe(GROUP_TAG, Ptr::try_fit_bytes_or_b64(phash)?)
            ),
            // error on get failure
            ("?#", 1..=1, "find by group# tag", |this:&Self, i: &[&[u8]]| name_str(this.get_by_tag(GROUP_TAG, Ptr::try_fit_bytes_or_b64(i[0])?))),
            (
                "@",
                1..=7,
                Some(true),
                "(namecomp)* - get the associated lns key",
                // error on get failure
                |this: &Self, args: &[&[u8]]| this.get_claim_link_ptr(Name::from(args)?, PUBKEY_TAG),
                // pass on get failure
                |this: &Self, phash: &[u8], _| this.get_by_tag_abe(PUBKEY_TAG, Ptr::try_fit_bytes_or_b64(phash)?)
            ),
            // error on get failure
            ("?@", 1..=1, "find by pubkey@ tag", |this:&Self, i: &[&[u8]]| name_str(this.get_by_tag(PUBKEY_TAG, Ptr::try_fit_bytes_or_b64(i[0])?)))
        ])
    }

    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[ScopeMacro {
            apply: |this, abe: &[ABE], scope| {
                let ctx = EvalCtx { scope };
                let mut it = abe.split(|v| v.is_fslash());
                let name = it.next().context("missing name")?;
                let expr = it.as_slice();
                let mut it = name.split(|v| v.is_colon());
                let _empty = it.next().context("arg delimited with ':'")?;
                ast::exact::<0>(_empty)?;
                let name : Name = eval(&ctx,it.as_slice())?.try_into()?;
                let claim = this.get_claim(name)?;
                let v: ApplyResult = eval(&pkt_ctx(ctx, &claim.pkt), expr).map(|v| v.concat()).map_err(|e| anyhow!(e.to_string())).into();
                v
            },
            info: ScopeMacroInfo {
                id: "lns",
                help: "[:comp]*/expr",
            },
        }]
    }
}

impl<R: LKS> EvalScopeImpl for PrivateLNS<R> {
    fn about(&self) -> (String, String) {
        ("private-lns".into(), "Only look at the private claims lookup tree. Makes no requests".into())
    }
    fn list_funcs(&self) -> &[linkspace_core::prelude::ScopeFunc<&Self>] {
        fncs!([
            (
                "private#",
                1..=7,
                Some(true),
                "(namecomp)* - get the associated lns group",
                |this: &Self, args: &[&[u8]]| this.get_claim_link_ptr(Name::from(args)?, GROUP_TAG ).require("no private claim set"),
                |this: &Self, phash: &[u8], _| this.get_by_tag_abe(GROUP_TAG, Ptr::try_fit_bytes_or_b64(phash)?)
            ),
            ("?private#", 1..=1, "find by group# tag",
             
             |this:&Self, i: &[&[u8]]| name_str(this.get_by_tag(GROUP_TAG, Ptr::try_fit_bytes_or_b64(i[0])?))
            ),
            (
                "private@",
                1..=7,
                Some(true),
                "(namecomp)* - get the associated lns key",
                |this: &Self, args: &[&[u8]]| this.get_claim_link_ptr(Name::from(args)?, PUBKEY_TAG).require("no private claim set"),
                |this: &Self, phash: &[u8], _| this.get_by_tag_abe(PUBKEY_TAG, Ptr::try_fit_bytes_or_b64(phash)?)
            ),
            ("?private@", 1..=1, "find by pubkey@ tag",
             |this:&Self, i: &[&[u8]]| name_str(this.get_by_tag(PUBKEY_TAG, Ptr::try_fit_bytes_or_b64(i[0])?))
            )
        ])
    }

    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[ScopeMacro {
            apply: |this, abe: &[ABE], scope| {
                let ctx = EvalCtx { scope };
                let mut it = abe.split(|v| v.is_fslash());
                let name = it.next().context("missing name")?;
                let mut expr = it.as_slice();
                if expr.is_empty() { expr = &linkspace_pkt::DEFAULT_POINT_FMT;}
                let mut it = name.split(|v| v.is_colon());
                let _empty = it.next().context("arg delimited with ':'")?;
                ast::exact::<0>(_empty)?;
                let name : Name = eval(&ctx,it.as_slice())?.try_into()?;
                let claim = this.get_claim(name)?.context("cant find claim")?;
                let v: ApplyResult = eval(&pkt_ctx(ctx, &claim.pkt), expr).map(|v| v.concat()).map_err(|e| anyhow!(e.to_string())).into();
                v
            },
            info: ScopeMacroInfo {
                id: "private-lns",
                help: "[:comp]*/expr",
            },
        }]
    }
}
