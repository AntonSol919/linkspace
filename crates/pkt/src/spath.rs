// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
/**
Serializable length delimited &[&[u8]]. e.g. /hello/world.
Holds an upto 8 components.
Components are non-empty, length delimited, bytes.

[SPath] and [SPathBuf] are similar to [std::path::Path] and [std::path::PathBuf].
[IPath] and [IPathBuf] are SPath's with a [u8;8] prefix that holds the number of components and their offset.
*/
use std::{borrow::Borrow, ops::Deref};

#[derive(Copy, Clone, PartialEq, Eq, Default, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SPathBytes<X: ?Sized> {
    spath_bytes: X,
}

use thiserror::Error;
#[derive(Error, Debug, PartialEq, Copy, Clone)]
pub enum PathError {
    #[error("Path only allows upto {MAX_PATH_LEN} components")]
    MaxLen,
    #[error("Exceeded maximum component size ({MAX_SPATH_COMPONENT_SIZE} bytes)")]
    ComponentSize,
    #[error("Component cant be of size 0")]
    ZeroComponent,
    #[error("Mismatch length for last component")]
    TailLength,
    #[error("Exceeds Max SPath size ({MAX_SPATH_SIZE})")]
    CapacityError,

    #[error("TODO")]
    IdxError,
    #[error("Missing Index. Expected at least 8 bytes (or none)")]
    MissingIdx,
    #[error("The offset of the non components is wrong")]
    EmptyComponentIdx,
    #[error("The offset does  not agree with the spath")]
    BadOffset,
}

/// Explicitly SPath bytes (analogous to [[str]])
pub type SPath = SPathBytes<[u8]>;


/// Owned SPath bytes (analogous to String)
pub type SPathBuf = SPathBytes<Vec<u8>>;
pub type ConstSPath<const N: usize> = SPathBytes<[u8; N]>;

impl AsRef<SPath> for SPath {
    fn as_ref(&self) -> &SPath {
        self
    }
}
impl<X: AsRef<[u8]>> FromIterator<X> for SPathBuf {
    fn from_iter<T: IntoIterator<Item = X>>(iter: T) -> Self {
        SPathBuf::try_from_iter(iter).unwrap()
    }
}

impl Deref for SPathBuf {
    type Target = SPath;
    fn deref(&self) -> &Self::Target {
        self.as_spath()
    }
}
impl<const N: usize> Deref for ConstSPath<N> {
    type Target = SPath;
    fn deref(&self) -> &Self::Target {
        SPath::from_unchecked(self.as_bytes())
    }
}
impl AsRef<SPath> for SPathBuf {
    fn as_ref(&self) -> &SPath {
        self
    }
}
impl<const N: usize> AsRef<SPath> for ConstSPath<N> {
    fn as_ref(&self) -> &SPath {
        self
    }
}
impl<'r> TryFrom<&'r [&'r [u8]]> for SPathBuf {
    type Error = PathError;
    fn try_from(bytes: &'r [&'r [u8]]) -> Result<Self, Self::Error> {
        SPathBuf::try_from_iter(bytes)
    }
}

impl Borrow<SPath> for SPathBuf {
    fn borrow(&self) -> &SPath {
        self
    }
}
impl ToOwned for SPath {
    type Owned = SPathBuf;
    fn to_owned(&self) -> Self::Owned {
        self.into_spathbuf()
    }
}

impl SPath {
    pub const fn empty() -> &'static SPath {
        SPath::from_unchecked(&[])
    }
    pub const fn from_slice(b: &[u8]) -> Result<&SPath, PathError> {
        let p = SPath::from_unchecked(b);
        if let Err(e) = p.check_components() {
            return Err(e);
        }
        Ok(p)
    }
    pub fn into_spathbuf(&self) -> SPathBuf {
        SPathBytes {
            spath_bytes: self.spath_bytes().to_vec(),
        }
    }
    pub const fn from_unchecked(b: &[u8]) -> &SPath {
        unsafe { std::mem::transmute(b) }
    }
    pub const fn spath_bytes(&self) -> &[u8] {
        &self.spath_bytes
    }
}

impl<const N: usize> ConstSPath<N> {
    #[track_caller]
    pub const fn from_raw(bytes: [u8; N]) -> ConstSPath<N> {
        if let Err(_e) = SPath::from_slice(&bytes) {
            panic!("Invalid raw bytes")
        }
        SPathBytes { spath_bytes: bytes }
    }

