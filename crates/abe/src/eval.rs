// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
This stuff is a mess.
**/
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::{collections::HashSet, convert::Infallible, ops::FromResidual, str::FromStr};
use thiserror::Error;

use crate::abtxt::as_abtxt;
use crate::abtxt::{as_abtxt_e, MAX_STR};
use crate::{ast::*, cut_ending_nulls2, cut_prefix_nulls};

const fn as_str(o: Option<Ctr>) -> &'static str {
    match o {
        Some(Ctr::Colon) => ":",
        Some(Ctr::FSlash) => "/",
        None => "",
    }
}
// would benefit from Vec<(TinyVec<32>,..)
pub enum ABItem<B = Vec<u8>> {
    Ctr(Ctr),
    Bytes(B),
}
impl From<ABItem> for ABE {
    fn from(value: ABItem) -> Self {
        match value {
            ABItem::Ctr(c) => ABE::Ctr(c),
            ABItem::Bytes(b) => ABE::Expr(Expr::Bytes(b)),
        }
    }
}

macro_rules! dbgprintln {
    ($($arg:tt)*) => {{
        if false{eprintln!($($arg)*)}
    }};
}

/** a list of (bytes,ctr) components.
It is an error for the lst to be empty.
**/
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ABList {
    pub lst: Vec<(Vec<u8>, Option<Ctr>)>,
}
impl From<ABList> for Vec<u8> {
    fn from(v: ABList) -> Self {
        v.concat()
    }
}
impl From<Vec<u8>> for ABList {
    fn from(value: Vec<u8>) -> Self {
        ABList {
            lst: vec![(value, None)],
        }
    }
}
impl From<ABList> for Vec<ABE> {
    fn from(val: ABList) -> Self {
        val.items().map(|i| i.into()).collect()
    }
}
pub type ABLIter = impl Iterator<Item = ABE>;
impl IntoIterator for ABList {
    type Item = ABE;
    type IntoIter = ABLIter;
    fn into_iter(self) -> Self::IntoIter {
        self.items().map(|i| i.into())
    }
}

impl From<&[u8]> for ABList {
    fn from(b: &[u8]) -> Self {
        Self::default().push_bytes(b)
    }
}
impl FromIterator<ABItem> for ABList {
    fn from_iter<T: IntoIterator<Item = ABItem>>(iter: T) -> Self {
        iter.into_iter().fold(ABList::default(), |a, i| a.push(i))
    }
}
pub fn abl<I: IntoIterator<Item = A>, A: AsRef<[u8]>>(bytes: I) -> ABList {
    abld(bytes, Ctr::Colon)
}
pub fn ablf<I: IntoIterator<Item = A>, A: AsRef<[u8]>>(bytes: I) -> ABList {
    abld(bytes, Ctr::FSlash)
}
pub fn abld<I: IntoIterator<Item = A>, A: AsRef<[u8]>>(bytes: I, delimiter: Ctr) -> ABList {
    let mut lst: Vec<_> = bytes
        .into_iter()
        .map(|b| (b.as_ref().to_vec(), Some(delimiter)))
        .collect();
    if let Some((_, c)) = lst.last_mut() {
        *c = None;
    }
    ABList { lst }
}

impl Display for ABList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (bytes, ctr) in &self.lst {
            f.write_str(&as_abtxt(bytes))?;
            f.write_str(as_str(*ctr))?;
        }
        Ok(())
    }
}

impl Debug for ABList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct ABDebug<I>(I, Option<Ctr>);
        impl<I: AsRef<[u8]>> Debug for ABDebug<I> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&as_abtxt_e(self.0.as_ref()))
            }
        }
        f.debug_list()
            .entries(self.lst.iter().map(|(v, c)| (ABDebug(v, *c), c)))
            .finish()
    }
}
impl Default for ABList {
    fn default() -> Self {
        Self {
            lst: vec![(vec![], None)],
        }
    }
}
pub fn iter_test() {
    fn a() -> ABList {
        ABList::default()
    }
    #[track_caller]
    fn eq(a: &ABList, b: &[Vec<&[u8]>]) {
        let b = b
            .iter()
            .map(|v: &Vec<_>| (v[0], v[1..].to_vec()))
            .collect::<Vec<(&[u8], Vec<&[u8]>)>>();
        assert_eq!(a.iter_fslash().collect::<Vec<(&[u8], Vec<&[u8]>)>>(), b)
    }
    eq(&a(), &[vec![&[]]]);
    eq(&a().push_ctr(Ctr::Colon), &[vec![&[], &[]]]);
    eq(&a().push_ctr(Ctr::FSlash), &[vec![&[]], vec![&[]]]);
    eq(
        &a().push_ctr(Ctr::FSlash).push_bytes(b"cc"),
        &[vec![&[]], vec![b"cc"]],
    );
    eq(
        &a().push_ctr(Ctr::Colon).push_bytes(b"aa"),
        &[vec![&[], b"aa"]],
    );
    let f = a()
        .push_bytes(b"aa")
        .push_ctr(Ctr::Colon)
        .push_bytes(b"bb")
        .push_ctr(Ctr::FSlash)
        .push_bytes(b"cc")
        .push_ctr(Ctr::Colon)
        .push_bytes(b"dd");

    eq(&f, &[vec![b"aa", b"bb"], vec![b"cc", b"dd"]]);
}
impl ABList {
    /**
    Return an iterator over colon delim vecs:
    Key take away is that every returned item (vec) has at least a head, but the head can be empty
    aa:bb/cc:dd => [aa,bb] , [cc,dd]
    aa/cc => [aa],[cc]
    aa/ => [aa],[ [] ]
    aa => [aa]
    aa: => [aa, [] ]
    :aa => [[],aa ]
    /cc => [ [] ] , [cc]
    /  =>  [ [] ] , [ [] ]  ( 2 items )
    : => [ [] , [] ] ( 1 items with 2 empty elem )
    '' (empty) => [ [] ]
    **/
    fn iter_fslash(&self) -> impl Iterator<Item = (&[u8], Vec<&[u8]>)> {
        let it = self.lst.split_inclusive(|(_, c)| *c == Some(Ctr::FSlash));
        it.map(|ls| {
            let mut it = ls.iter().map(|(b, _c)| b.as_slice());
            (it.next().unwrap(), it.collect())
        })
    }
    pub fn inner(&self) -> &[(Vec<u8>, Option<Ctr>)] {
        &self.lst
    }

