// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::fmt::Display;

use either::Either;
use linkspace_pkt::abe::ast::*;
use thiserror::Error;

impl ABEValidator for ExtPredicate {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        let (kind, rest) = take_expr_ctr2(b, is_colon)?;
        kind.as_bytes()?;
        let (op, rest) = take_expr_ctr2(rest, is_colon)?;
        op.as_bytes()?;
        take_first(rest)?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum TestEvalErr {
    #[error("err {}",.0)]
    Err(&'static str),
    #[error("unknown test kind '{}' - known tests : [{}]"
            ,.0,PredicateType::ALL.iter().map(|v| v.info().name).collect::<Vec<_>>().join(","))]
    ParseKind(AB<Vec<u8>>),
    #[error("predicate kind '{}' could not take '{}'",predicate.kind,predicate.val)]
    SelectAdd {
        predicate: Predicate,
        #[source]
        compile_err: anyhow::Error,
    },
}

impl TryFrom<ABList> for ExtPredicate {
    type Error = TestEvalErr;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        tracing::trace!(?value, "As predicate");
        let mut it = value.lst.into_iter();
        let (kind, c) = it.next().ok_or("missing kind").map_err(TestEvalErr::Err)?;
        if c != Some(Ctr::Colon) {
            return Err("unexepcted delim").map_err(TestEvalErr::Err);
        };
        let kind = match PredicateType::try_from_id(&kind) {
            Some(s) => s.into(),
            None => return Err(TestEvalErr::ParseKind(AB(kind))),
        };
        let (op, c) = it.next().ok_or("Missing op").map_err(TestEvalErr::Err)?;
        if c != Some(Ctr::Colon) {
            return Err("unexepcted delim").map_err(TestEvalErr::Err);
        };
        let op: ExtendedTestOp = std::str::from_utf8(&op)
            .ok()
            .and_then(|v| v.parse().ok())
            .ok_or(TestEvalErr::Err("Cant parse op"))?;
        let val = ABList { lst: it.collect() };
        if val.lst.is_empty() {
            return Err("Mising value").map_err(TestEvalErr::Err);
        }
        let predicate = ExtPredicate { kind, val, op };
        Ok(predicate)
    }
}

#[derive(FromStr, Display, Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExtendedTestOp {
    #[display(">=")]
    GreaterEq,
    #[display("<=")]
    LessEq,
    #[display("=*")]
    /// Set the last bytes to exactly equal to
    HeadMask,
    #[display("*=")]
    /// Set the last bytes to exactly equal to
    TailMask,
    #[display("{0}")]
    Op(TestOp),
}

#[derive(Clone, Debug)]
pub struct ExtPredicate {
    pub kind: RuleType,
    pub op: ExtendedTestOp,
    pub val: ABList,
}
impl From<Predicate> for ExtPredicate {
    fn from(val: Predicate) -> Self {
        ExtPredicate {
            kind: val.kind,
            op: ExtendedTestOp::Op(val.op),
            val: val.val,
        }
    }
}

impl ExtPredicate {
    pub fn try_iter(self) -> Result<impl Iterator<Item = Predicate>, TestEvalErr> {
        let ExtPredicate { kind, op, mut val } = self;
        let once_op: Option<TestOp> = match op {
            ExtendedTestOp::GreaterEq if val.lst.len() != 1 => {
                return Err(TestEvalErr::Err("Can't >= this value"))
            }
            ExtendedTestOp::LessEq if val.lst.len() != 1 => {
                return Err(TestEvalErr::Err("Can't <= this value"))
            }
            ExtendedTestOp::GreaterEq => {
                if super::uint::u8_be::sub_one(&mut val.lst[0].0).is_some() {
                    Some(TestOp::Greater)
                } else {
                    None
                }
            }
            ExtendedTestOp::LessEq => {
                if super::uint::u8_be::add_one(&mut val.lst[0].0).is_some() {
                    Some(TestOp::Less)
                } else {
                    None
                }
            }
            ExtendedTestOp::Op(op) => Some(op),
            ExtendedTestOp::TailMask | ExtendedTestOp::HeadMask => {
                let e = || TestEvalErr::Err("head/tail mask can only apply to fixed len fields");
                let size = kind.fixed_size().ok_or(e())?;
                let bytes = val.as_exact_bytes().map_err(|_| e())?;
                if bytes.len() > size {
                    return Err(TestEvalErr::Err("to much data for field"));
                }
                let missing = size - bytes.len();

                let (zeros, ones) = if op == ExtendedTestOp::TailMask {
                    (
                        [vec![255; missing].as_slice(), bytes].concat(),
                        [vec![0; missing].as_slice(), bytes].concat(),
                    )
                } else {
                    (
                        [bytes, vec![255; missing].as_slice()].concat(),
                        [bytes, vec![0; missing].as_slice()].concat(),
                    )
                };
                debug_assert_eq!(zeros.len(), size);
                return Ok(Either::Right(
                    [
                        Predicate {
                            kind,
                            op: TestOp::Mask0,
                            val: zeros.into(),
                        },
                        Predicate {
                            kind,
                            op: TestOp::Mask1,
                            val: ones.into(),
                        },
                    ]
                    .into_iter(),
                ));
            }
        };
        Ok(Either::Left(
            once_op.map(|op| Predicate { kind, op, val }).into_iter(),
        ))
    }
}

