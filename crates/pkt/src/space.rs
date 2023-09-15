// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
/**
Serializable length delimited &[&[u8]]. e.g. /hello/world.
Holds an upto 8 components.
[Space] and [SpaceBuf] are similar to [std::path::Path] and [std::path::PathBuf], except that the components are length delimited, and allow the null and '/' byte.
[RootedSpace] and [RootedSpaceBuf] are Space's with a [u8;8] prefix that holds the number of components and their offset. 
*/
use std::{borrow::Borrow, ops::Deref, ptr};

#[derive(Copy, Clone, PartialEq, Eq, Default, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SpaceBytes<X: ?Sized> {
    inner_bytes: X,
}
impl<X> SpaceBytes<X> where Self: AsRef<Space>{
    pub fn try_from_inner(inner_bytes: X) -> Result<Self,SpaceError> {
        let sp = SpaceBytes { inner_bytes };
        sp.as_ref().check_components()?;
        Ok(sp)
    }
}

use thiserror::Error;
#[derive(Error, Debug, PartialEq, Copy, Clone)]
pub enum SpaceError {
    #[error("spacename only accepts {MAX_SPACE_DEPTH} components")]
    MaxDepth,
    #[error("exceeded maximum component size ({MAX_SPACENAME_COMPONENT_SIZE} bytes)")]
    ComponentSize,
    #[error("component cant be of size 0")]
    ZeroComponent,
    #[error("mismatch length for last component")]
    TailLength,
    #[error("exceeds max path size ({MAX_SPACENAME_SIZE})")]
    CapacityError,

    #[error("bad path index")]
    MissingIdx,
    #[error("empty component offset")]
    EmptyComponentIdx,
    #[error("path index is wrong")]
    BadOffset,
}

/// Explicitly Space bytes (analogous to [[str]])
pub type Space = SpaceBytes<[u8]>;


/// Owned Space bytes (analogous to String)
pub type SpaceBuf = SpaceBytes<Vec<u8>>;
pub type StaticSpace<const N: usize> = SpaceBytes<[u8; N]>;

impl AsRef<Space> for Space {
    fn as_ref(&self) -> &Space {
        self
    }
}
impl<X: AsRef<[u8]>> FromIterator<X> for SpaceBuf {
    #[track_caller]
    fn from_iter<T: IntoIterator<Item = X>>(iter: T) -> Self {
        SpaceBuf::try_from_iter(iter).unwrap()
    }
}

impl Deref for SpaceBuf {
    type Target = Space;
    fn deref(&self) -> &Self::Target {
        self.as_space()
    }
}
impl<const N: usize> Deref for StaticSpace<N> {
    type Target = Space;
    fn deref(&self) -> &Self::Target {
        Space::from_unchecked(self.as_bytes())
    }
}
impl AsRef<Space> for SpaceBuf {
    fn as_ref(&self) -> &Space {
        self
    }
}
impl<const N: usize> AsRef<Space> for StaticSpace<N> {
    fn as_ref(&self) -> &Space {
        self
    }
}
impl<'r> TryFrom<&'r [&'r [u8]]> for SpaceBuf {
    type Error = SpaceError;
    fn try_from(bytes: &'r [&'r [u8]]) -> Result<Self, Self::Error> {
        SpaceBuf::try_from_iter(bytes)
    }
}

impl Borrow<Space> for SpaceBuf {
    fn borrow(&self) -> &Space {
        self
    }
}
impl ToOwned for Space {
    type Owned = SpaceBuf;
    fn to_owned(&self) -> Self::Owned {
        self.into_spacebuf()
    }
}

impl Space {
    pub const fn empty() -> &'static Space {
        Space::from_unchecked(&[])
    }
    pub const fn from_slice(b: &[u8]) -> Result<&Space, SpaceError> {
        let p = Space::from_unchecked(b);
        if let Err(e) = p.check_components() {
            return Err(e);
        }
        Ok(p)
    }
    pub fn into_spacebuf(&self) -> SpaceBuf {
        SpaceBytes {
            inner_bytes: self.space_bytes().to_vec(),
        }
    }
    pub const fn from_unchecked(b: &[u8]) -> &Space {
        unsafe { &*ptr::from_raw_parts(b.as_ptr().cast(),b.len())}
    }
    pub const fn space_bytes(&self) -> &[u8] {
        &self.inner_bytes
    }
}

