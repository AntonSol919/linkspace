use base64::{prelude::BASE64_URL_SAFE_NO_PAD, DecodeError, Engine};

use crate::{eval::{EvalScopeImpl, ScopeFunc, ApplyResult, ApplyErr}, fncs, ABE, cut_prefix_nulls};

fn bin(inp: &[&[u8]], radix: u32) -> Result<Vec<u8>, ApplyErr> {
    // FIXME probably want to better handle leading '000000000'
    let st = std::str::from_utf8(inp[0])?;
    let i = u128::from_str_radix(st, radix)?;
    if i == 0 {
        Ok(vec![0])
    } else {
        Ok(cut_prefix_nulls(&i.to_be_bytes()).to_vec())
    }
}

// FIXME ambiguous lengths
fn enc_bin(bytes:&[u8],opts:&[ABE], radix:u32) -> ApplyResult<String>{
    let len_set :Vec<usize>= opts.iter().map(|v| -> anyhow::Result<usize>{
        Ok(std::str::from_utf8(crate::ast::as_bytes(v)?)?.parse::<usize>()?)
    }).try_collect()?;
    if !len_set.is_empty() {
        if !len_set.contains(&bytes.len()) { return ApplyResult::NoValue;}
    }
    use std::fmt::Write;
    match radix {
        2 => {
            let mut st = String::with_capacity(bytes.len()*8);
            bytes.iter().for_each(|b|{let _ = write!(&mut st,"{b:08b}");});
            ApplyResult::Value(format!("[b{radix}:{st}]"))
        },
        16 => {
            let mut st = String::with_capacity(bytes.len()*2);
            bytes.iter().for_each(|b|{let _ = write!(&mut st,"{b:02X}");});
            ApplyResult::Value(format!("[b{radix}:{st}]"))
        },
        _ => ApplyResult::Err(anyhow::anyhow!("fixme: radix supported"))
    }
}

/// Implements [AAAAAA/b64] and [\0\0\xff/2b64]
#[derive(Copy, Clone, Debug)]
pub struct BaseNScope;
impl EvalScopeImpl for BaseNScope {
    fn about(&self) -> (String, String) {
        ("base-n".into(), "base{2,8,16,32,64} encoding - (b64 is url-safe no-padding)".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( @C "b2",1..=1, None, "decode binary",|_,i:&[&[u8]],_,_| bin(i,2), |_,b:&[u8],opts:&[ABE]| enc_bin(b, opts, 2)),
            ("b8",1..=1,"encode octets",|_,i:&[&[u8]]| bin(i,8)),
            ( @C "b16",1..=1, None, "decode hex",|_,i:&[&[u8]],_,_| bin(i,16), |_,b:&[u8],opts:&[ABE]| enc_bin(b, opts, 16)),
            ("?b",1..=1,"encode base64",|_,i:&[&[u8]]| Ok(base64(i[0]).into_bytes())),
            ("2mini",1..=1,"encode mini",|_,i:&[&[u8]]| Ok(mini_b64(i[0]).into_bytes())),
            ( @C "b", 1..=1, None, "decode base64",
               |_,i:&[&[u8]],_,_| Ok(base64_decode(i[0])?),
               |_,b:&[u8],opts:&[ABE]| -> ApplyResult<String>{
                   if opts.is_empty(){
                       return ApplyResult::Value(format!("[b:{}]",base64(b)));
                   }
                   for len_st in opts.iter().filter_map(|v| crate::ast::as_bytes(v).ok()){
                       let len = std::str::from_utf8(len_st)?.parse::<u32>()?;
                       if len as usize == b.len() {
                           return ApplyResult::Value(format!("[b:{}]",base64(b)));
                       }
                   }
                   ApplyResult::NoValue
               }
             )
        ])
    }
}



pub fn base64(b: impl AsRef<[u8]>) -> String {
    BASE64_URL_SAFE_NO_PAD.encode(b.as_ref())
}
pub fn base64_decode(st: impl AsRef<[u8]>) -> Result<Vec<u8>, DecodeError> {
    BASE64_URL_SAFE_NO_PAD.decode(st.as_ref())
}
pub fn mini_b64(v: &[u8]) -> String {
    let mut r = String::with_capacity(10);
    let len = v.len();
    let padc = len / 8;
    let st = base64(v);
    if v.len() <= 12 {
        return st;
    }
    r.push_str(&st[0..6]);
    r.push_str(&":".repeat(padc / 2));
    if padc % 2 != 0 {
        r.push('.');
    }
    r.push_str(&st[st.len() - 2..]);
    r
}
