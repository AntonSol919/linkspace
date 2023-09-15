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

impl Space {
    pub fn ablist(&self) -> ABList {
        self.iter().fold(ABList::default(), |s, a| {
            s.push_ctr(abe::ast::Ctr::FSlash).push_bytes(a)
        })
    }
}

use crate::*;
#[derive(Error, Debug)]
pub enum SpaceExprError {
    #[error("space expr must start with '/' and can't contain ':', or '\\n'")]
    BadCtr(Option<Ctr>),
    #[error("Space error {0}")]
    Space(#[from] SpaceError),
    #[error("{}",.0)]
    Custom(String),
}

pub type SpaceExpr = TypedABE<SpaceBuf>;
pub type RootedSpaceExpr = TypedABE<RootedSpaceBuf>;
impl TryFrom<ABList> for RootedSpaceBuf {
    type Error = SpaceExprError;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        Ok(SpaceBuf::try_from(value)?.try_into_rooted()?)
    }
}
impl From<SpaceBuf> for ABList {
    fn from(val: SpaceBuf) -> Self {
        val.ablist()
    }
}
impl ABEValidator for RootedSpaceBuf {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        SpaceBuf::check(b)
    }
}

impl SpaceBuf {
    pub fn try_from_ablist(mut ablist: ABList) -> Result<Self, SpaceExprError> {
        match ablist.first(){
            None => return Ok(SpaceBuf::new()),
            Some((ctr,bytes)) if *ctr != Some(Ctr::FSlash) && bytes.is_empty() => { ablist.pop_front(); }
            _ => {}
        }
        if let Ok(b) = ablist.as_exact_bytes() {
            return Ok(Space::from_slice(b)?.into_spacebuf());
        }
        let v : Vec<_> = ablist.unwrap().into_iter()
            .map(|(ctr,b)| if ctr != Some(Ctr::FSlash) { Err(SpaceExprError::BadCtr(ctr))} else { Ok(b)})
            .try_collect()?;
        return Ok(SpaceBuf::try_from_iter(v)?);
    }
}
impl TryFrom<ABList> for SpaceBuf {
    type Error = SpaceExprError;
    fn try_from(ablist: ABList) -> Result<Self, Self::Error> {
        SpaceBuf::try_from_ablist(ablist)
    }
}

impl ABEValidator for SpaceBuf {
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

pub fn space_str(v: &str) -> Result<SpaceBuf, ABEError<SpaceExprError>> {
    v.parse::<SpaceExpr>()?.eval(&EvalCtx {
        scope: core_scope(),
    })
}
impl FromStr for RootedSpaceBuf {
    type Err = ABEError<SpaceExprError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        space_str(s)?
            .try_into_rooted()
            .map_err(|e| ABEError::TryFrom(e.into()))
    }
}
impl FromStr for SpaceBuf {
    type Err = ABEError<SpaceExprError>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        space_str(s)
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

impl ToABE for Space {
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
impl Space {
    pub fn display_fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for seg in self.iter() {
            fmt_segm2(seg, f)?;
        }
        Ok(())
    }
}

impl Display for Space {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.display_fmt(f)
    }
}
impl<X> Display for RootedSpaceBytes<X>
where
    Self: AsRef<RootedSpace>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_ref().space(), f)
    }
}

impl Debug for Space {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let it = self.iter();
        f.debug_list().entries(it).finish()
    }
}
impl Debug for RootedSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.space(), f)
    }
}
impl Display for SpaceBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_space().display_fmt(f)
    }
}
impl Display for RootedSpaceBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.space().display_fmt(f)
    }
}
impl Debug for SpaceBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.as_space(), f)
    }
}
impl Debug for RootedSpaceBuf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt(self.space(), f)
    }
}

impl Serialize for Space {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            let v: Vec<_> = self.iter().map(as_abtxt).collect();
            v.serialize(serializer)
        } else {
            self.space_bytes().serialize(serializer)
        }
    }
}
impl Serialize for RootedSpace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.space().serialize(serializer)
    }
}
impl Serialize for RootedSpaceBuf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.space().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for &RootedSpace {
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
impl<'de> Deserialize<'de> for RootedSpaceBuf {
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
fn generate_valid() -> Vec<SpaceBuf> {
    vec![
        SpaceBuf::new(),
        SpaceBuf::from_iter(&[b"hello", b"world"]),
        SpaceBuf::from_iter(&[b"hello" as &[u8], &[255; MAX_SPACENAME_COMPONENT_SIZE]]),
    ]
}
#[cfg(test)]
#[test]
fn string_fmt() {
    for sp in generate_valid() {
        let st = sp.to_string();
        let parsed: SpaceBuf = st.parse().unwrap();
        assert_eq!(sp, parsed)
    }
}

#[test]
fn test_encode() {
    #[track_caller]
    fn eq(st: &str, r: &[&[u8]]) {
        let x = space_str(st).unwrap();
        let b = SpaceBuf::from_iter(r);
        assert_eq!(x, b)
    }
    assert!(space_str("noopen").is_err());
    eq("/ok", &[b"ok"]);
    assert!(space_str("/trail/").is_err());
    assert!(space_str(r#"/kk[kk"#).is_err());
    eq("/\\[k", &[b"[k"]);
}

#[derive(Copy, Clone, Debug)]
pub struct SpaceFE;

impl EvalScopeImpl for SpaceFE {
    fn about(&self) -> (String, String) {
        (
            "space".into(),
            r"space utils. Usually [//some/space] is the most readable".into(),
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
            ensure!(start <= 8 ,"space max depth is 8");
            ensure!(end >= start ,"end < start");
            Ok(start..end)
        }
        fn encode_sp(_:&SpaceFE,b:&[u8],_:&[ABE]) -> ApplyResult<String>{
           Space::from_slice(b).ok().map(|p|format!("[/{p}]")).into()
        }
        
        crate::abe::fncs!([
            ("?space", 1..=1, "decode space", |_, i: &[&[u8]]| Ok(
                Space::from_slice(i[0])?.to_string().into_bytes()
            )),
            (
                "si",
                2..=3,
                "space idx [start,?end]",
                |_, i: &[&[u8]]| {
                    Ok(Space::from_slice(i[0])?
                        .rooted()
                        .range(parse_range(i)?)
                        .space_bytes()
                        .to_vec())
                }
            ),
            (@C
                "s",
                1..=8,
                None,
                "build space from arguments - alternative to [//some/path] syntax",
                |_, i: &[&[u8]],_,_| { Ok(SpaceBuf::try_from_iter(i)?.space_bytes().to_vec()) },
                encode_sp
            )
        ])
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[
            ScopeMacro {
                apply: |_,inp:&[ABE],scope:&dyn Scope| {
                    let lst = abe::eval::eval(&EvalCtx { scope }, inp)?;
                    let p = SpaceBuf::try_from(lst)?;
                    ApplyResult::Value(p.unwrap())
                },
                info: ScopeMacroInfo { id: "", help: "the 'empty' eval for encoding space. i.e. [//some/space/val] creates the byte for /some/space/val" }
            },
            ScopeMacro {
                apply: |_,inp:&[ABE],scope:&dyn Scope| {
                    let mut lst = abe::eval::eval(&EvalCtx { scope }, inp)?;
                    lst.get_mut().retain(|v|  !v.1.is_empty());
                    let p = SpaceBuf::try_from(lst)?;
                    ApplyResult::Value(p.unwrap())
                },
                info: ScopeMacroInfo { id: "~", help: "similar to '//' but forgiving on empty components" }
            }
        ]
    }
}
