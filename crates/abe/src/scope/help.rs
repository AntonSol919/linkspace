use std::collections::HashSet;

use crate::{eval::{ScopeFunc, EvalScopeImpl, ApplyResult, EvalCtx, ScopeFuncInfo, none, ScopeMacro, ScopeMacroInfo}, scope_macro};

#[derive(Copy, Clone)]
pub struct Help;
impl EvalScopeImpl for Help {
    fn about(&self) -> (String, String) {
        ("help".into(), String::new())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |_, i: &[&[u8]], _, scope| {
                ApplyResult::Value({
                    if let Some(id) = i.get(0) {
                        let mut out = "".to_string();
                        scope.describe(&mut |name, about, fncs, evls| {
                            if !out.is_empty() {
                                return;
                            }
                            let fs: Vec<_> = fncs.collect();
                            let es: Vec<_> = evls.collect();
                            if fs.iter().any(|e| e.id.as_bytes() == *id)
                                || es.iter().any(|e| e.id.as_bytes() == *id)
                            {
                                let _ = fmt_describer(
                                    &mut out,
                                    &mut Default::default(),
                                    name,
                                    about,
                                    &mut fs.into_iter(),
                                    &mut es.into_iter(),
                                );
                            }
                        });
                        if out.is_empty() {
                            use std::fmt::Write;
                            write!(&mut out, "no such fnc found")?;
                        };
                        out.into_bytes()
                    } else {
                        EvalCtx { scope }.to_string().into_bytes()
                    }
                })
            },
            info: ScopeFuncInfo {
                id: "help",
                init_eq: None,
                argc: 0..=16,
                help: "help",
                to_abe: false,
            },
            to_abe: none,
        }]
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[scope_macro!(
            "help",
            "desribe current eval context",
            |_, _, scope| { Ok(crate::eval::EvalCtx { scope }.to_string().into_bytes()) }
        )]
    }
}

pub (crate) fn fmt_describer(
    f: &mut dyn std::fmt::Write,
    seen: &mut HashSet<&'static str>,
    name: &str,
    about: &str,
    funcs: &mut dyn Iterator<Item = ScopeFuncInfo>,
    evals: &mut dyn Iterator<Item = ScopeMacroInfo>,
) -> std::fmt::Result {
    let (mut fnc_head, mut evl_head) = (true, true);
    writeln!(f, "# {name}\n{about}")?;
    for ScopeFuncInfo {
        id,
        init_eq,
        argc,
        help,
        to_abe,
    } in funcs
    {
        if std::mem::take(&mut fnc_head) {
            writeln!(f, "## Functions")?;
        }
        let state = if seen.insert(id) {
            "        "
        } else {
            "<partial>"
        };
        let fslash = if init_eq != Some(false) { "[" } else { " " };
        let colon = if init_eq != Some(true) { "/" } else { " " };
        let encode = if to_abe { "?" } else { " " };
        writeln!(
            f,
            "- {id: <16} {fslash}{colon}{encode} {state} {argc:?}     {help}  "
        )?;
    }
    for ScopeMacroInfo { id, help } in evals {
        if std::mem::take(&mut evl_head) {
            writeln!(f, "## Macros")?;
        }
        writeln!(f, "- {id: <16} {help}  ")?;
    }
    writeln!(f)?;
    Ok(())
}
