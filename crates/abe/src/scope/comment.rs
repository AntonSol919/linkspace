use crate::{
    eval::{EvalScopeImpl, ScopeFunc},
    fnc,
};

#[derive(Copy, Clone)]
pub struct Comment;
impl EvalScopeImpl for Comment {
    fn about(&self) -> (String, String) {
        (
            "comment function / void function. evaluates to nothing".into(),
            String::new(),
        )
    }

    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[fnc!(
            "C",
            1..=16,
            "the comment function. all arguments are ignored. evaluates to ''",
            |_, _| Ok(vec![])
        )]
    }
}
