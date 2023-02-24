// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/* this stuff requires an overhaul. It should accept path_components. parts should be boxed for sized, and it can be made Copy*/

use linkspace_pkt::{abe::eval::ABList, *};

use crate::{
    predicate::{bitset_test::BitTestSet, exprs::RuleType},
    prelude::ExtPredicate,
};
use anyhow::{ensure, Context};

use super::{
    exprs::{Predicate, QScope},
    treekey::TreeKeys,
    value_test::*,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PktPredicates {
    pub pkt_types: TestSet<u8>,

    pub var_flags: TestSet<u8>,
    pub var_hop: TestSet<u32>,
    pub var_until: TestSet<u64>,
    pub var_ubits: [TestSet<u32>; 4],

    pub domain: TestSet<u128>,
    pub group: TestSet<U256>,
    pub pubkey: TestSet<U256>,
    pub hash: TestSet<U256>,
    pub pkt_size: TestSet<u16>,

    pub data_size: TestSet<u16>,
    pub links_len: TestSet<u16>,
    pub path_len: TestSet<u8>,
    pub create: TestSet<u64>,

    pub recv_stamp: Bound<u64>,

    pub state: StatePredicates,

    pub path_prefix: IPathBuf,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct StatePredicates {
    pub i_branch: TestSet<u32>,
    pub i_index: TestSet<u32>,
    pub i_new: TestSet<u32>,
    pub i_query: TestSet<u32>,
}
impl StatePredicates {
    pub fn check_db(&self) -> bool {
        self.i_branch.has_any() && self.i_index.has_any()
    }

    pub fn idx(&mut self, i: QScope) -> &mut TestSet<u32> {
        match i {
            QScope::Branch => &mut self.i_branch,
            QScope::Index => &mut self.i_index,
            QScope::New => &mut self.i_new,
            QScope::Query => &mut self.i_query,
        }
    }
    pub fn is_valid(&self) -> anyhow::Result<()> {
        if !self.i_query.has_any() {
            anyhow::bail!("maximum number of results is 0")
        }
        if !self.i_new.has_any() && (!self.i_index.has_any() || !self.i_branch.has_any()) {
            anyhow::bail!("both new and log have no accept conditions");
        }
        Ok(())
    }
}

impl Default for PktPredicates {
    fn default() -> Self {
        PktPredicates::DEFAULT
    }
}
impl PktPredicates {
    pub const DEFAULT: PktPredicates = PktPredicates {
        pkt_types: TestSet::DEFAULT,
        path_prefix: IPathBuf::DEFAULT,
        var_flags: TestSet::DEFAULT,
        var_hop: TestSet::DEFAULT,
        var_until: TestSet::DEFAULT,
        var_ubits: [TestSet::DEFAULT; 4],
        domain: TestSet::DEFAULT,
        group: TestSet::DEFAULT,
        pubkey: TestSet::DEFAULT,
        hash: TestSet::DEFAULT,
        pkt_size: TestSet::DEFAULT,
        data_size: TestSet::DEFAULT,
        links_len: TestSet::DEFAULT,
        path_len: TestSet::DEFAULT,
        create: TestSet::DEFAULT,
        recv_stamp: Bound::DEFAULT,
        state: StatePredicates {
            i_branch: TestSet::DEFAULT,
            i_index: TestSet::DEFAULT,
            i_new: TestSet::DEFAULT,
            i_query: TestSet::DEFAULT,
        },
    };
}
impl std::fmt::Display for PktPredicates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for p in self.iter() {
            writeln!(f, "{p}")?;
        }
        Ok(())
    }
}

