use crate::{eval::{ScopeFunc, EvalScopeImpl, ScopeMacro, Scope }, scope_macro, fncs, scope::uint::parse_b, ABE};
use anyhow::{anyhow, Context};

#[derive(Copy, Clone, Debug)]
pub struct LogicOps;
impl EvalScopeImpl for LogicOps {
    fn about(&self) -> (String, String) {
        ("logic ops".into(), "ops are : < > = 0 1 ".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        // TODO, extra crate for test_ops
        fncs!([
            (
                "size?",
                3..=3,
                "[in,OP,VAL] error unless size passes the test ( UNIMPLEMENTED )",
                |_, i: &[&[u8]]| {
                    let size = parse_b::<usize>(i[2])?;
                    let bytes = i[0];
                    let blen = bytes.len();
                    match i[1] {
                        b"=" => {
                            if blen != size {
                                return Err(anyhow!("expected {size} bytes got {blen}"));
                            } 
                        }
                        _ => return Err(anyhow!("unknown op")),
                    };
                    Ok(i[0].to_vec())
                }
            ),
            (
                "val?",
                3..=3,
                "[in,OP,VAL] error unless value passes the test ( UNIMPLMENTED)",
                |_, i: &[&[u8]]| {
                    let bytes = i[0];
                    match i[1] {
                        b"=" => {
                            if bytes != i[2] {
                                return Err(anyhow!("unequal bytes"));
                            } 
                        }
                        _ => return Err(anyhow!("unknown op")),
                    };
                    Ok(i[0].to_vec())
                }
            )
        ])
    }
    fn list_macros(&self) -> &[ScopeMacro<&Self>] {
        &[
            scope_macro!("or",":{EXPR}[:{EXPR}]* short circuit evaluate until valid return. Empty is valid, use {_/minsize?} to error on empty",
                  |_,i:&[ABE],scope:&dyn Scope|{
                      let mut it = i.split(|v| v.is_colon());
                      if !it.next().context("missing expr")?.is_empty(){ return Err(anyhow!("expected ':EXPR'"))};
                      let mut err = vec![];
                      for o in it{
                          match crate::eval::eval(scope,o){
                              Ok(b) => return Ok(b.concat()),
                              Err(e) => err.push((o,e)),
                          }
                      }
                      Err(anyhow!("{err:#?}"))
                  }
            )
        ]
    }
}
