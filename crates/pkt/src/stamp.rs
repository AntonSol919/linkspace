// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::time::{Duration, Instant, UNIX_EPOCH};

/// current time as big endian u64 microseconds since epoch
#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> Stamp {
    from_systime(std::time::SystemTime::now())
}
pub fn from_systime(time: std::time::SystemTime) -> Stamp {
    let v = time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_micros();
    Stamp::new(unsafe { v.try_into().unwrap_unchecked() })
}
pub fn as_systime(stamp: Stamp) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + Duration::from_micros(stamp.get())
}
pub fn as_instance(stamp: Stamp) -> Instant {
    // this seems dumb required 'now' twice.
    let now = now().get();
    let to = stamp.get();
    if now > to {
        Instant::now() - Duration::from_micros(now - to)
    } else {
        Instant::now() + Duration::from_micros(to - now)
    }
}
pub fn as_duration(stamp: Stamp) -> Duration {
    Duration::from_micros(stamp.get())
}

pub fn stamp_sub_stamp(a: Stamp, b: Stamp) -> Result<Stamp, Stamp> {
    if a.get() > b.get() {
        Ok((a.get() - b.get()).into())
    } else {
        Err((b.get() - a.get()).into())
    }
}
/// Returns error if the stamp is older then now.
pub fn stamp_age(stamp: Stamp) -> Result<Duration, Duration> {
    let now = now().get();
    let stamp = stamp.get();
    if now < stamp {
        return Err(Duration::from_micros(stamp - now));
    }
    Ok(Duration::from_micros(now - stamp))
}

/// saturating add
pub fn stamp_add(stamp: Stamp, dur: Duration) -> Stamp {
    let plus = dur.as_micros().min(u64::MAX.into());
    let plus = unsafe { u64::try_from(plus).unwrap_unchecked() };
    Stamp::new(stamp.get().saturating_add(plus))
}
pub fn checked_stamp_add(stamp: Stamp, dur: Duration) -> Option<Stamp> {
    let plus = dur.as_micros().min(u64::MAX.into());
    let plus = unsafe { u64::try_from(plus).unwrap_unchecked() };
    stamp.get().checked_add(plus).map(Stamp::new)
}
/// saturating sub
pub fn stamp_sub(stamp: Stamp, dur: Duration) -> Stamp {
    let sub = dur.as_micros().min(u64::MAX.into());
    let sub = unsafe { u64::try_from(sub).unwrap_unchecked() };
    Stamp::new(stamp.get().saturating_sub(sub))
}
pub fn checked_stamp_sub(stamp: Stamp, dur: Duration) -> Option<Stamp> {
    let sub = dur.as_micros().min(u64::MAX.into());
    let sub = unsafe { u64::try_from(sub).unwrap_unchecked() };
    stamp.get().checked_sub(sub).map(Stamp::new)
}
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use crate::Stamp;
#[wasm_bindgen(inline_js = r#"
export function date_now() {
  return Date.now();
}"#)]
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn date_now() -> f64;
}

#[cfg(target_arch = "wasm32")]
pub fn now() -> Stamp {
    Stamp::new((date_now() * 1000.0) as u64)
}
