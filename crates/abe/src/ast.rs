// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use serde::{Serialize, Deserialize};
use std::borrow::Cow;
use std::error::Error;
use std::fmt::Write;
use std::{fmt::Debug, fmt::Display};

use crate::abtxt::{
    as_abtxt, escape_default, ABTxtError, Byte, CtrChar, STD_ERR_CSET, STD_PLAIN_CSET,
};
use crate::eval::EvalError;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ABE {
    Ctr(Ctr),
    Expr(Expr),
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Bytes(Vec<u8>),
    Lst(Vec<ABE>),
}
#[derive(Clone, PartialEq, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Ctr {
    #[serde(rename = ":")]
    Colon = b':',
    #[serde(rename = "/")]
    FSlash = b'/',
}

impl FromIterator<ABE> for ABE {
    fn from_iter<T: IntoIterator<Item = ABE>>(iter: T) -> Self {
        ABE::Expr(Expr::Lst(iter.into_iter().collect()))
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Expr::Bytes(value.as_bytes().to_vec())
    }
}
impl<const N: usize> From<[u8; N]> for Expr {
    fn from(value: [u8; N]) -> Self {
        Expr::Bytes(value.to_vec())
    }
}

impl From<String> for Expr {
    fn from(value: String) -> Self {
        Expr::Bytes(value.into_bytes())
    }
}
impl From<Vec<u8>> for Expr {
    fn from(value: Vec<u8>) -> Self {
        Expr::Bytes(value)
    }
}
impl From<&[u8]> for Expr {
    fn from(value: &[u8]) -> Self {
        Expr::Bytes(value.to_vec())
    }
}

impl From<Vec<ABE>> for Expr {
    fn from(value: Vec<ABE>) -> Self {
        Expr::Lst(value)
    }
}

impl From<Expr> for ABE {
    fn from(value: Expr) -> Self {
        ABE::Expr(value)
    }
}
impl From<Ctr> for ABE {
    fn from(value: Ctr) -> Self {
        ABE::Ctr(value)
    }
}

// Match  result. More to parse
pub type MResult<'o, V> = Result<(V, &'o [ABE]), MatchError>;
// Exact val result
pub type VResult<'o, V> = Result<V, MatchError>;

pub struct MatchError {
    pub at: String,
    pub err: MatchErrorKind,
}
impl std::error::Error for MatchError {}
impl Debug for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
impl Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} << {} >>", self.err, self.at)
    }
}

#[derive(Error, Debug)]
pub enum MatchErrorKind {
    #[error("length mismatch max {max} has {has}")]
    MaxLen{ max: usize, has: usize },

