// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
use core::mem::size_of;
use serde::{Deserialize, Serialize};

/**
'tree' index order is a Ord of NetPkts defined by the order of the tuple
( group, domain, space_depth, spacename, pubkey, create stamp, hash )

*/

/// A wrapper around TreeKey ( see [[TreeKey::from_fields]] ) and [[TreeValue]]
/// This is the key/value by which packets are saved in the tree index.
#[derive(Clone, Copy, Serialize, Deserialize, PartialOrd, PartialEq)]
pub struct TreeEntry<K = Vec<u8>, V = TreeValueBytes> {
    pub btree_key: TreeKey<K>,
    pub val: V,
}

impl TreeKey {
    pub fn from_fields(
        group: GroupID,
        domain: Domain,
        sp_segm: u8,
        sp: &Space,
        key: Option<&PubKey>,
    ) -> TreeKey<Vec<u8>> {
        let mut btree_key: Vec<u8> = vec![];
        btree_key.extend_from_slice(&group.0);
        btree_key.extend_from_slice(&domain.0);
        btree_key.push(sp_segm);
        btree_key.extend_from_slice(sp.space_bytes());
        btree_key.extend_from_slice(&key.unwrap_or(&B64([0; 32])).0);
        TreeKey(btree_key)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(packed)]
pub struct TreeValue {
    create: Stamp,
    hash: LkHash,
    logptr: Stamp,
    links_len: U16,
    data_size: U16,
}
impl TreeValue {
    pub fn from_pkt(logptr: Stamp, pkt: impl NetPkt) -> TreeValue {
        TreeValue {
            create: *pkt.get_create_stamp(),
            hash: pkt.hash(),
            logptr,
            links_len: U16::new(pkt.get_links().len() as u16),
            data_size: U16::new(pkt.data().len() as u16),
        }
    }
    pub fn into_bytes(self) -> TreeValueBytes {
        unsafe { *(&self as *const TreeValue as *const TreeValueBytes) }
    }
}
pub type TreeValueBytes = [u8; size_of::<TreeValue>()];
impl TreeEntry {
    pub fn from_pkt(rstamp: Stamp, pkt: impl NetPkt) -> Option<Self> {
        let fields = pkt.as_point().fields();
        let (sp, space, key) = fields.common_idx()?;
        let val = TreeValue::from_pkt(rstamp, &pkt);
        Some(TreeEntry {
            btree_key: TreeKey::from_fields(sp.group, sp.domain, *space.space_depth(), space, key),
            val: val.into_bytes(),
        })
    }
}

/// Borrowed Variant
pub type TreeEntryRef<'a> = TreeEntry<&'a [u8], &'a TreeValueBytes>;
impl<'a> TreeEntryRef<'a> {
    pub fn from_db((btree_key, val): (&'a [u8], &'a TreeValueBytes)) -> TreeEntryRef<'a> {
        TreeEntry {
            btree_key: TreeKey(btree_key),
            val,
        }
    }
    pub fn to_owned(&self) -> TreeEntry {
        TreeEntry {
            btree_key: TreeKey(self.btree_key.0.to_vec()),
            val: *self.val,
        }
    }
}

impl<K: AsRef<[u8]>, V> TreeEntry<K, V> {
    pub fn btree_key(&self) -> TreeKey<&[u8]> {
        self.btree_key.as_ref()
    }
}

impl<K, V: AsRef<[u8]>> TreeEntry<K, V> {
    // A native endian idx into the pkt_log
    // Should never be sent to others
    fn val(&self) -> TreeValue {
        assert!(self.val.as_ref().len() == size_of::<TreeValue>());
        unsafe { std::ptr::read_unaligned(self.val.as_ref().as_ptr() as *const TreeValue) }
    }
    pub fn local_log_ptr(&self) -> Stamp {
        self.val().logptr
    }
    pub fn hash(&self) -> LkHash {
        self.val().hash
    }
    pub fn create(&self) -> Stamp {
        self.val().create
    }
    pub fn data_size(&self) -> U16 {
        self.val().data_size
    }
    pub fn links_len(&self) -> U16 {
        self.val().links_len
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Ord)]
pub struct TreeKey<B = Vec<u8>>(pub(crate) B);
#[repr(packed)]
pub struct KeyFixedHead {
    pub group: GroupID,
    pub domain: Domain,
    pub sp_len: u8,
}
impl TreeKey {
    pub fn from(b: &[u8]) -> Option<Self> {
        const MIN_LEN: usize = size_of::<KeyFixedHead>() + 1;
        if b.len() < MIN_LEN {
            println!("Invalid treekey? {:?} {} <  {}", b, b.len(), MIN_LEN);
            None
        } else {
            Some(TreeKey(b.to_vec()))
        }
    }
}

impl<B: AsRef<[u8]>> TreeKey<B> {
    #[track_caller]
    pub fn new(b: B) -> TreeKey<B> {
        let r = TreeKey(b);
        r.space().check_components().unwrap();
        r
    }
    pub fn pop_key(&self) -> (&[u8], PubKey) {
        let b = self.as_bytes();
        let (pre, key) = self.as_bytes().split_at(b.len() - size_of::<PubKey>());
        (pre, PubKey::try_fit_slice(key).unwrap())
    }
    pub fn space_and_key(&self) -> (&Space, PubKey) {
        let (bytes, key) = self.pop_key();
        let space = Space::from_unchecked(&bytes[size_of::<KeyFixedHead>()..]);
        (space, key)
    }
    pub fn fixed_head(&self) -> KeyFixedHead {
        unsafe { std::ptr::read_unaligned(self.as_bytes().as_ptr() as *const KeyFixedHead) }
    }
    pub fn pubkey(&self) -> PubKey {
        self.pop_key().1
    }
    pub fn space(&self) -> &Space {
        self.space_and_key().0
    }
    pub fn as_ref(&self) -> TreeKey<&[u8]> {
        TreeKey(self.0.as_ref())
    }
    pub fn take(self) -> B {
        self.0
    }
    pub fn fields(&self) -> (GroupID, Domain, u8, &Space, PubKey) {
        (
            self.group(),
            self.domain(),
            self.space_segments(),
            self.space(),
            self.pubkey(),
        )
    }
    pub fn domain(&self) -> Domain {
        self.fixed_head().domain
    }
    pub fn group(&self) -> GroupID {
        self.fixed_head().group
    }
    pub fn space_segments(&self) -> u8 {
        self.fixed_head().sp_len
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }
}
impl<K: AsRef<[u8]>, V: AsRef<[u8]>> std::fmt::Debug for TreeEntry<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (&self.btree_key, self.val()).fmt(f)
    }
}

impl<B: AsRef<[u8]>> std::fmt::Debug for TreeKey<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (g, d, spd, sp, k) = self.fields();
        let space = sp
            .check_components()
            .map(|_| format!("{sp}"))
            .map_err(|_e| format!("Invalid{:?}", sp.space_bytes()));
        f.debug_tuple("Key")
            .field(&g)
            .field(&d)
            .field(&spd)
            .field(&space)
            .field(&k.b64_mini())
            .finish()
    }
}

impl<K: AsRef<[u8]>, V: AsRef<[u8]>> std::fmt::Display for TreeEntry<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (g, d, spd, sp, k) = self.btree_key.fields();
        let key = if k == PRIVATE {
            String::new()
        } else {
            k.to_string()
        };
        let TreeValue {
            create,
            hash,
            logptr,
            links_len,
            data_size,
        } = self.val();
        write!(
            f,
            "{g}:{d}:{spd} {sp} {key} => ({create},{hash},{logptr},{links_len},{data_size})"
        )
    }
}
