// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.


use std::fmt::Display;

use abe::{ast::{Ctr,  MatchErrorKind} };
use anyhow::ensure;
use linkspace_core::prelude::*;

pub const MAX_LNS_NAME_LEN : usize = MAX_SPACE_DEPTH-1;
pub const MAX_LNS_NAME_SIZE : usize = MAX_SPACENAME_SIZE-8;

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
    #[error("LNS names are reversed spacenames - {0}")]
    Space(#[from]SpaceError)
}

pub type NameExpr = TypedABE<Name>;

#[derive(Clone,PartialEq,Eq)]
/// A reversed space constrained to fit lns
pub struct Name { space: SpaceBuf,name_type:NameType}
#[derive(Copy,Clone,PartialEq,Eq)]
pub enum NameType {
    Public,
    Local,
    File
}
impl NameType {
    pub fn from_tail(b:&[u8]) -> Self{
        match b {
            b"local" => NameType::Local,
            b"file" | b"~" => NameType::File,
            _ => NameType::Public
        }
    }
}


impl Name {
    pub fn root() -> Self { Name{space: SpaceBuf::new(), name_type:NameType::Public}}
    pub fn local() -> Self { Name{space: space_buf(&[b"local"]), name_type:NameType::Local}}
    pub(crate) fn file_path(&self) -> anyhow::Result<std::path::PathBuf>{
        ensure!(matches!(self.name_type , NameType::File));

        let rspace = self.claim_space();
        let lst = rspace.to_array();
        let piter = lst.iter()
            .map(|s| match std::str::from_utf8(s)?{
                 "claim.pkt" => anyhow::bail!("file keyname can't can't contain the word 'enckey'"),
                c => Ok(c)
            });
        let pb = [Ok("lns")].into_iter()
            .chain(piter)
            .chain([Ok("claim.pkt")]).try_collect()?;
        Ok(pb)
    }
    pub fn claim_space(&self) -> RootedSpaceBuf { CLAIM_PREFIX.rooted().join(&self.space).rooted()}
    pub fn claim_group(&self) -> Option<GroupID> {
        match self.name_type{
            NameType::Local => Some(PRIVATE),
            NameType::File => None,
            NameType::Public => Some(PUBLIC),
        }
    }
    pub fn from_space(space:&Space) -> Result<Name,NameError>{
        tracing::debug!(%space,"name from");
        let rcomps = space.to_array();
        if rcomps.len() > MAX_LNS_NAME_LEN { return Err(NameError::MaxLen)}
        if rcomps.is_empty(){ return Err(NameError::MinLen)}
        let name_type = NameType::from_tail(rcomps.first().unwrap());
        if space.space_bytes().len() > MAX_LNS_NAME_SIZE { return Err(NameError::MaxSize)}
        Ok(Name { space: space.into_spacebuf() ,name_type})
    }
    pub fn from(comps: &[&[u8]]) -> Result<Name,NameError>{
        if comps.is_empty(){ return Err(NameError::MinLen)}
        if comps.len() > MAX_LNS_NAME_LEN { return Err(NameError::MaxLen)}
        let special = NameType::from_tail(comps.last().unwrap());
        let it = comps.iter().rev();
        let space = SpaceBuf::try_from_iter(it)?;
        if space.space_bytes().len() > MAX_LNS_NAME_SIZE { return Err(NameError::MaxSize)}
        Ok(Name { space ,name_type: special})
    }
    pub fn space(&self) -> &Space {
        &self.space
    }

    pub fn name_type(&self) -> NameType {
        self.name_type
    }
}
impl TryFrom<ABList> for Name {
    type Error = NameError;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        if value.iter().any(|v| v.0 == Some(Ctr::FSlash)){
            return Err(NameError::ContainsFSlash);
        }
        let lst :Vec<_>= value.iter_bytes().collect();
        Name::from(&lst)
    }
}
impl ToABE for Name {
    fn write_abe(&self, out: &mut dyn FnMut(ABE)) {
        if self.space.is_empty(){
            return out(ABE::Expr(ast::Expr::Lst(abev!({ "LNS-ROOT" }))))
        }
        let arr = self.space.to_array();
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
        if comp > MAX_LNS_NAME_LEN{ MatchErrorKind::MaxLen { max: MAX_SPACE_DEPTH, has: comp }.at::<()>(b)?;}
        Ok(())
    }
}