fn as_rules_it2<X, Y: Into<ABList>>(
    kind: impl Into<RuleType>,
    it: impl IntoIterator<Item = (TestOp, X)>,
    map: impl Fn(X) -> Y,
) -> impl Iterator<Item = Predicate> {
    let kind = kind.into();
    it.into_iter().map(move |(op, val)| Predicate {
        kind,
        op,
        val: map(val).into(),
    })
}
impl PktPredicates {
    fn path_check(&mut self) -> anyhow::Result<()> {
        if let Some(i) = self.path_prefix.path_len().checked_sub(1) {
            self.path_len.try_add(TestOp::Greater, i)?;
        }
        ensure!(
            self.path_len
                .info(*self.path_prefix.path_len())
                .val
                .is_some(),
            "path '{}' incompatible with path_len predicates '{:?}'",
            self.path_prefix,
            self.path_len
        );
        Ok(())
    }
    pub fn to_str(&self, canonical: bool) -> String {
        self.iter()
            .map(|p| p.to_str(canonical))
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn iter(&self) -> impl Iterator<Item = Predicate> + '_ {
        let PktPredicates {
            pkt_types,
            domain,
            group,
            pubkey,
            hash,
            pkt_size,
            path_prefix,
            path_len,
            recv_stamp,
            create,
            state,
            data_size,
            links_len,
            var_flags,
            var_hop,
            var_until,
            var_ubits,
        } = self;
        use FieldEnum::*;
        let mut c = *state;
        let limits = crate::predicate::exprs::QSCOPES
            .into_iter()
            .flat_map(move |i| as_rules_it2(i, c.idx(i).rules(), U32::new));
        fn id<I>(i: I) -> I {
            i
        }
        as_rules_it2(PktTypeF, pkt_types.rules(), U8::new)
            .chain(as_rules_it2(VarNetFlagsF, var_flags.rules(), U8::new))
            .chain(as_rules_it2(VarHopF, var_hop.rules(), U32::new))
            .chain(as_rules_it2(VarStampF, var_until.rules(), Stamp::new))
            .chain(as_rules_it2(VarUBits0F, var_ubits[0].rules(), U32::new))
            .chain(as_rules_it2(VarUBits1F, var_ubits[1].rules(), U32::new))
            .chain(as_rules_it2(VarUBits2F, var_ubits[2].rules(), U32::new))
            .chain(as_rules_it2(VarUBits3F, var_ubits[3].rules(), U32::new))
            .chain(as_rules_it2(DomainF, domain.map(Domain::from).rules(), id))
            .chain(as_rules_it2(
                GroupIDF,
                group.map(|v| -> GroupID { v.into() }).rules(),
                id,
            ))
            .chain(as_rules_it2(
                PubKeyF,
                pubkey.map(|v| -> PubKey { v.into() }).rules(),
                id,
            ))
            .chain((!path_prefix.is_empty()).then(|| {
                Predicate::from(RuleType::PrefixPath, TestOp::Equal, path_prefix.ablist())
            }))
            .chain(as_rules_it2(
                PktHashF,
                hash.map(|v| -> LkHash { v.into() }).rules(),
                id,
            ))
            .chain(as_rules_it2(PointSizeF, pkt_size.rules(), U16::new))
            .chain(as_rules_it2(DataSizeF, data_size.rules(), U16::new))
            .chain(as_rules_it2(LinksLenF, links_len.rules(), U16::new))
            .chain(as_rules_it2(PathLenF, path_len.rules(), U8::new))
            .chain(as_rules_it2(
                RuleType::RecvStamp,
                recv_stamp.iter(),
                Stamp::new,
            ))
            .chain(as_rules_it2(CreateF, create.rules(), Stamp::new))
            .chain(limits)
    }

