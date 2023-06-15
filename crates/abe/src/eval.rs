// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::fmt::Write;
use std::ops::{Try, ControlFlow};

use anyhow::{ anyhow, Context};
use arrayvec::ArrayVec;
use std::fmt::{Debug, Display};
use std::{collections::HashSet, convert::Infallible, ops::FromResidual, str::FromStr};
use thiserror::Error;

use crate::abtxt::as_abtxt;
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
#[derive(Clone, PartialEq)]
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
pub fn clist<I: IntoIterator<Item = A>, A: AsRef<[u8]>>(elements: I) -> ABList {
    delmited_ablist(elements, Ctr::Colon)
}
pub fn flist<I: IntoIterator<Item = A>, A: AsRef<[u8]>>(elements: I) -> ABList {
    delmited_ablist(elements, Ctr::FSlash)
}
pub fn delmited_ablist<I: IntoIterator<Item = A>, A: AsRef<[u8]>>(elements: I, delimiter: Ctr) -> ABList {
    let mut lst: Vec<_> = elements
        .into_iter()
        .map(|b| (b.as_ref().to_vec(), Some(delimiter)))
        .collect();
    match lst.last_mut(){
        Some(v) => {v.1 = None; ABList{lst}},
        None => ABList::default(),
    }
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
                f.write_str(&as_abtxt(self.0.as_ref()))
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

impl ABList {
    
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
pub type ApplyErr = anyhow::Error;

impl<'o> TryFrom<&'o [ABE]> for ABList{
    type Error = &'o ABE;

    fn try_from(abe :&'o [ABE]) -> Result<Self, Self::Error> {
        if let Some(expr) = abe.iter().find(|v| matches!(v, ABE::Expr(Expr::Lst(_)))){ return Err(expr)}
        let abl = abe.iter().map(|v| match v {
            ABE::Ctr(c) => ABItem::Ctr(*c),
            ABE::Expr(Expr::Bytes(b)) => ABItem::Bytes(b.clone()),
            ABE::Expr(Expr::Lst(_))=>unreachable!()
        }).collect();
        Ok(abl)
    }
}



#[derive(Debug)]
/// Utility that impl's try for both None and Err
/// Semantically the None value means the caller has to decide whether to continue.
// TODO rename to Val, NoVal, Err
pub enum ApplyResult<V=Vec<u8>> {
    NoValue,
    Value(V),
    Err(ApplyErr),
}

impl<V> ApplyResult<V> {

    pub fn map<X>(self,f:impl FnOnce(V) -> X) -> ApplyResult<X>{
        match self {
            ApplyResult::NoValue => ApplyResult::NoValue,
            ApplyResult::Value(v) => ApplyResult::Value(f(v)),
            ApplyResult::Err(e) => ApplyResult::Err(e),
        }
    }
    pub fn into_ok(self) -> Result<Option<V>, ApplyErr> {
        match self {
            ApplyResult::NoValue => Ok(None),
            ApplyResult::Value(v) => Ok(Some(v)),
            ApplyResult::Err(e) => Err(e),
        }
    }
    pub fn into_opt(self) -> Option<Result<V, ApplyErr>> {
        match self {
            ApplyResult::NoValue => None,
            ApplyResult::Value(v) => Some(Ok(v)),
            ApplyResult::Err(e) => Some(Err(e)),
        }
    }
    pub fn arg_err<X: AsRef<[u8]>>(args: impl IntoIterator<Item = X>, expect: &str) -> Self {
        ApplyResult::Err(anyhow!("expect {expect} but got {:?}", clist(args)))
    }
    pub fn or_else(self, map: impl FnOnce() -> ApplyResult<V>) -> Self {
        if matches!(self, AR::NoValue) {
            map()
        } else {
            self
        }
    }
    pub fn require(self,msg:&'static str) -> Self {
        match self {
            ApplyResult::NoValue => ApplyResult::Err(anyhow!("no result").context(msg)),
            ApplyResult::Value(v) => ApplyResult::Value(v),
            ApplyResult::Err(e) => ApplyResult::Err(e)
        }
    }
    pub fn context<C>(self,context: C) -> Self where  C: Display + Send + Sync + 'static{
        match self {
            ApplyResult::NoValue => ApplyResult::NoValue,
            ApplyResult::Value(o) => ApplyResult::Value(o),
            ApplyResult::Err(e) => ApplyResult::Err(e.context(context)),
        }
    }
}
impl<V> FromResidual<ApplyResult<V>> for ApplyResult<V>{
    fn from_residual(residual: ApplyResult<V>) -> Self {
        residual
    }
}
impl<V> Try for ApplyResult<V>{
    type Output = V;

    type Residual = ApplyResult<V>;

    fn from_output(output: Self::Output) -> Self {
        ApplyResult::Value(output)
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        match self {
            ApplyResult::Value(v) => std::ops::ControlFlow::Continue(v),
            e => ControlFlow::Break(e),
        }
    }
}

impl<V> From<Option<V>> for ApplyResult<V>{
    fn from(value: Option<V>) -> Self {
        value.map(ApplyResult::Value)?
    }
}
impl<V> From<V> for ApplyResult<V>{
    fn from(value: V) -> Self {
        ApplyResult::Value(value)
    }
}
impl<V> FromResidual<Option<Infallible>> for ApplyResult<V> {
    fn from_residual(_residual: Option<Infallible>) -> Self {
        AR::NoValue
    }
}
impl<V,E: Into<ApplyErr>> From<Option<Result<V, E>>> for ApplyResult<V> {
    fn from(v: Option<Result<V, E>>) -> Self {
        match v {
            Some(v) => v.map_err(Into::into).into(),
            None => AR::NoValue,
        }
    }
}
impl<V,E: Into<ApplyErr>> FromResidual<Result<Infallible, E>> for ApplyResult<V> {
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        AR::Err(residual.unwrap_err().into())
    }
}

