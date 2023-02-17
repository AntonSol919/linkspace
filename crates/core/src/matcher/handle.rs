// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{ rc::{ Rc}, cell::Cell };
use super::watch::*;

pub trait InnerHandle {
    fn is_alive(&self) -> bool;
}
impl InnerHandle for () {
    fn is_alive(&self) -> bool {
        true
    }
}
pub trait Handle {
    type Inner: InnerHandle;
    fn is_eq(&self, inner: &Self::Inner) -> bool;
    fn is_alive(&self) -> bool;
    fn kind(&self) -> UWatchKind<()>;
}

impl<I> InnerHandle for Rc<Inner<I>>{
    fn is_alive(&self) -> bool {
        self.detach.get() || Rc::strong_count(&self) > 1  
    }
}
impl<I> Handle for RcHandle<I>{
    type Inner= Rc<Inner<I>>;
    fn is_eq(&self, inner: &Self::Inner) -> bool {
        Rc::as_ptr(&self.0) == Rc::as_ptr(inner)
    }

    fn is_alive(&self) -> bool {
        self.0.detach.get() || Rc::strong_count(&self.0) > 1  
    }

    fn kind(&self) -> UWatchKind<()> {
        self.0.info.info()
    }
}

#[must_use]
#[derive(Clone)]
pub struct RcHandle<I =()>(Rc<Inner<I>>);
impl<I> RcHandle<I>{
    pub fn new(w: UWatchKind<I>) -> (RcHandle<I>,Rc<Inner<I>>) {
        let handle = Rc::new(Inner{detach: Cell::new(false),info: w});
        (RcHandle(handle.clone()),handle)
    }
    pub fn detach(&self){
        self.0.detach.set(true)
    }
}
pub struct Inner<I=()> {
    info: UWatchKind<I>,
    detach: Cell<bool>
}
impl PartialEq for RcHandle{
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl<I> Drop for RcHandle<I> {
    fn drop(&mut self) {
        if !self.0.detach.get() && Rc::strong_count(&self.0) <= 2{  
            tracing::info!(watch=?self.kind(),"Dropping handle and will disable");
        }
    }
}