    pub fn items(self) -> impl Iterator<Item = ABItem> {
        self.lst.into_iter().flat_map(|(a, b)| {
            if a.is_empty() {
                None
            } else {
                Some(ABItem::Bytes(a))
            }
            .into_iter()
            .chain(b.map(ABItem::Ctr))
        })
    }
    pub fn item_refs(&self) -> impl Iterator<Item = ABItem<&[u8]>> {
        self.lst.iter().flat_map(|(a, b)| {
            if a.is_empty() {
                None
            } else {
                Some(ABItem::Bytes(a.as_slice()))
            }
            .into_iter()
            .chain(b.map(ABItem::Ctr))
        })
    }
    pub fn push(self, item: ABItem) -> Self {
        match item {
            ABItem::Ctr(c) => self.push_ctr(c),
            ABItem::Bytes(b) => self.push_bytes(b.as_ref()),
        }
    }
    pub fn push_bytes(mut self, b: &[u8]) -> Self {
        self.lst.last_mut().unwrap().0.extend_from_slice(b);
        self
    }
    pub fn push_ctr(mut self, ctr: Ctr) -> Self {
        self.lst.last_mut().unwrap().1 = Some(ctr);
        self.lst.push((vec![], None));
        self
    }
    pub fn into_exact_bytes(mut self) -> Result<Vec<u8>, Self> {
        if self.lst.len() == 1 {
            Ok(self.lst.pop().unwrap().0)
        } else {
            Err(self)
        }
    }
    pub fn as_exact_bytes(&self) -> Result<&[u8], &Self> {
        if self.lst.len() == 1 {
            Ok(&self.lst[0].0)
        } else {
            Err(self)
        }
    }
    pub fn take_prefix_bytes(&mut self, v: usize) -> Vec<u8> {
        let (a, _) = self.lst.first_mut().unwrap();
        let v = a.split_off(v);
        std::mem::replace(a, v)
    }

    /// Merges (bytes, ctr) into a sequence of bytes.
    /// this destroys ctr byte information. i.e.  [("/",:)] becomes /: and reparsing becomes [("",/),("",:)]
    /// Use display to print propery escaped values.
    pub fn concat(mut self) -> Vec<u8> {
        if let ([(bytes, ctr)], rest) = self.lst.split_at_mut(1) {
            bytes.extend_from_slice(as_str(*ctr).as_bytes());
            for (b, c) in rest {
                bytes.extend_from_slice(b);
                bytes.extend_from_slice(as_str(*c).as_bytes());
            }
        } else {
            unreachable!()
        }
        self.lst.into_iter().next().unwrap().0
    }

    pub fn bytes_2(&self) -> impl Iterator<Item = &[u8]> {
        self.lst
            .iter()
            .flat_map(|(b, ctr)| [b.as_slice(), as_str(*ctr).as_bytes()])
            .filter(|v| !v.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.lst.is_empty() || self.as_exact_bytes().map(|v| v.is_empty()).unwrap_or(false)
    }
}
pub type ApplyErr = Box<dyn std::error::Error + Send + Sync + 'static>;
pub enum ApplyResult {
    None,
    Ok(Vec<u8>),
    Err(ApplyErr),
}
impl ApplyResult {
    pub fn into_opt(self) -> Option<Result<Vec<u8>, ApplyErr>> {
        match self {
            ApplyResult::None => None,
            ApplyResult::Ok(v) => Some(Ok(v)),
            ApplyResult::Err(e) => Some(Err(e)),
        }
    }
    pub fn arg_err<X: AsRef<[u8]>>(args: impl IntoIterator<Item = X>, expect: &str) -> Self {
        ApplyResult::Err(format!("expect {expect} but got {:?}", abl(args)).into())
    }
}
impl FromResidual<Option<Infallible>> for ApplyResult {
    fn from_residual(_residual: Option<Infallible>) -> Self {
        AR::None
    }
}
impl<V: Into<ApplyErr>> From<Option<Result<Vec<u8>, V>>> for ApplyResult {
    fn from(v: Option<Result<Vec<u8>, V>>) -> Self {
        match v {
            Some(v) => v.map_err(Into::into).into(),
            None => AR::None,
        }
    }
}
impl<V: Into<ApplyErr>> FromResidual<Result<Infallible, V>> for ApplyResult {
    fn from_residual(residual: Result<Infallible, V>) -> Self {
        AR::Err(residual.unwrap_err().into())
    }
}

impl From<Result<Vec<u8>, ApplyErr>> for ApplyResult {
    fn from(v: Result<Vec<u8>, ApplyErr>) -> Self {
        match v {
            Ok(o) => AR::Ok(o),
            Err(e) => AR::Err(e),
        }
    }
}
impl ApplyResult {
    pub fn or_else(self, map: impl FnOnce() -> ApplyResult) -> Self {
        if matches!(self, AR::None) {
            map()
        } else {
            self
        }
    }
}
use ApplyResult as AR;

#[derive(Error)]
pub enum EvalError {
    #[error("evaluator err :  {} {}",as_abtxt_e(.0),.1)]
    SubEval(Vec<u8>, ApplyErr),
    #[error("no such e-func : '/{}'",as_abtxt_e(.0))]
    NoSuchSubEval(Vec<u8>),
    #[error("no such func : {}",as_abtxt_e(.0))]
    NoSuchFunc(Vec<u8>),
    #[error("func error : {} : {}",as_abtxt_e(.0), .1)]
    Func(Vec<u8>, ApplyErr),
    #[error("other error {}",.0)]
    Other(String),
}

impl std::fmt::Debug for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

pub type Describer<'cb> = &'cb mut dyn FnMut(
    &str,
    &str,
    &mut dyn Iterator<Item = ScopeFuncInfo>,
    &mut dyn Iterator<Item = ScopeEvalInfo>,
);
pub trait Scope {
    fn lookup_eval(&self, _id: &[u8], _abe: &[ABE], _ctx: &dyn Scope) -> ApplyResult {
        ApplyResult::None
    }
    fn lookup_apply(
        &self,
        id: &[u8],
        inp_and_args: &[&[u8]],
        init: bool,
        ctx: &dyn Scope,
    ) -> ApplyResult;
    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult;
    fn describe(&self, cb: Describer) {
        cb("todo", "", &mut std::iter::empty(), &mut std::iter::empty())
    }
}
#[derive(Copy, Clone)]
pub struct EScope<T>(pub T);
impl<T: EvalScopeImpl> Scope for EScope<T> {
    fn lookup_apply(&self, id: &[u8], inp: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        for ScopeFunc { apply, info, .. } in self.0.list_funcs() {
            if info.id.as_bytes() == id {
                if info.init_eq.is_some() && info.init_eq != Some(init) {
                    return ApplyResult::Err("function can not be applied this way".into());
                }
                if !info.argc.contains(&inp.len()) {
                    return ApplyResult::arg_err(inp, &format!("between {:?}", info.argc));
                }
                return apply(&self.0, inp, init, ctx);
            }
        }
        ApplyResult::None
    }

