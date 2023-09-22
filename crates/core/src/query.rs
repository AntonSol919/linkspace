// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{fmt::Display, ops::ControlFlow};

use anyhow::{ensure, Context };
use linkspace_pkt::{
    abe::{
        self,
        ast::Ctr,
        eval::{eval, ABList, EvalCtx, EvalScopeImpl, Scope, ScopeFunc},
        fncs, abconf::ABConf,
    },
    Domain, GroupID, RootedSpaceBuf, PubKey, AB,
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
            conf: ABConf::default(),
        }
    }
}
/**
A set of predicates and options.
Create with linkspace::lk_query, extend with lk_query_append, and stringify with lk_query_str
Argument to lk_get* and lk_watch.
**/
/*
internally this is a wrapper around ABConf that intercepts all options not starting with ':'.
Options starting with a field id are immediately interpreted and added to the predicate set.
*/
#[derive(Debug, Clone, Default)]
pub struct Query {
    pub predicates: PktPredicates,
    pub conf: ABConf
}
impl Display for Query {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.conf,f)?;
        self.predicates.fmt(f)?;
        Ok(())
    }
}

#[derive(Copy, Clone, parse_display::Display, parse_display::FromStr)]
#[display(style = "kebab-case")]
pub enum KnownOptions {
    /// which index to walk when reading from the database
    Mode,
    /// The arg is the query id under which to operate. Can be overwritten or closed. Is required for lk_watch but not for lk_get*.
    Qid,
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
        [Mode, Qid, Follow, NotifyClose ].into_iter()
    }
}

impl Query {
    pub const DEFAULT : Self = Query{
        predicates: PktPredicates::DEFAULT,
        conf: ABConf::DEFAULT
    };
    pub fn to_str(&self, canonical: bool) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        for opt in &*self.conf{
            writeln!(out, "{opt}").unwrap();
        }
        for p in self.predicates.iter() {
            writeln!(out, "{}", p.to_str(canonical)).unwrap();
        }
        out
    }
    pub fn add_option(&mut self, name: &str, values: &[&[u8]]) {
        let mut ab = ABList::DEFAULT;
        ab.push_v((Some(Ctr::Colon),name.as_bytes().to_vec()));
        values.iter().for_each(|o| ab.push_v((Some(Ctr::Colon),o.to_vec())));
        self.conf.push(ab);
    }

    pub fn add_option_abl(&mut self, opt: ABList) -> anyhow::Result<()> {
        ensure!(
            opt[0].0.is_some(),
            "options start with ':' or '/'. got {opt:?}"
        );
        self.conf.push(opt);
        Ok(())
    }
    pub fn options(&self) -> &ABConf{
        &self.conf
    }
    pub fn get_known_opt(&self, opt: KnownOptions) -> anyhow::Result<Option<Option<&[u8]>>>{
        self.get_option(opt.to_string().as_bytes())
    }
    /// get an option - i.e. the first statement starting with :name - '/name' returns an error
    pub fn get_option(&self, name: &[u8]) -> anyhow::Result<Option<Option<&[u8]>>> {
        self.conf.has_optional_value(&[&[],name]).transpose().map_err(|e| anyhow::anyhow!("option contains '/' : {e}"))
    }
    pub fn qid(&self) -> anyhow::Result<Option<Option<&[u8]>>> {
        self.get_known_opt(KnownOptions::Qid)
    }
    pub fn mode(&self) -> anyhow::Result<Option<Mode>> {
        match self.get_known_opt(KnownOptions::Mode)?.flatten(){
            None => Ok(None),
            Some(v) => Ok(Some(std::str::from_utf8(v)?.parse()?)),
        }
    }
    pub fn get_mode(&self) -> anyhow::Result<Mode> {
        self.mode().map(|o| o.unwrap_or(Mode::TREE_DESC))
    }
    pub fn add_stmt(&mut self, stmt:ABList) -> anyhow::Result<()>{
        if stmt[0].0.is_some(){
            self.add_option_abl(stmt)
        } else {
            self.predicates.add_ext_predicate(stmt.try_into().with_context(|| anyhow::anyhow!("could not turn stmt into valid extpred"))?)
        } 
    }
    pub fn parse(&mut self, multiline_stament: &[u8], ctx: &EvalCtx<impl Scope>) -> anyhow::Result<()> {
        for line in multiline_stament.split(|ch| *ch == b'\n') {
            if line.is_empty(){ continue;}
            let e = eval(ctx, &abe::parse_abe_strict_b(line)?)?;
            self.add_stmt(e)?;
        }
        Ok(())
    }

    pub fn hash_eq(h: linkspace_pkt::LkHash) -> Self {
        let mut predicates = PktPredicates::default();
        predicates.hash.add(TestOp::Equal, h.into());
        predicates.state.i_query.add(TestOp::Equal,0u32);
        let mut q= Query {
            predicates,
            conf:Default::default()
        };
        q.add_option(&KnownOptions::Mode.to_string(), &[Mode::HASH_ASC.to_string().as_bytes()]);
        q
    }
    /// does not restrict depth
    pub fn dgsk(domain: Domain, group: GroupID, prefix: RootedSpaceBuf, key: PubKey) -> Self {
        let mut predicates = PktPredicates::default();
        predicates.domain.add(TestOp::Equal, domain.into());
        predicates.group.add(TestOp::Equal, group.into());
        predicates.rspace_prefix = prefix;
        predicates.pubkey.add(TestOp::Equal, key.into());
        Query {
            predicates,
            conf: Default::default(),
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
            "[X] - try to read the first option ':X:val' - returns '' if only ':X' is found",
            |ewatch: &&Query, name: &[&[u8]]| {
                let optv = ewatch.get_option(name[0])?.map(|o| o.unwrap_or(&[]));
                optv.ok_or_else(|| anyhow::anyhow!("{} not set", AB(name[0])))
                    .map(Vec::from)
            }
        )])
    }
}
