use crate::{eval::{ScopeFunc, EvalScopeImpl, ScopeFuncInfo, ApplyResult, ScopeMacro, ScopeMacroInfo, Scope },  ast::{take_first, is_colon, parse_abe_strict_b}};
use anyhow::{anyhow, Context};

fn encode(inp:&[&[u8]], scope: &dyn Scope) -> anyhow::Result<String>{
    if inp.len() > 2 { return Err(anyhow!("Options not yet supported"))};
    let kind = std::str::from_utf8(inp[1]).context("bad encoder")?;
    Ok(crate::eval::encode(scope, inp[0], kind, false)?)
}

#[derive(Copy, Clone)]
pub struct Encode;
impl EvalScopeImpl for Encode {
    fn about(&self) -> (String, String) {
        (
            "encode".into(),
            "attempt an inverse of a set of functions".into(),
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            info: ScopeFuncInfo {
                id: "eval",
                init_eq: None,
                to_abe: false,
                argc: 1..=1,
                help: "parse and evaluate",
            },
            apply: |_, inp, _, scope| {
                let expr = parse_abe_strict_b(inp[0])?;
                ApplyResult::Value(crate::eval::eval(scope, &expr)?.concat())
            },
            to_abe: crate::eval::none,
        },
          ScopeFunc {
              info: ScopeFuncInfo {
                  id: "?",
                  init_eq: None,
                  to_abe: false, // TODO
                  argc: 2..=8,
                  help: "encode",
              },
              apply: |_, inp, _, scope| {

                  if inp.len() > 2 { return ApplyResult::Err(anyhow!("Options not yet supported"))};
                  let options = std::str::from_utf8(inp[1]).context("bad encoder")?;
                  let r = crate::eval::encode(scope, inp[0], options, false)?;
                  ApplyResult::Value(r.into_bytes())
              },
              to_abe: crate::eval::none,
          },
          ScopeFunc {
              info: ScopeFuncInfo {
                  id: "??",
                  init_eq: None,
                  to_abe: false, // TODO
                  argc: 2..=8,
                  help: "encode - strip out '[' ']'",
              },
              apply: |_, inp, _, scope| {
                  let r = encode(inp,scope)?;
                  let v = match r.strip_prefix('[').and_then(|o| o.strip_suffix(']')){
                      Some(v) => v.as_bytes().to_vec(),
                      None => r.into_bytes(),
                  };
                  ApplyResult::Value(v)
              },
              to_abe: crate::eval::none,
          },
          ScopeFunc {
              info: ScopeFuncInfo {
                  id: "???",
                  init_eq: None,
                  to_abe: false, // TODO
                  argc: 2..=8,
                  help: "encode - strip out '[func:' + ']'",
              },
              apply: |_, inp, _, scope| {
                  let r = encode(inp,scope)?;
                  let v = match r.strip_prefix('[').and_then(|o| o.strip_suffix(']')){
                      Some(v) => {
                          let mut it = v.as_bytes().split(|v| *v == b':');
                          it.next();
                          it.as_slice().to_vec()
                      },
                      None => r.into_bytes(),
                  };
                  ApplyResult::Value(v)
              },
              to_abe: crate::eval::none,
          },
        ]
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[
            ScopeMacro {
                info: ScopeMacroInfo { id: "?", help: "find an abe encoding for the value trying multiple reversal functions - [/fn:{opts}]* " },
                apply:|_,abe,scope|-> ApplyResult{
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_fslash());
                    let id = it.next().context("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = crate::eval::eval(scope, id)?.concat();
                    ApplyResult::Value(crate::eval::encode_abe(scope, &bytes, rest,false)?.into_bytes())
                }
            },
            ScopeMacro {
                info: ScopeMacroInfo { id: "~?", help: "same as '?' but ignores all errors" },
                apply:|_,abe,scope|-> ApplyResult{
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_fslash());
                    let id = it.next().context("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = crate::eval::eval(scope, id)?.concat();
                    ApplyResult::Value(crate::eval::encode_abe(scope, &bytes, rest,true)?.into_bytes())
                }
            },
            ScopeMacro {
                info: ScopeMacroInfo { id: "e", help: "eval inner expression list. Useful to avoid escapes: eg file:{/e:/some/dir:thing}:opts does not require escapes the '/' " },
                apply:|_,abe,scope|-> ApplyResult{
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    ApplyResult::Value(crate::eval::eval(scope , abe)?.concat())
                }
            },
        ]
    }
}