impl<V> From<Result<V, ApplyErr>> for ApplyResult<V> {
    fn from(v: Result<V, ApplyErr>) -> Self {
        match v {
            Ok(o) => AR::Value(o),
            Err(e) => AR::Err(e),
        }
    }
}
use ApplyResult as AR;

#[derive(Error)]
pub enum EvalError {
    #[error("evaluator err :  '{}' {}",as_abtxt(.0),.1)]
    SubEval(Vec<u8>, ApplyErr),
    #[error("no such e-func : '/{}'",as_abtxt(.0))]
    NoSuchSubEval(Vec<u8>),
    #[error("no such func : {}",as_abtxt(.0))]
    NoSuchFunc(Vec<u8>),
    #[error("func error : '{}' : {}",as_abtxt(.0), .1)]
    Func(Vec<u8>, ApplyErr),
    #[error(transparent)]
    Other(anyhow::Error),
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
    &mut dyn Iterator<Item = ScopeMacroInfo>,
);
pub trait Scope {
    fn try_apply_macro(&self, _id: &[u8], _abe: &[ABE], _ctx: &dyn Scope) -> ApplyResult {
        ApplyResult::NoValue
    }
    fn try_apply_func(
        &self,
        id: &[u8],
        inp_and_args: &[&[u8]],
        init: bool,
        ctx: &dyn Scope,
    ) -> ApplyResult;
    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String>;
    fn describe(&self, cb: Describer) {
        cb("todo", "", &mut std::iter::empty(), &mut std::iter::empty())
    }
}
#[derive(Copy, Clone)]
pub struct EScope<T>(pub T);
impl<T: EvalScopeImpl> Scope for EScope<T> {
    fn try_apply_func(&self, id: &[u8], inp: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        for ScopeFunc { apply, info, .. } in self.0.list_funcs() {
            if info.id.as_bytes() == id {
                if info.init_eq.is_some() && info.init_eq != Some(init) {
                    Err(anyhow!("function can not be applied this way"))?;
                }
                if !info.argc.contains(&inp.len()) {
                    return ApplyResult::arg_err(inp, &format!("between {:?}", info.argc));
                }
                return apply(&self.0, inp, init, ctx);
            }
        }
        ApplyResult::NoValue
    }

    fn describe(&self, cb: Describer) {
        let mut funcs = self.0.list_funcs().iter().map(|v| v.info.clone());
        let mut macros = self.0.list_macros().iter().map(|v| v.info);
        let (name, info) = self.0.about();
        cb(&name, &info, &mut funcs, &mut macros);
    }
    fn try_apply_macro(&self, id: &[u8], abe: &[ABE], funcs: &dyn Scope) -> ApplyResult {
        if let Some(e) = self
            .0
            .list_macros()
            .iter()
            .find(|i| i.info.id.as_bytes() == id)
        {
            return (e.apply)(&self.0, abe, funcs);
        }
        ApplyResult::NoValue
    }

    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String> {
        for ScopeFunc { to_abe, info, .. } in self.0.list_funcs() {
            if info.to_abe && info.id.as_bytes() == id {
                return to_abe(&self.0, bytes, options);
            }
        }
        AR::NoValue
    }
}
pub trait EvalScopeImpl {
    fn about(&self) -> (String, String) {
        (String::new(), String::new())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[]
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[]
    }
}
#[derive(Clone)]
pub struct ScopeFunc<T> {
    pub apply: fn(T, &[&[u8]], bool, &dyn Scope) -> ApplyResult,
    pub to_abe: fn(T, &[u8], &[ABE]) -> ApplyResult<String>,
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
pub struct ScopeMacro<T> {
    pub apply: fn(T, &[ABE], &dyn Scope) -> ApplyResult,
    pub info: ScopeMacroInfo,
}
#[derive(Copy, Clone)]
pub struct ScopeMacroInfo {
    pub id: &'static str,
    pub help: &'static str,
}

impl Scope for () {
    fn try_apply_func(
        &self,
        _id: &[u8],
        _args: &[&[u8]],
        _init: bool,
        _ctx: &dyn Scope,
    ) -> ApplyResult {
        AR::NoValue
    }