    #[error("length mismatch expect {expect} has {has}")]
    ExactLen { expect: usize, has: usize },
    #[error("minimum length mismatch expect {expect} has {has}")]
    TakeLen { expect: usize, has: usize },
    #[error("expected expr")]
    ExpectedExpr,
    #[error("expected bytes")]
    ExpectedBytes,
    #[error("expected lst")]
    ExpectedLst,
    #[error("expected '/'")]
    ExpectedFSlash,
    #[error("expected ':'")]
    ExpectedColon,
    #[error("unexpected kind")]
    Unexpected,
    #[error("{0}")]
    Other(&'static str),
}
impl MatchErrorKind {
    pub fn atp<V>(self, at: &ABE) -> MResult<V> {
        self.at(std::slice::from_ref(at))
    }
    pub fn at<V>(self, at: &[ABE]) -> MResult<V> {
        Err(MatchError {
            at: format!("{at:?}"),
            err: self,
        })
    }
    pub fn atp_v<V>(self, at: &ABE) -> VResult<V> {
        self.at_v(std::slice::from_ref(at))
    }
    pub fn at_v<V>(self, at: &[ABE]) -> VResult<V> {
        Err(MatchError {
            at: format!("{at:?}"),
            err: self,
        })
    }
}
use MatchErrorKind as ME;

pub fn take<const N: usize>(lst: &[ABE]) -> MResult<&[ABE; N]> {
    if lst.len() < N {
        ME::TakeLen {
            expect: N,
            has: lst.len(),
        }
        .at(lst)
    } else {
        Ok(lst.split_array_ref())
    }
}
pub fn take_first(lst: &[ABE]) -> MResult<&ABE> {
    let ([a], rest) = take(lst)?;
    Ok((a, rest))
}
pub fn exact<const N: usize>(lst: &[ABE]) -> VResult<&[ABE; N]> {
    match lst.try_into() {
        Ok(k) => Ok(k),
        Err(_) => ME::ExactLen {
            expect: N,
            has: lst.len(),
        }
        .at_v(lst),
    }
}
pub fn is_empty(lst: &[ABE]) -> VResult<&[ABE; 0]> {
    exact(lst)
}
pub fn as_lst(a: &ABE) -> VResult<&[ABE]> {
    match a {
        ABE::Expr(Expr::Lst(b)) => Ok(b),
        _ => ME::ExpectedLst.atp_v(a),
    }
}
pub fn one_or(a: &[ABE]) -> VResult<Option<&ABE>> {
    match a {
        [] => Ok(None),
        [s] => Ok(Some(s)),
        e => ME::ExactLen {
            expect: 0,
            has: e.len(),
        }
        .at_v(e),
    }
}
pub fn single(a: &[ABE]) -> VResult<&ABE> {
    match a {
        [s] => Ok(s),
        e => ME::ExactLen {
            expect: 0,
            has: e.len(),
        }
        .at_v(e),
    }
}
pub fn no_ctrs(a: &[ABE]) -> VResult<&[ABE]> {
    for e in a.iter() {
        if matches!(e, ABE::Ctr(_)) {
            return ME::Unexpected.atp_v(e);
        }
    }
    Ok(a)
}

pub fn as_expr(a: &ABE) -> VResult<&Expr> {
    match a {
        ABE::Expr(e) => Ok(e),
        _ => ME::ExpectedExpr.atp_v(a),
    }
}
pub fn as_abstr(a: &ABE) -> VResult<Cow<str>> {
    as_bytes(a).map(as_abtxt)
}
pub fn as_bytes(a: &ABE) -> VResult<&[u8]> {
    match a {
        ABE::Expr(Expr::Bytes(b)) => Ok(b),
        _ => ME::ExpectedBytes.atp_v(a),
    }
}
pub fn is_colon(a: &ABE) -> VResult<()> {
    match a {
        ABE::Ctr(Ctr::Colon) => Ok(()),
        _ => ME::ExpectedColon.atp_v(a),
    }
}
pub fn is_fslash(a: &ABE) -> VResult<()> {
    match a {
        ABE::Ctr(Ctr::FSlash) => Ok(()),
        _ => ME::ExpectedColon.atp_v(a),
    }
}
pub fn multi_ctr_expr(mut a: &[ABE], as_ctr: fn(&ABE) -> VResult<()>) -> (Vec<Expr>, &[ABE]) {
    let mut r = vec![];
    while let Ok((e, rest)) = take_ctr_expr(a, as_ctr) {
        a = rest;
        r.push(e.clone());
    }
    (r, a)
}
pub fn take_ctr_expr(a: &[ABE], as_ctr: fn(&ABE) -> VResult<()>) -> MResult<&Expr> {
    let ([ct, v], rest) = take(a)?;
    as_ctr(ct)?;
    as_expr(v).map(|e| (e, rest))
}
pub fn take_expr_ctr2(a: &[ABE], as_ctr: fn(&ABE) -> VResult<()>) -> MResult<&Expr> {
    let ([v, ct], rest) = take(a)?;
    as_ctr(ct)?;
    as_expr(v).map(|e| (e, rest))
}
/// check to see if the first is a match for as_ctr
pub fn strip_prefix(a: &[ABE], as_ctr: fn(&ABE) -> VResult<()>) -> VResult<&[ABE]> {
    let ([pre], rest) = take(a)?;
    as_ctr(pre)?;
    Ok(rest)
}

impl Expr {
    pub fn as_list(&self) -> Result<&[ABE], MatchError> {
        match self {
            Expr::Lst(l) => Ok(l),
            _ => Err(MatchError {
                at: self.to_string(),
                err: ME::ExpectedLst,
            }),
        }
    }
    pub fn as_bytes(&self) -> Result<&[u8], MatchError> {
        match self {
            Expr::Bytes(v) => Ok(v),
            _ => Err(MatchError {
                at: format!("lst {self}"),
                err: MatchErrorKind::ExpectedBytes,
            }),
        }
    }
    pub fn as_abstr(&self) -> Option<Cow<str>> {
        self.as_bytes().ok().map(as_abtxt)
    }
}

impl ABE {
    pub fn expr(&self) -> Result<&Expr, MatchError> {
        as_expr(self)
    }
    pub fn is_fslash(&self) -> bool {
        matches!(self, ABE::Ctr(Ctr::FSlash))
    }
    pub fn is_colon(&self) -> bool {
        matches!(self, ABE::Ctr(Ctr::Colon))
    }
}
impl Debug for ABE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ABE::Ctr(v) => Debug::fmt(v, f),
            ABE::Expr(v) => Debug::fmt(v, f),
        }
    }
}
impl Display for ABE {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ABE::Ctr(v) => Display::fmt(v, f),
            ABE::Expr(v) => Display::fmt(v, f),
        }
    }
}
impl Debug for Ctr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
impl Display for Ctr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(*self as u8 as char)
    }
}
impl Debug for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Bytes(bytes) => {
                for b in bytes {
                    f.write_str(escape_default(*b))?;
                }
                Ok(())
            }
            Expr::Lst(l) => {
                f.write_str("[")?;
                let mut it = l.iter();
                if let Some(n) = it.next() {
                    Display::fmt(n, f)?;
                    for n in it {
                        f.write_str(", ")?;
                        Display::fmt(n, f)?;
                    }
                }
                f.write_str("]")
            }
        }
    }
}
impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Bytes(bytes) => {
                for b in bytes {
                    f.write_str(escape_default(*b))?;
                }
                Ok(())
            }
            Expr::Lst(l) => {
                f.write_char('[')?;
                l.iter().try_for_each(|e| Display::fmt(e, f))?;
                f.write_char(']')
            }
        }
    }
}

