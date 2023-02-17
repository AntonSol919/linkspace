// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{fmt::Display, marker::PhantomData, ops::Deref, str::FromStr, string::FromUtf8Error};

use serde::{Deserialize, Serialize};

use crate::{ast::*, eval::*};

/// Impl this to combine with [TypedABE] and get a parsable, typed, abe expression
pub trait ABEValidator
where
    Self: TryFrom<ABList>,
{
    fn check(b: &[ABE]) -> Result<(), MatchError>;
}

/// A convenient type wrapper around expressions.
/// When V impl [ABEValidator] you get `FromStr` and `TryFrom<[u8]>` and [Self::eval].
/// The shape of the ABE result is checked before evaluation.
/// Note that `TypedABE<Vec<u8>>` is very different then `TypedABE<String>`.
/// The former consumes everything and concatenates all the seperators as standard bytes.
/// The later expects no seperators
#[derive(Clone, Default, PartialEq)]
pub struct TypedABE<V>(pub Vec<ABE>, PhantomData<V>);
impl<V> Serialize for TypedABE<V> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl<'de, V: ABEValidator> Deserialize<'de> for TypedABE<V> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let v = Vec::<ABE>::deserialize(deserializer)?;

        V::check(&v).map_err(|e| <D::Error as serde::de::Error>::custom(e.to_string()))?;
        Ok(TypedABE(v, PhantomData))
    }
}
impl From<Vec<ABE>> for TypedABE<Vec<u8>> {
    fn from(value: Vec<ABE>) -> Self {
        TypedABE(value, PhantomData)
    }
}

impl<A: ABEValidator> TypedABE<A> {
    pub fn eval_default(
        &self,
        default: A,
        ctx: &EvalCtx<impl Scope>,
    ) -> Result<A, ABEError<<A as TryFrom<ABList>>::Error>> {
        let ablst = eval(ctx, &self.0).map_err(ABEError::Eval)?;
        if ablst.is_empty() {
            return Ok(default);
        }
        ablst.try_into().map_err(ABEError::TryFrom)
    }
    pub fn eval(
        &self,
        ctx: &EvalCtx<impl Scope>,
    ) -> Result<A, ABEError<<A as TryFrom<ABList>>::Error>> {
        let ablst = eval(ctx, &self.0).map_err(ABEError::Eval)?;
        ablst.try_into().map_err(ABEError::TryFrom)
    }
    pub fn from(value: Vec<ABE>) -> Result<Self, MatchError> {
        A::check(&value)?;
        Ok(TypedABE(value, PhantomData))
    }
    pub fn new_unchecked(it: impl IntoIterator<Item = ABE>) -> Self {
        TypedABE(it.into_iter().collect(), PhantomData)
    }
    pub const fn from_unchecked(v: Vec<ABE>) -> Self {
        TypedABE(v, PhantomData)
    }
}
impl<A> TypedABE<A> {
    pub fn try_as<T: ABEValidator>(self) -> Result<TypedABE<T>, MatchError> {
        TypedABE::from(self.0)
    }
}

impl<A> Deref for TypedABE<A> {
    type Target = Vec<ABE>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<O: ABEValidator> TryFrom<&[ABE]> for TypedABE<O> {
    type Error = ABEError<<O as TryFrom<ABList>>::Error>;
    fn try_from(value: &[ABE]) -> Result<Self, Self::Error> {
        O::check(value).map_err(ABEError::MatchError)?;
        Ok(TypedABE(value.to_vec(), PhantomData))
    }
}
impl<O> From<TypedABE<O>> for Expr {
    fn from(val: TypedABE<O>) -> Self {
        Expr::Lst(val.0)
    }
}
impl<O> From<TypedABE<O>> for Vec<ABE> {
    fn from(val: TypedABE<O>) -> Self {
        val.0
    }
}
impl<O: ABEValidator> TryFrom<&[u8]> for TypedABE<O> {
    type Error = ABEError<O::Error>;
    fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
        let abe = parse_abe_b(s).map_err(ABEError::Parse)?;
        O::check(&abe).map_err(ABEError::MatchError)?;
        Ok(TypedABE(abe, PhantomData))
    }
}
impl<O: ABEValidator> FromStr for TypedABE<O> {
    type Err = ABEError<O::Error>;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.as_bytes().try_into()
    }
}
impl<O> Display for TypedABE<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.iter().try_for_each(|v| Display::fmt(v, f))?;
        Ok(())
    }
}
impl<O> std::fmt::Debug for TypedABE<O> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.0).finish()
    }
}

/// Concatenate control characters as is
impl ABEValidator for Vec<u8> {
    fn check(_b: &[ABE]) -> Result<(), MatchError> {
        Ok(())
    }
}
impl ABEValidator for ABList {
    fn check(_b: &[ABE]) -> Result<(), MatchError> {
        Ok(())
    }
}
pub type AnyABE = TypedABE<ABList>;

impl ABEValidator for String {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        let [_] = exact(b)?;
        Ok(())
    }
}
impl TryFrom<ABList> for String {
    type Error = ABEError<FromUtf8Error>;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        let b = value.into_exact_bytes().map_err(|_| {
            ABEError::MatchError(MatchError {
                at: "".into(),
                err: MatchErrorKind::ExpectedExpr,
            })
        })?;
        String::from_utf8(b).map_err(ABEError::TryFrom)
    }
}
pub fn eval_vec<A: ABEValidator>(
    v: Vec<TypedABE<A>>,
    e: &EvalCtx<impl Scope>,
) -> Result<Vec<A>, ABEError<<A as TryFrom<ABList>>::Error>> {
    v.into_iter().map(|v| v.eval(e)).try_collect()
}

pub trait ToABE {
    fn to_abe_str(&self) -> String {
        print_abe(self.to_abe())
    }
    fn to_abe(&self) -> Vec<ABE>;
}