impl<const N: usize> StaticSpace<N> {
    #[track_caller]
    pub const fn from_raw(bytes: [u8; N]) -> StaticSpace<N> {
        if let Err(_e) = Space::from_slice(&bytes) {
            panic!("Invalid raw bytes")
        }
        SpaceBytes { inner_bytes: bytes }
    }
    

    pub const fn as_static(&'static self) -> &'static Space {
        Space::from_unchecked(&self.inner_bytes)
    }
    pub fn to_owned(&self) -> SpaceBuf {
        SpaceBuf {
            inner_bytes: self.inner_bytes.to_vec(),
        }
    }
}

impl<X: Borrow<[u8]>> Borrow<[u8]> for SpaceBytes<X> {
    fn borrow(&self) -> &[u8] {
        self.inner_bytes.borrow()
    }
}

impl<X: AsRef<[u8]>> SpaceBytes<X> {
    pub fn into_spacebuf(&self) -> SpaceBuf {
        SpaceBytes {
            inner_bytes: self.inner_bytes.as_ref().to_vec(),
        }
    }
    pub fn calc_depth(&self) -> usize{
        self.as_space().iter().count()
    }
    pub fn as_space(&self) -> &Space {
        Space::from_unchecked(self.inner_bytes.as_ref())
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.inner_bytes.as_ref()
    }
    pub fn inner(&self) -> &X {
        &self.inner_bytes
    }
    pub fn unwrap(self) -> X {
        self.inner_bytes
    }

    #[track_caller]
    pub fn try_join(&self, path: &Space) -> Result<SpaceBuf, SpaceError> {
        self.into_spacebuf().extend_from_iter(path.iter())
    }
    #[track_caller]
    pub fn join(&self, path: &Space) -> SpaceBuf {
        self.try_join(path).unwrap()
    }
}

pub fn space_buf(components: &[&[u8]]) -> SpaceBuf {
    SpaceBuf::from_iter(components)
}

impl SpaceBuf {
    pub const fn new() -> SpaceBuf {
        SpaceBytes {
            inner_bytes: vec![],
        }
    }
    
