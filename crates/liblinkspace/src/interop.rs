// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#[doc(hidden)]
impl Into<linkspace_common::core::query::Query> for crate::Query {
    fn into(self) -> linkspace_common::core::query::Query {
        self.0
    }
}
#[doc(hidden)]
impl From<linkspace_common::core::query::Query> for crate::Query {
    fn from(value: linkspace_common::core::query::Query) -> Self {
        crate::Query(value)
    }
}
#[doc(hidden)]
impl From<linkspace_common::runtime::Linkspace> for crate::Linkspace {
    fn from(value: linkspace_common::runtime::Linkspace) -> Self {
        crate::Linkspace(value)
    }
}
#[doc(hidden)]
impl Into<linkspace_common::runtime::Linkspace> for crate::Linkspace {
    fn into(self) -> linkspace_common::runtime::Linkspace {
        self.0
    }
}
