// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use byte_fmt::abe::{eval::ABList, ABEValidator, FitSliceErr};

use crate::*;

pub fn print_links(l: &[Link]) -> String {
    l.iter()
        .map(|Link { tag, ptr: pointer }| format!("{tag}\t{pointer}"))
        .collect::<Vec<String>>()
        .join("\n")
}

impl Link {
    pub const DEFAULT : Link = Link { ptr: B64([0;32]),tag:AB([0;16])};
    #[track_caller]
    pub fn new(tag: impl AsRef<[u8]>, ptr: impl Into<LkHash>) -> Link {
        Link {
            ptr: ptr.into(),
            tag: ab(tag.as_ref()),
        }
    }
    pub fn try_from(tag: impl AsRef<[u8]>, pointer: LkHash) -> Result<Link, FitSliceErr> {
        AB::try_fit_byte_slice(tag.as_ref()).map(|tag| Link { tag, ptr: pointer })
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ABELinkErr {
    #[error("To many items")]
    ToManyItems,
    #[error("Missing item. Expected tag:pointer ")]
    MissingTagOrPointer,
    #[error("Tag does not fit {0}")]
    Tag(FitSliceErr),
    #[error("Point erdoes not fit {0}")]
    Pointer(B64Error),
    #[error("Missing colon. Expected tag:pointer ")]
    ExpectedColon,
}

use abe::ast::*;
impl ABEValidator for Link {
    fn check(lst: &[ABE]) -> Result<(), MatchError> {
        let [tag, c, ptr] = exact(lst)?;
        as_expr(tag)?;
        is_colon(c)?;
        as_expr(ptr)?;
        Ok(())
    }
}
impl TryFrom<ABList> for Link {
    type Error = ABELinkErr;
    fn try_from(mut value: ABList) -> Result<Self, Self::Error> {
        use ABELinkErr::*;
        let pointer = value.lst.pop().ok_or(MissingTagOrPointer)?;
        let tag = value.lst.pop().ok_or(MissingTagOrPointer)?;
        if !value.is_empty() {
            return Err(ToManyItems);
        }
        if tag.1 != Some(Ctr::Colon) {
            return Err(ExpectedColon);
        }
        let tag = AB::<[u8; 16]>::try_fit_byte_slice(&tag.0).map_err(Tag)?;
        let pointer = B64::try_fit_bytes_or_b64(&pointer.0).map_err(Pointer)?;
        Ok(Link { tag, ptr: pointer })
    }
}