    fn describe(&self, cb: Describer) {
        let mut fncx = self.0.list_funcs().iter().map(|v| v.info.clone());
        let mut evls = self.0.list_eval().iter().map(|v| v.info);
        let (name, info) = self.0.about();
        cb(&name, &info, &mut fncx, &mut evls);
    }
    fn lookup_eval(&self, id: &[u8], abe: &[ABE], funcs: &dyn Scope) -> ApplyResult {
        if let Some(e) = self
            .0
            .list_eval()
            .iter()
            .find(|i| i.info.id.as_bytes() == id)
        {
            return (e.apply)(&self.0, abe, funcs);
        }
        ApplyResult::None
    }

    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult {
        for ScopeFunc { to_abe, info, .. } in self.0.list_funcs() {
            if info.to_abe && info.id.as_bytes() == id {
                return to_abe(&self.0, bytes, options);
            }
        }
        AR::None
    }
}
pub trait EvalScopeImpl {
    fn about(&self) -> (String, String) {
        ("".into(), "".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[]
    }
    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[]
    }
}
#[derive(Clone)]
pub struct ScopeFunc<T> {
    pub apply: fn(T, &[&[u8]], bool, &dyn Scope) -> ApplyResult,
    pub to_abe: fn(T, &[u8], &[ABE]) -> ApplyResult,
    pub info: ScopeFuncInfo,
}
#[derive(Clone)]
pub struct ScopeFuncInfo {
    pub id: &'static str,
    pub init_eq: Option<bool>,
    pub to_abe: bool,
    pub argc: std::ops::RangeInclusive<usize>,
    pub help: &'static str,
}
pub struct ScopeEval<T> {
    pub apply: fn(T, &[ABE], &dyn Scope) -> ApplyResult,
    pub info: ScopeEvalInfo,
}
#[derive(Copy, Clone)]
pub struct ScopeEvalInfo {
    pub id: &'static str,
    pub help: &'static str,
}

impl Scope for () {
    fn lookup_apply(
        &self,
        _id: &[u8],
        _args: &[&[u8]],
        _init: bool,
        _ctx: &dyn Scope,
    ) -> ApplyResult {
        AR::None
    }

    fn lookup_eval(&self, _id: &[u8], _abe: &[ABE], _scopes: &dyn Scope) -> ApplyResult {
        AR::None
    }

    fn describe(&self, cb: Describer) {
        cb("()", "", &mut std::iter::empty(), &mut std::iter::empty())
    }

    fn encode(&self, _id: &[u8], _options: &[ABE], _bytes: &[u8]) -> ApplyResult {
        AR::None
    }
}
impl<A: Scope> Scope for Option<A> {
    fn lookup_apply(
        &self,
        id: &[u8],
        inpt_and_args: &[&[u8]],
        init: bool,
        ctx: &dyn Scope,
    ) -> ApplyResult {
        self.as_ref()
            .map(|x| x.lookup_apply(id, inpt_and_args, init, ctx))
            .unwrap_or(ApplyResult::None)
    }
    fn describe(&self, cb: Describer) {
        match self {
            Some(s) => s.describe(cb),
            None => cb(
                &format!("Unset<{}>", std::any::type_name::<A>()),
                "",
                &mut std::iter::empty(),
                &mut std::iter::empty(),
            ),
        }
    }
    fn lookup_eval(&self, id: &[u8], abe: &[ABE], scopes: &dyn Scope) -> ApplyResult {
        self.as_ref()
            .map(|x| x.lookup_eval(id, abe, scopes))
            .unwrap_or(ApplyResult::None)
    }
    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult {
        self.as_ref()
            .map(|v| v.encode(id, options, bytes))
            .unwrap_or(AR::None)
    }
}

impl Scope for &dyn Scope {
    fn lookup_eval(&self, id: &[u8], abe: &[ABE], scopes: &dyn Scope) -> ApplyResult {
        (**self).lookup_eval(id, abe, scopes)
    }
    fn lookup_apply(
        &self,
        id: &[u8],
        inpt_and_args: &[&[u8]],
        init: bool,
        ctx: &dyn Scope,
    ) -> ApplyResult {
        (**self).lookup_apply(id, inpt_and_args, init, ctx)
    }

    fn describe(&self, cb: Describer) {
        (**self).describe(cb)
    }

    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult {
        (**self).encode(id, options, bytes)
    }
}
impl<A: Scope> Scope for &A {
    fn lookup_eval(&self, id: &[u8], abe: &[ABE], scopes: &dyn Scope) -> ApplyResult {
        (**self).lookup_eval(id, abe, scopes)
    }

