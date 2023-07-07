use anyhow::ensure;
// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use byte_fmt::{
    abe::{
        self,
        abtxt::as_abtxt,
        ast::{is_fslash, take_ctr_expr, ABEError, Ctr, MatchError},
        convert::TypedABE,
        eval::ABList,
        ABEValidator, ToABE, ABE, scope::{core_scope, uint::parse_b},
    },
    eval::{
         ApplyErr, ApplyResult, EvalCtx, EvalScopeImpl, Scope, ScopeMacro, ScopeMacroInfo,
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
    #[error("spath expr must start with '/' and can't contain ':', '\\n', or '\\t' ")]
    BadCtr(Option<Ctr>),
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
        Ok(SPathBuf::try_from(value)?.try_ipath()?)
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
    pub fn try_from_ablist(mut ablist: ABList) -> Result<Self, SPathExprErr> {
        match ablist.first(){
            None => return Ok(SPathBuf::new()),
            Some((ctr,bytes)) if *ctr != Some(Ctr::FSlash) && bytes.is_empty() => { ablist.pop_front(); }
            _ => {}
        }
        if let Ok(b) = ablist.as_exact_bytes() {
            return Ok(SPath::from_slice(b)?.into_spathbuf());
        }
        let v : Vec<_> = ablist.unwrap().into_iter()
            .map(|(ctr,b)| if ctr != Some(Ctr::FSlash) { Err(SPathExprErr::BadCtr(ctr))} else { Ok(b)})
            .try_collect()?;
        return Ok(SPathBuf::try_from_iter(v)?);
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
            .try_ipath()
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
        write!(f, "/[b:{b64}]")?;
    } else if let Ok(b) = <[u8; 16]>::try_from(seg) {
        match as_abtxt(&b){
            std::borrow::Cow::Borrowed(b) => write!(f,"/{b}")?,
            std::borrow::Cow::Owned(_) => write!(f,"/{}",AB(b).to_abe_str())?,
        }
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
            let v: Vec<_> = self.iter().map(as_abtxt).collect();
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
        SPathBuf::from_iter(&[b"hello", b"world"]),
        SPathBuf::from_iter(&[b"hello" as &[u8], &[255; MAX_SPATH_COMPONENT_SIZE]]),
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
        let b = SPathBuf::from_iter(r);
        assert_eq!(x, b)
    }
    assert!(spath_str("noopen").is_err());
    eq("/ok", &[b"ok"]);
    assert!(spath_str("/trail/").is_err());
    assert!(spath_str(r#"/kk[kk"#).is_err());
    eq("/\\[k", &[b"[k"]);
}

#[derive(Copy, Clone, Debug)]
pub struct PathFE;

impl EvalScopeImpl for PathFE {
    fn about(&self) -> (String, String) {
        (
            "path".into(),
            r"path utils. Usually [//some/path] is the most readable".into(),
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
            ensure!(start <= 8 ,"paths only have upto 8 components");
            ensure!(end >= start ,"end < start");
            Ok(start..end)
        }
        fn encode_sp(_:&PathFE,b:&[u8],_:&[ABE]) -> ApplyResult<String>{
           SPath::from_slice(b).ok().map(|p|format!("[/{p}]")).into()
        }
        
        crate::abe::fncs!([
            ("?p", 1..=1, "decode path", |_, i: &[&[u8]]| Ok(
                SPath::from_slice(i[0])?.to_string().into_bytes()
            )),
            (
                "path_idx",
                2..=3,
                "path idx [start,?end]",
                |_, i: &[&[u8]]| {
                    Ok(SPath::from_slice(i[0])?
                        .ipath()
                        .range(parse_range(i)?)
                        .spath_bytes()
                        .to_vec())
                }
            ),
            (@C
                "p",
                1..=8,
                None,
                "build path from arguments",
                |_, i: &[&[u8]],_,_| { Ok(SPathBuf::try_from_iter(i)?.spath_bytes().to_vec()) },
                encode_sp
            )
        ])
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[
            ScopeMacro {
                apply: |_,inp:&[ABE],scope:&dyn Scope| {
                    let lst = abe::eval::eval(&EvalCtx { scope }, inp)?;
                    let p = SPathBuf::try_from(lst)?;
                    ApplyResult::Value(p.unwrap())
                },
                info: ScopeMacroInfo { id: "", help: "the 'empty' eval for encoding paths . i.e. [//some/spath/val] creates the byte for /some/spath/val" }
            },
            ScopeMacro {
                apply: |_,inp:&[ABE],scope:&dyn Scope| {
                    let mut lst = abe::eval::eval(&EvalCtx { scope }, inp)?;
                    lst.get_mut().retain(|v|  !v.1.is_empty());
                    let p = SPathBuf::try_from(lst)?;
                    ApplyResult::Value(p.unwrap())
                },
                info: ScopeMacroInfo { id: "~", help: "similar to '//' but forgiving on empty components" }
            }
        ]
    }
}
