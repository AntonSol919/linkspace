use crate::NetPkt;
use serde::{Deserialize, Serialize};
use std::hash::Hash;

use super::tree_order::*;

/**
A newtype around any T:NetPkt that implements Eq (and Hash) based on pkt hash and Ord on the standard tree index order.
*/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PktCmp<T>(pub T);

impl<T: NetPkt> Hash for PktCmp<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash_ref().hash(state)
    }
}
impl<T: NetPkt> PartialEq for PktCmp<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.hash_ref() == other.0.hash_ref()
    }
}
impl<T: NetPkt> Eq for PktCmp<T> {}
impl<T: NetPkt> Ord for PktCmp<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // FIXME: Can be done faster

        let this = TreeEntry::from_pkt(0.into(), &self.0).ok_or(self.0.hash_ref());
        let other = TreeEntry::from_pkt(0.into(), &other.0).ok_or(other.0.hash_ref());
        this.partial_cmp(&other).unwrap()
    }
}
impl<T: NetPkt> PartialOrd for PktCmp<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
