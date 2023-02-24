// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use bytefmt::{
    abe::{
        self,
        abtxt::as_abtxt_e,
        ast::{is_fslash, take_ctr_expr, ABEError, Ctr, MatchError},
        convert::TypedABE,
        eval::ABList,
        ABEValidator, ToABE, ABE,
    },
    eval::{
        parse_b, ApplyErr, ApplyResult, EvalCtx, EvalScopeImpl, Scope, ScopeEval, ScopeEvalInfo,
        ScopeFunc,
    },
    *,
};
use core::fmt::{self, Debug, Display};
use serde::{Deserialize, Serialize};
use std::{ops::Range, str::FromStr};
use thiserror::Error;

impl SPath {
    pub fn ablist(&self) -> ABList {
        self.iter().fold(ABList::default(), |s, a| {
            s.push_ctr(abe::ast::Ctr::FSlash).push_bytes(a)
        })
    }
}

use crate::*;
#[derive(Error, Debug)]
pub enum SPathExprErr {
    #[error("Spath expr cant contain ':' or '\\n' ")]
    BadCtr(Ctr),
    #[error("SPath error {0}")]
    SPath(#[from] PathError),
    #[error("{}",.0)]
    Custom(String),
}

pub type SPathExpr = TypedABE<SPathBuf>;
pub type IPathExpr = TypedABE<IPathBuf>;
impl TryFrom<ABList> for IPathBuf {
    type Error = SPathExprErr;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        Ok(SPathBuf::try_from(value)?.try_idx()?)
    }
}
impl From<SPathBuf> for ABList {
    fn from(val: SPathBuf) -> Self {
        val.ablist()
    }
}
impl ABEValidator for IPathBuf {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        SPathBuf::check(b)
    }
}

impl SPathBuf {
    pub fn try_from_ablist(ablist: ABList) -> Result<Self, SPathExprErr> {
        if let Ok(b) = ablist.as_exact_bytes() {
            return Ok(SPath::from_slice(b)?.into_spathbuf());
        }
        let mut it = ablist.inner().iter().map(|(bytes, ctr)| match ctr {
            None | Some(Ctr::FSlash) => Ok(bytes),
            Some(e) => Err(SPathExprErr::BadCtr(*e)),
        });
        if let Some(v) = it.next() {
            if !v?.is_empty() {
                return Err(SPathExprErr::Custom(format!(
                    "SPath starts with '/' got {ablist}"
                )));
            }
            let lst = it.collect::<Result<Vec<_>, _>>()?;
            return Ok(SPathBuf::try_from_iter(lst)?);
        }
        Ok(SPathBuf::new())
    }
}
impl TryFrom<ABList> for SPathBuf {
    type Error = SPathExprErr;
    fn try_from(ablist: ABList) -> Result<Self, Self::Error> {
        SPathBuf::try_from_ablist(ablist)
    }
}

impl ABEValidator for SPathBuf {
    fn check(mut b: &[ABE]) -> Result<(), MatchError> {
        if b.len() == 1 {
            abe::ast::as_expr(&b[0])?;
            return Ok(())
        }
        while !b.is_empty() {
            let (_, next) = take_ctr_expr(b, is_fslash)?;
            b = next;
        }
        Ok(())
    }
}

/*
pub fn spath_eval(ctx: &EvalCtx<impl Env,impl Func>, abe: impl AsRef<[ABE]>) -> Result<SPathBuf, SPathExprErr>{
    let abe = abe.as_ref();
    let mut it = abe.split(|v| v == &ABE::Ctr(Ctr::FSlash));
    let spath = SPathBuf::new();
    match it.next(){
        Some(v) => if !v.is_empty(){ return Err(SPathExprErr::Custom(format!("SPath starts with '/' got {:?}",abe)))},
        None => return Ok(spath),
    }
    let lst = it.map(|abe| eval(ctx,abe)?.into_exact_bytes().map_err(SPathExprErr::SegmentContainsCtr))
        .collect::<Result<Vec<Vec<u8>>,SPathExprErr>>()?;
    return Ok(SPathBuf::try_from(lst)?)
}
*/

pub fn spath_str(v: &str) -> Result<SPathBuf, ABEError<SPathExprErr>> {
    v.parse::<SPathExpr>()?.eval(&EvalCtx {
        scope: core_scope(),
    })
}
impl FromStr for IPathBuf {
    type Err = ABEError<SPathExprErr>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        spath_str(s)?
            .try_idx()
            .map_err(|e| ABEError::TryFrom(e.into()))
    }
}
impl FromStr for SPathBuf {
    type Err = ABEError<SPathExprErr>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        spath_str(s)
    }
}
pub fn fmt_segm2(seg: &[u8], f: &mut fmt::Formatter) -> fmt::Result {
    if let Ok(b) = <[u8; 32]>::try_from(seg) {
        let b64 = B64(b).to_string();
        write!(f, "/{{b:{b64}}}")?;
    } else if let Ok(b) = <[u8; 16]>::try_from(seg) {
        let abt = AB(b).to_abe_str();
        write!(f, "/{abt}")?;
    } else {
        write!(f, "/{}", AB(seg))?
    }
    Ok(())
}

