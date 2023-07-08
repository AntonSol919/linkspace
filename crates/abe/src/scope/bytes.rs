use crate::{
    abtxt::as_abtxt,
    cut_prefix_nulls,
    eval::{ApplyErr, ApplyResult, EvalScopeImpl, ScopeFunc},
    fncs,
    scope::uint::parse_b,
    ABE,
};
use anyhow::anyhow;

#[derive(Copy, Clone, Debug)]
pub struct BytesFE;

impl EvalScopeImpl for BytesFE {
    fn about(&self) -> (String, String) {
        ("bytes".into(), "Byte padding/trimming".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fn pad(
            inp: &[&[u8]],
            left: bool,
            default_pad: u8,
            check_len: bool,
            fixed: bool,
        ) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];

            let len = inp
                .get(1)
                .filter(|v| !v.is_empty())
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            let bytes = if !fixed {
                bytes
            } else {
                let nlen = len.min(bytes.len());
                if left {
                    &bytes[..nlen]
                } else {
                    &bytes[bytes.len() - nlen..]
                }
            };
            if len < bytes.len() {
                if check_len {
                    return Err(anyhow!(
                        "exceeds length {len} ( use  ~[lr]pad or [lr]fixed )"
                    ));
                };
                return Ok(bytes.to_vec());
            };
            let tmp_pad = [default_pad];
            let padb = inp.get(2).copied().unwrap_or(&tmp_pad);
            if padb.len() != 1 {
                return Err(anyhow!("pad byte should be a single byte"));
            };
            let mut v = vec![padb[0]; len];
            if !left {
                &mut v[0..bytes.len()]
            } else {
                &mut v[len - bytes.len()..]
            }
            .copy_from_slice(bytes);
            Ok(v)
        }

        fn cut(inp: &[&[u8]], left: bool, check_len: bool) -> Result<Vec<u8>, ApplyErr> {
            let bytes = inp[0];
            let len = inp
                .get(1)
                .map(|i| parse_b(i))
                .transpose()?
                .unwrap_or(16usize);
            if len > bytes.len() {
                if check_len {
                    return Err(anyhow!(
                        "less than length {len} ( use '~[lr]cut or [lr]fixed"
                    ));
                }
                return Ok(bytes.to_vec());
            };
            Ok(if left {
                &bytes[..len]
            } else {
                &bytes[bytes.len() - len..]
            }
            .to_vec())
        }
        fn encode_a(_: &BytesFE, b: &[u8], _: &[ABE]) -> ApplyResult<String> {
            let cut_b = as_abtxt(cut_prefix_nulls(b));
            let len = b.len();
            ApplyResult::Value(if len == 16 {
                format!("[a:{cut_b}]")
            } else {
                format!("[a:{cut_b}:{len}]")
            })
        }
        