    fn try_apply_macro(&self, _id: &[u8], _abe: &[ABE], _scopes: &dyn Scope) -> ApplyResult {
        AR::NoValue
    }

    fn describe(&self, cb: Describer) {
        cb("()", "", &mut std::iter::empty(), &mut std::iter::empty())
    }

    fn try_encode(&self, _id: &[u8], _options: &[ABE], _bytes: &[u8]) -> ApplyResult<String> {
        AR::NoValue
    }
}
impl<A: Scope> Scope for Option<A> {
    fn try_apply_func(
        &self,
        id: &[u8],
        inpt_and_args: &[&[u8]],
        init: bool,
        ctx: &dyn Scope,
    ) -> ApplyResult {
        self.as_ref()
            .map(|x| x.try_apply_func(id, inpt_and_args, init, ctx))
            .unwrap_or(ApplyResult::NoValue)
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
    fn try_apply_macro(&self, id: &[u8], abe: &[ABE], scopes: &dyn Scope) -> ApplyResult {
        self.as_ref()
            .map(|x| x.try_apply_macro(id, abe, scopes))
            .unwrap_or(ApplyResult::NoValue)
    }
    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String> {
        self.as_ref()
            .map(|v| v.try_encode(id, options, bytes))
            .unwrap_or(AR::NoValue)
    }
}

impl Scope for &dyn Scope {
    fn try_apply_macro(&self, id: &[u8], abe: &[ABE], scopes: &dyn Scope) -> ApplyResult {
        (**self).try_apply_macro(id, abe, scopes)
    }
    fn try_apply_func(
        &self,
        id: &[u8],
        inpt_and_args: &[&[u8]],
        init: bool,
        ctx: &dyn Scope,
    ) -> ApplyResult {
        (**self).try_apply_func(id, inpt_and_args, init, ctx)
    }

    fn describe(&self, cb: Describer) {
        (**self).describe(cb)
    }

    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String> {
        (**self).try_encode(id, options, bytes)
    }
}
impl<A: Scope> Scope for &A {
    fn try_apply_macro(&self, id: &[u8], abe: &[ABE], scopes: &dyn Scope) -> ApplyResult {
        (**self).try_apply_macro(id, abe, scopes)
    }

    fn try_apply_func(&self, id: &[u8], args: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        (**self).try_apply_func(id, args, init, ctx)
    }
    fn describe(&self, cb: Describer) {
        (**self).describe(cb)
    }
    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String> {
        (**self).try_encode(id, options, bytes)
    }
}
impl<A: Scope, B: Scope> Scope for (A, B) {
    #[inline(always)]
    fn try_apply_func(&self, id: &[u8], args: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        self.0
            .try_apply_func(id, args, init, ctx)
            .or_else(|| self.1.try_apply_func(id, args, init, ctx))
    }
    fn try_apply_macro(&self, id: &[u8], abe: &[ABE], scope: &dyn Scope) -> ApplyResult {
        self.0
            .try_apply_macro(id, abe, scope)
            .or_else(|| self.1.try_apply_macro(id, abe, scope))
    }
    fn describe(&self, cb: Describer) {
        self.0.describe(cb);
        self.1.describe(cb);
    }
    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String> {
        self.0
            .try_encode(id, options, bytes)
            .or_else(|| self.1.try_encode(id, options, bytes))
    }
}
impl<A: Scope, B: Scope, C: Scope> Scope for (A, B, C) {
    #[inline(always)]
    fn try_apply_func(&self, id: &[u8], args: &[&[u8]], init: bool, ctx: &dyn Scope) -> ApplyResult {
        ((&self.0, &self.1), &self.2).try_apply_func(id, args, init, ctx)
    }
    fn try_apply_macro(&self, id: &[u8], abe: &[ABE], scope: &dyn Scope) -> ApplyResult {
        ((&self.0, &self.1), &self.2).try_apply_macro(id, abe, scope)
    }

    fn describe(&self, cb: Describer) {
        self.0.describe(cb);
        self.1.describe(cb);
        self.2.describe(cb);
    }

