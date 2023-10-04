/*
Copyright Anton Sol

This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
*/
use crate::{space::*, MAX_ROOTED_SPACENAME_SIZE, MAX_SPACENAME_COMPONENT_SIZE, MAX_SPACE_DEPTH};
use std::{borrow::Borrow, ops::Deref, ptr};

/// An RootedSpace is an [[Space]] with 8 bytes prefix: [depth(i.e. #components), \[component_offset;7\]]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RootedSpaceBytes<X: ?Sized> {
    rooted_bytes: X,
}
pub type RootedSpace = RootedSpaceBytes<[u8]>;
pub type RootedSpaceBuf = RootedSpaceBytes<Vec<u8>>;
pub type RootedStaticSpace<const U: usize> = RootedSpaceBytes<[u8; U]>;

impl ToOwned for RootedSpace {
    type Owned = RootedSpaceBuf;
    fn to_owned(&self) -> Self::Owned {
        self.into_buf()
    }
}
impl Borrow<RootedSpace> for RootedSpaceBuf {
    fn borrow(&self) -> &RootedSpace {
        self.deref()
    }
}
impl<const U: usize> AsRef<RootedSpace> for RootedStaticSpace<U> {
    fn as_ref(&self) -> &RootedSpace {
        RootedSpace::from_unchecked(&self.rooted_bytes)
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
pub const fn rspace1<const C0: usize>(c0: &[u8; C0]) -> RootedStaticSpace<{ C0 + 9 }> {
    assert!(C0 < MAX_SPACENAME_COMPONENT_SIZE);
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
    match RootedSpaceBytes::<[u8; C0 + 9]>::from(r) {
        Ok(v) => v,
        Err(_) => panic!("invalid space len"),
    }
}
impl<const N: usize> RootedSpaceBytes<[u8; N]> {
    pub const fn from(b: [u8; N]) -> Result<Self, SpaceError> {
        let p = RootedSpace::from_unchecked(&b);
        if let Err(e) = p.check_components() {
            return Err(e);
        }
        Ok(RootedSpaceBytes { rooted_bytes: b })
    }
}

impl Deref for RootedSpaceBuf {
    type Target = RootedSpace;
    fn deref(&self) -> &Self::Target {
        RootedSpace::from_unchecked(self.rooted_bytes.as_slice())
    }
}
impl<const C: usize> Deref for RootedStaticSpace<C> {
    type Target = RootedSpace;
    fn deref(&self) -> &Self::Target {
        RootedSpace::from_unchecked(&self.rooted_bytes)
    }
}
impl Deref for RootedSpace {
    type Target = Space;
    fn deref(&self) -> &Self::Target {
        self.space()
    }
}
impl TryInto<RootedSpaceBuf> for SpaceBuf {
    type Error = SpaceError;
    fn try_into(self) -> Result<RootedSpaceBuf, Self::Error> {
        self.try_into_rooted()
    }
}
impl Default for &RootedSpace {
    fn default() -> Self {
        RootedSpace::EMPTY
    }
}
impl Default for RootedSpaceBuf {
    fn default() -> Self {
        Self::new()
    }
}
impl RootedSpaceBuf {
    pub const DEFAULT: Self = Self::new();
    pub const fn from_unchecked(rooted_bytes: Vec<u8>) -> Self {
        RootedSpaceBuf { rooted_bytes }
    }
    #[track_caller]
    pub fn append(mut self, component: &[u8]) -> Self {
        self.try_append_component(component).unwrap();
        self
    }

    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    pub fn try_append_component(&mut self, component: &[u8]) -> Result<&mut Self, SpaceError> {
        let bs = &mut self.rooted_bytes;
        if component.len() > MAX_SPACENAME_COMPONENT_SIZE {
            return Err(SpaceError::ComponentSize);
        }
        if component.is_empty() {
            return Err(SpaceError::ZeroComponent);
        }
        if bs.is_empty() {
            *bs = vec![0; 8]
        }
        if bs[0] >= MAX_SPACE_DEPTH as u8 {
            return Err(SpaceError::CapacityError);
        }
        if bs.len() + component.len() + 1 > MAX_ROOTED_SPACENAME_SIZE {
            return Err(SpaceError::MaxDepth);
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
    pub fn join(self, i: &Space) -> RootedSpaceBuf {
        self.try_join(i).unwrap()
    }
    pub fn try_join(mut self, i: &Space) -> Result<RootedSpaceBuf, SpaceError> {
        self.extend_from_iter(i.iter())?;
        Ok(self)
    }
    pub fn extend_from_iter(
        &mut self,
        i: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<(), SpaceError> {
        for s in i.into_iter() {
            self.try_append_component(s.as_ref())?;
        }
        Ok(())
    }
    //FIXME : this is ineficient
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = impl AsRef<[u8]>>,
    ) -> Result<RootedSpaceBuf, SpaceError> {
        let mut new = RootedSpaceBuf::new();
        new.extend_from_iter(iter)?;
        Ok(new)
    }
    pub const fn new() -> Self {
        RootedSpaceBytes {
            rooted_bytes: Vec::new(),
        }
    }
}

impl RootedSpace {
    pub const fn from(b: &[u8]) -> Result<&RootedSpace, SpaceError> {
        let p = RootedSpace::from_unchecked(b);
        if let Err(e) = p.check_components() {
            return Err(e);
        }
        Ok(p)
    }
    pub fn into_buf(&self) -> RootedSpaceBuf {
        RootedSpaceBuf {
            rooted_bytes: self.rooted_bytes.to_vec(),
        }
    }
    pub const EMPTY: &'static RootedSpace = RootedSpace::from_unchecked(&[]);
    pub const fn rooted_bytes(&self) -> &[u8] {
        &self.rooted_bytes
    }
    pub const fn empty() -> &'static RootedSpace {
        Self::EMPTY
    }
    pub const fn from_unchecked(b: &[u8]) -> &RootedSpace {
        unsafe { &*ptr::from_raw_parts(b.as_ptr().cast(), b.len()) }
    }

    pub const fn check_components(&self) -> Result<(), SpaceError> {
        if self.rooted_bytes.is_empty() {
            return Ok(());
        }
        if self.rooted_bytes.len() < 8 {
            return Err(SpaceError::MissingIdx);
        }
        if self.rooted_bytes.len() > MAX_ROOTED_SPACENAME_SIZE {
            return Err(SpaceError::MaxDepth);
        }
        let (mut len, idx, space) = self.fields();
        let mut iter = match space.checked_iter() {
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
                        return Err(SpaceError::EmptyComponentIdx);
                    };
                    len += 1;
                }
            };
            if idx[i as usize + 1] as usize != ptr {
                return Err(SpaceError::BadOffset);
            }
            i += 1;
        }
        Ok(())
    }