pub type PredicateExpr = TypedABE<ExtPredicate>;
#[derive(Debug, Clone)]
pub struct Predicate {
    pub kind: RuleType,
    pub op: TestOp,
    pub val: ABList,
}
impl Display for Predicate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_str(true))
    }
}

impl Predicate {
    pub fn to_str(&self, canonical: bool) -> String {
        if canonical {
            let val = self.kind.canonical(&self.val);
            let lst = abe!( (self.kind.to_string()) : (self.op.to_string()) : +(val));
            print_abe(lst)
        } else {
            let lst = abe!( (self.kind.to_string()) : (self.op.to_string()) : +(self.val.clone()));
            print_abe(lst)
        }
    }
    pub fn from_slice(kind: impl Into<RuleType>, op: impl Into<TestOp>, val: &[u8]) -> Predicate {
        Predicate::from(kind, op, val)
    }
    pub fn from(
        kind: impl Into<RuleType>,
        op: impl Into<TestOp>,
        val: impl Into<ABList>,
    ) -> Predicate {
        Predicate {
            kind: kind.into(),
            op: op.into(),
            val: val.into(),
        }
    }
}
impl ToABE for Predicate {
    fn to_abe(&self) -> Vec<ABE> {
        abev!( (self.kind.to_string()) : (self.op.to_string()) : +(self.kind.canonical(&self.val)))
    }
}

use crate::eval::*;
use linkspace_pkt::{FieldEnum, SPathBuf, AB};
use parse_display::{Display, FromStr};

use crate::predicate::TestOp;
use crate::prelude::predicate_type::PredicateType;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr)]
pub enum QScope {
    #[display("i_branch")]
    Branch,
    #[display("i_db")]
    Index,
    #[display("i_new")]
    New,
    #[display("i")]
    Query,
}
pub const QSCOPES: [QScope; 4] = [QScope::Query, QScope::New, QScope::Index, QScope::Branch];

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr)]
pub enum RuleType {
    #[display("{0}")]
    Field(FieldEnum),
    #[display("recv")]
    RecvStamp,
    #[display("prefix")]
    PrefixPath,
    #[display("{0}")]
    Limit(QScope),
}
impl RuleType {
    pub fn iter_all() -> impl Iterator<Item = RuleType> {
        FieldEnum::LIST
            .map(RuleType::Field)
            .into_iter()
            .chain([RuleType::RecvStamp, RuleType::PrefixPath])
            .chain(QSCOPES.map(RuleType::Limit))
    }

    pub fn try_canonical(self, abl: ABList) -> anyhow::Result<Vec<ABE>> {
        match self {
            RuleType::Field(f) => f.try_to_abe(abl).ok_or(anyhow::anyhow!("Field to abe err")),
            RuleType::RecvStamp => Ok(linkspace_pkt::Stamp::try_from(abl)?.to_abe()),
            RuleType::PrefixPath => Ok(SPathBuf::try_from(abl)?.to_abe()),
            RuleType::Limit(_) => Ok(linkspace_pkt::U32::try_from(abl)?.to_abe()),
        }
    }
    pub fn canonical(self, abl: &ABList) -> Vec<ABE> {
        self.try_canonical(abl.clone())
            .unwrap_or_else(|_| abl.clone().into())
    }
    pub fn fixed_size(self) -> Option<usize> {
        match self {
            RuleType::Field(f) => f.fixed_size(),
            RuleType::RecvStamp => Some(8),
            RuleType::PrefixPath => None,
            RuleType::Limit(_) => Some(4),
        }
    }
}
impl From<FieldEnum> for RuleType {
    fn from(f: FieldEnum) -> Self {
        RuleType::Field(f)
    }
}
impl From<QScope> for RuleType {
    fn from(f: QScope) -> Self {
        RuleType::Limit(f)
    }
}
