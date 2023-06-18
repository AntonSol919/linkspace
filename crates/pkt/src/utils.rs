// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.


pub(crate) fn none<X, Y>(_f: Y) -> Option<X> {
    None
}
use crate::LkHash;

/// TODO: In theory we can optimize away a hash step and ddos protection
pub type LkHashMap<V> = ::std::collections::HashMap<LkHash, V>;
pub type LkHashSet = ::std::collections::HashSet<LkHash>;


