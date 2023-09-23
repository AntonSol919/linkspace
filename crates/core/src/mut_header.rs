// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{
    predicate::{exprs::TestEvalErr, TestOp},
    prelude::{ExtPredicate, Predicate},
};
/**
A mutation to a netpkt header. can force set specific bits to 0 or 1.
**/
use anyhow::Context;
use linkspace_pkt::abe::ABEValidator;
use linkspace_pkt::{
    abe::{ast::MatchError, eval::*, TypedABE, ABE},
    NetPktHeader,
};

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub struct NetHeaderMutate {
    pub clear: NetPktHeader,
    pub set: NetPktHeader,
}
impl NetHeaderMutate {
    pub const DEFAULT: Self = NetHeaderMutate {
        clear: NetPktHeader::cfrom([255; 32]),
        set: NetPktHeader::cfrom([0; 32]),
    };
    pub fn mutate(&self, h: &mut NetPktHeader) {
        let b: &mut [u128; 2] = unsafe { &mut *(h as *mut NetPktHeader as *mut [u128; 2]) };
        let clear: &[u128; 2] =
            unsafe { &*(&self.clear as *const NetPktHeader as *const [u128; 2]) };
        let set: &[u128; 2] = unsafe { &*(&self.set as *const NetPktHeader as *const [u128; 2]) };
        b[0] &= clear[0];
        b[0] |= set[0];
        b[1] &= clear[1];
        b[1] |= set[1];
    }
    pub fn from_lst(lst: &[MutFieldExpr], scope: &dyn Scope) -> anyhow::Result<Self> {
        let mut m = NetHeaderMutate::DEFAULT;
        for v in lst {
            let r = v.clone().eval(scope)?;
            for pre in r.0.try_iter()? {
                m.try_add_rule(pre)?;
            }
        }
        Ok(m)
    }

    pub fn try_add_rule(&mut self, v: Predicate) -> anyhow::Result<()> {
        let s = match v.kind {
            crate::predicate::exprs::RuleType::Field(f) => f,
            _ => anyhow::bail!("wrong field type"),
        };
        let clear = s
            .mut_route(&mut self.clear)
            .context("Only variable route fields can be mutated")?;
        let set = s.mut_route(&mut self.set).unwrap();
        mutate(v.op, v.val, clear, set)
    }
}

pub type MutFieldExpr = TypedABE<MutField>;
#[derive(Clone)]
pub struct MutField(ExtPredicate);
impl ABEValidator for MutField {
    fn check(b: &[ABE]) -> Result<(), MatchError> {
        ExtPredicate::check(b)
    }
}
impl TryFrom<ABList> for MutField {
    type Error = TestEvalErr;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        ExtPredicate::try_from(value).map(MutField)
    }
}

pub fn mutate(op: TestOp, val: ABList, clear: &mut [u8], set: &mut [u8]) -> anyhow::Result<()> {
    match op {
        TestOp::Equal => {
            let v = val
                .into_exact_bytes()
                .map_err(|_| anyhow::anyhow!("Eq expects exact bytes"))?;
            anyhow::ensure!(v.len() == clear.len());
            clear.iter_mut().for_each(|v| *v = 255);
            set.iter_mut().zip(v).for_each(|(set, val)| *set = val);
        }
        e => anyhow::bail!("op not yet supported {}", e),
    }
    Ok(())
}
