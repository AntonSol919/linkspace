// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.


use std::fmt::Display;

use abe::{ast::{Ctr,  MatchErrorKind} };
use anyhow::ensure;
use linkspace_core::prelude::*;

pub const MAX_LNS_NAME_LEN : usize = MAX_PATH_LEN-1;
pub const MAX_LNS_NAME_SIZE : usize = MAX_SPATH_SIZE-8;

use thiserror::Error;

use super::CLAIM_PREFIX;

#[derive(Error, Debug, PartialEq, Copy, Clone)]
pub enum NameError{
    #[error("LNS names are colon separated bytes")]
    ContainsFSlash,
    #[error("LNS names require at least one component")]
    MinLen,
    #[error("LNS names only allow upto {MAX_LNS_NAME_LEN} components")]
    MaxLen,
    #[error("LNS names only allow upto {MAX_LNS_NAME_SIZE}-len() bytes combined")]
    MaxSize,
    #[error("LNS names are reversed spaths - {0}")]
    Path(#[from]PathError)
}

pub type NameExpr = TypedABE<Name>;

#[derive(Clone,PartialEq,Eq)]
/// A reversed spath constrained to fit the lns path
pub struct Name { spath: SPathBuf,special:Option<SpecialName>}
#[derive(Copy,Clone,PartialEq,Eq)]
pub enum SpecialName {
    Local,
    File
}
impl SpecialName {
    pub fn from(b:&[u8]) -> Option<Self>{
        match b {
            b"local" => Some(SpecialName::Local),
            b"file" | b"~" => Some(SpecialName::File),
            _ => None
        }
    }
}


impl Name {
    pub fn root() -> Self { Name{spath: SPathBuf::new(), special:None}}
    pub fn local() -> Self { Name{spath: spath_buf(&[b"local"]), special:Some(SpecialName::Local)}}
    pub(crate) fn file_path(&self) -> anyhow::Result<std::path::PathBuf>{
        ensure!(matches!(self.special , Some(SpecialName::File)));
        self.claim_ipath().collect().into_iter()
            .map(|s| match std::str::from_utf8(s)?{
                 "key" => anyhow::bail!("file keyname can't contain the word 'key'"),
                c => Ok(c)
            }).chain([Ok("key")]).try_collect()
    }
    pub fn claim_ipath(&self) -> IPathBuf { CLAIM_PREFIX.ipath().join(&self.spath).ipath()}
    pub fn claim_group(&self) -> Option<GroupID> {
        match self.special{
            Some(SpecialName::Local) => Some(PRIVATE),
            Some(SpecialName::File) => None,
            None => Some(PUBLIC),
        }
    }
    pub fn from_spath(path:&SPath) -> Result<Name,NameError>{
        let rcomps = path.collect();
        if rcomps.is_empty(){ return Err(NameError::MinLen)}
        if rcomps.len() > MAX_LNS_NAME_LEN { return Err(NameError::MaxLen)}
        let special = SpecialName::from(rcomps.first().unwrap());
        if path.spath_bytes().len() > MAX_LNS_NAME_SIZE { return Err(NameError::MaxSize)}
        Ok(Name { spath: path.into_spathbuf() ,special})
    }
    pub fn from(comps: &[&[u8]]) -> Result<Name,NameError>{
        if comps.is_empty(){ return Err(NameError::MinLen)}
        if comps.len() > MAX_LNS_NAME_LEN { return Err(NameError::MaxLen)}
        let special = SpecialName::from(comps.last().unwrap());
        let it = comps.iter().rev();
        let path = SPathBuf::try_from_iter(it)?;
        if path.spath_bytes().len() > MAX_LNS_NAME_SIZE { return Err(NameError::MaxSize)}
        Ok(Name { spath: path ,special})
    }
    pub fn spath(&self) -> &SPath {
        &self.spath
    }

    pub fn special(&self) -> Option<SpecialName> {
        self.special
    }
}
impl TryFrom<ABList> for Name {
    type Error = NameError;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        if value.lst.iter().any(|v| v.1 == Some(Ctr::FSlash)){
            return Err(NameError::ContainsFSlash);
        }
        let lst :Vec<_>= value.lst.iter().map(|v| v.0.as_slice()).collect();
        Name::from(&lst)
    }
}
impl ToABE for Name {
    fn write_abe(&self, out: &mut dyn FnMut(ABE)) {
        if self.spath.is_empty(){
            return out(ABE::Expr(ast::Expr::Lst(abev!({ "LNS-ROOT" }))))
        }
        let arr = self.spath.collect();
        let mut it = arr.iter().rev();
        let first = it.next().unwrap();
        out(ABE::Expr(ast::Expr::Bytes(first.to_vec())));
        for comp in it{
            abe!( : (**comp) ).for_each(&mut *out)
        }
    }
}
impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_abe_str())
    }
}
impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_abe_str())
    }
}


impl ABEValidator for Name {
    fn check(b: &[ABE]) -> Result<(), ast::MatchError> {
        let mut comp = 0;
        for i in b{
            match i {
                ABE::Ctr(Ctr::Colon) => comp +=1,
                ABE::Ctr(Ctr::FSlash) => {MatchErrorKind::ExpectedColon.atp::<()>(i)?;},
                ABE::Expr(_) => {},
            }
        }
        if comp > MAX_LNS_NAME_LEN{ MatchErrorKind::MaxLen { max: MAX_PATH_LEN, has: comp }.at::<()>(b)?;}
        Ok(())
    }
}
