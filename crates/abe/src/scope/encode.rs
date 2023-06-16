use crate::{eval::{ScopeFunc, EvalScopeImpl, ScopeFuncInfo, EvalCtx, ApplyResult, ScopeMacro, ScopeMacroInfo}, parse_abe_b, ast::{take_first, is_colon}};
use anyhow::{anyhow, Context};

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
                let expr = parse_abe_b(inp[0])?;
                let ctx = EvalCtx { scope };
                ApplyResult::Value(crate::eval::eval(&ctx, &expr)?.concat())
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
                  let ctx = EvalCtx{scope};
                  if inp.len() > 2 { return ApplyResult::Err(anyhow!("Options not yet supported"))};
                  let kind = std::str::from_utf8(inp[1]).context("bad encoder")?;
                  let r = crate::eval::encode(&ctx, inp[0], kind, false)?;
                  ApplyResult::Value(r.into_bytes())
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
                    let ctx = EvalCtx{scope};
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_fslash());
                    let id = it.next().context("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = crate::eval::eval(&ctx, id)?.concat();
                    ApplyResult::Value(crate::eval::encode_abe(&ctx, &bytes, rest,false)?.into_bytes())
                }
            },
            ScopeMacro {
                info: ScopeMacroInfo { id: "~?", help: "same as '?' but ignores all errors" },
                apply:|_,abe,scope|-> ApplyResult{
                    let ctx = EvalCtx{scope};
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let mut it = abe.split(|v| v.is_fslash());
                    let id = it.next().context("missing argument")?;
                    let rest = it.as_slice();
                    let bytes = crate::eval::eval(&ctx, id)?.concat();
                    ApplyResult::Value(crate::eval::encode_abe(&ctx, &bytes, rest,true)?.into_bytes())
                }
            },
            ScopeMacro {
                info: ScopeMacroInfo { id: "e", help: "eval inner expression list. Useful to avoid escapes: eg file:{/e:/some/dir:thing}:opts does not require escapes the '/' " },
                apply:|_,abe,scope|-> ApplyResult{
                    let (head,abe) = take_first(abe)?;
                    is_colon(head)?;
                    let ctx = EvalCtx{scope};
                    ApplyResult::Value(crate::eval::eval(&ctx, abe)?.concat())
                }
            },
        ]
    }
}
