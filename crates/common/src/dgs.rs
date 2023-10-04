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
pub struct DGSExpr {
    pub domain: DomainExpr,
    pub group: HashExpr,
    pub space: SpaceExpr,
}
impl DGSExpr {
    /// Returns eval, and mutates this value to be statically resolvable
    pub fn resolve(&mut self, scope: &dyn Scope) -> anyhow::Result<DGS> {
        let dgs = self.eval(scope)?;
        *self = DGSExpr {
            domain: DomainExpr::from_unchecked(dgs.domain.to_abe()),
            group: HashExpr::from_unchecked(dgs.group.to_abe()),
            space: SpaceExpr::from_unchecked(dgs.space.to_abe()),
        };
        debug_assert_eq!(self.eval(scope).unwrap(), dgs);
        Ok(dgs)
    }
    pub fn eval(&self, scope: &dyn Scope) -> anyhow::Result<DGS> {
        let domain = self.domain.eval(scope);
        let group = self.group.eval(scope);
        let space = self.space.eval(scope)?.try_into_rooted();
        match (domain, group, space) {
            (Ok(domain), Ok(group), Ok(space)) => Ok(DGS {
                domain,
                group,
                space,
            }),
            (d, g, p) => {
                let scope = anyhow::anyhow!(
                    "{:?} : {:?} : {:?} ",
                    d.as_ref().map(|v| v.to_abe_str()),
                    g.as_ref().map(|g| g.to_abe_str()),
                    p.as_ref().map(|p| p.to_abe_str())
                );
                if let Err(e) = d {
                    return Err(e)
                        .context(self.domain.to_string())
                        .context("eval domain")
                        .context(scope);
                }
                if let Err(e) = g {
                    return Err(e)
                        .context(self.group.to_string())
                        .context("eval group")
                        .context(scope);
                }
                if let Err(e) = p {
                    return Err(e)
                        .context(self.space.to_string())
                        .context("eval space")
                        .context(scope);
                }
                unreachable!()
            }
        }
    }
    pub fn as_test_exprs(self) -> impl Iterator<Item = Vec<ABE>> {
        let DGSExpr {
            domain,
            group,
            space,
        } = self;
        let mut prefix = None;
        if !space.is_empty() {
            prefix = Some(abev!("prefix" : "=" : +(space.0)));
        }
        [
            abev!("domain" : "=" : +(domain.0)),
            abev!("group" : "=" : +(group.0)),
        ]
        .into_iter()
        .chain(prefix)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct DGS {
    pub domain: Domain,
    pub group: GroupID,
    pub space: RootedSpaceBuf,
}
impl DGS {
    pub fn as_predicates(&self) -> impl Iterator<Item = Predicate> {
        [
            Predicate::from_slice(
                RuleType::SpacePrefix,
                TestOp::Equal,
                self.space.space_bytes(),
            ),
            Predicate::from_slice(FieldEnum::DomainF, TestOp::Equal, &*self.domain),
            Predicate::from_slice(FieldEnum::GroupIDF, TestOp::Equal, &*self.group),
        ]
        .into_iter()
    }
    pub fn as_predicate_exprs(&self) -> impl Iterator<Item = PredicateExpr> {
        [
            abe!(<ExtPredicate> "prefix" : "=" : +(self.space.to_abe())),
            abe!(<ExtPredicate> "domain" : "=" : +(self.domain.to_abe())),
            abe!(<ExtPredicate> "group"  : "=" : +(self.group.to_abe())),
        ]
        .into_iter()
    }
}

impl FromStr for DGSExpr {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use abe::*;
        let ast = parse_abe_strict_b(s.as_bytes())?;

        let (dgp, rest) = try_take_dgs(&ast)?;
        is_empty(rest)?;
        Ok(dgp)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DGSDExpr {
    pub dgs: DGSExpr,
    pub depth_limit: u8,
}
impl DGSDExpr {
    pub fn predicate_exprs(self) -> anyhow::Result<impl Iterator<Item = Vec<ABE>>> {
        let mut prefix_rule = None;
        if self.depth_limit != MAX_SPACE_DEPTH as u8
            && !self.dgs.space.is_empty()
            && !self.dgs.space.0.iter().any(|v| v.is_fslash())
        {
            anyhow::bail!(
                "can't use subrange expr with an evaluated space ( dont know its length ).
        Must add ':**' or manually set -- depth ..."
            )
        }

        if self.depth_limit < MAX_SPACE_DEPTH as u8 {
            let prefix_len = self
                .dgs
                .space
                .0
                .iter()
                .filter(|v| v.is_fslash())
                .count()
                .min(8) as u8;
            let exclude = prefix_len.saturating_add(self.depth_limit).min(8) + 1;
            prefix_rule = Some(abev!("depth" : "<" : +(U8(exclude).to_abe())));
        }
        Ok(self.dgs.as_test_exprs().chain(prefix_rule))
    }
}

pub fn try_take_dgs(ast: &[ABE]) -> anyhow::Result<(DGSExpr, &[ABE])> {
    use abe::*;
    let mut it = ast.split(|v| matches!(v, ABE::Ctr(Ctr::Colon)));

    let domain = match it.next().unwrap_or(&[]) {
        &[] => TypedABE::from_unchecked(crate::thread_local::domain().to_abe()),
        v => v.try_into()?,
    };
    let group = match it.next().unwrap_or(&[]) {
        &[] => TypedABE::from_unchecked(crate::thread_local::group().to_abe()),
        v => v.try_into()?,
    };
    let space = it.next().unwrap_or_default().try_into()?;
    Ok((
        DGSExpr {
            domain,
            group,
            space,
        },
        it.as_slice(),
    ))
}

pub fn dgpd(ast: &[ABE]) -> anyhow::Result<DGSDExpr> {
    let (dgs, rest) = try_take_dgs(ast)?;
    let mut subsegment_limit = 0;
    if !rest.is_empty() {
        let (l, rest) = try_take_subsegm_expr(rest)?;
        is_empty(rest)?;
        subsegment_limit = l;
    }
    Ok(DGSDExpr {
        dgs,
        depth_limit: subsegment_limit,
    })
}

impl FromStr for DGSDExpr {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use abe::*;

        let ast = parse_abe_strict_b(s.as_bytes())?;
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
        Expr::Bytes(s) => match s.as_slice() {
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
            b"**" => MAX_SPACE_DEPTH as u8,
            _e => bail!("{} is an invalid depth", e),
        },
        Expr::Lst(_) => bail!("todo"),
    };
    Ok((depth, rest))
}
