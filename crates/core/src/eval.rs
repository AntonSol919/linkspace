// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::consts::PRIVATE;
use crate::consts::PUBLIC;
use crate::consts::TEST_GROUP_ID;
pub use crate::stamp_fmt::StampEF;
pub use linkspace_pkt::abe::eval::*;
pub use linkspace_pkt::abe::*;
pub use linkspace_pkt::abe::*;
use linkspace_pkt::core_scope;
pub use linkspace_pkt::B64EvalFnc;
use linkspace_pkt::EvalCore;
use linkspace_pkt::GroupID;
use linkspace_pkt::SPathFncs;
use linkspace_pkt::B64;

pub const EVAL0_1: &str = "0.1";

pub fn std_ctx() -> EvalCtx<EvalStd> {
    std_ctx_v(EVAL0_1)
}
pub type EvalStd = (
    ((EvalCore, EScope<StaticLNS>), EScope<StampEF>),
    EScope<SPathFncs>,
);
pub const fn std_ctx_v(_version: &str) -> EvalCtx<EvalStd> {
    EvalCtx {
        scope: (
            ((core_scope(), EScope(StaticLNS)), EScope(StampEF{fixed_now:None})),
            EScope(SPathFncs),
        ),
    }
}

#[derive(Copy, Clone, Debug)]
pub struct StaticLNS;
impl EvalScopeImpl for StaticLNS {
    fn about(&self) -> (String, String) {
        (
            "static-lns".into(),
            "static lns for local only [#:0] and public [#:pub]".into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[
            ScopeFunc {
                apply: |_, i, _, _| match i[0] {
                    b"0" => ApplyResult::Ok(PRIVATE.0.to_vec()),
                    b"pub" => ApplyResult::Ok(PUBLIC.0.to_vec()),
                    b"test" => ApplyResult::Ok(TEST_GROUP_ID.0.to_vec()),
                    _ => ApplyResult::None,
                },
                info: ScopeFuncInfo {
                    id: "#",
                    init_eq: Some(true),
                    argc: 1..=16,
                    help: "resolve #:0 , #:pub, and #:test without a db",
                    to_abe: true,
                },
                to_abe: |_, i, _| {
                    let g = GroupID::try_fit_slice(i).ok()?;
                    let b = if g == PRIVATE {
                        "[#:0]"
                    } else if g == PUBLIC {
                        "[#:pub]"
                    } else if g == *TEST_GROUP_ID {
                        "[#:test]"
                    } else {
                        return ApplyResult::None;
                    };
                    ApplyResult::Ok(b.as_bytes().to_vec())
                },
            },
            ScopeFunc {
                apply: |_, i, _, _| match i[0] {
                    b"none" => ApplyResult::Ok(PRIVATE.0.to_vec()),
                    _ => ApplyResult::None,
                },
                info: ScopeFuncInfo {
                    id: "@",
                    init_eq: Some(true),
                    argc: 1..=16,
                    help: "resolve @:none",
                    to_abe: true,
                },
                to_abe: |_, i, _| {
                    if *i == [0; 32] {
                        ApplyResult::Ok(b"[@:none]".to_vec())
                    } else {
                        ApplyResult::None
                    }
                },
            },
            
        ]
    }
}
fn _rev_lookup(i: &[&[u8]], group_mode: Option<bool>) -> ApplyResult {
    let b = B64::try_fit_slice(i[0])?;
    match b {
        b if b == PUBLIC => Ok(b"[#:pub]".to_vec()),
        b if b == *TEST_GROUP_ID => Ok(b"[#:test]".to_vec()),
        b if b == PRIVATE => match group_mode {
            Some(false) => Ok(b"[@:none]".to_vec()),
            _ => Ok(b"[#:0]".to_vec()),
        },
        _ => return ApplyResult::None,
    }
    .into()
}
