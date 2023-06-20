// Copyright Anton Sol
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
Utility to create packets from multiple byte slices without allocating.
**/
#[derive(Clone,Debug)]
pub struct ByteSegments<'a>(pub(crate) [&'a [u8]; 8]);

impl<'a> ExactSizeIterator for ByteSegments<'a> {
    #[inline(always)]
    fn len(&self) -> usize {
        // Wrapping ok because Self is only created from valid packets. 
        self.0.iter().fold(0, |a, b| a.wrapping_add( b.len()))
    }
    #[inline]
    fn is_empty(&self) -> bool {
        self.0 == [&[] as &[u8];8]
    }
}

impl<'a> Iterator for ByteSegments<'a> {
    type Item = u8;
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.len();
        (len, Some(len))
    }
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some((h, tail)) = self.0[0].split_first() {
            self.0[0] = tail;
            return Some(*h);
        }
        // TODO : might be better to set self.0[0] = tail. Depends on if compiler sees through
        if let Some((h, tail)) = self.0[1].split_first() {
            self.0[1] = tail;
            return Some(*h);
        }
        if let Some((h, tail)) = self.0[2].split_first() {
            self.0[2] = tail;
            return Some(*h);
        }
        if let Some((h, tail)) = self.0[3].split_first() {
            self.0[3] = tail;
            return Some(*h);
        }
        if let Some((h, tail)) = self.0[4].split_first() {
            self.0[4] = tail;
            return Some(*h);
        }
        if let Some((h, tail)) = self.0[5].split_first() {
            self.0[5] = tail;
            return Some(*h);
        }
        if let Some((h, tail)) = self.0[6].split_first() {
            self.0[6] = tail;
            return Some(*h);
        }
        if let Some((h, tail)) = self.0[7].split_first() {
            self.0[7] = tail;
            return Some(*h);
        }
        None
    }
}
#[test]
pub fn it() {
    let byes = ByteSegments::from_array([b"hello", b"world"]);
    let len = byes.len();
    let b: Vec<u8> = byes.collect();
    assert_eq!(b.len(), len);
    assert_eq!(b, b"helloworld")
}

impl<'a> ByteSegments<'a> {
    #[inline(always)]
    pub const fn from_array<const N: usize>(segments: [&'a [u8]; N]) -> Self {
        assert!(N < 8,"not supported");
        let mut r : [&'a [u8];8]= [&[]; 8];
        let mut i = 0;
        while i < N {
            r[i] = segments[i];
            i = i.wrapping_add(1);
        }
        ByteSegments(r)
    }
    #[inline]
    pub const fn as_slice(&self) -> &[&'a [u8]] {
        &self.0
    }

    #[must_use]
    #[inline(always)]
    pub(crate) fn push_front(self, head: &'a [u8]) -> Self {
        let [a, b, c, d, e, f, g, h] = self.0;
        debug_assert!(h.is_empty(), "segmented packet construction stacked to deep");
        ByteSegments([head, a, b, c, d, e, f, g])
    }
    #[inline]
    pub fn to_bytes(self) -> Box<[u8]> {
        self.0.concat().into_boxed_slice()
    }

    /// # Safety
    ///
    /// dest needs to be initialized to fit the entire length;
    #[inline(always)]
    pub const unsafe fn write_segments_unchecked(self, mut dest: *mut u8)  -> *mut u8{
        let mut i = 0;
        while i < 8 {
            let len = self.0[i].len();
            core::ptr::copy_nonoverlapping(self.0[i].as_ptr(), dest, len);
            dest = dest.add(len);
            i = i.wrapping_add(1);
        }
        dest
    }
    #[inline(always)]
    pub fn io_slices(self) -> [std::io::IoSlice<'a>; 8] {
        self.0.map(std::io::IoSlice::new)
    }
    #[inline(always)]
    pub fn write_into(self, mut dest: impl std::io::Write) -> std::io::Result<()> {
        dest.write_all_vectored(&mut self.io_slices())
    }
}
