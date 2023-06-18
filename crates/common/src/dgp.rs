// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.



/*
TODO rewrite.
This is rather messy as its old code missing some insights gained later.
*/

use abe::ast::{is_empty, Ctr};
use anyhow::{bail, Context};
use linkspace_core::{
    predicate::exprs::{Predicate, PredicateExpr, RuleType},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DGPExpr {
    pub domain: DomainExpr,
    pub group: HashExpr,
    pub path: SPathExpr,
}
impl DGPExpr {
    /// Returns eval, and mutates this value to be statically resolvable
    pub fn resolve(&mut self, ctx: &EvalCtx<impl Scope>) -> anyhow::Result<DGP> {
        let dgp = self.eval(ctx)?;
        *self = DGPExpr {
            domain: DomainExpr::from_unchecked(dgp.domain.to_abe()),
            group: HashExpr::from_unchecked(dgp.group.to_abe()),
            path: SPathExpr::from_unchecked(dgp.path.to_abe()),
        };
        debug_assert_eq!(self.eval(ctx).unwrap(), dgp);
        Ok(dgp)
    }
    pub fn eval(&self, ctx: &EvalCtx<impl Scope>) -> anyhow::Result<DGP> {
        let domain = self.domain.eval(ctx);
        let group = self.group.eval(ctx);
        let path = self.path.eval(ctx)?.try_ipath();
        match (domain, group, path) {
            (Ok(domain), Ok(group), Ok(path)) => Ok(DGP {
                domain,
                group,
                path,
            }),
            (d, g, p) => {
                let ctx = anyhow::anyhow!(
                    "{:?} : {:?} : {:?} ",
                    d.as_ref().map(|v| v.to_abe_str()),
                    g.as_ref().map(|g| g.to_abe_str()),
                    p.as_ref().map(|p| p.to_abe_str())
                );
                if let Err(e) = d {
                    return Err(e)
                        .context(self.domain.to_string())
                        .context("eval domain")
                        .context(ctx);
                }
                if let Err(e) = g {
                    return Err(e)
                        .context(self.group.to_string())
                        .context("eval group")
                        .context(ctx);
                }
                if let Err(e) = p {
                    return Err(e)
                        .context(self.path.to_string())
                        .context("eval path")
                        .context(ctx);
                }
                unreachable!()
            }
        }
    }
    pub fn as_test_exprs(self) -> impl Iterator<Item = Vec<ABE>> {
        let DGPExpr { domain, group, path } = self;
        let mut prefix = None;
        if !path.is_empty() {
            prefix = Some(
                abev!("prefix" : "=" : +(path.0))
            );
        }
        [
            abev!("domain" : "=" : +(domain.0)),
            abev!("group" : "=" : +(group.0))
        ]
        .into_iter()
        .chain(prefix)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DGP {
    pub domain: Domain,
    pub group: GroupID,
    pub path: IPathBuf,
}
impl DGP {
    pub fn as_predicates(&self) -> impl Iterator<Item = Predicate> {
        [
            Predicate::from_slice(RuleType::PrefixPath, TestOp::Equal, self.path.spath_bytes()),
            Predicate::from_slice(FieldEnum::DomainF, TestOp::Equal, &*self.domain),
            Predicate::from_slice(FieldEnum::GroupIDF, TestOp::Equal, &*self.group),
        ]
        .into_iter()
    }
    pub fn as_predicate_exprs(&self) -> impl Iterator<Item = PredicateExpr> {
        [
            abe!(<ExtPredicate> "prefix" : "=" : +(self.path.to_abe())),
            abe!(<ExtPredicate> "domain" : "=" : +(self.domain.to_abe())),
            abe!(<ExtPredicate> "group"  : "=" : +(self.group.to_abe())),
        ]
        .into_iter()
    }
}

impl FromStr for DGPExpr {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use abe::*;
        let ast = parse_abe(s)?;

        let (dgp, rest) = try_take_dgp(&ast)?;
        is_empty(rest)?;
        Ok(dgp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DGPDExpr {
    pub dgp: DGPExpr,
    pub subsegment_limit: u8,
}
impl DGPDExpr {
    pub fn predicate_exprs(self) -> anyhow::Result<impl Iterator<Item = Vec<ABE>>> {
        let mut prefix_rule = None;
        if self.subsegment_limit != MAX_PATH_LEN as u8 && !self.dgp.path.is_empty() && !self.dgp.path.0.iter().any(|v| v.is_fslash()) {
                        anyhow::bail!("can't use subrange expr with an evaluated spath ( dont know its length ).
        Must add ':**' and manually set -- path_len ...")
                    }

        if self.subsegment_limit < MAX_PATH_LEN as u8 {
            let prefix_len = self
                .dgp
                .path
                .0
                .iter()
                .filter(|v| v.is_fslash())
                .count()
                .min(8) as u8;
            let exclude = prefix_len.saturating_add(self.subsegment_limit).min(8) + 1;
            prefix_rule = Some(abev!("path_len" : "<" : +(U8(exclude).to_abe())));
        }
        Ok(self.dgp.as_test_exprs().chain(prefix_rule))
    }
}

pub fn try_take_dgp(ast: &[ABE]) -> anyhow::Result<(DGPExpr, &[ABE])> {
    use abe::*;
    let mut it = ast.split(|v| matches!(v, ABE::Ctr(Ctr::Colon)));

    let domain = match it.next().unwrap_or(&[]){
        &[] => TypedABE::from_unchecked(crate::static_env::domain().to_abe()),
        v => v.try_into()?,
    };
    let group = match it.next().unwrap_or(&[]){
        &[] => TypedABE::from_unchecked(crate::static_env::group().to_abe()),
        v => v.try_into()?
    };
    let path = it.next().unwrap_or_default().try_into()?;
    Ok((
        DGPExpr {
            domain,
            group,
            path,
        },
        it.as_slice(),
    ))
}

pub fn dgpd(ast: &[ABE]) -> anyhow::Result<DGPDExpr> {

    let (dgp, rest) = try_take_dgp(ast)?;
    let mut subsegment_limit = 0;
    if !rest.is_empty() {
        let (l, rest) = try_take_subsegm_expr(rest)?;
        is_empty(rest)?;
        subsegment_limit = l;
    }
    Ok(DGPDExpr {
        dgp,
        subsegment_limit,
    })
}

impl FromStr for DGPDExpr {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use abe::*;

        let ast = parse_abe(s)?;
        dgpd(&ast)
    }
}

pub fn try_take_subsegm_expr(ast: &[ABE]) -> anyhow::Result<(u8, &[ABE])> {
    use abe::ast::*;
    let ([e], rest) = match take(ast) {
        Ok(e) => e,
        Err(_) => return Ok((0, &[])),
    };
    let depth = match as_expr(e)? {
        Expr::Bytes(s) => {
            match s.as_slice() {
                b"0" => 0,
                b"1" => 1,
                b"2" => 2,
                b"3" => 3,
                b"4" => 4,
                b"5" => 5,
                b"6" => 6,
                b"7" => 7,
                b"8" => 8,
                b"*" => 1,
                b"**" => MAX_PATH_LEN as u8,
                _e => bail!("{} is an invalid depth",e)
            }
        }
        Expr::Lst(_) => bail!("todo"),
    };
    Ok((depth, rest))
}
