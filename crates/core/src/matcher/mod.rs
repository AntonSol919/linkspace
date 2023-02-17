// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]
use std::ops::ControlFlow;

use serde::Serialize;

pub mod matcher2;
pub type Cf<B = ()> = ControlFlow<B>;

pub fn remove_first<T, F>(vec: &mut Vec<T>, f: F) -> Option<T>
where
    F: FnMut(&T) -> bool,
{
    vec.iter().position(f).map(|v| vec.remove(v))
}
#[derive(Clone, Debug)]
/// Wrapper to impl Serialize
pub struct DSpan(pub tracing::Span);
impl Serialize for DSpan {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        format!("{:?}", self.0).serialize(serializer)
    }
}