    fn lookup_apply(&self, id: &[u8], args: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        (**self).lookup_apply(id, args, init, ctx)
    }
    fn describe(&self, cb: Describer) {
        (**self).describe(cb)
    }
    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult {
        (**self).encode(id, options, bytes)
    }
}
impl<A: Scope, B: Scope> Scope for (A, B) {
    #[inline(always)]
    fn lookup_apply(&self, id: &[u8], args: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        self.0
            .lookup_apply(id, args, init, ctx)
            .or_else(|| self.1.lookup_apply(id, args, init, ctx))
    }
    fn lookup_eval(&self, id: &[u8], abe: &[ABE], scope: &dyn Scope) -> ApplyResult {
        self.0
            .lookup_eval(id, abe, scope)
            .or_else(|| self.1.lookup_eval(id, abe, scope))
    }
    fn describe(&self, cb: Describer) {
        self.0.describe(cb);
        self.1.describe(cb);
    }
    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult {
        self.0
            .encode(id, options, bytes)
            .or_else(|| self.1.encode(id, options, bytes))
    }
}
impl<A: Scope, B: Scope, C: Scope> Scope for (A, B, C) {
    #[inline(always)]
    fn lookup_apply(&self, id: &[u8], args: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        ((&self.0, &self.1), &self.2).lookup_apply(id, args, init, ctx)
    }
    fn lookup_eval(&self, id: &[u8], abe: &[ABE], scope: &dyn Scope) -> ApplyResult {
        ((&self.0, &self.1), &self.2).lookup_eval(id, abe, scope)
    }

    fn describe(&self, cb: Describer) {
        self.0.describe(cb);
        self.1.describe(cb);
        self.2.describe(cb);
    }

    fn encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult {
        self.0
            .encode(id, options, bytes)
            .or_else(|| self.1.encode(id, options, bytes))
            .or_else(|| self.2.encode(id, options, bytes))
    }
}

#[derive(Copy, Clone, Default)]
pub struct EvalCtx<SCOPE> {
    pub scope: SCOPE,
}

impl<B: Scope> EvalCtx<B> {
    pub fn scope<F2: Scope>(self, e: F2) -> EvalCtx<(B, F2)> {
        EvalCtx {
            scope: (self.scope, e),
        }
    }
    pub fn pre_scope<F2: Scope>(self, e: F2) -> EvalCtx<(F2, B)> {
        EvalCtx {
            scope: (e, self.scope),
        }
    }
    pub fn reref(&self) -> EvalCtx<&B> {
        EvalCtx { scope: &self.scope }
    }
    pub fn dynr(&self) -> EvalCtx<&dyn Scope> {
        EvalCtx { scope: &self.scope }
    }
    pub fn boxed<'b>(self) -> EvalCtx<Box<dyn Scope + 'b>>
    where
        B: 'b,
    {
        EvalCtx {
            scope: Box::new(self.scope),
        }
    }
}

fn match_expr(depth: usize, ctx: &EvalCtx<impl Scope>, expr: &ABE) -> Result<ABItem, EvalError> {
    match expr {
        ABE::Ctr(c) => {
            dbgprintln!("Match Ctr({c})  (depth={depth})");
            Ok(ABItem::Ctr(*c))
        }
        ABE::Expr(Expr::Bytes(b)) => {
            dbgprintln!("Match bytes({}) (depth={depth})", as_abtxt_e(b));
            Ok(ABItem::Bytes(b.to_vec()))
        }
        ABE::Expr(Expr::Lst(ls)) => {
            dbgprintln!("Match lst[{}] (depth={depth})", ls.len());
            let inner_abl = match ls.as_slice() {
                [ABE::Ctr(Ctr::FSlash), ref tail @ ..] => {
                    let (id, rest): (&[u8], &[ABE]) = match tail {
                        [] => return Err(EvalError::Other("missing eval name".into())),
                        [ABE::Expr(Expr::Lst(_)), ..] => {
                            return Err(EvalError::Other(
                                "var eval name resolution disabled".into(),
                            ))
                        }
                        [ABE::Ctr(Ctr::Colon), ref rest @ ..] => {
                            let mut result = vec![];
                            dump_abe_bytes(&mut result, rest);
                            return Ok(ABItem::Bytes(result));
                        }
                        // enable {//...}
                        [ABE::Ctr(Ctr::FSlash), ..] => (&[], tail),
                        [ABE::Expr(Expr::Bytes(ref id)), ref r @ ..] => (id, r),
                    };
                    dbgprintln!("Eval({})", as_abtxt_e(id));
                    match ctx.scope.lookup_eval(id, rest, &ctx.scope) {
                        ApplyResult::None => return Err(EvalError::NoSuchSubEval(id.to_vec())),
                        ApplyResult::Ok(b) => return Ok(ABItem::Bytes(b)),
                        ApplyResult::Err(e) => return Err(EvalError::SubEval(id.to_vec(), e)),
                    }
                }
                [ABE::Expr(Expr::Lst(_)), ..] => {
                    Err(EvalError::Other("var name resolution disabled".into()))?
                }
                _ => _eval(depth + 1, ctx, ls)?,
            };

            fn call(
                scope: &impl Scope,
                id: &[u8],
                input_and_args: &[&[u8]],
                init: bool,
            ) -> Result<Vec<u8>, EvalError> {
                dbgprintln!(
                    "Call({init},id={},inp={:?} )",
                    as_abtxt_e(id),
                    input_and_args
                );
                match scope.lookup_apply(id, input_and_args, init, &scope) {
                    ApplyResult::None => Err(EvalError::NoSuchFunc(id.to_vec())),
                    ApplyResult::Ok(b) => Ok(b),
                    ApplyResult::Err(e) => Err(EvalError::Func(id.to_vec(), e)),
                }
            }
            let it = inner_abl
                .lst
                .split_inclusive(|(_, c)| *c == Some(Ctr::FSlash));
            let mut calls = it.map(|ls| ls.iter().map(|(b, _c)| b.as_slice()));
            let mut stack = [&[] as &[u8]; 16];
            let mut init_id_args = match calls.next() {
                None => return Err(EvalError::Other("empty {{}} not enabled".into())),
                Some(v) => v,
            };
            let mut id = init_id_args.next().unwrap_or(&[]);
            let argc = stack
                .iter_mut()
                .zip(&mut init_id_args)
                .fold(0, |i, (slot, slice)| {
                    *slot = slice;
                    i + 1
                });
            if init_id_args.next().is_some() {
                return Err(EvalError::Other("more than 16 args not supported".into()));
            }
            let args = &stack[..argc];
            dbgprintln!("Start: '{}' - {:?} ", as_abtxt_e(id), args);
            let mut bytes = call(&ctx.scope, id, args, true)?;
            for mut id_and_args in calls {
                stack = [&[] as &[u8]; 16];
                id = id_and_args.next().unwrap_or(&[]);
                stack[0] = bytes.as_slice();
                let argc =
                    1 + stack[1..]
                        .iter_mut()
                        .zip(&mut id_and_args)
                        .fold(0, |i, (slot, slice)| {
                            *slot = slice;
                            i + 1
                        });
                if id_and_args.next().is_some() {
                    return Err(EvalError::Other("more than 16 args not supported".into()));
                }
                let args = &stack[..argc];
                dbgprintln!(
                    "'{}' -> '{}' :: {:?} ::",
                    as_abtxt_e(&bytes),
                    as_abtxt_e(id),
                    args
                );
                bytes = call(&ctx.scope, id, args, false)?;
            }
            Ok(ABItem::Bytes(bytes))
        }
    }
}

pub fn eval(ctx: &EvalCtx<impl Scope>, abe: &[ABE]) -> std::result::Result<ABList, EvalError> {
    dbgprintln!("init ({})", print_abe(abe));
    match _eval(0, ctx, abe) {
        Ok(l) => {
            dbgprintln!("result ({})", l);
            Ok(l)
        }
        Err(e) => {
            dbgprintln!("result Err({})", e);
            Err(e)
        }
    }
}
pub fn _eval(
    depth: usize,
    ctx: &EvalCtx<impl Scope>,
    abe: &[ABE],
) -> std::result::Result<ABList, EvalError> {
    abe.iter()
        .map(|expr| match_expr(depth, ctx, expr))
        .try_collect()
}

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("option encoding error expected [ scope [:{{opts}}]? ]*  ")]
    OptionError,
    #[error("option parse error{}",.0)]
    ParseError(#[source] ASTParseError),
    #[error("scope '{}' encode {}",.0,.1)]
    ScopeEncode(String, ApplyErr),
}

