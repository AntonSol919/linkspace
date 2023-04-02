// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    borrow::Cow,
    io::{Cursor, Write},
};

use thiserror::Error;

use crate::FitSliceErr;
pub const MAX_STR: &str = "f";
pub const MAXB_CHAR: u8 = b'f';
pub const MAX_ESCAPED: &str = "\\f";

#[derive(Error, Debug)]
pub enum ABTxtError {
    #[error("{0}")]
    FitSlice(#[from] FitSliceErr),
    #[error("Unable to parse txt as bytes {}({:?}) at {}",byte,*byte as char , idx)]
    ParseError { byte: u8, idx: usize },
    #[error("Invalid Hex seq")]
    InvalidHex,
    #[error("Other {0}")]
    Other(&'static str),
}

#[derive(Debug, Clone, PartialEq, Copy)]
#[repr(u32)]
pub enum CtrChar {
    ForwardSlash = 0b01,
    Colon = 0b10,
    Newline = 0b100,
    Tab = 0b1000,
    OpenBracket = 0b10000,
    CloseBracket = 0b100000,
}
impl CtrChar {
    pub fn try_from_char(b: u8) -> Option<CtrChar> {
        Some(match b {
            b'/' => CtrChar::ForwardSlash,
            b'[' => CtrChar::OpenBracket,
            b']' => CtrChar::CloseBracket,
            b':' => CtrChar::Colon,
            b'\n' => CtrChar::Newline,
            b'\t' => CtrChar::Tab,
            _ => return None,
        })
    }
    pub fn as_char(self) -> u8 {
        match self {
            CtrChar::ForwardSlash => b'/',
            CtrChar::Colon => b':',
            CtrChar::Newline => b'\n',
            CtrChar::Tab => b'\t',
            CtrChar::OpenBracket => b'[',
            CtrChar::CloseBracket => b']',
        }
    }

    pub fn is_bracket(&self) -> bool {
        matches!(self, CtrChar::CloseBracket | CtrChar::OpenBracket)
    }
    pub fn is_ex_delim(&self) -> bool {
        matches!(self, CtrChar::Newline | CtrChar::Tab)
    }
    pub fn is_internal_delim(&self) -> bool {
        matches!(self, CtrChar::Colon | CtrChar::ForwardSlash)
    }
}
pub enum Byte<'a> {
    Finished,
    Ctr { kind: CtrChar, rest: &'a [u8] },
    Byte { byte: u8, rest: &'a [u8] },
}

pub const STD_PLAIN_CSET: u32 = 0;
pub const STD_ERR_CSET: u32 = CtrChar::Newline as u32 | CtrChar::Tab as u32;
pub(crate) fn next_byte(
    ascii_bytes: &[u8],
    idx: usize,
    plain_cset: u32,
    err_cset: u32,
) -> Result<Byte, ABTxtError> {
    let (b, rest) = match ascii_bytes.split_first() {
        Some(v) => v,
        None => return Ok(Byte::Finished),
    };
    let byte = *b;
    let ctr = match b {
        b'/' => CtrChar::ForwardSlash,
        b'[' => CtrChar::OpenBracket,
        b']' => CtrChar::CloseBracket,
        b':' => CtrChar::Colon,
        b'\n' => CtrChar::Newline,
        b'\t' => CtrChar::Tab,
        b'\\' => {
            let (s, mut rest) = rest
                .split_first()
                .ok_or(ABTxtError::ParseError { idx, byte })?;
            let byte = match s {
                b'x' => {
                    let byte = hex_byte(rest)?;
                    rest = &rest[2..];
                    byte
                }
                b']' => b']',
                b'[' => b'[',
                b'n' => b'\n',
                b'r' => b'\r',
                b':' => b':',
                b't' => b'\t',
                b'\\' => b'\\',
                b'/' => b'/',
                b'0' => 0,
                e if *e == MAXB_CHAR => 255,
                _b => return Err(ABTxtError::ParseError { idx, byte }),
            };
            return Ok(Byte::Byte { byte, rest });
        }
        b'\r' => return Err(ABTxtError::ParseError { idx, byte }),
        0x20..=0x7e => return Ok(Byte::Byte { byte: *b, rest }),
        _b => return Err(ABTxtError::ParseError { idx, byte }),
    };
    if err_cset & (ctr as u32) != 0 {
        Err(ABTxtError::ParseError { idx, byte })
    } else if plain_cset & (ctr as u32) != 0 {
        return Ok(Byte::Byte { byte: *b, rest });
    } else {
        return Ok(Byte::Ctr { kind: ctr, rest });
    }
}

#[inline(always)]
fn parse_abtxt(
    string: &[u8],
    dest: &mut impl Write,
    max: Option<usize>,
) -> Result<usize, ABTxtError> {
    let mut rest = string;
    let len = rest.len();
    let mut total = 0;
    if string == b"\\E" {
        return Ok(0);
    }
    loop {
        let idx = len - rest.len();
        match next_byte(rest, idx, STD_PLAIN_CSET, STD_ERR_CSET)? {
            Byte::Finished => return Ok(total),
            Byte::Byte { byte, rest: tail } => {
                if let Some(m) = max {
                    if total >= m {
                        return Err(FitSliceErr {
                            size: Some(m),
                            got: Ok(idx),
                        }
                        .into());
                    }
                }
                match dest.write(&[byte]) {
                    Ok(0) => {
                        return Err(FitSliceErr {
                            size: None,
                            got: Ok(idx),
                        }
                        .into())
                    }
                    Ok(_) => {}
                    Err(e) => todo!("{:?}", e), // Prob unreachable
                };
                total += 1;
                rest = tail;
            }
            Byte::Ctr { kind: _, rest: _ } => {
                return Err(ABTxtError::ParseError { byte: rest[0], idx })
            }
        }
    }
}

pub fn parse_abtxt_into(string: impl AsRef<[u8]>, dest: &mut [u8]) -> Result<usize, ABTxtError> {
    parse_abtxt(string.as_ref(), &mut Cursor::new(dest), None)
}
pub fn parse_abtxt_upto_max(string: impl AsRef<[u8]>, max: usize) -> Result<Vec<u8>, ABTxtError> {
    let mut content = Vec::new();
    parse_abtxt(
        string.as_ref(),
        &mut Cursor::<&mut Vec<u8>>::new(&mut content),
        Some(max),
    )?;
    Ok(content)
}

fn hex_byte(s: &[u8]) -> Result<u8, ABTxtError> {
    let mut ch = 0;
    if s.len() < 2 {
        return Err(ABTxtError::InvalidHex);
    }
    let b0 = s[0];
    let b1 = s[1];
    ch += 0x10
        * match b0 {
            b'0'..=b'9' => b0 - b'0',
            b'a'..=b'f' => 10 + (b0 - b'a'),
            b'A'..=b'F' => 10 + (b0 - b'A'),
            _ => return Err(ABTxtError::InvalidHex),
        };
    ch += match b1 {
        b'0'..=b'9' => b1 - b'0',
        b'a'..=b'f' => 10 + (b1 - b'a'),
        b'A'..=b'F' => 10 + (b1 - b'A'),
        _ => return Err(ABTxtError::InvalidHex),
    };
    Ok(ch)
}
const HEX_CHARS_LOWER: &[u8; 16] = b"0123456789abcdef";

pub const fn hex_lookup(byte: u8) -> [u8; 4] {
    let mut b = *b"\\x00";
    b[2] = HEX_CHARS_LOWER[(byte >> 4) as usize];
    b[3] = HEX_CHARS_LOWER[(byte & 0xf) as usize];
    b
}

pub fn escaped_byte(byte: u8) -> bool {
    escape_default(byte).as_bytes().len() > 1
}
pub const HEX_LOOKUP: [[u8; 4]; 256] = {
    let mut v = [[0; 4]; 256];
    let mut idx = 0;
    while idx < 256 {
        v[idx] = hex_lookup(idx as u8);
        idx += 1;
    }
    v
};
pub const ARR_IDX: [u8; 256] = {
    let mut v = [0; 256];
    let mut idx = 0;
    while idx < 256 {
        v[idx] = idx as u8;
        idx += 1;
    }
    v
};

pub const ESCAPE_LOOKUP: [&str; 256] = {
    let mut lookup = [""; 256];
    let mut idx = 0usize;
    while idx < 256 {
        let st = if idx == 0 {
            "\\0"
        } else if idx == 255 {
            MAX_ESCAPED
        } else if idx == b'\n' as usize {
            "\\n"
        } else if idx == b'\t' as usize {
            "\\t"
        } else if idx == b'\r' as usize {
            "\\r"
        } else if idx == b':' as usize {
            "\\:"
        } else if idx == b'\\' as usize {
            "\\\\"
        } else if idx == b'/' as usize {
            "\\/"
        } else if idx == b'[' as usize {
            "\\["
        } else if idx == b']' as usize {
            "\\]"
        } else if idx >= 0x20 && idx <= 0x7e {
            let st = std::slice::from_ref(&ARR_IDX[idx]);
            match std::str::from_utf8(st) {
                Ok(r) => r,
                Err(_) => todo!(),
            }
        } else {
            match std::str::from_utf8(&HEX_LOOKUP[idx] as &[u8]) {
                Ok(r) => r,
                Err(_) => todo!(),
            }
        };
        lookup[idx] = st;
        idx += 1;
    }
    lookup
};

pub fn escape_default(byte: u8) -> &'static str {
    ESCAPE_LOOKUP[byte as usize]
}

pub fn as_abtxt(bytes: &[u8]) -> Cow<str> {
    if bytes.iter().all(|v| !escaped_byte(*v)) {
        return std::str::from_utf8(bytes).unwrap().into();
    }
    let mut st = String::with_capacity(bytes.len() * 4);
    for b in bytes {
        st.push_str(escape_default(*b));
    }
    st.into()
}

#[test]
fn enc() {
    let st = "hello🌍";
    assert_eq!(
        st.as_bytes(),
        &[104, 101, 108, 108, 111, 240, 159, 140, 141]
    );
    let v = as_abtxt(st.as_bytes());
    assert_eq!(v, r#"hello\xf0\x9f\x8c\x8d"#);

    assert_eq!(crate::cut_prefix_nulls(&[0, 0, 1, 2]), &[1, 2])
}