    pub const fn as_static(&'static self) -> &'static SPath {
        SPath::from_unchecked(&self.spath_bytes)
    }
    pub fn to_owned(&self) -> SPathBuf {
        SPathBuf {
            spath_bytes: self.spath_bytes.to_vec(),
        }
    }
}

impl<X: Borrow<[u8]>> Borrow<[u8]> for SPathBytes<X> {
    fn borrow(&self) -> &[u8] {
        self.spath_bytes.borrow()
    }
}

impl<X: AsRef<[u8]>> SPathBytes<X> {
    pub fn into_spathbuf(&self) -> SPathBuf {
        SPathBytes {
            spath_bytes: self.spath_bytes.as_ref().to_vec(),
        }
    }
    pub fn as_spath(&self) -> &SPath {
        SPath::from_unchecked(self.spath_bytes.as_ref())
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.spath_bytes.as_ref()
    }
    pub fn inner(&self) -> &X {
        &self.spath_bytes
    }
    pub fn unwrap(self) -> X {
        self.spath_bytes
    }

    #[track_caller]
    pub fn try_join(&self, path: &SPath) -> Result<SPathBuf, PathError> {
        self.into_spathbuf().extend_from_iter(path.iter())
    }
    #[track_caller]
    pub fn join(&self, path: &SPath) -> SPathBuf {
        self.try_join(path).unwrap()
    }
}

pub fn spath_buf(components: &[&[u8]]) -> SPathBuf {
    SPathBuf::from(components)
}

impl SPathBuf {
    pub const fn new() -> SPathBuf {
        SPathBytes {
            spath_bytes: vec![],
        }
    }
    pub const fn from_vec_unchecked(bytes: Vec<u8>) -> SPathBuf {
        SPathBuf { spath_bytes: bytes }
    }
    pub fn extend_from_iter(
        mut self,
        i: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<Self, PathError> {
        let mut count = self.iter().count();
        for s in i.into_iter() {
            self = self.try_push(s.as_ref())?;
            count += 1;
        }
        if count > MAX_PATH_LEN {
            return Err(PathError::MaxLen);
        }
        Ok(self)
    }
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<SPathBuf, PathError> {
        SPathBytes::new().extend_from_iter(iter)
    }
    #[track_caller]
    pub fn from(iter: impl IntoIterator<Item = impl AsRef<[u8]>>) -> SPathBuf {
        SPathBuf::try_from_iter(iter).unwrap()
    }

    pub fn truncate_last(&mut self) {
        if let Some(l) = self.iter().last().map(|v| v.len() + 1) {
            self.spath_bytes.truncate(self.spath_bytes.len() - l);
        }
    }

    pub fn components(&self) -> Result<Vec<Vec<u8>>, PathError> {
        let iter = self.as_ref().iter();
        let res = iter.map(Vec::from).collect();
        if iter.spath.spath_bytes.is_empty() {
            Ok(res)
        } else {
            Err(PathError::TailLength)
        }
    }
    /// Panics if the component is larger then 250 bytes;
    pub fn push(self, s: impl AsRef<[u8]>) -> Self {
        self.try_push(s.as_ref()).unwrap()
    }

    pub fn try_append(self, p: &SPath) -> Result<SPathBuf, PathError> {
        self.extend_from_iter(p.iter())
    }
    pub fn try_prepend(mut self, p: &SPath) -> Result<SPathBuf, PathError> {
        if self.spath_bytes.len() + p.spath_bytes.len() > MAX_SPATH_SIZE {
            return Err(PathError::CapacityError);
        }
        self.spath_bytes.splice(0..0, p.spath_bytes.iter().cloned());
        Ok(self)
    }
    pub fn try_push(mut self, component: &[u8]) -> Result<Self, PathError> {
        if component.len() > MAX_SPATH_COMPONENT_SIZE {
            return Err(PathError::ComponentSize);
        }
        if component.is_empty() {
            return Err(PathError::ZeroComponent);
        }
        if self.spath_bytes.len() + component.len() + 1 > MAX_SPATH_SIZE {
            return Err(PathError::CapacityError);
        }
        self.spath_bytes.push(component.len() as u8);
        self.spath_bytes.extend_from_slice(component);
        Ok(self)
    }
    #[must_use]
    pub fn push_front(self, component: &[u8]) -> SPathBuf {
        self.try_push_front(component).unwrap()
    }
    pub fn try_push_front(mut self, component: &[u8]) -> Result<SPathBuf, PathError> {
        if component.len() > MAX_SPATH_COMPONENT_SIZE {
            return Err(PathError::ComponentSize);
        }
        if component.is_empty() {
            return Err(PathError::ZeroComponent);
        }
        if self.spath_bytes.len() + component.len() + 1 > MAX_SPATH_SIZE {
            return Err(PathError::CapacityError);
        }
        self.spath_bytes.splice(
            0..0,
            [component.len() as u8]
                .into_iter()
                .chain(component.iter().cloned()),
        );
        Ok(self)
    }
}

impl SPath {
    pub const fn is_empty(&self) -> bool {
        self.spath_bytes.is_empty()
    }
    pub fn head(&self) -> Option<&[u8]> {
        self.iter().next()
    }
    pub fn tail(&self) -> &SPath {
        let mut sp = self.iter();
        sp.next();
        sp.spath()
    }
    pub fn parent(&self) -> Option<&SPath> {
        if self.is_empty() {
            return None;
        }
        Some(self.split_last().0)
    }
    pub fn pop(&self) -> (&SPath, Option<&[u8]>) {
        let (parent, last) = self.split_last();
        (parent, last.and_then(|v| v.iter().next()))
    }
    pub fn split_last(&self) -> (&SPath, Option<&SPath>) {
        let mut this = self;
        let mut last = None;

        while !this.is_empty() {
            last = Some(this);
            let len = this.spath_bytes[0] as usize;
            this = SPath::from_unchecked(&this.spath_bytes[len + 1..]);
        }
        match last {
            None => (this, None),
            Some(v) => {
                let mid = self.spath_bytes.len() - v.spath_bytes.len();
                let (a, b) = self.byte_split_at(mid);
                (a, Some(b))
            }
        }
    }
    pub const fn check_components(&self) -> Result<u8, PathError> {
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

    pub fn split_first(&self) -> Option<(&[u8], &SPath)> {
        if self.is_empty() {
            return None;
        }
        let len = self.spath_bytes[0] as usize;
        let bytes = &self.spath_bytes[1..len + 1];
        Some((bytes, SPath::from_unchecked(&self.spath_bytes[len + 1..])))
    }
    pub const fn first(&self) -> &[u8] {
        match self.spath_bytes.split_first() {
            Some((len, bytes)) => bytes.split_at(*len as usize).0,
            None => &[],
        }
    }

    pub fn split_at(&self, n: usize) -> (&SPath, &SPath) {
        let mut it = self.iter();
        match (&mut it).advance_by(n) {
            Ok(_) => self.byte_split_at(self.spath_bytes.len() - it.spath.spath_bytes.len()),
            Err(_) => (self, SPath::empty()),
        }
    }
    pub fn byte_split_at(&self, mid: usize) -> (&SPath, &SPath) {
        let (a, b) = self.spath_bytes.split_at(mid);
        (SPath::from_unchecked(a), SPath::from_unchecked(b))
    }
    pub fn byte_slice(&self, i: usize) -> &SPath {
        SPath::from_unchecked(&self.spath_bytes[i..])
    }
    pub fn starts_with(&self, prefix: &SPath) -> bool {
        self.spath_bytes.starts_with(&prefix.spath_bytes)
    }
    pub fn strip_prefix(&self, prefix: &SPath) -> Option<&SPath> {
        if !self.spath_bytes.starts_with(&prefix.spath_bytes) {
            return None;
        }
        Some(SPath::from_unchecked(
            &self.spath_bytes[prefix.spath_bytes.len()..],
        ))
    }
    pub fn drop_prefix(&self, prefix: &SPath) -> &SPath {
        self.strip_prefix(prefix)
            .expect(" Target does not start with prefix")
    }
    pub fn collect(&self) -> arrayvec::ArrayVec<&[u8],MAX_PATH_LEN>{
        arrayvec::ArrayVec::from_iter(self.iter())
    }
    pub fn iter(&self) -> &SPathIter {
        unsafe { std::mem::transmute(self) }
    }
    pub const fn checked_iter(&self) -> Result<&CheckedSPathIter, PathError> {
        if self.spath_bytes.len() > MAX_SPATH_SIZE {
            return Err(PathError::MaxLen);
        };
        Ok(unsafe { std::mem::transmute(self) })
    }
    /// Return the components and the spath upto and including that component
    pub fn track(&self) -> Track {
        Track {
            full: self,
            at: self,
        }
    }
    /// turn '/a/b/c' => '|', '/a', '/a/b', '/a/b/c'
    pub fn scan_spaths(&self) -> impl Iterator<Item = &SPath> {
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

    /// Byte size of the spath. Not to be confused with 'len'
    pub const fn size(&self) -> usize {
        self.spath_bytes.len()
    }
}

#[repr(transparent)]
pub struct CheckedSPathIter {
    spath: SPath,
}

impl CheckedSPathIter {
    pub const EMPTY: &Self = unsafe { std::mem::transmute(&[] as &[u8]) };
    pub const fn next_sp_c(&self) -> Option<Result<(&SPath, &Self), PathError>> {
        if self.spath.spath_bytes.is_empty() {
            return None;
        }
        let len = self.spath.spath_bytes[0] as usize;
        if len > MAX_SPATH_COMPONENT_SIZE {
            return Some(Err(PathError::ComponentSize));
        }
        if len == 0 {
            return Some(Err(PathError::ZeroComponent));
        }
        match self.spath.spath_bytes.get(..=len) {
            None => Some(Err(PathError::TailLength)),
            Some(segm) => {
                let n = self.spath.spath_bytes.get(len + 1..).unwrap_or(&[]);
                let n = unsafe { std::mem::transmute(n) };
                Some(Ok((SPath::from_unchecked(segm), n)))
            }
        }
    }
    pub const fn next_c(&self) -> Option<Result<(&[u8], &Self), PathError>> {
        match self.next_sp_c() {
            Some(Ok((s, r))) => Some(Ok((s.spath_bytes.get(1..).unwrap(), r))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

impl<'a> Iterator for &'a CheckedSPathIter {
    type Item = Result<&'a [u8], PathError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_c()? {
            Ok((next, it)) => {
                *self = it;
                Some(Ok(next))
            }
            Err(e) => {
                *self = CheckedSPathIter::EMPTY;
                Some(Err(e))
            }
        }
    }
}

#[repr(transparent)]
pub struct SPathIter {
    spath: SPath,
}
impl std::fmt::Debug for SPathIter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SPathIter").field("spath", &self.spath.to_string()).finish()
    }
}
impl SPathIter {
    pub fn spath(&self) -> &SPath {
        &self.spath
    }
}
impl<'a> Iterator for &'a SPathIter {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        if self.spath.spath_bytes.is_empty() {
            return None;
        }
        let len = self.spath.spath_bytes[0] as usize;
        if len > MAX_SPATH_COMPONENT_SIZE || len == 0 {
            panic!("Invalid spath");
        }
        let bytes = &self.spath.spath_bytes.get(1..=len)?;
        *self = match self.spath.spath_bytes.get(len + 1..) {
            Some(b) => SPath::from_unchecked(b).iter(),
            None => SPath::from_unchecked(&[]).iter(),
        };
        Some(bytes)
    }
}

#[test]
fn try_pop() {
    crate::spath!(pub const COMMON = [b"a",b"b",b"aa"]);
    let v = vec![1u8, 97, 1, 98, 2, 97, 97];
    let mut common = SPathBuf::from_vec_unchecked(v);
    common.check_components().unwrap();
    assert_eq!(common.as_ref(), COMMON.as_ref());
    let (a, b) = common.split_last();
    assert_eq!(a.spath_bytes(), &[1, 97, 1, 98]);
    assert_eq!(b.unwrap().spath_bytes(), &[2, 97, 97]);
    common.spath_bytes.push(2);
    assert_eq!(common.check_components(), Err(PathError::TailLength));
    common.spath_bytes.pop();
    common.spath_bytes.push(0);
    assert_eq!(common.check_components(), Err(PathError::ZeroComponent));
    common.spath_bytes.pop();

    common.spath_bytes[2] = 7;
    assert_eq!(common.check_components(), Err(PathError::TailLength));
    common.spath_bytes[2] = 251;
    assert_eq!(common.check_components(), Err(PathError::ComponentSize));
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
macro_rules! const_spath {
    ($($e:expr),*) => {
        $crate::SPathBytes::from_raw($crate::len_prefix_concat!($($e),*))
	  };
}

#[macro_export]
macro_rules! spath {
    // /hack to fix len calculation in nightly
	  (pub const $i:ident = [$($e:expr),*]) => {
		    pub const $i : $crate::SPathBytes<[u8;$crate::total_len!($($e),*)]>= $crate::const_spath!($($e),*);
	  };
    ($($e:expr),*) => {
        $crate::const_spath!($($e),*)
    }
}

#[track_caller]
pub const fn prefix_len<const N: usize>(component: [u8; N]) -> [u8; N + 1] {
    if N > MAX_SPATH_COMPONENT_SIZE {
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
    if S > MAX_SPATH_SIZE {
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

spath!(pub const TEST_SP = [b"test"]);

#[test]
fn concat() {
    assert_eq!([1, 0, 2, 0, 0], len_prefix_concat!(&[0], &[0, 0]));
}
#[test]
fn pop_slice() {
    let mut v = SPathBuf::from(&[b"hello", b"world"]);
    let x = SPathBuf::from(&[b"hello"]);
    assert_eq!(v.drop_prefix(&*x), &*SPathBuf::from(&[b"world"]));
    v.truncate_last();
    assert_eq!(v, x)
}

#[test]
fn check() {
    use crate::*;
    let spath = SPathBuf::try_from_iter(&["hello", "world"]).unwrap();
    let mut it = spath.checked_iter().unwrap();
    assert_eq!(it.next().unwrap().unwrap(), b"hello" as &[u8]);
    assert_eq!(it.next().unwrap().unwrap(), b"world" as &[u8]);
    assert!(it.next().is_none());
    let v = SPathBuf::from(&[b"a", b"b", b"c"]);
    assert_eq!(v.check_components().unwrap(), 3);
}

#[test]
fn track() {
    let v = SPathBuf::from(&[b"a", b"b", b"c"]);
    let mut track = v.track();
    assert_eq!(&track.upto().spath_bytes, &[] as &[u8]);
    track = track.next().unwrap();
    assert_eq!(&track.upto().spath_bytes, &[1, b'a']);
    track = track.next().unwrap();
    assert_eq!(&track.upto().spath_bytes, &[1, b'a', 1, b'b']);
    track = track.next().unwrap();
    assert_eq!(&track.upto().spath_bytes, &[1, b'a', 1, b'b', 1, b'c']);
}
pub struct Track<'a> {
    full: &'a SPath,
    at: &'a SPath,
}
impl<'o> Track<'o> {
    pub fn full(&self) -> &'o SPath {
        self.full
    }
    pub fn component(&self) -> Option<&'o [u8]> {
        self.at.iter().next()
    }
    pub fn rest(&self) -> &'o SPath {
        self.at
    }
    pub fn upto(&self) -> &'o SPath {
        let mid = self.full.spath_bytes.len() - self.at.spath_bytes.len();
        self.full.byte_split_at(mid).0
    }
    pub fn next(&self) -> Option<Track<'o>> {
        let mut it = self.at.iter();
        it.next().map(|_| Track {
            full: self.full,
            at: it.spath(),
        })
    }
}

impl SPath {
    #[track_caller]
    pub fn ipath(&self) -> IPathBuf {
        self.try_ipath().unwrap()
    }
    pub fn try_ipath(&self) -> Result<IPathBuf, PathError> {
        if self.is_empty() {
            return Ok(IPathBuf::new());
        }
        let mut data = Vec::with_capacity(self.size() + 8);
        data.extend_from_slice(&[0; 8]);

        let mut it = self.checked_iter()?;
        let mut count = 0u8;
        for i in 0..8 {
            data[i] = data.len() as u8 - 8;
            if let Some(v) = it.next().transpose()? {
                data.push(v.len() as u8);
                data.extend_from_slice(v);
                count += 1;
            }
        }
        data[0] = count;
        let v = IPathBuf::from_unchecked(data);
        debug_assert!(
            v.check_components().is_ok(),
            "{:?} {:?} {:?}",
            v.check_components(),
            v.ipath_bytes(),
            v.spath_bytes()
        );
        Ok(v)
    }
}
