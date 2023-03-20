// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{ str::FromStr, time::Duration};

use crate::pkt::abe::eval::*;
use anyhow::{bail, Context, anyhow};
use linkspace_pkt::{checked_stamp_add, checked_stamp_sub, stamp_add, stamp_sub, Stamp};
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone)]
pub enum StampFmt {
    Delta {
        from: Stamp,
        percision: usize,
        units: usize,
    },
    Date,
    U64,
}
impl Default for StampFmt {
    fn default() -> Self {
        StampFmt::Delta {
            from: linkspace_pkt::now(),
            percision: 9,
            units: 2,
        }
    }
}
impl StampFmt {
    pub fn delta(percision: usize, units: usize) -> StampFmt {
        StampFmt::Delta {
            from: linkspace_pkt::now(),
            percision,
            units,
        }
    }
    pub fn stringify(&self, stamp: Stamp) -> String {
        match *self {
            StampFmt::Delta {
                from,
                percision,
                units,
            } => delta_stamp_p(from, stamp, percision, units),
            StampFmt::Date => todo!(),
            StampFmt::U64 => stamp.get().to_string(),
        }
    }
}

pub const DELTA_FORMATS: [(&str, Duration);9] = [
    (
        "Y",
        Duration::SECOND
            .checked_mul(60 * 60 * 24 * 365 + 6 * 60 * 60)
            .unwrap(),
    ),
    (
        "M",
        Duration::SECOND.checked_mul(60 * 60 * 24 * 30).unwrap(),
    ),
    ("W", Duration::SECOND.checked_mul(60 * 60 * 24 * 7).unwrap()),
    ("D", Duration::SECOND.checked_mul(60 * 60 * 24).unwrap()),
    ("h", Duration::SECOND.checked_mul(60 * 60).unwrap()),
    ("m", Duration::SECOND.checked_mul(60).unwrap()),
    ("s", Duration::SECOND),
    ("l", Duration::MILLISECOND),
    ("u", Duration::MICROSECOND),
];

pub fn char_as_dur(c: u8) -> Option<&'static (&'static str, Duration)> {
    DELTA_FORMATS.iter().find(|v| v.0.as_bytes()[0] == c)
}

pub fn parse_duration_str(st: impl AsRef<[u8]>) -> anyhow::Result<Duration> {
    let mut dur = Duration::ZERO;
    let mut fmts = DELTA_FORMATS.iter();
    for segm in st.as_ref().split_inclusive(|a| a.is_ascii_alphabetic()) {
        let (unit, digits) = segm.split_last().unwrap();
        let count = u32::from_str(std::str::from_utf8(digits).unwrap())?;
        loop {
            let (ch, val) = fmts.next().with_context(|| match char_as_dur(*unit) {
                Some(_) => format!("Out of order '{}'", *unit as char),
                None => format!("Could not find unit {} ", *unit as char),
            })?;
            if ch.as_bytes()[0] == *unit {
                dur += *val * count;
                break;
            }
        }
    }
    Ok(dur)
}

#[test]
fn str2dur2str() {
    fn k(st: &str) {
        let dur = parse_duration_str(st).unwrap();
        let st2 = duration_as_stamp_dfmt(dur);
        assert_eq!(st, st2)
    }
    k("12Y3D1l");
    k("1D")
}

pub fn delta_stamp(now: Stamp, other: Stamp) -> String {
    delta_stamp_p(now, other, 9, 2)
}
pub fn delta_stamp_p(now: Stamp, other: Stamp, percision: usize, units: usize) -> String {
    let (micros, sign) = if other.get() > now.get() {
        (other.get() - now.get(), "+")
    } else {
        (now.get() - other.get(), "-")
    };
    format!(
        "{}{}",
        sign,
        duration_to_string(Duration::from_micros(micros), percision, units, true, " ")
    )
}
pub fn duration_to_string(
    mut delta: Duration,
    percision: usize,
    units: usize,
    zeros: bool,
    delim: &str,
) -> String {
    let formats = &DELTA_FORMATS[..percision];
    let mut st = String::new();
    let parts = formats
        .iter()
        .filter(|(_, v)| v <= &delta)
        .take(units)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return format!("0{}", formats.last().unwrap().0);
    }
    for (unit, dur) in &parts {
        let c = delta.div_duration_f64(*dur).floor() as u32;
        delta = delta.saturating_sub(*dur * c);
        if zeros || c != 0 {
            st.push_str(&format!("{delim}{}{}", c as u64, unit));
        }
    }
    st
}
pub fn duration_as_stamp_dfmt(duration: Duration) -> String {
    duration_to_string(duration, DELTA_FORMATS.len(), 10, false, "")
}