    pub const fn from_vec_unchecked(bytes: Vec<u8>) -> SpaceBuf {
        SpaceBuf { inner_bytes: bytes }
    }
    pub fn extend_from_iter(
        mut self,
        i: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<Self, SpaceError> {
        let mut depth = self.iter().count();
        for s in i.into_iter() {
            self = self.try_push(s.as_ref())?;
            depth += 1;
        }
        if depth > MAX_SPACE_DEPTH {
            return Err(SpaceError::MaxDepth);
        }
        Ok(self)
    }
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<SpaceBuf, SpaceError> {
        SpaceBytes::new().extend_from_iter(iter)
    }


    pub fn truncate_last(&mut self) {
        if let Some(l) = self.iter().last().map(|v| v.len() + 1) {
            self.inner_bytes.truncate(self.inner_bytes.len() - l);
        }
    }

    pub fn components(&self) -> Result<Vec<Vec<u8>>, SpaceError> {
        let iter = self.as_ref().iter();
        let res = iter.map(Vec::from).collect();
        if iter.space.inner_bytes.is_empty() {
            Ok(res)
        } else {
            Err(SpaceError::TailLength)
        }
    }
    /// Panics if the component is larger then 250 bytes;
    pub fn push(self, s: impl AsRef<[u8]>) -> Self {
        self.try_push(s.as_ref()).unwrap()
    }

    pub fn try_append(self, p: &Space) -> Result<SpaceBuf, SpaceError> {
        self.extend_from_iter(p.iter())
    }
    pub fn try_prepend(mut self, p: &Space) -> Result<SpaceBuf, SpaceError> {
        if self.inner_bytes.len() + p.inner_bytes.len() > MAX_SPACENAME_SIZE {
            return Err(SpaceError::CapacityError);
        }
        self.inner_bytes.splice(0..0, p.inner_bytes.iter().cloned());
        Ok(self)
    }
    pub fn try_push(mut self, component: &[u8]) -> Result<Self, SpaceError> {
        if component.len() > MAX_SPACENAME_COMPONENT_SIZE {
            return Err(SpaceError::ComponentSize);
        }
        if component.is_empty() {
            return Err(SpaceError::ZeroComponent);
        }
        if self.inner_bytes.len() + component.len() + 1 > MAX_SPACENAME_SIZE {
            return Err(SpaceError::CapacityError);
        }
        self.inner_bytes.push(component.len().try_into().unwrap());
        self.inner_bytes.extend_from_slice(component);
        Ok(self)
    }
    #[must_use]
    pub fn push_front(self, component: &[u8]) -> SpaceBuf {
        self.try_push_front(component).unwrap()
    }
    pub fn try_push_front(mut self, component: &[u8]) -> Result<SpaceBuf, SpaceError> {
        if component.len() > MAX_SPACENAME_COMPONENT_SIZE {
            return Err(SpaceError::ComponentSize);
        }
        if component.is_empty() {
            return Err(SpaceError::ZeroComponent);
        }
        if self.inner_bytes.len() + component.len() + 1 > MAX_SPACENAME_SIZE {
            return Err(SpaceError::CapacityError);
        }
        self.inner_bytes.splice(
            0..0,
            [component.len().try_into().unwrap()]
                .into_iter()
                .chain(component.iter().cloned()),
        );
        Ok(self)
    }
}


impl Space {
    pub const fn is_empty(&self) -> bool {
        self.inner_bytes.is_empty()
    }
    pub fn head(&self) -> Option<&[u8]> {
        self.iter().next()
    }
    pub fn tail(&self) -> &Space {
        let mut sp = self.iter();
        sp.next();
        sp.space()
    }
    pub fn parent(&self) -> Option<&Space> {
        if self.is_empty() {
            return None;
        }
        Some(self.split_last().0)
    }
    pub fn pop(&self) -> (&Space, Option<&[u8]>) {
        let (parent, last) = self.split_last();
        (parent, last.and_then(|v| v.iter().next()))
    }
    pub fn split_last(&self) -> (&Space, Option<&Space>) {
        let mut this = self;
        let mut last = None;

        while !this.is_empty() {
            last = Some(this);
            let len :usize = this.inner_bytes[0].into();
            this = Space::from_unchecked(&this.inner_bytes[len + 1..]);
        }
        match last {
            None => (this, None),
            Some(v) => {
                let mid = self.inner_bytes.len() - v.inner_bytes.len();
                let (a, b) = self.byte_split_at(mid);
                (a, Some(b))
            }
        }
    }
    pub const fn check_components(&self) -> Result<u8, SpaceError> {
        match self.checked_iter() {
            Err(e) => Err(e),
            Ok(mut iter) => {
                let mut i = 0;
                while let Some(s) = iter.next_c() {
                    match s {
                        Ok((_, next_it)) => iter = next_it,
                        Err(e) => return Err(e),
                    }
                    i += 1;
                }
                Ok(i)
            }
        }
    }

    pub fn split_first(&self) -> Option<(&[u8], &Space)> {
        if self.is_empty() {
            return None;
        }
        let len = usize::from(self.inner_bytes[0]);
        let bytes = &self.inner_bytes[1..len + 1];
        Some((bytes, Space::from_unchecked(&self.inner_bytes[len + 1..])))
    }
    #[allow(clippy::as_conversions)]
    pub const fn first(&self) -> &[u8] {
        match self.inner_bytes.split_first() {
            Some((len, bytes)) => bytes.split_at(*len as usize).0,
            None => &[],
        }
    }