/// Encode bytes with scopes given as "scope1:{opts}/scope2:{opts}/"
/// :scope/{opts}:
/// e.g. lns:{known}/b64/uint will attempt to encode bytes through locally known lns, otherwise use b64, and finnally attempt uint
pub fn encode(
    ctx: &EvalCtx<impl Scope>,
    bytes: &[u8],
    options: &str,
) -> std::result::Result<String, EncodeError> {
    let lst = parse_abe(options).map_err(EncodeError::ParseError)?;
    encode_abe(ctx, bytes, &lst)
}
pub fn encode_abe(
    ctx: &EvalCtx<impl Scope>,
    bytes: &[u8],
    options: &[ABE],
) -> std::result::Result<String, EncodeError> {
    let mut it = options.split(|v| v.is_fslash());
    if options.is_empty() {
        it.next();
    }
    for scope_opts in it {
        let (func_id, mut args) = take_first(scope_opts).map_err(|_| EncodeError::OptionError)?;
        if !args.is_empty() {
            args = strip_prefix(args, is_colon).map_err(|_| EncodeError::OptionError)?;
        }
        let func_id = as_bytes(func_id).map_err(|_| EncodeError::OptionError)?;
        //eprintln!("Try {}",as_abtxt(func_id));
        match ctx.scope.encode(func_id, args, bytes) {
            ApplyResult::None => {}
            ApplyResult::Ok(r) => {
                debug_assert!(
                    eval(ctx, &parse_abe_b(&r).expect("bug: encode fmt"))
                        .unwrap_or_else(|_| panic!("bug: encode-eval ({})", &as_abtxt(&r)))
                        .as_exact_bytes()
                        .expect("bug: encode multi")
                        == bytes,
                    "bug: eval(encode)"
                );
                let st = String::from_utf8(r).unwrap();
                //eprintln!("ok {st}");
                return Ok(st);
            }
            ApplyResult::Err(e) => {
                return Err(EncodeError::ScopeEncode(as_abtxt_e(func_id).to_string(), e))
            }
        }
    }
    Ok(as_abtxt(bytes).to_string())
}

#[macro_export]
macro_rules! eval_fnc {
    ( $id:expr, $help:literal,$fnc:expr) => {
        $crate::eval::ScopeEval {
            info: $crate::eval::ScopeEvalInfo {
                id: $id,
                help: $help,
            },
            apply: |a, b: &[ABE], c| -> $crate::eval::ApplyResult {
                let r: Result<Vec<u8>, $crate::eval::ApplyErr> = $fnc(a, b, c);
                $crate::eval::ApplyResult::from(r)
            },
        }
    };
}
#[macro_export]
macro_rules! fnc {
    ( $id:expr, $argc:expr, $help:literal,$fnc:expr, { id : $to_abe:expr}) => {
        $crate::fnc!($id,$argc,None,$help,$fnc,
                     |_,bytes:&[u8],options:&[$crate::ABE]| -> $crate::eval::ApplyResult{
                         let st : Option<String> = $to_abe(bytes,options);
                         match st {
                             None => $crate::eval::ApplyResult::None,
                             Some(st) => $crate::eval::ApplyResult::Ok(format!("{{{}:{}}}",$id,st).into_bytes())
                         }
                     }
        )
    };

    ( $id:expr, $argc:expr, $help:literal,$fnc:expr) => {
        $crate::fnc!($id,$argc,None,$help,$fnc)
    };
    ( $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr ) => {
        $crate::fnc!(@C $id , $argc , $init, $help, |a,b,_init:bool,_ctx:&dyn $crate::eval::Scope| $fnc(a,b), $crate::eval::none)
    };
    ( $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr, $to_abe:expr ) => {
        $crate::fnc!(@C $id , $argc , $init, $help, |a,b,_init:bool,_ctx:&dyn $crate::eval::Scope| $fnc(a,b), $to_abe)
    };
    ( @C $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr, none) => {
        $crate::eval::ScopeFunc{
            info: $crate::eval::ScopeFuncInfo { id: $id, init_eq: $init, argc: $argc, help: $help, to_abe: false},
            apply: |a,b:&[&[u8]],init:bool,ctx:&dyn $crate::eval::Scope| -> $crate::eval::ApplyResult {
                let r : Result<Vec<u8>,$crate::eval::ApplyErr> = $fnc(a,b,init,ctx);
                $crate::eval::ApplyResult::from(r)
            },
            to_abe:none
        }
    };
    ( @C $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr, $to_abe:expr) => {
        $crate::eval::ScopeFunc{
            info: $crate::eval::ScopeFuncInfo { id: $id, init_eq: $init, argc: $argc, help: $help, to_abe: true },
            apply: |a,b:&[&[u8]],init:bool,ctx:&dyn $crate::eval::Scope| -> $crate::eval::ApplyResult {
                let r : Result<Vec<u8>,$crate::eval::ApplyErr> = $fnc(a,b,init,ctx);
                $crate::eval::ApplyResult::from(r)
            },
            to_abe:$to_abe
        }
    };
}