    fn try_encode(&self, id: &[u8], options: &[ABE], bytes: &[u8]) -> ApplyResult<String> {
        self.0
            .try_encode(id, options, bytes)
            .or_else(|| self.1.try_encode(id, options, bytes))
            .or_else(|| self.2.try_encode(id, options, bytes))
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
            dbgprintln!("Match bytes('{}') (depth={depth})", as_abtxt(b));
            Ok(ABItem::Bytes(b.to_vec()))
        }
        ABE::Expr(Expr::Lst(ls)) => {
            dbgprintln!("Match lst[{}] (depth={depth})", ls.len());
            let inner_abl = match ls.as_slice() {
                [ABE::Ctr(Ctr::FSlash), ref tail @ ..] => {
                    let (id, rest): (&[u8], &[ABE]) = match tail {
                        [] => return Err(EvalError::Other(anyhow!("missing eval name"))),
                        [ABE::Expr(Expr::Lst(_)), ..] => {
                            return Err(EvalError::Other(
                                anyhow!("var eval name resolution disabled")
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
                    dbgprintln!("Eval('{}')", as_abtxt(id));
                    match ctx.scope.try_apply_macro(id, rest, &ctx.scope) {
                        ApplyResult::NoValue => return Err(EvalError::NoSuchSubEval(id.to_vec())),
                        ApplyResult::Value(b) => return Ok(ABItem::Bytes(b)),
                        ApplyResult::Err(e) => return Err(EvalError::SubEval(id.to_vec(), e)),
                    }
                }
                [ABE::Expr(Expr::Lst(_)), ..] => {
                    Err(EvalError::Other(anyhow!("function names can not be expressions")))?
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
                    as_abtxt(id),
                    input_and_args
                );
                match scope.try_apply_func(id, input_and_args, init, &scope) {
                    ApplyResult::NoValue => Err(EvalError::NoSuchFunc(id.to_vec())),
                    ApplyResult::Value(b) => Ok(b),
                    ApplyResult::Err(e) => Err(EvalError::Func(id.to_vec(), e)),
                }
            }
            let it = inner_abl
                .lst
                .split_inclusive(|(_, c)| *c == Some(Ctr::FSlash));
            let mut calls = it.map(|ls| ls.iter().map(|(b, _c)| b.as_slice()));
            let mut stack = [&[] as &[u8]; 16];
            let mut init_id_args = match calls.next() {
                None => return Err(EvalError::Other(anyhow!("empty {{}} not enabled"))),
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
                return Err(EvalError::Other(anyhow!("more than 16 args not supported")));
            }
            let args = &stack[..argc];
            dbgprintln!("Start: '{}' - {:?} ", as_abtxt(id), args);
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
                    return Err(EvalError::Other(anyhow!("more than 16 args not supported")));
                }
                let args = &stack[..argc];
                dbgprintln!(
                    "'{}' -> '{}' :: {:?} ::",
                    as_abtxt(&bytes),
                    as_abtxt(id),
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
    #[error("option encoding error expected [ scope [:{{opts}}]? ]* - {0}")]
    OptionError(anyhow::Error),
    #[error("option parse error")]
    ParseError(#[source] ASTParseError),
    #[error("Encoder '{0}' failed - set ignore_err or use /~? to suppress\n{1}")]
    ScopeEncode(String, #[source] ApplyErr),
    #[error("No suitable encoding found - add a '/:' for fallback to abtxt encoding ")]
    NoneFound,
}

/// Encode bytes with scopes given as "scope1:{opts}/scope2:{opts}/"
/// :scope/{opts}:
/// e.g. lns:{known}/b64/uint will attempt to encode bytes through locally known lns, otherwise use b64, and finnally attempt uint
pub fn encode(
    ctx: &EvalCtx<impl Scope>,
    bytes: &[u8],
    options: &str,
    ignore_encoder_errors:bool
) -> std::result::Result<String, EncodeError> {
    let lst = parse_abe(options).map_err(EncodeError::ParseError)?;
    encode_abe(ctx, bytes, &lst,ignore_encoder_errors)
}
pub fn encode_abe(
    ctx: &EvalCtx<impl Scope>,
    bytes: &[u8],
    options: &[ABE],
    ignore_encoder_errors:bool
) -> std::result::Result<String, EncodeError> {
    // TODO options should prob be ABList
    let mut it = options.split(|v| v.is_fslash());
    if options.is_empty() {
        it.next();
    }
    for scope_opts in it {
        let (func_id,args) :(&[u8],&[ABE]) = match scope_opts {
            [ABE::Ctr(Ctr::Colon)] => (&[],&[]),
            [] => return Ok(as_abtxt(bytes).into_owned()),
            [ABE::Expr(Expr::Bytes(b))] => (b.as_slice(),&[]),
            [ABE::Expr(Expr::Bytes(b)),ABE::Ctr(Ctr::Colon), ref rest @ .. ] => (b.as_slice(),rest),
            e => return Err(EncodeError::OptionError(anyhow!("expected function id + args ( got '{}')",print_abe(e)))),
        };
        //eprintln!("Try {}",as_abtxt(func_id));
        match ctx.scope.try_encode(func_id, args, bytes) {
            ApplyResult::NoValue => {}
            ApplyResult::Value(st) => {
                debug_assert_eq!(
                    eval(ctx, &parse_abe(&st).expect("bug: encode fmt"))
                        .unwrap_or_else(|_| panic!("bug: encode-eval ({})", &st))
                        .as_exact_bytes()
                        .expect("bug: encode multi")
                        , bytes,
                    "bug: eval(encode) {st}"
                );
                return Ok(st);
            }
            ApplyResult::Err(e) => {
                if !ignore_encoder_errors{return Err(EncodeError::ScopeEncode(as_abtxt(func_id).to_string(), e))}
            }
        }
    }
    Err(EncodeError::NoneFound)
}

#[macro_export]
macro_rules! eval_fnc {
    ( $id:expr, $help:literal,$fnc:expr) => {
        $crate::eval::ScopeMacro {
            info: $crate::eval::ScopeMacroInfo {
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
                     |_,bytes:&[u8],options:&[$crate::ABE]| -> $crate::eval::ApplyResult<String>{
                         let st : Option<String> = $to_abe(bytes,options);
                         match st {
                             None => $crate::eval::ApplyResult::NoValue,
                             Some(st) => $crate::eval::ApplyResult::Value(format!("[{}:{}]",$id,st))
                         }
                     }
        )
    };

    ( $id:expr, $argc:expr, $help:literal,$fnc:expr) => {
        $crate::fnc!($id,$argc,None,$help,$fnc)
    };
    ( $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr ) => {
        $crate::fnc!(@C $id , $argc , $init, $help, |a,b,_init:bool,_ctx:&dyn $crate::eval::Scope| $fnc(a,b), none)
    };
    ( $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr, $to_abe:expr ) => {
        $crate::fnc!(@C $id , $argc , $init, $help, |a,b,_init:bool,_ctx:&dyn $crate::eval::Scope| $fnc(a,b), $to_abe)
    };
    ( @C $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr, none) => {
        $crate::eval::ScopeFunc{
            info: $crate::eval::ScopeFuncInfo { id: $id, init_eq: $init, argc: $argc, help: $help, to_abe: false},
            apply: |a,b:&[&[u8]],init:bool,ctx:&dyn $crate::eval::Scope| -> $crate::eval::ApplyResult {
                $fnc(a,b,init,ctx).into()
            },
            to_abe:$crate::eval::none
        }
    };
    ( @C $id:expr, $argc:expr, $init:expr, $help:literal,$fnc:expr, $to_abe:expr) => {
        $crate::eval::ScopeFunc{
            info: $crate::eval::ScopeFuncInfo { id: $id, init_eq: $init, argc: $argc, help: $help, to_abe: true },
            apply: |a,b:&[&[u8]],init:bool,ctx:&dyn $crate::eval::Scope| -> $crate::eval::ApplyResult {
                $fnc(a,b,init,ctx).into()
            },
            to_abe:$to_abe
        }
    };
}

pub fn none<T>(_t: &T, _bytes: &[u8], _opts: &[ABE]) -> ApplyResult<String> {
    ApplyResult::NoValue
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
                        if !inp.iter().all(|v| v.len() == inp[0].len()){ return Err(anyhow!("Mismatch length"))}
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
                        if !inp.iter().all(|v| v.len() == inp[0].len()){ return Err(anyhow!("Mismatch length"))}
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
                  if val.len() > 16 { return Err(anyhow!("ints larger than 16 bytes (fixme)"))}
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
                 if val.len() > 16 { return Err(anyhow!("ints larger than 16 bytes (fixme)"));}
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
            "Byte padding/trimming".into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fn pad(inp: &[&[u8]], left: bool, default_pad: u8,check_len:bool,fixed: bool) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];

            let len = inp
                .get(1)
                .filter(|v| !v.is_empty())
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            let bytes = if !fixed {
                bytes
            }else {
                let nlen = len.min(bytes.len());
                if left{ &bytes[..nlen] }
                else {&bytes[bytes.len() - nlen..]}
            };
            if len < bytes.len() {
                if check_len { return Err(anyhow!("exceeds length {len} ( use  ~[lr]pad or [lr]fixed )"))};
                return Ok(bytes.to_vec());
            };
            let tmp_pad = [default_pad];
            let padb = inp.get(2).copied().unwrap_or(&tmp_pad);
            if padb.len() != 1 {
                return Err(anyhow!("pad byte should be a single byte"));
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
        
        fn cut(inp: &[&[u8]], left: bool,check_len:bool) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];
            let len = inp
                .get(1)
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            if len > bytes.len() {
                if check_len { return Err( anyhow!("less than length {len} ( use '~[lr]cut or [lr]fixed")) }
                return Ok(bytes.to_vec());
            };
            Ok(if left {
                &bytes[..len]
            } else {
                &bytes[bytes.len() - len..]
            }
            .to_vec())
        }
        fn encode_a(_:&BytesFE, b:&[u8], _:&[ABE]) -> ApplyResult<String>{
            let cut_b = as_abtxt(cut_prefix_nulls(b));
            let len = b.len();
            ApplyResult::Value(if len == 16 { format!("[a:{cut_b}]")}else{format!("[a:{cut_b}:{len}]")})
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
        fn slice(inp: &[&[u8]]) -> Result<Vec<u8>, ApplyErr> {
            #[derive(Debug,Copy,Clone)]
            struct SignedInt{neg:bool,val:usize}
            fn parse_b_signed(bytes:Option<&[u8]>) -> anyhow::Result<Option<SignedInt>>{
                let bytes = bytes.unwrap_or(&[]);
                if bytes.is_empty() { return Ok(None)}
                let (neg,val_b) = bytes.strip_prefix(b"-").map(|s|(true,s)).unwrap_or((false,bytes));
                Ok(Some(SignedInt{neg,val: std::str::from_utf8(val_b)?.parse()?}))
            }
            let bytes = inp[0];
            let len = bytes.len() as isize;
            let start = parse_b_signed(inp.get(1).copied())?;
            let end = parse_b_signed(inp.get(2).copied())?;
            let step : isize = inp.get(3).map(|b| anyhow::Ok::<isize>(std::str::from_utf8(b)?.parse()?)).transpose()?.unwrap_or(1);
            if step == 0 {
                return Ok(vec![]);
            }
            let (sb,eb) = if step >= 0 {  (0, len) } else { (len - 1, -1) };
            
            let to_bound= |v:Option<SignedInt>| -> Option<isize>{
                match v{
                    Some(SignedInt { neg:false, val }) => Some((val as isize).clamp(sb.min(eb),sb.max(eb))),
                    Some(SignedInt { neg:true, val }) => Some((len-val as isize).clamp(sb.min(eb),sb.max(eb))),
                    None => None,
                }
            };
            let mut i = to_bound(start).unwrap_or(sb);
            let end = to_bound(end).unwrap_or(eb);
            
            Ok(std::iter::from_fn(||{
                let in_range = if step >= 0 { i< end} else { i> end};
                if !in_range { return None}
                let result = *bytes.get(i as usize)?;
                i+= step;
                Some(result)
            }).collect())
        }

        fncs!([
            ("",1..=16,"the '' (empty) fnc can be used to start an expr such as {:12/u8} which is the same as {u8:12}",
             |_,i:&[&[u8]]| Ok(i.concat()),
             { id : |b:&[u8],_| Some(as_abtxt(b).to_string()) }
            ),
            ("?a",1..=1,"encode bytes into ascii-bytes format",|_,i:&[&[u8]]| Ok(as_abtxt(i[0]).into_owned().into_bytes())),
            ("?a0",1..=1,"encode bytes into ascii-bytes format but strip prefix '0' bytes",
             |_,i:&[&[u8]]| Ok(as_abtxt(cut_prefix_nulls(i[0])).into_owned().into_bytes())),
            (@C "a",1..=3,None,"[bytes,length = 16,pad_byte = \\0] - alias for 'lpad'",|_,i:&[&[u8]],_,_| pad(i,true,0,true,false),
             encode_a),
            ("f",1..=3,"same as 'a' but uses \\xff as padding ",|_,i:&[&[u8]]| pad(i,true,255,true,false)),
            ("lpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad input bytes",|_,i:&[&[u8]]| pad(i,true,0,true,false)),
            ("rpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad input bytes",|_,i:&[&[u8]]| pad(i,false,0,true,false)),
            ("~lpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad input bytes",|_,i:&[&[u8]]| pad(i,true,0,false,false)),
            ("~rpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad input bytes",|_,i:&[&[u8]]| pad(i,false,0,false,false)),

            ("lcut",1..=2,"[bytes,length = 16] - left cut input bytes",|_,i:&[&[u8]]| cut(i,true,true)),
            ("rcut",1..=2,"[bytes,length = 16] - right cut input bytes",|_,i:&[&[u8]]| cut(i,false,true)),
            ("~lcut",1..=2,"[bytes,length = 16] - lcut without error",|_,i:&[&[u8]]| cut(i,true,false)),
            ("~rcut",1..=2,"[bytes,length = 16] - lcut without error",|_,i:&[&[u8]]| cut(i,false,false)),
            ("lfixed",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad and cut input bytes",|_,i:&[&[u8]]| pad(i,true,0,false,true)),
            ("rfixed",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad and cut input bytes",|_,i:&[&[u8]]| pad(i,false,0,false,true)),

            ("slice",1..=4,"[bytes,start=0,stop=len,step=1] - python like slice indexing",|_,i:&[&[u8]]|slice(i)),
            ("b2",1..=1,"decode binary",|_,i:&[&[u8]]| bin(i,2)),
            ("b8",1..=1,"decode octets",|_,i:&[&[u8]]| bin(i,8)),
            ("b16",1..=1,"decode hex",|_,i:&[&[u8]]| bin(i,16)),
            ("~utf8",1..=1,"lossy encode as utf8",|_,i:&[&[u8]]| AR::Value(bstr::BStr::new(&i[0]).to_string().into_bytes()))

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
                                return Err(anyhow!("expected {size} bytes got {blen}"));
                            } 
                        }
                        _ => return Err(anyhow!("unknown op")),
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
                                return Err(anyhow!("unequal bytes"));
                            } 
                        }
                        _ => return Err(anyhow!("unknown op")),
                    };
                    Ok(i[0].to_vec())
                }
            )
        ])
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[
            eval_fnc!("or",":{EXPR}[:{EXPR}]* short circuit evaluate until valid return. Empty is valid, use {_/minsize?} to error on empty",
                  |_,i:&[ABE],scope:&dyn Scope|{
                      let mut it = i.split(|v| v.is_colon());
                      if !it.next().context("missing expr")?.is_empty(){ return Err(anyhow!("expected ':EXPR'"))};
                      let mut err = vec![];
                      for o in it{
                          match eval(&EvalCtx { scope }, o){
                              Ok(b) => return Ok(b.concat()),
                              Err(e) => err.push((o,e)),
                          }
                      }
                      Err(anyhow!("{err:#?}"))
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
                AR::Value(eval(&ctx, &expr)?.concat())
            },
            to_abe: none,
        },
          ScopeFunc {
              info: ScopeFuncInfo {
                  id: "?",
                  init_eq: None,
                  to_abe: false, // TODO
                  argc: 2..=8,
                  help: "encode",
              },
              apply: |_, inp, _, scope| {
                  let ctx = EvalCtx{scope};
                  if inp.len() > 2 { return ApplyResult::Err(anyhow!("Options not yet supported"))};
                  let kind = std::str::from_utf8(inp[1]).context("bad encoder")?;
                  let r = encode(&ctx, inp[0], kind, false)?;
                  AR::Value(r.into_bytes())
              },
              to_abe: none,
          },
        ]
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[
            ScopeMacro {
                info: ScopeMacroInfo { id: "?", help: "find an abe encoding for the value trying multiple reversal functions - [/fn:{opts}]* " },
                apply:|_,abe,scope|-> ApplyResult{
                    let ctx = EvalCtx{scope};
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_fslash());
                    let id = it.next().context("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = eval(&ctx, id)?.concat();
                    AR::Value(encode_abe(&ctx, &bytes, rest,false)?.into_bytes())
                }
            },
            ScopeMacro {
                info: ScopeMacroInfo { id: "~?", help: "same as '?' but ignores all errors" },
                apply:|_,abe,scope|-> ApplyResult{
                    let ctx = EvalCtx{scope};
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_fslash());
                    let id = it.next().context("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = eval(&ctx, id)?.concat();
                    AR::Value(encode_abe(&ctx, &bytes, rest,true)?.into_bytes())
                }
            },
            ScopeMacro {
                info: ScopeMacroInfo { id: "e", help: "eval inner expression list. Useful to avoid escapes: eg file:{/e:/some/dir:thing}:opts does not require escapes the '/' " },
                apply:|_,abe,scope|-> ApplyResult{
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let ctx = EvalCtx{scope};
                    ApplyResult::Value(eval(&ctx, abe)?.concat())
                }
            },
        ]
    }
}

#[derive(Copy, Clone)]
pub struct Help;
impl EvalScopeImpl for Help {
    fn about(&self) -> (String, String) {
        ("help".into(), String::new())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |_, i: &[&[u8]], _, scope| {
                ApplyResult::Value({
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
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
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
    evals: &mut dyn Iterator<Item = ScopeMacroInfo>,
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
            writeln!(f, "## Functions")?;
        }
        let state = if seen.insert(id) {
            "        "
        } else {
            "<partial>"
        };
        let fslash = if init_eq != Some(false) { "[" } else { " " };
        let colon = if init_eq != Some(true) { "/" } else { " " };
        let encode = if to_abe { "?" } else { " " };
        writeln!(
            f,
            "- {id: <16} {fslash}{colon}{encode} {state} {argc:?}     {help}  "
        )?;
    }
    for ScopeMacroInfo { id, help } in evals {
        if std::mem::take(&mut evl_head) {
            writeln!(f, "## Macros")?;
        }
        writeln!(f, "- {id: <16} {help}  ")?;
    }
    writeln!(f)?;
    Ok(())
}

impl<A: Scope> Display for EvalCtx<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "The context has one or more scopes active")?;
        writeln!(f, "Each scope has functions and macros")?;
        writeln!(f, "For each function the option set  ['[' , '/' , '?'] is given")?;
        writeln!(f, "These refers to its use as:")?;
        writeln!(f, " '['  => Can be used to open   '[func/..]'")?;
        writeln!(f, " ':'  => Can be used in a pipe '[../func]'")?;
        writeln!(f, " '?'  => Can be 'reversed' to some extend '[../?:func]' || [?:..:func]")?;

