use std::str::FromStr;

use crate::{eval::{ScopeFunc, EvalScopeImpl, ApplyErr}, fncs, cut_ending_nulls2};
use anyhow::anyhow;

pub fn parse_b<T: FromStr>(b: &[u8]) -> Result<T, ApplyErr>
where
    <T as FromStr>::Err: std::error::Error + Send + Sync + 'static,
{
    Ok(std::str::from_utf8(b)?.parse()?)
}
pub fn carry_add_be(bytes: &mut [u8], val: &[u8]) -> bool {
    debug_assert!(bytes.len() == val.len());
    let mut carry = false;
    let mut idx = bytes.len() - 1;
    loop {
        let (ni, nc) = bytes[idx].carrying_add(val[idx], carry);
        bytes[idx] = ni;
        carry = nc;
        if idx == 0 {
            break;
        }
        idx -= 1;
    }
    carry
}
pub fn carry_sub_be(bytes: &mut [u8], val: &[u8]) -> bool {
    debug_assert!(bytes.len() == val.len());
    let mut carry = false;
    let mut idx = bytes.len() - 1;
    loop {
        let (ni, nc) = bytes[idx].borrowing_sub(val[idx], carry);
        bytes[idx] = ni;
        carry = nc;
        if idx == 0 {
            break;
        }
        idx -= 1;
    }
    carry
}

#[derive(Copy, Clone, Debug)]
pub struct UIntFE;
impl EvalScopeImpl for UIntFE {
    fn about(&self) -> (String, String) {
        ("UInt".into(), "Unsigned integer functions".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( "+" , 1..=16,   "Saturating addition. Requires all inputs to be equal size",
                    |_,inp:&[&[u8]]| {
                        if !inp.iter().all(|v| v.len() == inp[0].len()){ return Err(anyhow!("Mismatch length"))}
                        let mut r = inp[0].to_vec();
                        for i in &inp[1..]{
                            if carry_add_be(&mut r, i){
                                r.iter_mut().for_each(|v| *v = 255);
                                return Ok(r)
                            }
                        }
                        Ok(r)
                    }
            ),
            ( "-" , 1..=16,   "Saturating subtraction. Requires all inputs to be equal size",
                    |_,inp:&[&[u8]]| {
                        if !inp.iter().all(|v| v.len() == inp[0].len()){ return Err(anyhow!("Mismatch length"))}
                        let mut r = inp[0].to_vec();
                        for i in &inp[1..]{
                            if carry_sub_be(&mut r, i){
                                r.iter_mut().for_each(|v| *v = 0);
                                return Ok(r)
                            }
                        }
                        Ok(r)
                    }
            ),
            ( "u8" , 1..=1,   "parse 1 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u8>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u8::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u16" , 1..=1,  "parse 2 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u16>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u16::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u32" , 1..=1,  "parse 4 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u32>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u32::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u64" , 1..=1,  "parse 8 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u64>(inp[0])?.to_be_bytes().to_vec()),
               { id : |b:&[u8],_| b.try_into().ok().map(u64::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "u128" , 1..=1, "parse 16 byte", |_,inp:&[&[u8]]| Ok(parse_b::<u128>(inp[0])?.to_be_bytes().to_vec()) ,
               { id : |b:&[u8],_| b.try_into().ok().map(u128::from_be_bytes).map(|t| t.to_string()) }
            ),
            ( "?u" , 1..=1, "Print big endian bytes as decimal",
              |_,inp:&[&[u8]]| {
                  let val = inp[0];
                  if val.len() > 16 { return Err(anyhow::anyhow!("ints larger than 16 bytes (fixme)"))}
                  let mut v = [0;16];
                  v[16-val.len()..].copy_from_slice(val);
                  Ok(u128::from_be_bytes(v).to_string().into_bytes())
              }
              ),
            ( "lu" , 1..=1, "parse little endian byte (upto 16)",
              |_,inp:&[&[u8]]| Ok(cut_ending_nulls2(&parse_b::<u128>(inp[0])?.to_le_bytes()).to_vec())
            ),
            ( "lu8" , 1..=1, "parse 1 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u8>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu16" , 1..=1, "parse 2 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u16>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu32" , 1..=1, "parse 4 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u32>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu64" , 1..=1, "parse 8 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u64>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "lu128" , 1..=1, "parse 16 little endian byte", |_,inp:&[&[u8]]| Ok(parse_b::<u128>(inp[0])?.to_le_bytes().to_vec()) ),
            ( "?lu",1..=1,"print little endian number",
             |_,inp:&[&[u8]]| {
                 let val = inp[0];
                 if val.len() > 16 { return Err(anyhow::anyhow!("ints larger than 16 bytes (fixme)"));}
                 let mut v = [0;16];
                 v[0..val.len()].copy_from_slice(val);
                 Ok(u128::from_le_bytes(v).to_string().into_bytes())
             })
        ])
    }
}