pub fn none<T>(_t: &T, _bytes: &[u8], _opts: &[ABE]) -> ApplyResult {
    ApplyResult::None
}

#[macro_export]
macro_rules! fncs {
    ( [$( ( $($fni:tt)* ) ),* ] ) => {
        &[ $( $crate::fnc!($($fni)*) ),*]
    };
}
pub fn parse_b<T: FromStr>(b: &[u8]) -> Result<T, ApplyErr>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(std::str::from_utf8(b)?.parse()?)
}
pub fn carry_add_be(bytes: &mut [u8], val: &[u8]) -> bool {
    debug_assert!(bytes.len() == val.len());
    let mut carry = false;
    let mut idx = bytes.len() - 1;
    loop {
        let (ni, nc) = bytes[idx].carrying_add(val[idx], carry);
        bytes[idx] = ni;
        carry = nc;
        if idx == 0 {
            break;
        }
        idx -= 1;
    }
    carry
}
pub fn carry_sub_be(bytes: &mut [u8], val: &[u8]) -> bool {
    debug_assert!(bytes.len() == val.len());
    let mut carry = false;
    let mut idx = bytes.len() - 1;
    loop {
        let (ni, nc) = bytes[idx].borrowing_sub(val[idx], carry);
        bytes[idx] = ni;
        carry = nc;
        if idx == 0 {
            break;
        }
        idx -= 1;
    }
    carry
}