    /// warn - becomes invalid on state on error
    pub fn add_ext_predicate(&mut self, predicate: ExtPredicate) -> anyhow::Result<()> {
        for p in predicate.try_iter()? {
            self.add_predicate(&p)?;
        }
        Ok(())
    }
    /// warn - becomes invalid on state on error
    pub fn add_predicate(&mut self, pred: &Predicate) -> anyhow::Result<()> {
        self.and(pred)
            .with_context(|| pred.clone())
            .with_context(|| format!("Error adding rule '{}'", pred.kind))?;
        Ok(())
    }
    // is in invalid state on error
    fn and(&mut self, rule: &Predicate) -> anyhow::Result<()> {
        tracing::debug!(%rule,"new rule");
        tracing::trace!(%self);
        let Predicate { kind, val, op } = rule;
        let val = val.clone();
        let op = *op;
        match kind {
            RuleType::Field(f) => {
                self.pkt_types
                    .try_add(TestOp::Mask1, f.info().pkts.bits())
                    .with_context(|| format!("incompatible pkt typs:{rule:?}" ))?;
                match f {
                    FieldEnum::PktTypeF => self.pkt_types.try_add(op, U8::try_from(val)?.0)?,
                    FieldEnum::PointSizeF => {
                        self.pkt_size.try_add(op, U16::try_from(val)?.get())?
                    }
                    FieldEnum::PktHashF => {
                        self.hash.try_add(op, LkHash::try_from(val)?.into())?;
                        if op == TestOp::Equal {
                            self.state.i_query.try_add(TestOp::Equal, 0u32.into())?;
                        }
                    },
                    FieldEnum::DomainF => self
                        .domain
                        .try_add(op, Domain::try_from(val)?.uint().get())?,
                    FieldEnum::PathF => {
                        ensure!(op == TestOp::Equal, "path only supports equallity");
                        let path: IPathBuf = SPathBuf::try_from(val)?.try_idx()?;
                        self.path_len.try_add(TestOp::Equal, path.len() as u8)?;
                        ensure!(
                            path.starts_with(&self.path_prefix),
                            "prefix conflicting with path"
                        );
                        self.path_prefix = path;
                    }
                    FieldEnum::PathLenF => {
                        self.path_len.try_add(op, U8::try_from(val)?.0)?;
                        self.path_check()?;
                    }
                    FieldEnum::GroupIDF => {
                        self.group.try_add(op, GroupID::try_from(val)?.into())?
                    }
                    FieldEnum::CreateF => self.create.try_add(op, U64::try_from(val)?.get())?,
                    FieldEnum::PubKeyF => {
                        self.pubkey.try_add(op, PubKey::try_from(val)?.into())?;
                    }
                    FieldEnum::SignatureF => todo!(),
                    FieldEnum::DataF => todo!(),
                    FieldEnum::VarNetFlagsF => self.var_flags.try_add(op, U8::try_from(val)?.0)?,
                    FieldEnum::VarHopF => self.var_hop.try_add(op, U32::try_from(val)?.get())?,
                    FieldEnum::VarStampF => {
                        self.var_until.try_add(op, U64::try_from(val)?.get())?
                    }
                    FieldEnum::VarUBits0F => {
                        self.var_ubits[0].try_add(op, U32::try_from(val)?.get())?
                    }
                    FieldEnum::VarUBits1F => {
                        self.var_ubits[1].try_add(op, U32::try_from(val)?.get())?
                    }
                    FieldEnum::VarUBits2F => {
                        self.var_ubits[2].try_add(op, U32::try_from(val)?.get())?
                    }
                    FieldEnum::VarUBits3F => {
                        self.var_ubits[3].try_add(op, U32::try_from(val)?.get())?
                    }
                    FieldEnum::PathComp0F => todo!(),
                    FieldEnum::PathComp1F => todo!(),
                    FieldEnum::PathComp2F => todo!(),
                    FieldEnum::PathComp3F => todo!(),
                    FieldEnum::PathComp4F => todo!(),
                    FieldEnum::PathComp5F => todo!(),
                    FieldEnum::PathComp6F => todo!(),
                    FieldEnum::PathComp7F => todo!(),
                    FieldEnum::DataSizeF => {
                        self.data_size.try_add(op, U16::try_from(val)?.get())?
                    }
                    FieldEnum::LinksLenF => {
                        self.links_len.try_add(op, U16::try_from(val)?.get())?
                    }
                }
            }
            RuleType::RecvStamp => self.recv_stamp.try_add(op, U64::try_from(val)?.get())?,
            RuleType::PrefixPath => {
                ensure!(op == TestOp::Equal, "prefix only supports equallity ");
                let sp = SPathBuf::try_from(val)?;
                if self.path_prefix.starts_with(&sp) {
                    tracing::trace!(old=%self.path_prefix,new=%sp,"Current prefix already more specific")
                } else if sp.starts_with(&self.path_prefix) {
                    let sp = sp.try_idx()?;
                    tracing::trace!(old=%self.path_prefix,new=%sp,new_len=sp.path_len(),"Setting more specific prefix");
                    self.path_prefix = sp;
                    self.path_check()?;
                } else {
                    anyhow::bail!("disjoin spath {:?} <> {:?}", &*sp, &*self.path_prefix);
                };
            }
            RuleType::Limit(l) => {
                self.state.idx(*l).add(op, U32::try_from(val)?.get());
                self.state.is_valid()?;
            }
        };
        Ok(())
    }

    pub fn compile_tree_keys(&self, cstamp_old_first: bool) -> anyhow::Result<TreeKeys> {
        let cstamp = self.create.bound.stamp_range(cstamp_old_first);
        Ok(TreeKeys {
            domain: self.domain,
            group: self.group,
            ipath: self.path_prefix.clone(),
            depth: BitTestSet::from_rules(&self.path_len),
            cstamp,
            pubkey: self.pubkey,
        })
    }
}