impl ToABE for SPath {
    fn to_abe(&self) -> Vec<ABE> {
        let mut v = vec![];
        for seg in self.iter() {
            v.push(ABE::Ctr(Ctr::FSlash));
            if seg.len() == 32 {
                v.extend(LkHash::try_from(seg).unwrap().to_abe())
            } else if seg.len() == 16 {
                v.extend(Domain::try_from(seg).unwrap().to_abe())
            } else {
                v.push(ABE::Expr(seg.into()))
            };
        }
        v
    }
}
impl SPath {
    pub fn display_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for seg in self.iter() {
            fmt_segm2(seg, f)?;
        }
        Ok(())
    }
}

impl Display for SPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display_fmt(f)
    }
}
impl<X> Display for IPathBytes<X>
where
    Self: AsRef<IPath>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_ref().spath(), f)
    }
}

impl Debug for SPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let it = self.iter();
        f.debug_list().entries(it).finish()
    }
}
impl Debug for IPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.spath(), f)
    }
}
impl Display for SPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_spath().display_fmt(f)
    }
}
impl Display for IPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.spath().display_fmt(f)
    }
}
impl Debug for SPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_spath(), f)
    }
}
impl Debug for IPathBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.spath(), f)
    }
}

impl Serialize for SPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            let v: Vec<_> = self.iter().map(as_abtxt_e).collect();
            v.serialize(serializer)
        } else {
            self.spath_bytes().serialize(serializer)
        }
    }
}
impl Serialize for IPath {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.spath().serialize(serializer)
    }
}
impl Serialize for IPathBuf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.spath().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for &IPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            todo!()
        } else {
            todo!()
        }
    }
}
impl<'de> Deserialize<'de> for IPathBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            todo!()
        } else {
            todo!()
        }
    }
}
#[cfg(test)]
fn generate_valid() -> Vec<SPathBuf> {
    vec![
        SPathBuf::new(),
        SPathBuf::from(&[b"hello", b"world"]),
        SPathBuf::from(&[b"hello" as &[u8], &[255; MAX_SPATH_COMPONENT_SIZE]]),
    ]
}
#[cfg(test)]
#[test]
fn string_fmt() {
    for sp in generate_valid() {
        let st = sp.to_string();
        let parsed: SPathBuf = st.parse().unwrap();
        assert_eq!(sp, parsed)
    }
}

#[test]
fn test_encode() {
    #[track_caller]
    fn eq(st: &str, r: &[&[u8]]) {
        let x = spath_str(st).unwrap();
        let b = SPathBuf::from(r);
        assert_eq!(x, b)
    }
    assert!(spath_str("noopen").is_err());
    eq("/ok", &[b"ok"]);
    assert!(spath_str("/trail/").is_err());
    assert!(spath_str(r#"/kk{kk"#).is_err());
    eq("/\\{k", &[b"{k"]);
}

#[derive(Copy, Clone, Debug)]
pub struct SPathFncs;
impl EvalScopeImpl for SPathFncs {
    fn about(&self) -> (String, String) {
        (
            "path".into(),
            r"spath and ipath utls. Usually you'll want {//some/path}".into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fn parse_range(i: &[&[u8]]) -> Result<Range<usize>, ApplyErr> {
            let start = parse_b::<usize>(i[1])?;
            let end = i
                .get(2)
                .map(|i| parse_b::<usize>(i))
                .transpose()?
                .unwrap_or(start + 1)
                .max(9);
            if start > 8 {
                return Err("paths go have upto 8 components".into());
            }
            if end < start {
                return Err("end < start".into());
            }
            Ok(start..end)
        }
        crate::abe::fncs!([
            ("?sp", 1..=1, "decode spath", |_, i: &[&[u8]]| Ok(
                SPath::from_slice(i[0])?.to_string().into_bytes()
            )),
            ("?ip", 1..=1, "decode ipath", |_, i: &[&[u8]]| Ok(
                IPath::from(i[0])?.to_string().into_bytes()
            )),
            (
                "ipcomp",
                2..=3,
                "ipath select [start,?end]",
                |_, i: &[&[u8]]| {
                    Ok(IPath::from(i[0])?
                        .range(parse_range(i)?)
                        .idx()
                        .ipath_bytes()
                        .to_vec())
                }
            ),
            (
                "spcomp",
                2..=3,
                "ipath select [start,?end]",
                |_, i: &[&[u8]]| {
                    Ok(SPath::from_slice(i[0])?
                        .idx()
                        .range(parse_range(i)?)
                        .spath_bytes()
                        .to_vec())
                }
            ),
            (
                "ip",
                1..=8,
                "build ipath from arguments",
                |_, i: &[&[u8]]| { Ok(IPathBuf::try_from_iter(i)?.ipath_bytes().to_vec()) }
            ),
            (
                "sp",
                1..=8,
                "build spath from arguments",
                |_, i: &[&[u8]]| { Ok(SPathBuf::try_from_iter(i)?.spath_bytes().to_vec()) }
            )
        ])
    }
    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[
            ScopeEval {
                apply: |_,inp:&[ABE],scope:&dyn Scope| {
                    let lst = abe::eval::eval(&EvalCtx { scope }, inp)?;
                    let p = SPathBuf::try_from(lst)?;
                    ApplyResult::Ok(p.unwrap())
                },
                info: ScopeEvalInfo { id: "", help: "the 'empty' eval for build spath. i.e. {//some/spath/val} creates the byte for /some/spath/val" }
            }
        ]
    }
}