use thiserror::Error;

use self::collect::ABTok;
#[derive(Error, Debug)]
pub enum ASTParseError {
    #[error("ABTxt Error {}",.0)]
    AB(#[from] ABTxtError),
    #[error("Unmatched ']'")]
    UnmatchedClose,
    #[error("Unmatched '['")]
    UnmatchedOpen,
    #[error("Newline in brackets")]
    NewlineInBrackets,
}




mod collect {
    use std::iter::Peekable;

    use crate::{abtxt::CtrChar, ABE};

    use super::{Ctr, Expr};

    #[derive(Clone, PartialEq)]
    pub enum ABTok {
        Bytes(Vec<u8>),
        Ctr(CtrChar),
    }
    /// turn a linear sequence of tokens into a ast. 
    pub fn collect(tokens: &mut Peekable<impl Iterator<Item = ABTok>>) -> Option<ABE> {
        match tokens.next() {
            None => None,
            Some(t) => Some(match t {
                ABTok::Ctr(CtrChar::Colon) => ABE::Ctr(Ctr::Colon),
                ABTok::Ctr(CtrChar::ForwardSlash) => ABE::Ctr(Ctr::FSlash),
                ABTok::Ctr(CtrChar::CloseBracket) => return None,
                ABTok::Ctr(CtrChar::OpenBracket) => ABE::Expr(Expr::Lst(
                    ::std::iter::from_fn(|| collect(tokens)).collect(),
                )),
                ABTok::Ctr(_) => todo!("Make unreachable"),
                ABTok::Bytes(mut b) => {
                    while let Some(ABTok::Bytes(o)) = tokens.peek(){
                        b.extend_from_slice(o);
                        tokens.next();
                    }
                    ABE::Expr(Expr::Bytes(b))
                },
            }),
        }
    } 
}


/// parse a string into abe. With parse_unencoded true, bytes outside the printable ascii range (0x20..0xfe) 

pub fn parse_abe(st: impl AsRef<[u8]>,parse_unencoded:bool) -> Result<Vec<ABE>, ASTParseError> {
    parse_abe_b(st.as_ref(),parse_unencoded)
}
pub fn parse_abe_b(st: &[u8],parse_unencoded:bool) -> Result<Vec<ABE>, ASTParseError> {
    if parse_unencoded{ parse_abe_with_unencoded_b(st)}
    else { parse_abe_strict_b(st)}
}

/// ALl bytes must be valid ABE. 
pub fn parse_abe_strict_b(st: &[u8]) -> Result<Vec<ABE>, ASTParseError> {
    let mut depth = 0;
    let mut r: Vec<ABTok> = vec![];
    let mut bytes = vec![];
    let mut todo = st;
    let len = todo.len();
    loop {
        match crate::abtxt::next_byte(todo, len - todo.len(), STD_PLAIN_CSET, STD_ERR_CSET)? {
            Byte::Finished => {
                if depth != 0 {
                    return Err(ASTParseError::UnmatchedOpen);
                }
                if !bytes.is_empty() {
                    r.push(ABTok::Bytes(bytes));
                }
                let mut it = r.into_iter().peekable();
                return Ok(std::iter::from_fn(|| collect::collect(&mut it)).collect());
            }
            Byte::Ctr { kind, rest } => {
                todo = rest;
                // Open Brackets increase the depth, and match their close brackets
                match kind {
                    CtrChar::OpenBracket => depth += 1,
                    CtrChar::CloseBracket if depth == 0 => {
                        return Err(ASTParseError::UnmatchedClose)
                    }
                    CtrChar::CloseBracket => depth -= 1,
                    _ => {}
                };
                if !bytes.is_empty() {
                    r.push(ABTok::Bytes(std::mem::take(&mut bytes)));
                }
                r.push(ABTok::Ctr(kind))
            }
            Byte::Byte { byte, rest } => {
                todo = rest;
                bytes.push(byte);
            }
        }
    }
}


/// In contrast to [[parse_abe_strict_b]] this function does not error bytes outside the range 0x20..0xfe, but reads them as-is. 
pub fn parse_abe_with_unencoded_b(st: &[u8]) -> Result<Vec<ABE>, ASTParseError> {
    let mut depth = 0;
    let mut r: Vec<ABTok> = vec![];
    let mut bytes = vec![];
    let mut todo = st;
    let len = todo.len();
    loop {
        let b = match crate::abtxt::next_byte(todo, len - todo.len(), STD_PLAIN_CSET,STD_ERR_CSET){
            Ok(o) => o,
            Err(ABTxtError::ParseError { byte, idx:_}) => {todo = &todo[1..];bytes.push(byte); continue;}
            Err(e) => return Err(e.into())
        };
        match b{
            Byte::Finished => {
                if depth != 0 {
                    return Err(ASTParseError::UnmatchedOpen);
                }
                if !bytes.is_empty() {
                    r.push(ABTok::Bytes(bytes));
                }
                let mut it = r.into_iter().peekable();
                return Ok(std::iter::from_fn(|| collect::collect(&mut it)).collect());
            }
            Byte::Ctr { kind, rest } => {
                todo = rest;
                // Open Brackets increase the depth, and match their close brackets
                match kind {
                    CtrChar::OpenBracket => depth += 1,
                    CtrChar::CloseBracket if depth == 0 => {
                        return Err(ASTParseError::UnmatchedClose)
                    }
                    CtrChar::CloseBracket => depth -= 1,
                    _ => {}
                };
                if !bytes.is_empty() {
                    r.push(ABTok::Bytes(std::mem::take(&mut bytes)));
                }
                r.push(ABTok::Ctr(kind))
            }
            Byte::Byte { byte, rest } => {
                todo = rest;
                bytes.push(byte);
            }
        }
    }
}


/// Split abe into top level components
/// See next_byte for the meaning of cset
pub fn tokenize_abe(
    st: &str,
    plain_cset: u32,
    err_cset: u32,
) -> Result<Vec<(u8,bool,&str)>, ASTParseError> {
    Ok(tokenize_abe_b(st.as_bytes(), plain_cset, err_cset)
        .map(|i| i.map(|(c,i, b)| (c,i,unsafe { std::str::from_utf8_unchecked(b) })))
        .try_collect()?)
}

#[test]
pub fn abesplit(){
    
    let v : Vec<_> = tokenize_abe_b(b"hello/world\nok",0, 0).try_collect().unwrap();
    assert_eq!(&*v,&[(0,false,b"hello" as &[u8]),(b'/',false,b"world"),(b'\n',false,b"ok")]);

    let _v : Vec<_> = tokenize_abe_b(b"[test[thing]]",0, 0).try_collect().unwrap();
}

/// Split abe into top level components
pub fn tokenize_abe_b(
    st: &[u8],
    plain_cset: u32,
    err_cset: u32,
) -> impl Iterator<Item=Result<(u8,bool,&[u8]),ASTParseError>>{
    let mut depth = 0;
    let mut head_ctr = 0;
    let mut todo = st;
    let mut start_comp = 0;
    let mut with_inner = false;
    let len = todo.len();
    std::iter::from_fn(move ||{
        if todo.is_empty(){ return None};
        loop {
            let byte = match crate::abtxt::next_byte(todo, len - todo.len(), plain_cset, err_cset){
                Ok(b) => b,
                Err(e) => return Some(Err(e.into()))
            } ;
            match byte {
                Byte::Finished => {
                    if depth != 0 {
                        return Some(Err(ASTParseError::UnmatchedOpen));
                    }
                    let at = len - todo.len();
                    todo = &[];
                    return Some(Ok((head_ctr,with_inner,&st[start_comp..at])));
                }
                Byte::Ctr { kind, rest } => {
                    if depth == 0 && !kind.is_bracket() {
                        let at = len - todo.len();
                        let r = (head_ctr,std::mem::take(&mut with_inner),&st[start_comp..at]);
                        todo = rest;
                        start_comp = len - todo.len();
                        head_ctr = kind.as_char();
                        return Some(Ok(r));
                    } else {
                        with_inner = true;
                        todo = rest;
                        match kind {
                            CtrChar::OpenBracket => depth += 1,
                            CtrChar::CloseBracket if depth == 0 => {
                                return Some(Err(ASTParseError::UnmatchedClose))
                            }
                            CtrChar::CloseBracket => depth -= 1,
                            _ => {}
                        }
                    }
                }
                Byte::Byte { rest, .. } => {
                    todo = rest;
                }
            }
        }
    })
}
                       
pub fn print_abe<B: std::borrow::Borrow<ABE>>(v: impl IntoIterator<Item = B>) -> String {
    v.into_iter().map(|v| v.borrow().to_string()).collect()
}

/// replace occurances of pattern with new. Checked depth first with no overlap
pub fn replace_abe(inp: &mut Vec<ABE>, pattern: &[ABE], new: &[ABE]) {
    for el in inp.iter_mut() {
        if let ABE::Expr(Expr::Lst(ref mut lst)) = el {
            replace_abe(lst, pattern, new)
        }
    }
    let mut i = 0;
    while let Some(r) = &inp[i..].windows(pattern.len()).position(|w| w == pattern) {
        i += r;
        inp.splice(i..(i + pattern.len()), new.to_vec());
        i += new.len();
    }
}
#[test]
fn replace() {
    use crate::*;
    let mut v = abev!( "hello" : "world");
    let find: Vec<_> = abev!("hello");
    let val: Vec<_> = abev!("world");
    replace_abe(&mut v, &find, &val);
    assert_eq!(v, abev!("world" : "world"));
    let mut v = abev!( "hello" : { "hello" / "hello" });
    let find: Vec<_> = abev!( / "hello");
    replace_abe(&mut v, &find, &val);
    assert_eq!(v, abev!("hello" : { "hello" "world"}));
}

#[derive(Debug)]
pub enum ABEError<E> {
    TryFrom(E),
    MatchError(MatchError),
    Eval(EvalError),
    Parse(ASTParseError),
}
impl<E: std::fmt::Display> Display for ABEError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ABEError::TryFrom(e) => Display::fmt(e, f),
            ABEError::MatchError(e) => Display::fmt(e, f),
            ABEError::Eval(e) => Display::fmt(e, f),
            ABEError::Parse(e) => Display::fmt(e, f),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for ABEError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ABEError::TryFrom(o) => Some(o),
            ABEError::MatchError(o) => Some(o),
            ABEError::Eval(o) => Some(o),
            ABEError::Parse(o) => Some(o),
        }
    }
}
