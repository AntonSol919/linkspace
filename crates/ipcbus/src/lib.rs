// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(maybe_uninit_slice)]

#[cfg(not(target_arch = "wasm32"))]
pub mod procbus;
#[cfg(not(target_arch = "wasm32"))]
mod udp_multicast;
#[cfg(not(target_arch = "wasm32"))]
pub use procbus::*;

#[cfg(target_arch = "wasm32")]
pub mod wasmbus;
#[cfg(target_arch = "wasm32")]
pub use wasmbus::*;
