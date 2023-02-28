// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use parse_display::{Display, FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Copy, FromStr, Display)]
#[display(style = "lowercase")]
pub enum Order {
    Asc,
    Desc,
}
impl Order {
    pub fn is_asc(self) -> bool {
        self == Order::Asc
    }
    pub fn asc(b: bool) -> Self {
        if b {
            Order::Asc
        } else {
            Order::Desc
        }
    }
    pub fn desc(b: bool) -> Self {
        if !b {
            Order::Asc
        } else {
            Order::Desc
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, FromStr, Display)]
#[display(style = "lowercase")]
pub enum Table {
    Hash,
    Tree,
    Log,
}
#[derive(Debug, Clone, PartialEq, Eq, Copy, FromStr, Display)]
#[display("{table}-{order}")]
pub struct Mode {
    pub table: Table,
    pub order: Order,
}
impl Default for Mode {
    fn default() -> Mode {
        Mode::TREE_DESC
    }
}
impl Mode {
    pub const HASH_ASC: Mode = Mode { table: Table::Hash, order: Order::Asc};
    pub const TREE_DESC: Mode = Mode {
        table: Table::Tree,
        order: Order::Desc,
    };
    #[must_use]
    pub fn ord_asc(mut self, m: impl Into<bool>) -> Self {
        self.order = Order::asc(m.into());
        self
    }
}
