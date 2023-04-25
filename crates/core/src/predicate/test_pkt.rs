// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::pkt::field_ids::FieldEnum;
/**
This needs performance testing and probably a rewrite.
Current design: Every uniq <FIELD,SetOfTests> has a type generated that impls pktstreamtest.
With ~20 fields and 4! number of SetOfTests this is probably draining the cache very fast.
**/
use std::fmt::Debug;
use std::marker::PhantomData;

use crate::{pkt::*, predicate::TestOp};

use super::exprs::Predicate;
use super::pkt_predicates::PktPredicates;
use super::TestVal;
use super::{exprs::RuleType, Bound, TestSet, UInt};

/// One or more tests to run on a packet
pub trait PktStreamTest
where
    Self: std::fmt::Debug,
{
    fn test(&self, _pkt: &NetPktPtr) -> bool;
    /// Run test, and return a RuleType if it returns false
    fn result(&self, pkt: &NetPktPtr) -> Result<(), RuleType> {
        if self.test(pkt) {
            Ok(())
        } else {
            Err(self.get_field())
        }
    }
    fn get_field(&self) -> RuleType;
    fn as_rules(&self) -> Box<dyn Iterator<Item = Predicate> + '_>;
}
impl PktStreamTest for [Box<dyn PktStreamTest>] {
    fn result(&self, pkt: &NetPktPtr) -> Result<(), RuleType> {
        for t in self {
            t.result(pkt)?;
        }
        Ok(())
    }
    fn test(&self, pkt: &NetPktPtr) -> bool {
        self.iter().all(|v| {
            let ok = v.test(pkt);
            tracing::trace!(ok,v=?v,"test");
            ok
        })
    }
    fn get_field(&self) -> RuleType {
        panic!()
    }

    fn as_rules(&self) -> Box<dyn Iterator<Item = Predicate> + '_> {
        Box::new(self.iter().flat_map(|v| v.as_rules()))
    }
}

#[derive(Debug)]
pub struct SPathPrefix(SPathBuf);
impl PktStreamTest for SPathPrefix {
    fn test(&self, pkt: &NetPktPtr) -> bool {
        // FIXME: impl ipath starts_with and replace this
        pkt.path().map(|p| p.starts_with(&self.0)).unwrap_or(false)
    }
    fn get_field(&self) -> RuleType {
        RuleType::PrefixPath
    }

    fn as_rules(&self) -> Box<(dyn Iterator<Item = Predicate> + 'static)> {
        Box::new(std::iter::once(Predicate::from(
            RuleType::PrefixPath,
            TestOp::Equal,
            self.0.ablist(),
        )))
    }
}
impl PktStreamTest for PointTypeFlags {
    fn test(&self, pkt: &NetPktPtr) -> bool {
        self.contains(pkt.point_header().point_type)
    }
    fn get_field(&self) -> RuleType {
        RuleType::Field(FieldEnum::PktTypeF)
    }

    fn as_rules(&self) -> Box<(dyn Iterator<Item = Predicate> + 'static)> {
        Box::new(std::iter::empty()) // TODO
    }
}
fn is_some<T: Default + PartialEq>(t: T) -> Option<T> {
    if t == Default::default() {
        None
    } else {
        Some(t)
    }
}

pub fn compile_predicates(
    r: &PktPredicates,
) -> (
    impl Iterator<Item = (Box<dyn PktStreamTest>, RuleType)>,
    Bound<u64>,
) {
    let PktPredicates {
        pkt_types,
        domain,
        group,
        pubkey,
        hash,
        pkt_size,
        path_prefix,
        path_len,
        create,
        links_len,
        data_size,
        recv_stamp,
        state: _,
        var_flags,
        var_hop,
        var_until,
        var_ubits,
    } = r;

    let it = into_tests::<PktTypeF, _>(pkt_types)
        .chain(into_tests::<VarNetFlagsF, _>(var_flags))
        .chain(into_tests::<VarHopF, _>(var_hop))
        .chain(into_tests::<VarStampF, _>(var_until))
        .chain(into_tests::<VarUBits0F, _>(&var_ubits[0]))
        .chain(into_tests::<VarUBits1F, _>(&var_ubits[1]))
        .chain(into_tests::<VarUBits2F, _>(&var_ubits[2]))
        .chain(into_tests::<VarUBits3F, _>(&var_ubits[3]))
        .chain(into_tests::<PktHashF, _>(hash))
        .chain(into_tests::<GroupIDF, _>(group))
        .chain(into_tests::<PubKeyF, _>(pubkey))
        .chain(into_tests::<DomainF, _>(domain))
        .chain(into_tests::<CreateF, _>(create))
        .chain(into_tests::<PointSizeF, _>(pkt_size))
        .chain(into_tests::<DataSizeF, _>(data_size))
        .chain(into_tests::<LinksLenF, _>(links_len))
        .chain(into_tests::<PathLenF, _>(path_len))
        //.chain( into_tests::<LinksLenF,_>(&links))
        .chain(is_some(path_prefix.clone()).map(|v| {
            (
                Box::new(SPathPrefix(v.spath().to_owned())) as Box<dyn PktStreamTest>,
                RuleType::PrefixPath,
            )
        }));
    (it, *recv_stamp)
}

pub struct NetPktPredicate<F, T, O>(T, PhantomData<(F, O)>);
impl<A, B, C> Debug for NetPktPredicate<A, B, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NetPktPredicate").finish()
    }
}
impl<O, F, T> PktStreamTest for NetPktPredicate<F, T, O>
where
    T: TestVal<O>,
    F: SFieldVal<O> + NamedField,
{
    fn test(&self, pkt: &NetPktPtr) -> bool {
        let val = F::get_val(pkt);
        self.0.test(&val)
    }
    fn get_field(&self) -> RuleType {
        F::ENUM.into()
    }
    fn as_rules(&self) -> Box<dyn Iterator<Item = Predicate> + '_> {
        todo!()
    }
}

pub fn into_tests<F, V>(
    test_set: &TestSet<V>,
) -> impl Iterator<Item = (Box<dyn PktStreamTest>, RuleType)>
where
    V: UInt,
    F: SFieldVal<V> + NamedField,
{
    compile_stest::<F, V>(test_set)
        .map(|b| (b, F::ENUM.into()))
        .into_iter()
}
pub fn compile_stest<F, V>(tests: &TestSet<V>) -> Option<Box<dyn PktStreamTest>>
where
    V: UInt,
    F: SFieldVal<V> + NamedField,
{
    if tests == &TestSet::DEFAULT {
        return None;
    }
    let (eq, l, g, m0, m1) = tests.unpack();
    if let Some(eq) = eq {
        return Some(Box::new(NetPktPredicate(eq, PhantomData::<(F, V)>)));
    }
    macro_rules! to_test_type {
        ( $current:expr, [ $val:expr  ]) => {
            match $val {
                None => return Some(Box::new(NetPktPredicate($current,PhantomData::<(F,V)>))),
                Some(v) => return Some(Box::new(NetPktPredicate(($current,v),PhantomData::<(F,V)>))),
            }
        };
        ( $current:expr, [  $val:expr , $($v:expr),*]) => {
            match $val {
                None => to_test_type!( $current , [$($v),*] ) ,
                Some(v) => to_test_type!( ($current,v) , [$($v),*] ),
            }
        };
    }
    to_test_type!((), [l, g, m0, m1])
}
