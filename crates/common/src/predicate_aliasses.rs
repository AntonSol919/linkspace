// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::path::PathBuf;

use abe::{convert::AnyABE, TypedABE};
use anyhow::Context;
use clap::Args;
use linkspace_core::{
    predicate::exprs::{PredicateExpr, QScope},
    prelude::*,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Args)]
#[group(skip)]
pub struct WithFiles<R: clap::FromArgMatches + clap::Args> {
    #[clap(long)]
    pub file: Vec<PathBuf>,
    #[clap(flatten)]
    pub opts: R,
}

impl<R: clap::FromArgMatches + clap::Args> WithFiles<R> {
    pub fn lines(&self) -> anyhow::Result<impl Iterator<Item = anyhow::Result<AnyABE>> + '_> {
        let files = self
            .file
            .iter()
            .map(|v| {
                std::fs::read_to_string(v)
                    .with_context(|| v.to_string_lossy().to_string())
                    .map(|a| (v, a))
            })
            .try_collect::<Vec<_>>()?;
        Ok(files.into_iter().flat_map(|(path, data)| {
            data.lines()
                .enumerate()
                .map(|(i, line)| {
                    line.parse()
                        .with_context(|| format!("{}:{}  {}", path.to_string_lossy(), i, line))
                })
                .collect::<Vec<_>>()
                .into_iter()
        }))
    }
}

// Serde is impl'd for python/wasm FFI.
#[derive(Debug, Clone, Serialize, Deserialize, Default, Args)]
#[serde(default)]
#[group(skip)]
pub struct ExtViewCLIOpts {
    #[clap(flatten)]
    pub aliasses: PredicateAliasses,
    #[clap(last = true)]
    pub exprs: Vec<AnyABE>,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}
#[derive(Debug, Clone, Serialize, Deserialize, Default, Args)]
#[serde(default)]
/// aliasses for a set of common predicates
pub struct PredicateAliasses {
    /// only match locally indexed pkts           | i_new:=:{u32:0}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long, alias = "no-new")]
    pub index: bool,
    /// only match new unindexed pkts             | i_index:=:{u32:0}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long, alias = "no-index")]
    pub new: bool,

    /// match upto max packets.                   | i:<:{u32:max}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long)]
    pub max: Option<u32>,

    /// match upto max per (dm,grp,path,key) pkts | i:<:{u32:max_branch}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long)]
    pub max_branch: Option<u32>,
    /// match upto max from local index           | i_index:<:{u32:max_index}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long)]
    pub max_index: Option<u32>,
    #[serde(skip_serializing_if = "is_default")]
    /// match upto max unindexed pkts             | i_new:<:{u32:max_new}
    #[clap(long)]
    pub max_new: Option<u32>,

    /// match only signed pkts                    | pubkey:>:{@:none}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long, conflicts_with = "unsigned")]
    pub signed: bool,

    /// match only unsigned pkts                  | pubkey:=:{@:none}
    #[serde(skip_serializing_if = "is_default")]
    #[clap(long)]
    pub unsigned: bool,
    /*
    // links_len , data_len
     #[serde(skip_serializing_if ="Option::is_some")]
     #[clap(long)]
     pub pubkey: Option<PubKeyExpr>,
     #[serde(skip_serializing_if ="Option::is_some")]
     #[clap(long)]
     pub recv_before: Option<StampExpr>,
     #[serde(skip_serializing_if ="Option::is_some")]
     #[clap(long)]
     pub recv_after: Option<StampExpr>,

     #[serde(skip_serializing_if ="Option::is_none")]
     #[clap(long,alias="after")]
     pub create_after: Option<StampExpr>,
     #[serde(skip_serializing_if ="Option::is_none")]
     #[clap(long,alias="before")]
     pub create_before: Option<StampExpr>,

     #[serde(skip_serializing_if ="Option::is_none")]
     #[clap(long)]
     pub create_after_eq_int: Option<u64>,
     #[serde(skip_serializing_if ="Option::is_none")]
     #[clap(long)]
     pub create_before_eq_int: Option<u64>,

     #[serde(skip_serializing_if ="Option::is_some")]
     #[clap(long)]
     pub recv_before_int: Option<u64>,
     #[serde(skip_serializing_if ="Option::is_some")]
     #[clap(long)]
     pub recv_after_int: Option<u64>,
     */
}