    pub fn split_at(&self, n: usize) -> (&Space, &Space) {
        let mut it = self.iter();
        match (&mut it).advance_by(n) {
            Ok(_) => self.byte_split_at(self.inner_bytes.len() - it.space.inner_bytes.len()),
            Err(_) => (self, Space::empty()),
        }
    }
    pub fn byte_split_at(&self, mid: usize) -> (&Space, &Space) {
        let (a, b) = self.inner_bytes.split_at(mid);
        (Space::from_unchecked(a), Space::from_unchecked(b))
    }
    pub fn byte_slice(&self, i: usize) -> &Space {
        Space::from_unchecked(&self.inner_bytes[i..])
    }
    pub fn starts_with(&self, prefix: &Space) -> bool {
        self.inner_bytes.starts_with(&prefix.inner_bytes)
    }
    pub fn strip_prefix(&self, prefix: &Space) -> Option<&Space> {
        if !self.inner_bytes.starts_with(&prefix.inner_bytes) {
            return None;
        }
        Some(Space::from_unchecked(
            &self.inner_bytes[prefix.inner_bytes.len()..],
        ))
    }
    pub fn drop_prefix(&self, prefix: &Space) -> &Space {
        self.strip_prefix(prefix)
            .expect(" Target does not start with prefix")
    }
    pub fn to_array(&self) -> arrayvec::ArrayVec<&[u8],MAX_SPACE_DEPTH>{
        arrayvec::ArrayVec::from_iter(self.iter())
    }
    pub fn iter(&self) -> &SpaceIter {
        unsafe{&*ptr::from_raw_parts(ptr::from_ref(self).cast(),self.inner_bytes.len())}
    }
    pub const fn checked_iter(&self) -> Result<&CheckedSpaceIter, SpaceError> {
        if self.inner_bytes.len() > MAX_SPACENAME_SIZE {
            return Err(SpaceError::MaxDepth);
        };
        Ok(unsafe{&*ptr::from_raw_parts(ptr::from_ref(self).cast(),self.inner_bytes.len())})
    }
    /// Return the components and the space upto and including that component
    pub fn track(&self) -> Track {
        Track {
            full: self,
            at: self,
        }
    }
    /// turn '/a/b/c' => '|', '/a', '/a/b', '/a/b/c'
    pub fn scan_space(&self) -> impl Iterator<Item = &Space> {
        let mut tk = self.track();
        std::iter::from_fn(move || {
            if let Some(v) = tk.next() {
                tk = v;
                Some(tk.upto())
            } else {
                None
            }
        })
    }