#[derive(Copy, Clone, Debug)]
pub struct UIntFE;
impl EvalScopeImpl for UIntFE {
    fn about(&self) -> (String, String) {
        ("UInt".into(), "Unsigned integer functions".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( "+" , 1..=16,   "Saturating addition. Requires all inputs to be equal size",
                    |_,inp:&[&[u8]]| {
                        if !inp.iter().all(|v| v.len() == inp[0].len()){ return Err("Mismatch length".into())}
                        let mut r = inp[0].to_vec();
                        for i in &inp[1..]{
                            if carry_add_be(&mut r, i){
                                r.iter_mut().for_each(|v| *v = 255);
                                return Ok(r)
                            }
                        }
                        Ok(r)
                    }
            ),
            ( "-" , 1..=16,   "Saturating subtraction. Requires all inputs to be equal size",
                    |_,inp:&[&[u8]]| {
                        if !inp.iter().all(|v| v.len() == inp[0].len()){ return Err("Mismatch length".into())}
                        let mut r = inp[0].to_vec();
                        for i in &inp[1..]{
                            if carry_sub_be(&mut r, i){
                                r.iter_mut().for_each(|v| *v = 0);
                                return Ok(r)
                            }
                        }
                        Ok(r)
                    }
            ),
            ( "u8" , 1..=1,   "parse 1 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u8>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u8::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u16" , 1..=1,  "parse 2 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u16>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u16::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u32" , 1..=1,  "parse 4 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u32>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u32::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u64" , 1..=1,  "parse 8 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u64>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u64::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u128" , 1..=1, "parse 16 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u128>(inp[0])?.to_be_bytes().to_vec()) ,
               { id : |b:&[u8],_| b.try_into().ok().map(u128::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "?u" , 1..=1, "Print big endian bytes as decimal",
              |_,inp:&[&[u8]]| {
                  let val = inp[0];
                  if val.len() > 16 { return Err("ints larger than 16 bytes (fixme)".into())}
                  let mut v = [0;16];
                  v[16-val.len()..].copy_from_slice(val);
                  Ok(u128::from_be_bytes(v).to_string().into_bytes())
              }
              ),
            ( "lu" , 1..=1, "parse little endian byte (upto 16)",
              |_,inp:&[&[u8]]| Ok(cut_ending_nulls2(&parse_b::<u128>(inp[0])?.to_le_bytes()).to_vec())
            ),
            ( "lu8" , 1..=1, "parse 1 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u8>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu16" , 1..=1, "parse 2 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u16>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu32" , 1..=1, "parse 4 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u32>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu64" , 1..=1, "parse 8 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u64>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu128" , 1..=1, "parse 16 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u128>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "?lu",1..=1,"print little endian number",
             |_,inp:&[&[u8]]| {
                 let val = inp[0];
                 if val.len() > 16 { return Err("ints larger than 16 bytes (fixme)".into());}
                 let mut v = [0;16];
                 v[0..val.len()].copy_from_slice(val);
                 Ok(u128::from_le_bytes(v).to_string().into_bytes())
             })
        ])
    }
}

#[derive(Copy, Clone, Debug)]
pub struct BytesFE;
impl EvalScopeImpl for BytesFE {
    fn about(&self) -> (String, String) {
        (
            "bytes".into(),
            "Byte padding/trimming and ascii-byte reflection functions".into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fn pad(inp: &[&[u8]], left: bool, default_pad: u8) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];
            let len = inp
                .get(1)
                .filter(|v| !v.is_empty())
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            if len < bytes.len() {
                return Err(format!("exceeds length {len} ( use '/cut:{len}' to cut )").into());
            };
            let tmp_pad = [default_pad];
            let padb = inp.get(2).copied().unwrap_or(&tmp_pad);
            if padb.len() != 1 {
                return Err("pad byte should be a single byte".into());
            };
            let mut v = vec![padb[0]; len];
            if !left {
                &mut v[0..bytes.len()]
            } else {
                &mut v[len - bytes.len()..]
            }
            .copy_from_slice(bytes);
            Ok(v)
        }
        fn trim(inp: &[&[u8]], left: bool) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];
            let len = inp
                .get(1)
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            let len = len.min(bytes.len());
            Ok(if left {
                &bytes[..len]
            } else {
                &bytes[bytes.len() - len..]
            }
            .to_vec())
        }
        fn cut(inp: &[&[u8]], left: bool) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];
            let len = inp
                .get(1)
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            if len > bytes.len() {
                return Err(
                    format!("less than length {len} ( use ':p to expand before cutting')").into(),
                );
            };
            Ok(if left {
                &bytes[..len]
            } else {
                &bytes[bytes.len() - len..]
            }
            .to_vec())
        }
        fn bin(inp: &[&[u8]], radix: u32) -> Result<Vec<u8>, ApplyErr> {
            // FIXME probably want to better handle leading '000000000'
            let st = std::str::from_utf8(inp[0])?;
            let i = u128::from_str_radix(st, radix)?;
            if i == 0 {
                Ok(vec![0])
            } else {
                Ok(cut_prefix_nulls(&i.to_be_bytes()).to_vec())
            }
        }
        fncs!([
            ("",1..=16,"the blank fnc can be use to start an expr such as {:12/u8} which is the same as {u8:12}",
             |_,i:&[&[u8]]| Ok(i.concat())),
            ("?a",1..=1,"encode bytes into ascii-bytes format",|_,i:&[&[u8]]| Ok(as_abtxt(i[0]).into_owned().into_bytes())),
            ("?a0",1..=1,"encode bytes into ascii-bytes format but strip prefix '0' bytes",
             |_,i:&[&[u8]]| Ok(as_abtxt(cut_prefix_nulls(i[0])).into_owned().into_bytes())),
            ("a",1..=3,"[bytes,length = 16,pad_byte = \\0] - alias for 'pad<'",|_,i:&[&[u8]]| pad(i,true,0)),
            (MAX_STR,1..=3,"same as 'a' but uses \\xff as padding ",|_,i:&[&[u8]]| pad(i,true,255)),
            ("pad<",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad input bytes",|_,i:&[&[u8]]| pad(i,true,0)),
            ("pad>",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad input bytes",|_,i:&[&[u8]]| pad(i,false,0)),
            ("cut<",1..=2,"[bytes,length = 16] - left cut input bytes",|_,i:&[&[u8]]| cut(i,true)),
            ("cut>",1..=2,"[bytes,length = 16] - right cut input bytes",|_,i:&[&[u8]]| cut(i,false)),
            ("trim<",1..=2,"[bytes,length = 16] - left trim ( cut< without error )",|_,i:&[&[u8]]| trim(i,true)),
            ("trim>",1..=2,"[bytes,length = 16] - right trim ( cut> without error )",|_,i:&[&[u8]]| trim(i,false)),

            ("b2",1..=1,"decode binary",|_,i:&[&[u8]]| bin(i,2)),
            ("b8",1..=1,"decode octets",|_,i:&[&[u8]]| bin(i,8)),
            ("b16",1..=1,"decode hex",|_,i:&[&[u8]]| bin(i,16))
        ])
    }
}


#[derive(Copy, Clone, Debug)]
pub struct LogicOps;
impl EvalScopeImpl for LogicOps {
    fn about(&self) -> (String, String) {
        ("logic ops".into(), "ops are : < > = 0 1 ".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        // TODO, extra crate for test_ops
        fncs!([
            (
                "size?",
                3..=3,
                "[in,OP,VAL] error unless size passes the test ( UNIMPLEMENTED )",
                |_, i: &[&[u8]]| {
                    let size = parse_b::<usize>(i[2])?;
                    let bytes = i[0];
                    let blen = bytes.len();
                    match i[1] {
                        b"=" => {
                            if blen != size {
                                return Err(format!("expected {size} bytes got {blen}").into());
                            } else {
                            }
                        }
                        _ => return Err("unknown op".into()),
                    };
                    Ok(i[0].to_vec())
                }
            ),
            (
                "val?",
                3..=3,
                "[in,OP,VAL] error unless value passes the test ( UNIMPLMENTED)",
                |_, i: &[&[u8]]| {
                    let bytes = i[0];
                    match i[1] {
                        b"=" => {
                            if bytes != i[2] {
                                return Err("unequal bytes".into());
                            } else {
                            }
                        }
                        _ => return Err("unknown op".into()),
                    };
                    Ok(i[0].to_vec())
                }
            )
        ])
    }
    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[
            eval_fnc!("or",":{EXPR}[:{EXPR}]* short circuit evaluate until valid return. Empty is valid, use {_/minsize?} to error on empty",
                  |_,i:&[ABE],scope:&dyn Scope|{
                      let mut it = i.split(|v| v.is_colon());
                      if !it.next().ok_or("missing expr")?.is_empty(){ return Err("expected ':EXPR'".into())};
                      let mut err = vec![];
                      for o in it{
                          match eval(&EvalCtx { scope }, o){
                              Ok(b) => return Ok(b.concat()),
                              Err(e) => err.push((o,e)),
                          }
                      }
                      Err(format!("{err:#?}").into())
                  }
            )
        ]
    }
}