    pub fn space(&self) -> &Space {
        self.fields().2
    }
    #[inline(always)]
    #[allow(clippy::as_conversions)]
    pub const fn fields(&self) -> (u8, [u8; 9], &Space) {
        let b = &self.rooted_bytes;
        if b.is_empty() {
            return (0, [0; 9], Space::empty());
        }
        let (h, space) = b.split_at(8);
        let len = if space.len() > 255 {
            255
        } else {
            space.len() as u8
        };
        (
            h[0],
            [0, h[1], h[2], h[3], h[4], h[5], h[6], h[7], len],
            Space::from_unchecked(space),
        )
    }

    /// component count
    pub const fn space_depth(&self) -> &u8 {
        match self.rooted_bytes.first() {
            Some(v) => v,
            None => &0,
        }
    }
    pub const fn is_empty(&self) -> bool {
        self.depth() == 0
    }
    #[allow(clippy::as_conversions)]
    pub const fn depth(&self) -> usize {
        *self.space_depth() as usize
    }
    pub const fn components(&self) -> [&Space; 8] {
        pre_idx_comp(&self.rooted_bytes)
    }

    pub fn iter(&self) -> impl Iterator<Item = &[u8]> {
        (0..self.depth()).map(|i| self.comp(i))
    }
    pub fn comps_bytes(&self) -> [&[u8]; 8] {
        pre_idx_comp(&self.rooted_bytes).map(|b| b.first())
    }
    #[allow(clippy::as_conversions)]
    pub fn range(&self, r: std::ops::Range<usize>) -> &Space {
        let (_, segm, bytes) = self.fields();
        let bytes = &bytes.space_bytes()[segm[r.start] as usize..segm[r.end] as usize];
        Space::from_unchecked(bytes)
    }
    #[allow(clippy::as_conversions)]
    pub fn last(&self) -> &[u8] {
        self.comp((*self.space_depth() as usize).saturating_sub(1))
    }
    #[inline(always)]
    pub fn comp(&self, i: usize) -> &[u8] {
        static COMPS: [fn(&RootedSpace) -> &[u8]; 8] = [
            RootedSpace::comp0,
            RootedSpace::comp1,
            RootedSpace::comp2,
            RootedSpace::comp3,
            RootedSpace::comp4,
            RootedSpace::comp5,
            RootedSpace::comp6,
            RootedSpace::comp7,
        ];
        COMPS[i](self)
    }
    pub const fn comp0(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[0].first()
    }
    pub const fn comp1(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[1].first()
    }
    pub const fn comp2(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[2].first()
    }
    pub const fn comp3(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[3].first()
    }
    pub const fn comp4(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[4].first()
    }
    pub const fn comp5(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[5].first()
    }
    pub const fn comp6(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[6].first()
    }
    pub const fn comp7(&self) -> &[u8] {
        pre_idx_comp(&self.rooted_bytes)[7].first()
    }
}

#[inline(always)]
#[allow(clippy::as_conversions)]
const fn pre_idx_comp(b: &[u8]) -> [&Space; 8] {
    let mut result = [Space::from_unchecked(b); 8];
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
        result[i] = Space::from_unchecked(slice);
        i += 1;
    }
    result
}

pub fn rspace_buf(components: &[&[u8]]) -> RootedSpaceBuf {
    RootedSpaceBuf::try_from_iter(components).unwrap()
}
#[test]
fn rspace_range() {
    let comp = &[b"zero" as &[u8], b"one", b"two", b"three", b"four"];
    let p = rspace_buf(comp);
    assert_eq!(p.range(0..8), &*space_buf(comp));
    assert_eq!(p.range(0..2), &*space_buf(&comp[0..2]));
    assert_eq!(p.range(2..4), &*space_buf(&comp[2..4]));
    assert_eq!(p.range(4..4), &*space_buf(&comp[4..4]));
    assert_eq!(p.range(2..8), &*space_buf(&comp[2..5]));
}
#[test]
fn space_roots() {
    let space = rspace_buf(&[b"hello", b"world"]);
    space.check_components().unwrap();

    let v = space_buf(&[b"he", b"llo"]);
    let i: RootedSpaceBuf = v.clone().try_into_rooted().unwrap();
    assert_eq!(
        i.rooted_bytes,
        vec![2, 3, 7, 7, 7, 7, 7, 7, 2, 104, 101, 3, 108, 108, 111]
    );
    let (count, ends, sp) = i.fields();
    assert_eq!(&sp.space_bytes(), &[2, 104, 101, 3, 108, 108, 111]);
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
