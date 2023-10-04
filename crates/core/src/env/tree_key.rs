use linkspace_pkt::FieldEnum;

// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::prelude::RuleType;

/// check if this type can be answered by a treekey
pub const fn treekey_checked(r: RuleType) -> bool {
    match r {
        #[allow(clippy::match_like_matches_macro)]
        RuleType::Field(f) => match f {
            FieldEnum::PktHashF => true,
            FieldEnum::PubKeyF => true,
            FieldEnum::GroupIDF => true,
            FieldEnum::DomainF => true,
            FieldEnum::CreateF => true,
            FieldEnum::DepthF => true,
            FieldEnum::LinksLenF => true,
            FieldEnum::DataSizeF => true,
            _ => false,
        },
        RuleType::RecvStamp => true,
        RuleType::SpacePrefix => true,
        RuleType::Limit(_) => false,
    }
}