        fncs!([
            ("?a",1..=1,"encode bytes into ascii-bytes format",|_,i:&[&[u8]]| Ok(as_abtxt(i[0]).into_owned().into_bytes())),
            ("?a0",1..=1,"encode bytes into ascii-bytes format but strip prefix '0' bytes",
             |_,i:&[&[u8]]| Ok(as_abtxt(cut_prefix_nulls(i[0])).into_owned().into_bytes())),
            (@C "a",1..=3,None,"[bytes,length = 16,pad_byte = \\0] - alias for 'lpad'",|_,i:&[&[u8]],_,_| pad(i,true,0,true,false),
             encode_a),
            ("f",1..=3,"same as 'a' but uses \\xff as padding ",|_,i:&[&[u8]]| pad(i,true,255,true,false)),
            ("lpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad input bytes",|_,i:&[&[u8]]| pad(i,true,0,true,false)),
            ("rpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad input bytes",|_,i:&[&[u8]]| pad(i,false,0,true,false)),
            ("~lpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad input bytes",|_,i:&[&[u8]]| pad(i,true,0,false,false)),
            ("~rpad",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad input bytes",|_,i:&[&[u8]]| pad(i,false,0,false,false)),

            ("lcut",1..=2,"[bytes,length = 16] - left cut input bytes",|_,i:&[&[u8]]| cut(i,true,true)),
            ("rcut",1..=2,"[bytes,length = 16] - right cut input bytes",|_,i:&[&[u8]]| cut(i,false,true)),
            ("~lcut",1..=2,"[bytes,length = 16] - lcut without error",|_,i:&[&[u8]]| cut(i,true,false)),
            ("~rcut",1..=2,"[bytes,length = 16] - lcut without error",|_,i:&[&[u8]]| cut(i,false,false)),
            ("lfixed",1..=3,"[bytes,length = 16,pad_byte = \\0] - left pad and cut input bytes",|_,i:&[&[u8]]| pad(i,true,0,false,true)),
            ("rfixed",1..=3,"[bytes,length = 16,pad_byte = \\0] - right pad and cut input bytes",|_,i:&[&[u8]]| pad(i,false,0,false,true)),
            ("replace",3..=3,"[bytes,from,to] - replace pattern from to to",|_,i:&[&[u8]]| replace(i.try_into().unwrap())),

            ("slice",1..=4,"[bytes,start=0,stop=len,step=1] - python like slice indexing",|_,i:&[&[u8]]|{
                slice(i[0],&i[1..]).map(|i| i.copied().collect::<Vec<u8>>() )
            }),
            ("~utf8",1..=1,"lossy encode as utf8",|_,i:&[&[u8]]| ApplyResult::Value(bstr::BStr::new(&i[0]).to_string().into_bytes()))

        ])
    }
}
fn replace([mut b, from,to]:[&[u8];3]) -> Vec<u8>{
    let mut r =Vec::with_capacity(b.len());
    while let Some((h,rest)) = b.split_first(){
        match b.strip_prefix(from){
            Some(rest) => {r.extend_from_slice(to); b = rest},
            None => {r.push(*h); b = rest},
        }
    }
    r
}

/// python like slice indexing
pub fn slice<'o, F>(bytes:&'o [F], args: &[&[u8]]) -> Result<impl Iterator<Item=&'o F>+'o, ApplyErr> {
    #[derive(Debug, Copy, Clone)]
    struct SignedInt {
        neg: bool,
        val: usize,
    }
    fn parse_b_signed(bytes: Option<&[u8]>) -> anyhow::Result<Option<SignedInt>> {
        let bytes = bytes.unwrap_or(&[]);
        if bytes.is_empty() {
            return Ok(None);
        }
        let (neg, val_b) = bytes
            .strip_prefix(b"-")
            .map(|s| (true, s))
            .unwrap_or((false, bytes));
        Ok(Some(SignedInt {
            neg,
            val: std::str::from_utf8(val_b)?.parse()?,
        }))
    }
    let len = bytes.len() as isize;
    let start = parse_b_signed(args.get(0).copied())?;
    let end = parse_b_signed(args.get(1).copied())?;
    let step: isize = args
        .get(2)
        .map(|b| anyhow::Ok::<isize>(std::str::from_utf8(b)?.parse()?))
        .transpose()?
        .unwrap_or(1);
    
    let (sb, eb) = if step >= 0 { (0, len) } else { (len - 1, -1) };

    let to_bound = |v: Option<SignedInt>| -> Option<isize> {
        match v {
            Some(SignedInt { neg: false, val }) => {
                Some((val as isize).clamp(sb.min(eb), sb.max(eb)))
            }
            Some(SignedInt { neg: true, val }) => {
                Some((len - val as isize).clamp(sb.min(eb), sb.max(eb)))
            }
            None => None,
        }
    };
    let mut i = to_bound(start).unwrap_or(sb);
    let end = to_bound(end).unwrap_or(eb);

    Ok(std::iter::from_fn(move || {
        let in_range = if step >= 0 { i < end } else { i > end };
        if !in_range || step == 0{
            return None;
        }
        let result = bytes.get(i as usize)?;
        i += step;
        Some(result)
    }))
}
