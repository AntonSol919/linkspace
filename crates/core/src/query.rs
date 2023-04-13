// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{fmt::Display, ops::ControlFlow};

use anyhow::ensure;
use linkspace_pkt::{
    abe::{
        self,
        ast::Ctr,
        eval::{eval, ABItem, ABList, EvalCtx, EvalScopeImpl, Scope, ScopeFunc},
        fncs,
    },
    Domain, GroupID, IPathBuf, PubKey, AB,
};
use tracing::debug_span;

use crate::{
    env::query_mode::Mode,
    matcher::WatchEntry,
    prelude::{PktPredicates, TestOp},
};

impl From<PktPredicates> for Query {
    fn from(val: PktPredicates) -> Self {
        Query {
            predicates: val,
            options: vec![],
        }
    }
}
/**
See the (guide#query)[./guide/index.html].
A set of predicates and options.
Create with linkspace::lk_query, extend with lk_query_append, and stringify with lk_query_str
Argument to lk_get and lk_watch.
**/
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub predicates: PktPredicates,
    pub options: Vec<ABList>,
}
impl Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for o in &self.options {
            writeln!(f, "{o}")?;
        }
        self.predicates.fmt(f)?;
        Ok(())
    }
}

#[derive(Copy, Clone, parse_display::Display, parse_display::FromStr)]
#[display(style = "kebab-case")]
pub enum KnownOptions {
    /// which index to walk when reading from the database
    Mode,
    /// The arg is the ID under which to operate. Can be overwritten or closed. Is required for lk_watch but not for lk_get*.
    Id,
    /// try and also return the linked packets.
    Follow,
    /// (not supported by lk_watch) - append the request on finish - ignores the first callback Break to deliver the request on dropping
    NotifyClose,
}
impl KnownOptions {
    //todo make static
    pub fn as_bytes(self) -> Vec<u8>{
        self.to_string().into_bytes()
    }
    pub fn iter_all() -> impl Iterator<Item = Self> {
        use KnownOptions::*;
        [Mode, Id, Follow, NotifyClose ].into_iter()
    }
}

impl Query {
    pub fn to_str(&self, canonical: bool) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for opt in &self.options {
            writeln!(out, "{opt}").unwrap();
        }
        for p in self.predicates.iter() {
            writeln!(out, "{}", p.to_str(canonical)).unwrap();
        }
        out
    }
    pub fn add_option(&mut self, name: &str, values: &[&[u8]]) {
        let name = [
            ABItem::Ctr(Ctr::Colon),
            ABItem::Bytes(name.as_bytes().to_vec()),
        ];
        let vals = values
            .iter()
            .flat_map(|v| [ABItem::Ctr(Ctr::Colon), ABItem::Bytes(v.to_vec())]);
        let abl = name.into_iter().chain(vals).collect();
        self.options.push(abl);
    }

    pub fn add_option_abl(&mut self, opt: ABList) -> anyhow::Result<()> {
        ensure!(
            opt.lst[0].0.is_empty() ,
            "options start with ':' (or '/' to clear) got {opt:?}"
        );
        self.options.push(opt);
        Ok(())
    }
    pub fn get_known_opt(&self, opt: KnownOptions) -> Option<&ABList> {
        self.get_option(opt.to_string().as_bytes())
    }
    /// get an option. i.e. the last statement starting with `:XXX` and return the entire statement
    pub fn get_option(&self, name: &[u8]) -> Option<&ABList> {
        self.options.iter().rev().find(|a| a.lst[1].0 == name).filter(|v| v.lst[0].1.unwrap() == Ctr::Colon)
    }
    /// If the option has 0 or 1 arguments this will return the arg.
    pub fn get_option_bytes(&self, name: &[u8]) -> Option<anyhow::Result<&[u8]>> {
        self.get_option(name).map(|abl| {
            ensure!(
                abl.lst.len() < 4 && abl.lst[1].1 != Some(Ctr::FSlash),
                "Bad options argument expected at most 1 arg, got {abl}"
            );
            Ok(abl.lst.get(2).map(|v| v.0.as_slice()).unwrap_or(&[]))
        })
    }
    pub fn id(&self) -> Option<anyhow::Result<&[u8]>> {
        self.get_option_bytes(KnownOptions::Id.to_string().as_bytes())
    }
    pub fn mode(&self) -> Option<anyhow::Result<Mode>> {
        self.get_option_bytes(KnownOptions::Mode.to_string().as_bytes())
            .map(|r| r.and_then(|b| Ok(std::str::from_utf8(b)?.parse()?)))
    }
    pub fn get_mode(&self) -> anyhow::Result<Mode> {
        Ok(self.mode().transpose()?.unwrap_or(Mode::TREE_DESC))
    }
    pub fn add_stmt(&mut self, stmt:ABList) -> anyhow::Result<()>{
        if stmt.lst[0].0.is_empty() && stmt.lst[0].1 == Some(Ctr::Colon) {
            self.add_option_abl(stmt)
        } else {
            self.predicates.add_ext_predicate(stmt.try_into()?)
        } 
    }
    pub fn parse(&mut self, multiline_stament: &[u8], ctx: &EvalCtx<impl Scope>) -> anyhow::Result<()> {
        for line in multiline_stament.split(|ch| *ch == b'\n') {
            if line.is_empty(){ continue;}
            let e = eval(ctx, &abe::parse_abe_b(line)?)?;
            self.add_stmt(e)?;
        }
        Ok(())
    }

    pub fn hash_eq(h: linkspace_pkt::LkHash) -> Self {
        let mut predicates = PktPredicates::default();
        predicates.hash.add(TestOp::Equal, h.into());
        predicates.state.i_query.add(TestOp::Equal,0u32.into());
        let mut q= Query {
            predicates,
            options: Default::default()
        };
        q.add_option(&KnownOptions::Mode.to_string(), &[Mode::HASH_ASC.to_string().as_bytes()]);
        q
    }
    /// does not restrict depth
    pub fn dgpk(domain: Domain, group: GroupID, prefix: IPathBuf, key: PubKey) -> Self {
        let mut predicates = PktPredicates::default();
        predicates.domain.add(TestOp::Equal, domain.into());
        predicates.group.add(TestOp::Equal, group.into());
        predicates.path_prefix = prefix;
        predicates.pubkey.add(TestOp::Equal, key.into());
        Query {
            predicates,
            options: Default::default(),
        }
    }
}

pub type CompiledQuery = Box<dyn FnMut(&dyn linkspace_pkt::NetPkt) -> (bool, ControlFlow<()>)>;
impl Query {
    /// currently rather slow.
    pub fn compile(
        self,
    ) -> anyhow::Result<CompiledQuery>{
        let mut we = WatchEntry::new(vec![], self, 0, (), debug_span!("todo span"))?;
        Ok(Box::new(move |pkt| we.test_dyn(pkt)))
    } 
}

impl<'o> EvalScopeImpl for &'o Query {
    fn about(&self) -> (String, String) {
        ("watchopts".into(), "get options set in the extwatch".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([(
            "opt",
            1..=1,
            Some(true),
            "[X] - try get byte value associated with X",
            |ewatch: &&Query, name: &[&[u8]]| {
                let optv = ewatch.get_option_bytes(name[0]).transpose()?;
                optv.ok_or_else(|| anyhow::anyhow!("{} not set", AB(name[0])))
                    .map(Vec::from)
            }
        )])
    }
}