        let mut err = Ok(());
        let mut set = HashSet::<&'static str>::new();
        self.scope.describe(&mut |name, about, fncs, macros| {
            if err.is_err() {
                return;
            }
            err = fmt_describer(f, &mut set, name, about, fncs, macros);
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
                    out.push(b'[');
                    dump_abe_bytes(out, l);
                    out.push(b']');
                }
            },
        }
    }
}

#[derive(Copy,Clone)]
pub struct Comment;
impl EvalScopeImpl for Comment{
    fn about(&self) -> (String, String) {
        ("comment function / void function. evaluates to nothing".into(), String::new())
    }

    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[
            fnc!("C",1..=16,"the comment function. all arguments are ignored. evaluates to ''",|_,_| Ok(vec![]))
        ]
    }

    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[]
    }
}

#[derive(Clone, Debug)]
pub struct ArgV<'o>(pub ArrayVec<&'o [u8],8>);
impl<'o> ArgV<'o>{
    pub fn try_fit(v: &'o [&'o [u8]]) -> Option<Self>{
        v.try_into().map(ArgV).ok()
    }
}
impl<'o> EvalScopeImpl for ArgV<'o>{
    fn about(&self) -> (String, String) {
        ("user input list".into(), "Provide values, access with [0] [1] .. [7] ".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( "0" , 0..=0,Some(true), "argv[0]", |t:&Self,_| Ok(t.0.get(0).context("no 0 value")?.to_vec())),
            ( "1" , 0..=0,Some(true), "argv[1]", |t:&Self,_| Ok(t.0.get(1).context("no 1 value")?.to_vec())),
            ( "2" , 0..=0,Some(true), "argv[2]", |t:&Self,_| Ok(t.0.get(2).context("no 2 value")?.to_vec())),
            ( "3" , 0..=0,Some(true), "argv[3]", |t:&Self,_| Ok(t.0.get(3).context("no 3 value")?.to_vec())),
            ( "4" , 0..=0,Some(true), "argv[4]", |t:&Self,_| Ok(t.0.get(4).context("no 4 value")?.to_vec())),
            ( "5" , 0..=0,Some(true), "argv[5]", |t:&Self,_| Ok(t.0.get(5).context("no 5 value")?.to_vec())),
            ( "6" , 0..=0,Some(true), "argv[6]", |t:&Self,_| Ok(t.0.get(6).context("no 6 value")?.to_vec())),
            ( "7" , 0..=0,Some(true), "argv[7]", |t:&Self,_| Ok(t.0.get(7).context("no 7 value")?.to_vec()))
        ])
    }
}