impl PredicateAliasses {
    pub fn as_predicates(&self) -> impl Iterator<Item = PredicateExpr> {
        let PredicateAliasses {
            max,
            signed,
            unsigned,
            index,
            new,
            max_branch,
            max_index,
            max_new,
        } = self;
        let signed = signed
            .then(||
                  abev!((PktTypeF::NAME) : "1" : +(U8::new(PktTypeFlags::SIGNATURE.bits()).abe_bits()))
            );

        let unsigned = unsigned
            .then(||
                  abev!( (PktTypeF::NAME) : "0" :+(U8::new(!PktTypeFlags::SIGNATURE.bits()).abe_bits()))
            );
        let max = max.map(|i| abev!( (QScope::Query.to_string()) : "<" : +(U32::from(i).to_abe())));
        let max_new =
            max_new.map(|i| abev!( (QScope::New.to_string()) : "<" : +(U32::from(i).to_abe())));
        let max_log =
            max_index.map(|i| abev!( (QScope::Index.to_string()) : "<" : +(U32::from(i).to_abe())));
        let max_branch = max_branch
            .map(|i| abev!( (QScope::Branch.to_string()) : "<" : +(U32::from(i).to_abe())));

        /*
        let pubkey = pubkey.as_ref().map(|p| abev!( (PubKeyF::NAME) : "=" : +(p.iter().cloned())));

        let create_before = create_before.as_ref().map(|p| abev!( (CreateF::NAME) : "<" : +(p.iter().cloned())));
        let create_after= create_after.as_ref().map(|p| abev!( (CreateF::NAME) : ">" : +(p.iter().cloned())));
        let recv_before= recv_before.as_ref().map(|p| abev!((RuleType::RecvStamp.to_string()) : "<" : +(p.iter().cloned())));
        let recv_after = recv_after.as_ref().map(|p| abev!( (RuleType::RecvStamp.to_string()) : ">" : +(p.iter().cloned())));

        let create_before_int = create_before_eq_int
            .and_then(|v| v.checked_add(1))
            .map(|p| abev!( (CreateF::NAME) : "<" : +(Stamp::new(p).to_abe())));
        let create_after_int = create_after_eq_int
            .and_then(|v| v.checked_sub(1))
            .map(|p| abev!( (CreateF::NAME) : ">" : +(Stamp::new(p).to_abe())));
        let recv_before_int = recv_before_int.as_ref().map(|p| abev!((RuleType::RecvStamp.to_string()) : "<" : +(Stamp::new(*p).to_abe())));
        let recv_after_int = recv_after_int.as_ref().map(|p| abev!( (RuleType::RecvStamp.to_string()) : ">" : +(Stamp::new(*p).to_abe())));
        */

        let new = new.then(|| abev!( (QScope::Index.to_string()) : "<" : +(U32::ZERO.to_abe())));
        let log = index.then(|| abev!( (QScope::New.to_string()) : "<" : +(U32::ZERO.to_abe())));
        signed
            .into_iter()
            .chain(unsigned)
            .chain(max)
            .chain(max_new)
            .chain(max_log)
            .chain(max_branch)
            /*
            .chain(pubkey)
            .chain(create_before)
            .chain(create_after)
            .chain(recv_before)
            .chain(recv_after)
            .chain(create_before_int)
            .chain(create_after_int)
            .chain(recv_before_int)
            .chain(recv_after_int)
            */
            .chain(log)
            .chain(new)
            .map(TypedABE::from_unchecked)
    }
}
