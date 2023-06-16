
use std::marker::PhantomData;

use crate::{eval::{ScopeFunc, EvalScopeImpl, Scope, ApplyResult, Describer, ScopeFuncInfo}, fncs, ABE};

use anyhow::Context;
use arrayvec::ArrayVec;

use super::{uint::parse_b, bytes::slice};


#[derive(Clone, Debug)]
pub struct ArgV<'o>(pub ArrayVec<&'o [u8],8>);
impl<'o> ArgV<'o>{
    pub fn try_fit(v: &'o [&'o [u8]]) -> Option<Self>{
        v.try_into().map(ArgV).ok()
    }
}
impl<'o> EvalScopeImpl for ArgV<'o>{
    fn about(&self) -> (String, String) {
        ("user input list".into(), "Provide values, access with [0] [1] .. [7] ".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            ( "0" , 0..=0,Some(true), "argv[0]", |t:&Self,_| Ok(t.0.get(0).context("no 0 value")?.to_vec())),
            ( "1" , 0..=0,Some(true), "argv[1]", |t:&Self,_| Ok(t.0.get(1).context("no 1 value")?.to_vec())),
            ( "2" , 0..=0,Some(true), "argv[2]", |t:&Self,_| Ok(t.0.get(2).context("no 2 value")?.to_vec())),
            ( "3" , 0..=0,Some(true), "argv[3]", |t:&Self,_| Ok(t.0.get(3).context("no 3 value")?.to_vec())),
            ( "4" , 0..=0,Some(true), "argv[4]", |t:&Self,_| Ok(t.0.get(4).context("no 4 value")?.to_vec())),
            ( "5" , 0..=0,Some(true), "argv[5]", |t:&Self,_| Ok(t.0.get(5).context("no 5 value")?.to_vec())),
            ( "6" , 0..=0,Some(true), "argv[6]", |t:&Self,_| Ok(t.0.get(6).context("no 6 value")?.to_vec())),
            ( "7" , 0..=0,Some(true), "argv[7]", |t:&Self,_| Ok(t.0.get(7).context("no 7 value")?.to_vec()))
        ])
    }
}

//FIXME: ArgV and ArgList should prob be merged
pub struct ArgList<'o,A,B>(pub A,PhantomData<&'o B>);
impl<'o,A,B> ArgList<'o,A,B>{
    pub fn new(lst:A) -> Self { ArgList(lst,PhantomData)}
}
impl<'o,A: AsRef<[B]>+'o, B: AsRef<[u8]>+'o> Scope for ArgList<'o,A,B> {
    fn try_apply_func(
        &self,
        id: &[u8],
        args: &[&[u8]],
        init: bool,
        _ctx: &dyn Scope,
    ) -> ApplyResult {
        if id == b"argv"{
            let slicing = self.0.as_ref();
            let using = slice(slicing,args)?;
            let mut r = vec![];
            using.for_each(|b| r.extend_from_slice(b.as_ref()));
            return ApplyResult::Value(r)
        }
        let idx = parse_b::<usize>(id).ok()?;
        if !args.is_empty() || !init { return ApplyResult::Err(anyhow::anyhow!("argv takes not arguments and cant be chained"));}
        match self.0.as_ref().get(idx){
            Some(i) => ApplyResult::Value(i.as_ref().to_vec()),
            None => ApplyResult::Err(anyhow::anyhow!("argv oob - tried {idx} but only {} are set",self.0.as_ref().len()))
        }
    }

    fn try_apply_macro(&self, _id: &[u8], _abe: &[ABE], _scopes: &dyn Scope) -> ApplyResult {
        ApplyResult::NoValue
    }

    fn describe(&self, cb: Describer) {
        let it = std::iter::from_fn(||{
            Some(ScopeFuncInfo {
                id :"<nth e.g. '0' '1'...>",
                init_eq: Some(true),
                to_abe: false,
                argc: 0..=0,
                help: "argv[nth]"
            })    
        }).take(self.0.as_ref().len());
        let reflect = ScopeFuncInfo {
            id: "argv",
            init_eq: None,
            to_abe:false,
            argc:1..=4,
            help:"argv slice idx - 'argv:-1' "
        };
        let mut it = it.chain(Some(reflect));
        cb("argv", "", &mut it, &mut std::iter::empty())
    }

    fn try_encode(&self, _id: &[u8], _options: &[ABE], _bytes: &[u8]) -> ApplyResult<String> {
        ApplyResult::NoValue
    }
}