#[test]
fn try_applyresult(){
    
    fn one() -> ApplyResult<isize>{
        let v = ApplyResult::Value(1)?;
        ApplyResult::Value(v)
    }
    fn one_opt() -> ApplyResult<isize>{
        let v:isize = Some(1)?;
        ApplyResult::Value(v)
    }
    fn one_ok() -> ApplyResult<isize>{
        let v:isize = Ok::<isize,anyhow::Error>(1)?;
        ApplyResult::Value(v)
    }
    fn one_ok_some() -> ApplyResult<isize>{
        let v:isize = Ok::<Option<isize>,anyhow::Error>(Some(1))??;
        ApplyResult::Value(v)
    }
    assert!(matches!(one(),ApplyResult::Value(1)));
    assert!(matches!(one_ok(),ApplyResult::Value(1)));
    assert!(matches!(one_opt(),ApplyResult::Value(1)));
    assert!(matches!(one_ok_some(),ApplyResult::Value(1)));

    fn none() -> ApplyResult<isize>{
        let _v: ApplyResult<isize> = None?;
        ApplyResult::Value(1)
    }
    fn none_v() -> ApplyResult<isize>{
        let v:isize = None?;
        ApplyResult::Value(v)
    }
    fn none_errv() -> ApplyResult<isize>{
        let v:Result<isize,anyhow::Error> = None?;
        ApplyResult::Value(v?)
    } 
    fn none_ok() -> ApplyResult<isize>{
        let v:isize = Ok::<_,anyhow::Error>(None)??;
        ApplyResult::Value(v)
    }
    assert!(matches!(none(),ApplyResult::NoValue));
    assert!(matches!(none_v(),ApplyResult::NoValue));
    assert!(matches!(none_errv(),ApplyResult::NoValue));
    assert!(matches!(none_ok(),ApplyResult::NoValue));

    fn err() -> ApplyResult<isize>{
        let _v: ApplyResult<isize> = Err(anyhow!("err"))?;
        ApplyResult::Value(1)
    }
    fn err_v() -> ApplyResult<isize>{
        let _v: isize = Err(anyhow!("err"))?;
        ApplyResult::Value(1)
    }
    fn some_err_v() -> ApplyResult<isize>{
        let _v: isize = Some(Err(anyhow!("err")))??;
        ApplyResult::Value(1)
    } 
    assert!(matches!(err(),ApplyResult::Err(_)));
    assert!(matches!(err_v(),ApplyResult::Err(_)));
    assert!(matches!(some_err_v(),ApplyResult::Err(_)));

    fn required() -> ApplyResult<isize>{
        let v : Option<isize> = None;
        v.context("missing")?.into()
    }
    assert!(matches!(required(),ApplyResult::Err(_)));
}