#[derive(Copy, Clone)]
pub struct Encode;
impl EvalScopeImpl for Encode {
    fn about(&self) -> (String, String) {
        (
            "encode".into(),
            "attempt an inverse of a set of functions".into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            info: ScopeFuncInfo {
                id: "eval",
                init_eq: None,
                to_abe: false,
                argc: 1..=1,
                help: "parse and evaluate",
            },
            apply: |_, inp, _, scope| {
                let expr = parse_abe_b(inp[0])?;
                let ctx = EvalCtx { scope };
                AR::Ok(eval(&ctx, &expr)?.concat())
            },
            to_abe: none,
        }]
    }
    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[
            ScopeEval {
                info: ScopeEvalInfo { id: "?", help: "find an abe encoding for the value trying multiple reversal functions - [/fn:{opts}]* " },
                apply:|_,abe,scope|-> ApplyResult{
                    let ctx = EvalCtx{scope};
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_colon());
                    let id = it.next().ok_or("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = eval(&ctx, id)?.concat();
                    AR::Ok(encode_abe(&ctx, &bytes, rest)?.into_bytes())
                }
            },
            ScopeEval {
                info: ScopeEvalInfo { id: "e", help: "eval inner expression list. Useful to avoid escapes: eg file:{/e:/some/dir:thing}:opts does not require escapes the '/' " },
                apply:|_,abe,scope|-> ApplyResult{
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let ctx = EvalCtx{scope};
                    ApplyResult::Ok(eval(&ctx, abe)?.concat())
                }
            },
        ]
    }
}

#[derive(Copy, Clone)]
pub struct Help;
impl EvalScopeImpl for Help {
    fn about(&self) -> (String, String) {
        ("help".into(), "".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |_, i: &[&[u8]], _, scope| {
                ApplyResult::Ok({
                    if let Some(id) = i.get(0) {
                        let mut out = "".to_string();
                        scope.describe(&mut |name, about, fncs, evls| {
                            if !out.is_empty() {
                                return;
                            }
                            let fs: Vec<_> = fncs.collect();
                            let es: Vec<_> = evls.collect();
                            if fs.iter().any(|e| e.id.as_bytes() == *id)
                                || es.iter().any(|e| e.id.as_bytes() == *id)
                            {
                                let _ = fmt_describer(
                                    &mut out,
                                    &mut Default::default(),
                                    name,
                                    about,
                                    &mut fs.into_iter(),
                                    &mut es.into_iter(),
                                );
                            }
                        });
                        if out.is_empty() {
                            write!(&mut out, "no such fnc found")?;
                        };
                        out.into_bytes()
                    } else {
                        EvalCtx { scope }.to_string().into_bytes()
                    }
                })
            },
            info: ScopeFuncInfo {
                id: "help",
                init_eq: None,
                argc: 0..=16,
                help: "help",
                to_abe: false,
            },
            to_abe: none,
        }]
    }
    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[eval_fnc!(
            "help",
            "desribe current eval context",
            |_, _, scope| { Ok(EvalCtx { scope }.to_string().into_bytes()) }
        )]
    }
}

fn fmt_describer(
    f: &mut dyn Write,
    seen: &mut HashSet<&'static str>,
    name: &str,
    about: &str,
    funcs: &mut dyn Iterator<Item = ScopeFuncInfo>,
    evals: &mut dyn Iterator<Item = ScopeEvalInfo>,
) -> std::fmt::Result {
    let (mut fnc_head, mut evl_head) = (true, true);
    writeln!(f, "# {name}\n{about}")?;
    for ScopeFuncInfo {
        id,
        init_eq,
        argc,
        help,
        to_abe,
    } in funcs
    {
        if std::mem::take(&mut fnc_head) {
            writeln!(f, "## functions")?;
        }
        let state = if seen.insert(id) {
            "        "
        } else {
            "<partial>"
        };
        let fslash = if init_eq != Some(false) { "/" } else { " " };
        let colon = if init_eq != Some(true) { ":" } else { " " };
        let encode = if to_abe { "?" } else { " " };
        writeln!(
            f,
            "- {id: <16} {fslash}{colon}{encode} {state} {argc:?}     {help}  "
        )?;
    }
    for ScopeEvalInfo { id, help } in evals {
        if std::mem::take(&mut evl_head) {
            writeln!(f, "## eval")?;
        }
        writeln!(f, "- {id: <16} {help}  ")?;
    }
    writeln!(f)?;
    Ok(())
}

impl<A: Scope> Display for EvalCtx<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "==scopes==")?;
        let mut err = Ok(());
        let mut set = HashSet::<&'static str>::new();
        self.scope.describe(&mut |name, about, fncs, revals| {
            if err.is_err() {
                return;
            }
            err = fmt_describer(f, &mut set, name, about, fncs, revals);
        });
        err
    }
}

/// Note that this destroys context information. \/ and / resolve to the same
pub fn dump_abe_bytes(out: &mut Vec<u8>, abe: &[ABE]) {
    for item in abe {
        match item {
            ABE::Ctr(e) => out.push(*e as u8),
            ABE::Expr(e) => match e {
                Expr::Bytes(b) => out.extend_from_slice(b),
                Expr::Lst(l) => {
                    out.push(b'{');
                    dump_abe_bytes(out, l);
                    out.push(b'}');
                }
            },
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct ArgV<'o>(pub [Option<&'o [u8]>;8]);
impl<'o> ArgV<'o>{
    pub fn try_fit(v: &'o [&'o [u8]]) -> Option<Self>{
        if v.len() > 8 { return None}
        let mut it = v.iter().copied();
        Some(ArgV([
            it.next(),it.next(),it.next(),it.next(),
            it.next(),it.next(),it.next(),it.next()
        ]))
    }
}
impl<'o> EvalScopeImpl for ArgV<'o>{
    fn about(&self) -> (String, String) {
        ("user input list".into(), "Provide values, access with {0} {1} .. {7} ".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( "0" , 0..=0, "user val", |t:&Self,_| Ok(t.0[0].ok_or("no 0 value")?.to_vec())),
            ( "1" , 0..=0, "user val", |t:&Self,_| Ok(t.0[1].ok_or("no 1 value")?.to_vec())),
            ( "2" , 0..=0, "user val", |t:&Self,_| Ok(t.0[2].ok_or("no 2 value")?.to_vec()))
        ])
    }
}
