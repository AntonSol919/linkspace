// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WriteResult<New = (), Old = New> {
    New(New),
    Old(Old),
}
impl<A, B> WriteResult<A, B> {
    pub fn is_old(&self) -> bool {
        matches!(self, WriteResult::Old(_))
    }
    pub fn as_ref(&self) -> WriteResult<&A, &B> {
        match &self {
            WriteResult::New(a) => WriteResult::New(a),
            WriteResult::Old(b) => WriteResult::Old(b),
        }
    }
    pub fn new_value(self) -> Option<A> {
        match self {
            WriteResult::New(a) => Some(a),
            WriteResult::Old(_) => None,
        }
    }
    pub fn is_new(&self) -> bool {
        matches!(self, WriteResult::New(_))
    }
    pub fn unref(self) -> WriteResult {
        unref(self)
    }
    pub fn map_new<X>(self, f: impl FnOnce(A) -> X) -> WriteResult<X, B> {
        match self {
            WriteResult::New(v) => WriteResult::New(f(v)),
            WriteResult::Old(v) => WriteResult::Old(v),
        }
    }
}
impl<A> WriteResult<A, A> {
    pub fn from(is_new: bool, v: A) -> WriteResult<A, A> {
        if is_new {
            WriteResult::New(v)
        } else {
            WriteResult::Old(v)
        }
    }
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> WriteResult<B, B> {
        match self {
            WriteResult::New(v) => WriteResult::New(f(v)),
            WriteResult::Old(v) => WriteResult::Old(f(v)),
        }
    }
    /// (Result,is_new)
    pub fn unwrap(self) -> (A, bool) {
        match self {
            WriteResult::New(v) => (v, true),
            WriteResult::Old(v) => (v, false),
        }
    }
}

pub fn unref<A, B>(v: WriteResult<A, B>) -> WriteResult {
    match v {
        WriteResult::New(_) => WriteResult::New(()),
        WriteResult::Old(_) => WriteResult::Old(()),
    }
}