pub fn stamp_from_digits(s: impl AsRef<[u8]>) -> anyhow::Result<Stamp> {
    match s.as_ref() {
        b"++" => Ok(Stamp::MAX),
        b"0" | b"--" => Ok(Stamp::ZERO),
        v if v.iter().all(|v| v.is_ascii_digit()) => {
            Ok(Stamp::new(::std::str::from_utf8(v)?.parse::<u64>()?))
        }
        _ => bail!("expected digits or '++' '0' "),
    }
}
pub fn stamp_from_now(s: impl AsRef<[u8]>, now: Stamp) -> anyhow::Result<Stamp> {
    let s = s.as_ref();
    if s == b"now" {
        return Ok(now);
    };
    if let Some(plus) = s.strip_prefix(b"+") {
        Ok(stamp_add(now, parse_duration_str(plus)?))
    } else if let Some(sub) = s.strip_prefix(b"-") {
        Ok(stamp_sub(now, parse_duration_str(sub)?))
    } else {
        bail!("could not parse {:?}", std::str::from_utf8(s));
    }
}
/// Using now= None means relative stamp decoding is disabled
pub fn stamp_from_str(s: impl AsRef<[u8]>, now: Option<Stamp>) -> anyhow::Result<Stamp> {
    if let Ok(s) = stamp_from_digits(s.as_ref()) {
        return Ok(s);
    }
    let now = now.context("'Now' not set in this context")?;
    stamp_from_now(s, now)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DurationStr(pub Duration);
impl DurationStr {
    pub fn stamp(self) -> Stamp {
        Stamp::new(self.0.as_micros() as u64)
    }
}
#[allow(non_upper_case_globals)]
impl DurationStr {
    pub const Y: Duration = Duration::SECOND
        .checked_mul(60 * 60 * 24 * 365 + 6 * 60 * 60)
        .unwrap();
    pub const M: Duration = Duration::SECOND.checked_mul(60 * 60 * 24 * 30).unwrap();
    pub const W: Duration = Duration::SECOND.checked_mul(60 * 60 * 24 * 7).unwrap();
    pub const D: Duration = Duration::SECOND.checked_mul(60 * 60 * 24).unwrap();
    pub const h: Duration = Duration::SECOND.checked_mul(60 * 60).unwrap();
    pub const m: Duration = Duration::SECOND.checked_mul(60).unwrap();
    pub const s: Duration = Duration::SECOND;
    pub const l: Duration = Duration::MILLISECOND;
    pub const u: Duration = Duration::MICROSECOND;
    pub const UNITS: [(&'static str, Duration); 9] = [
        ("Y", Self::Y),
        ("M", Self::M),
        ("W", Self::W),
        ("D", Self::D),
        ("h", Self::h),
        ("m", Self::m),
        ("s", Self::s),
        ("l", Self::l),
        ("u", Self::u),
    ];
}
impl FromStr for DurationStr {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_duration_str(s).map(DurationStr)
    }
}

#[derive(Copy, Clone, Debug,Default)]
pub struct StampEF{pub fixed_now: Option<Stamp>}
impl StampEF {
    pub fn now(&self) -> Stamp { self.fixed_now.unwrap_or_else(linkspace_pkt::now)}
    pub fn apply(&self,mut stamp: Stamp, mut args: &[&[u8]]) -> Result<Vec<u8>, ApplyErr> {
        let mut result = None;
        while result.is_none() && !args.is_empty() {
            (stamp, args) = match args {
                [[], args @ ..] => (stamp, args),
                [[b'+',b'+', r @ ..], args @ ..] => (
                    checked_stamp_add(stamp, parse_duration_str(r)?).unwrap_or(Stamp::MAX),
                    args,
                ),
                [[b'+', r @ ..], args @ ..] => (
                    checked_stamp_add(stamp, parse_duration_str(r)?).context("stamp overflow. use '++' for saturating add")?,
                    args,
                ),
                [[b'-',b'-', r @ ..], args @ ..] => (
                    checked_stamp_sub(stamp, parse_duration_str(r)?).unwrap_or(Stamp::ZERO),
                    args,
                ),
                [[b'-', r @ ..], args @ ..] => (
                    checked_stamp_sub(stamp, parse_duration_str(r)?).context("stamp underflow. use '--' for saturating sub")?,
                    args,
                ),
                [b"ticks", rest @ ..] => {
                    args = rest;
                    result = Some(delta_stamp(Stamp::ZERO,stamp).into_bytes());
                    break;
                },
                [b"val", rest @ ..] => {
                    args = rest;
                    result = Some(delta_stamp_p(Stamp::ZERO,stamp,9,9).into_bytes());
                    break;
                },
                [b"delta", rest @ ..] => {
                    args = rest;
                    result = Some(delta_stamp(self.now(), stamp).into_bytes());
                    break;
                }
                [b"str", rest @ ..] => {
                    args = rest;
                    let dt =
                        time::OffsetDateTime::from_unix_timestamp_nanos(stamp.get() as i128 * 1000);
                    let default = time::format_description::well_known::Rfc2822;
                    let st: String = dt
                        .ok()
                        .and_then(|dt| dt.format(&default).ok())
                        .unwrap_or_else(|| delta_stamp_p(Stamp::ZERO,stamp,9,9));
                    result = Some(st.into_bytes());
                    break;
                }
                e => return Err(anyhow!("unknown args '{}'", clist(e)).into()),
            };
        }
        if !args.is_empty() {
            return Err(anyhow!("bad trailing args '{}'", clist(args)).into());
        }
        Ok(result.unwrap_or_else(|| stamp.0.to_vec()))
    }
}

impl EvalScopeImpl for StampEF {
    fn about(&self) -> (String, String) {
        (
            "stamp".into(),
            r#"utilities for stamp values (big endian u64 microsecond since unix epoch)
arguments consists of ( [+-][YMWDhmslu]usize : )* (str | delta | ticks | val)?
"#
            .into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        crate::eval::fncs!([
            (@C "s",0..=16,None,"if chained, mutate 8 bytes input as stamp (see scope help). if used as head assume stamp 0",
             |s:&Self,i:&[&[u8]],init,_| if init {s.apply(Stamp::ZERO,i) } else {s.apply(Stamp::try_from(i[0])?,&i[1..])},none),
            ("now",0..=16,Some(true),"current systemtime",|s:&Self,i:&[&[u8]]| s.apply(s.now(),i)),
            ("epoch",0..=16,Some(true),"unix epoch / zero time",|s:&Self,i:&[&[u8]]| s.apply(Stamp::ZERO,i)),
            ("s++",0..=16,Some(true),"max stamp",|s:&Self,i:&[&[u8]]| s.apply(Stamp::MAX,i))
        ])
    }
}
