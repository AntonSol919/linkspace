// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use byte_fmt::abe::TypedABE;
use crate::*;
pub type StampExpr = TypedABE<Stamp>;
pub type TagExpr = TypedABE<Tag>;

pub type DomainExpr = TypedABE<Domain>;


pub type HashExpr = TypedABE<LkHash>;
pub type PExpr = TypedABE<Ptr>;
pub type GroupExpr = TypedABE<GroupID>;
pub type PubKeyExpr = TypedABE<PubKey>;

pub type LinkExpr = TypedABE<Link>;
