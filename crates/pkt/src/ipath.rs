/*
Copyright Anton Sol

This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/
use crate::{spath::*, MAX_IPATH_SIZE, MAX_PATH_LEN, MAX_SPATH_COMPONENT_SIZE};
use std::{borrow::Borrow, ops::Deref, ptr};

/// An IPath is an [[SPath]] with 8 bytes prefix: (length, \[component_offset;7\])
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct IPathBytes<X: ?Sized> {
    ipath_bytes: X,
}
pub type IPathC<const U: usize> = IPathBytes<[u8; U]>;
pub type IPath = IPathBytes<[u8]>;

//pub type IPathStack = IPathBytes<([u8;MAX_IPATH_SIZE],())>;

impl ToOwned for IPath {
    type Owned = IPathBuf;
    fn to_owned(&self) -> Self::Owned {
        self.into_ipathbuf()
    }
}
impl Borrow<IPath> for IPathBuf {
    fn borrow(&self) -> &IPath {
        self.deref()
    }
}
impl<const U: usize> AsRef<IPath> for IPathC<U> {
    fn as_ref(&self) -> &IPath {
        IPath::from_unchecked(&self.ipath_bytes)
    }
}


#[allow(clippy::as_conversions,clippy::cast_possible_truncation)]
pub const fn ipath1<const C0: usize>(c0: &[u8; C0]) -> IPathC<{ C0 + 9 }> {
    assert!(C0 < MAX_SPATH_COMPONENT_SIZE );
    let mut r = [0u8; C0 + 9];
    r[0] = 1;
    let mut i = 1;
    while i < 8 {
        r[i] = (C0 + 1) as u8;
        i += 1;
    }
    r[i] = C0 as u8;
    let mut i = 0;
    while i < C0 {
        r[9 + i] = c0[i];
        i += 1;
    }
    match IPathBytes::<[u8; C0 + 9]>::from(r) {
        Ok(v) => v,
        Err(_) => panic!("invalid ipath len"),
    }
}
impl<const N: usize> IPathBytes<[u8; N]> {
    pub const fn from(b: [u8; N]) -> Result<Self, PathError> {
        let p = IPath::from_unchecked(&b);
        if let Err(e) = p.check_components() {
            return Err(e);
        }
        Ok(IPathBytes { ipath_bytes: b })
    }
}

pub type IPathBuf = IPathBytes<Vec<u8>>;
impl Deref for IPathBuf {
    type Target = IPath;
    fn deref(&self) -> &Self::Target {
        IPath::from_unchecked(self.ipath_bytes.as_slice())
    }
}
impl<const C: usize> Deref for IPathC<C> {
    type Target = IPath;
    fn deref(&self) -> &Self::Target {
        IPath::from_unchecked(&self.ipath_bytes)
    }
}
impl Deref for IPath {
    type Target = SPath;
    fn deref(&self) -> &Self::Target {
        self.spath()
    }
}
impl TryInto<IPathBuf> for SPathBuf {
    type Error = PathError;
    fn try_into(self) -> Result<IPathBuf, Self::Error> {
        self.try_ipath()
    }
}
impl Default for &IPath {
    fn default() -> Self {
        IPath::EMPTY
    }
}
impl Default for IPathBuf {
    fn default() -> Self {
        Self::new()
    }
}
impl IPathBuf {
    pub const DEFAULT: Self = Self::new();
    pub const fn from_unchecked(ipath_bytes: Vec<u8>) -> Self {
        IPathBuf { ipath_bytes }
    }
    #[track_caller]
    pub fn append(mut self, component: &[u8]) -> Self {
        self.try_append_component(component).unwrap();
        self
    }

    #[allow(clippy::as_conversions,clippy::cast_possible_truncation)]
    pub fn try_append_component(&mut self, component: &[u8]) -> Result<&mut Self, PathError> {
        let bs = &mut self.ipath_bytes;
        if component.len() > MAX_SPATH_COMPONENT_SIZE {
            return Err(PathError::ComponentSize);
        }
        if component.is_empty() {
            return Err(PathError::ZeroComponent);
        }
        if bs.is_empty() {
            *bs = vec![0; 8]
        }
        if bs[0] >= MAX_PATH_LEN as u8{
            return Err(PathError::CapacityError);
        }
        if bs.len() + component.len() + 1 > MAX_IPATH_SIZE {
            return Err(PathError::MaxLen);
        }

        let new_len = bs[0] + 1;
        bs[0] = new_len;
        bs.push(component.len() as u8);
        bs.extend_from_slice(component);
        let len = bs.len() as u8 - 8;
        bs[new_len as usize..8].iter_mut().for_each(|a| {
            *a = len;
        });
        Ok(self)
    }
    pub fn join(self, i: &SPath) -> IPathBuf {
        self.try_join(i).unwrap()
    }
    pub fn try_join(mut self, i: &SPath) -> Result<IPathBuf, PathError> {
        self.extend_from_iter(i.iter())?;
        Ok(self)
    }
    pub fn extend_from_iter(
        &mut self,
        i: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<(), PathError> {
        for s in i.into_iter() {
            self.try_append_component(s.as_ref())?;
        }
        Ok(())
    }
    //FIXME : this is ineficient
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<IPathBuf, PathError> {
        let mut new = IPathBuf::new();
        new.extend_from_iter(iter)?;
        Ok(new)
    }
    pub const fn new() -> Self {
        IPathBytes {
            ipath_bytes: Vec::new(),
        }
    }
}

impl IPath {
    pub const fn from(b: &[u8]) -> Result<&IPath, PathError> {
        let p = IPath::from_unchecked(b);
        if let Err(e) = p.check_components() {
            return Err(e);
        }
        Ok(p)
    }
    pub fn into_ipathbuf(&self) -> IPathBuf {
        IPathBuf {
            ipath_bytes: self.ipath_bytes.to_vec(),
        }
    }
    pub const EMPTY: &IPath = IPath::from_unchecked(&[]);
    pub const fn ipath_bytes(&self) -> &[u8] {
        &self.ipath_bytes
    }
    pub const fn empty() -> &'static IPath {
        Self::EMPTY
    }
    pub const fn from_unchecked(b: &[u8]) -> &IPath {

        unsafe { &*ptr::from_raw_parts(b.as_ptr().cast(), b.len())}
    }

    pub const fn check_components(&self) -> Result<(), PathError> {
        if self.ipath_bytes.is_empty() {
            return Ok(());
        }
        if self.ipath_bytes.len() < 8 {
            return Err(PathError::MissingIdx);
        }
        if self.ipath_bytes.len() > MAX_IPATH_SIZE {
            return Err(PathError::MaxLen);
        }
        let (mut len, idx, spath) = self.fields();
        let mut iter = match spath.checked_iter() {
            Ok(o) => o,
            Err(e) => return Err(e),
        };
        let mut i = 0;
        let mut ptr = 0;
        while i < 8 {
            match iter.next_sp_c().transpose() {
                Err(e) => return Err(e),
                Ok(Some((segm, next))) => {
                    iter = next;
                    ptr += segm.size();
                }
                Ok(None) => {
                    if len != i {
                        return Err(PathError::EmptyComponentIdx);
                    };
                    len += 1;
                }
            };
            if idx[i as usize + 1] as usize != ptr {
                return Err(PathError::BadOffset);
            }
            i += 1;
        }
        Ok(())
    }

    pub fn spath(&self) -> &SPath {
        self.fields().2
    }
    #[inline(always)]
    #[allow(clippy::as_conversions)]
    pub const fn fields(&self) -> (u8, [u8; 9], &SPath) {
        let b = &self.ipath_bytes;
        if b.is_empty() {
            return (0, [0; 9], SPath::empty());
        }
        let (h, spath) = b.split_at(8);
        let len = if spath.len() > 255 {
            255
        } else {
            spath.len() as u8
        };
        (
            h[0],
            [0, h[1], h[2], h[3], h[4], h[5], h[6], h[7], len],
            SPath::from_unchecked(spath),
        )
    }

    /// component count
    pub const fn path_len(&self) -> &u8 {
        match self.ipath_bytes.first(){
            Some(v) => v,
            None => &0,
        }
    }
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }
    #[allow(clippy::len_without_is_empty)] // wtf?
    #[allow(clippy::as_conversions)]
    pub const fn len(&self) -> usize {
        *self.path_len() as usize
    }
    pub const fn components(&self) -> [&SPath; 8] {
        pre_idx_comp(&self.ipath_bytes)
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u8]> {
        (0..self.len()).map(|i| self.comp(i))
    }
    pub fn comps_bytes(&self) -> [&[u8]; 8] {
        pre_idx_comp(&self.ipath_bytes).map(|b| b.first())
    }
    #[allow(clippy::as_conversions)]
    pub fn range(&self, r: std::ops::Range<usize>) -> &SPath {
        let (_, segm, bytes) = self.fields();
        let bytes = &bytes.spath_bytes()[segm[r.start] as usize..segm[r.end] as usize];
        SPath::from_unchecked(bytes)
    }
    #[allow(clippy::as_conversions)]
    pub fn last(&self) -> &[u8] {
        self.comp((*self.path_len() as usize).saturating_sub(1))
    }
    #[inline(always)]
    pub fn comp(&self, i: usize) -> &[u8] {
        static COMPS: [fn(&IPath) -> &[u8]; 8] = [
            IPath::path0,
            IPath::path1,
            IPath::path2,
            IPath::path3,
            IPath::path4,
            IPath::path5,
            IPath::path6,
            IPath::path7,
        ];
        COMPS[i](self)
    }
    pub const fn path0(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[0].first()
    }
    pub const fn path1(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[1].first()
    }
    pub const fn path2(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[2].first()
    }
    pub const fn path3(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[3].first()
    }
    pub const fn path4(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[4].first()
    }
    pub const fn path5(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[5].first()
    }
    pub const fn path6(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[6].first()
    }
    pub const fn path7(&self) -> &[u8] {
        pre_idx_comp(&self.ipath_bytes)[7].first()
    }
}

#[inline(always)]
#[allow(clippy::as_conversions)]
const fn pre_idx_comp(b: &[u8]) -> [&SPath; 8] {
    let mut result = [SPath::from_unchecked(b); 8];
    if b.is_empty() {
        return result;
    }
    let (h, rest) = b.split_at(8);
    let p = rest.as_ptr();
    let ptrs = [
        0,
        h[1],
        h[2],
        h[3],
        h[4],
        h[5],
        h[6],
        h[7],
        rest.len() as u8,
    ];
    let mut i = 0;
    while i < 8 {
        let slice = unsafe {
            std::slice::from_raw_parts(
                p.add(ptrs[i] as usize),
                ptrs[i + 1].wrapping_sub(ptrs[i]) as usize,
            )
        };
        result[i] = SPath::from_unchecked(slice);
        i += 1;
    }
    result
}

pub fn ipath_buf(components: &[&[u8]]) -> IPathBuf {
    IPathBuf::try_from_iter(components).unwrap()
}
#[test]
fn ipathrange() {
    let comp = &[b"zero" as &[u8], b"one", b"two", b"three", b"four"];
    let p = ipath_buf(comp);
    assert_eq!(p.range(0..8), &*spath_buf(comp));
    assert_eq!(p.range(0..2), &*spath_buf(&comp[0..2]));
    assert_eq!(p.range(2..4), &*spath_buf(&comp[2..4]));
    assert_eq!(p.range(4..4), &*spath_buf(&comp[4..4]));
    assert_eq!(p.range(2..8), &*spath_buf(&comp[2..5]));
}
#[test]
fn ip_iter() {}
#[test]
fn spath_idx() {
    let spath = ipath_buf(&[b"hello", b"world"]);
    spath.check_components().unwrap();

    let v = spath_buf(&[b"he", b"llo"]);
    let i: IPathBuf = v.clone().try_ipath().unwrap();
    assert_eq!(
        i.ipath_bytes,
        vec![2, 3, 7, 7, 7, 7, 7, 7, 2, 104, 101, 3, 108, 108, 111]
    );
    let (count, ends, sp) = i.fields();
    assert_eq!(&sp.spath_bytes(), &[2, 104, 101, 3, 108, 108, 111]);
    assert_eq!(count, 2);
    assert_eq!(ends, [0, 3, 7, 7, 7, 7, 7, 7, 7]);
    let mut it = sp.checked_iter().unwrap();
    assert_eq!(it.next().unwrap().unwrap(), b"he");
    assert_eq!(it.next().unwrap().unwrap(), b"llo");
    assert_eq!(it.next(), None);
    sp.check_components().unwrap();
    i.check_components().unwrap();
    //    let arrr = [0,1,2,3,4,5,6,7].map(|v|i.comp(v));
}