    /// Byte size of the space. Not to be confused with 'depth'
    pub const fn size(&self) -> usize {
        self.inner_bytes.len()
    }
}

#[repr(transparent)]
pub struct CheckedSpaceIter {
    space: Space,
}

impl CheckedSpaceIter {
    pub const EMPTY: &'static Self = unsafe { &*ptr::from_raw_parts(ptr::from_ref(Space::empty()).cast(), 0)};
    #[allow(clippy::as_conversions)]
    pub const fn next_sp_c(&self) -> Option<Result<(&Space, &Self), SpaceError>> {
        if self.space.inner_bytes.is_empty() {
            return None;
        }
        let len = self.space.inner_bytes[0] as usize;
        if len > MAX_SPACENAME_COMPONENT_SIZE {
            return Some(Err(SpaceError::ComponentSize));
        }
        if len == 0 {
            return Some(Err(SpaceError::ZeroComponent));
        }
        if self.space.inner_bytes.len() <= len {
            return Some(Err(SpaceError::TailLength));
        }
        let (segm,rest) = self.space.inner_bytes.split_at(len+1);
        let rest : *const Self= std::ptr::from_raw_parts(rest.as_ptr().cast(),rest.len());
        Some(Ok((Space::from_unchecked(segm),unsafe{&*rest})))
    }
    pub const fn next_c(&self) -> Option<Result<(&[u8], &Self), SpaceError>> {
        match self.next_sp_c() {
            Some(Ok((s, r))) => Some(Ok((s.inner_bytes.split_at(1).1, r))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

impl<'a> Iterator for &'a CheckedSpaceIter {
    type Item = Result<&'a [u8], SpaceError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_c()? {
            Ok((next, it)) => {
                *self = it;
                Some(Ok(next))
            }
            Err(e) => {
                *self = CheckedSpaceIter::EMPTY;
                Some(Err(e))
            }
        }
    }
}

#[repr(transparent)]
pub struct SpaceIter {
    space: Space,
}
impl std::fmt::Debug for SpaceIter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpaceIter").field("space", &self.space.to_string()).finish()
    }
}
impl SpaceIter {
    pub fn space(&self) -> &Space {
        &self.space
    }
}
impl<'a> Iterator for &'a SpaceIter {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        if self.space.inner_bytes.is_empty() {
            return None;
        }
        let len = usize::from(self.space.inner_bytes[0]);
        if len > MAX_SPACENAME_COMPONENT_SIZE || len == 0 {
            panic!("Invalid space");
        }
        let bytes = &self.space.inner_bytes.get(1..=len)?;
        *self = match self.space.inner_bytes.get(len + 1..) {
            Some(b) => Space::from_unchecked(b).iter(),
            None => Space::from_unchecked(&[]).iter(),
        };
        Some(bytes)
    }
}

#[test]
fn try_pop() {
    crate::space!(pub const COMMON = [b"a",b"b",b"aa"]);
    let v = vec![1u8, 97, 1, 98, 2, 97, 97];
    let mut common = SpaceBuf::from_vec_unchecked(v);
    common.check_components().unwrap();
    assert_eq!(common.as_ref(), COMMON.as_ref());
    let (a, b) = common.split_last();
    assert_eq!(a.space_bytes(), &[1, 97, 1, 98]);
    assert_eq!(b.unwrap().space_bytes(), &[2, 97, 97]);
    common.inner_bytes.push(2);
    assert_eq!(common.check_components(), Err(SpaceError::TailLength));
    common.inner_bytes.pop();
    common.inner_bytes.push(0);
    assert_eq!(common.check_components(), Err(SpaceError::ZeroComponent));
    common.inner_bytes.pop();

    common.inner_bytes[2] = 7;
    assert_eq!(common.check_components(), Err(SpaceError::TailLength));
    common.inner_bytes[2] = 251;
    assert_eq!(common.check_components(), Err(SpaceError::ComponentSize));
}


#[macro_export]
macro_rules! len_prefix_concat {
    ($e:expr) => { $crate::prefix_len(*$e)};
    ($e:expr, $($tail:expr),*) => {
        $crate::concat_components($crate::len_prefix_concat!($e),$crate::len_prefix_concat!($($tail),*))
    };
}
#[macro_export]
macro_rules! total_len {
    ($e:expr, $($tail:expr),*) => { $e.len() + 1 + $crate::total_len!($($tail),*)};
    ($e:expr) => {$e.len() + 1};
}

#[macro_export]
macro_rules! static_space {
    ($($e:expr),*) => {
        $crate::SpaceBytes::from_raw($crate::len_prefix_concat!($($e),*))
	  };
}

#[macro_export]
macro_rules! space {
    // /hack to fix len calculation in nightly
	  (pub const $i:ident = [$($e:expr),*]) => {
		    pub const $i : $crate::SpaceBytes<[u8;$crate::total_len!($($e),*)]>= $crate::static_space!($($e),*);
	  };
    ($($e:expr),*) => {
        $crate::static_space!($($e),*)
    }
}

#[track_caller]
#[allow(clippy::as_conversions)]
pub const fn prefix_len<const N: usize>(component: [u8; N]) -> [u8; N + 1] {
    if N > MAX_SPACENAME_COMPONENT_SIZE {
        panic!()
    }
    if N == 0 {
        panic!()
    }
    let mut result = [0; N + 1];
    result[0] = N as u8;
    let mut i = 0;
    while i < N {
        result[i + 1] = component[i];
        i += 1;
    }
    result
}
pub const fn concat_components<const B: usize, const S: usize>(
    base: [u8; B],
    component: [u8; S],
) -> [u8; B + S] {
    if S > MAX_SPACENAME_SIZE {
        panic!()
    };
    let mut result = [0; B + S];
    let mut i = 0;
    while i < B {
        result[i] = base[i];
        i += 1;
    }
    while i < B + S {
        result[i] = component[i - B];
        i += 1;
    }
    result
}

space!(pub const TEST_SP = [b"test"]);

#[test]
fn concat() {
    assert_eq!([1, 0, 2, 0, 0], len_prefix_concat!(&[0], &[0, 0]));
}
#[test]
fn pop_slice() {
    let mut v = SpaceBuf::from_iter(&[b"hello", b"world"]);
    let x = SpaceBuf::from_iter(&[b"hello"]);
    assert_eq!(v.drop_prefix(&*x), &*SpaceBuf::from_iter(&[b"world"]));
    v.truncate_last();
    assert_eq!(v, x)
}

#[test]
fn check() {
    let mut empty = CheckedSpaceIter::EMPTY;
    assert!(empty.next().is_none(), "bad empty iter?");
    assert!(CheckedSpaceIter::EMPTY.space.inner_bytes.is_empty(), "bad empty iter?");

    use crate::*;
    let space = SpaceBuf::try_from_iter(&["hello", "world"]).unwrap();
    let mut it = space.checked_iter().unwrap();
    assert_eq!(it.next().unwrap().unwrap(), b"hello" as &[u8]);
    assert_eq!(it.next().unwrap().unwrap(), b"world" as &[u8]);
    assert!(it.next().is_none());
    let v = SpaceBuf::from_iter(&[b"a", b"b", b"c"]);
    assert_eq!(v.check_components().unwrap(), 3);
}

#[test]
fn track() {
    let v = SpaceBuf::from_iter(&[b"a", b"b", b"c"]);
    let mut track = v.track();
    assert_eq!(&track.upto().inner_bytes, &[] as &[u8]);
    track = track.next().unwrap();
    assert_eq!(&track.upto().inner_bytes, &[1, b'a']);
    track = track.next().unwrap();
    assert_eq!(&track.upto().inner_bytes, &[1, b'a', 1, b'b']);
    track = track.next().unwrap();
    assert_eq!(&track.upto().inner_bytes, &[1, b'a', 1, b'b', 1, b'c']);
}
pub struct Track<'a> {
    full: &'a Space,
    at: &'a Space,
}
impl<'o> Track<'o> {
    pub fn full(&self) -> &'o Space {
        self.full
    }
    pub fn component(&self) -> Option<&'o [u8]> {
        self.at.iter().next()
    }
    pub fn rest(&self) -> &'o Space {
        self.at
    }
    pub fn upto(&self) -> &'o Space {
        let mid = self.full.inner_bytes.len() - self.at.inner_bytes.len();
        self.full.byte_split_at(mid).0
    }
    pub fn next(&self) -> Option<Track<'o>> {
        let mut it = self.at.iter();
        it.next().map(|_| Track {
            full: self.full,
            at: it.space(),
        })
    }
}

impl Space {
    #[track_caller]
    pub fn rooted(&self) -> RootedSpaceBuf {
        self.try_into_rooted().unwrap()
    }
    pub fn try_into_rooted(&self) -> Result<RootedSpaceBuf, SpaceError> {
        if self.is_empty() {
            return Ok(RootedSpaceBuf::new());
        }
        let mut data = Vec::with_capacity(self.size() + 8);
        data.extend_from_slice(&[0; 8]);

        let mut it = self.checked_iter()?;
        let mut count = 0u8;
        for i in 0..8 {
            data[i] = unsafe {u8::try_from(data.len()).unwrap_unchecked().checked_sub(8).unwrap_unchecked()};
            if let Some(v) = it.next().transpose()? {
                data.push( unsafe {v.len().try_into().unwrap_unchecked()});
                data.extend_from_slice(v);
                count += 1;
            }
        }
        data[0] = count;
        let v = RootedSpaceBuf::from_unchecked(data);
        debug_assert!(
            v.check_components().is_ok(),
            "{:?} {:?} {:?}",
            v.check_components(),
            v.rooted_bytes(),
            v.space_bytes()
        );
        Ok(v)
    }
}
