// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(maybe_uninit_slice, file_set_times)]

#[cfg(feature = "inotify")]
pub use inotify::ProcBus;
#[cfg(feature = "inotify")]
pub mod inotify;

#[cfg(not(target_arch = "wasm32"))]
mod udp_multicast;
#[cfg(not(target_arch = "wasm32"))]
pub mod udp_procbus;
#[cfg(not(target_arch = "wasm32"))]
pub use udp_procbus::*;

#[cfg(target_arch = "wasm32")]
pub mod wasmbus;
#[cfg(target_arch = "wasm32")]
pub use wasmbus::*;
