// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use abe::{convert::AnyABE };
use clap::Args;
use linkspace_core::{
    predicate::exprs::{ QScope},
    prelude::{*, predicate_type::PredicateType},
};




#[derive(Debug, Clone,   Default, Args)]
#[group(skip)]
pub struct ExtWatchCLIOpts {
    #[clap(flatten)]
    pub aliases: PredicateAliases,
    #[clap(last = true)]
    pub exprs: Vec<AnyABE>,
}

#[derive(Debug, Clone, Default, Args)]
/// aliases for a set of common predicates
pub struct PredicateAliases {
    /// only match locally indexed pkts           | i_new:=:{u32:0}
    #[clap(long, alias = "no-new")]
    pub index: bool,
    /// only match new unindexed pkts             | i_index:=:{u32:0}
    #[clap(long, alias = "no-index")]
    pub new: bool,

    /// match upto max packets.                   | i:<:{u32:max}
    #[clap(long)]
    pub max: Option<u32>,

    /// match upto max per (dm,grp,path,key) pkts | i:<:{u32:max_branch}
    #[clap(long)]
    pub max_branch: Option<u32>,
    /// match upto max from local index           | i_index:<:{u32:max_index}
    #[clap(long)]
    pub max_index: Option<u32>,
    /// match upto max unindexed pkts             | i_new:<:{u32:max_new}
    #[clap(long)]
    pub max_new: Option<u32>,

    /// match only signed pkts                    | pubkey:>:{@:none}
    #[clap(long, conflicts_with = "unsigned")]
    pub signed: bool,

    /// match only unsigned pkts                  | pubkey:=:{@:none}
    #[clap(long)]
    pub unsigned: bool,

    #[clap(long)]
    /// Add :id option (generates id)
    pub watch: bool,
    #[clap(long)]
    /// set :id option id (implies --watch)
    pub id: Option<AnyABE>,
    #[clap(long)]
    /// Add :follow option
    pub follow: bool,

    #[clap(long)]
    /// add recv:<:{now:+..} 
    pub ttl : Option<String>
}

impl PredicateAliases {
    // TODO use PredicateType instead of RuleType
    pub fn as_predicates(self) -> impl Iterator<Item = Vec<ABE>> {
        let PredicateAliases {
            max,
            signed,
            unsigned,
            index,
            new,
            max_branch,
            max_index,
            max_new,
            watch,
            id,
            follow,
            ttl,
        } = self;
        let signed = signed
            .then(||
                  abev!((PktTypeF::NAME) : "1" : +(U8::new(PointTypeFlags::SIGNATURE.bits()).abe_bits()))
            );

        let unsigned = unsigned
            .then(||
                  abev!( (PktTypeF::NAME) : "0" :+(U8::new(!PointTypeFlags::SIGNATURE.bits()).abe_bits()))
            );
        let max = max.map(|i| abev!( (QScope::Query.to_string()) : "<" : +(U32::from(i).to_abe())));
        let max_new =
            max_new.map(|i| abev!( (QScope::New.to_string()) : "<" : +(U32::from(i).to_abe())));
        let max_log =
            max_index.map(|i| abev!( (QScope::Index.to_string()) : "<" : +(U32::from(i).to_abe())));
        let max_branch = max_branch
            .map(|i| abev!( (QScope::Branch.to_string()) : "<" : +(U32::from(i).to_abe())));


        let new = new.then(|| abev!( (QScope::Index.to_string()) : "<" : +(U32::ZERO.to_abe())));
        let log = index.then(|| abev!( (QScope::New.to_string()) : "<" : +(U32::ZERO.to_abe())));

        let watch = id.map(|v| v.unwrap()).or(watch.then(|| abev!("default")))
            .map(|v| abev!( : (KnownOptions::Id.to_string()) : +(v)).into());

        let ttl = ttl.map(|v| abev!( (PredicateType::Recv.to_string()) : "<" : { "now" : "+" v}).into());

        let follow = follow.then(|| abev!(: (KnownOptions::Follow.to_string())));

        watch.into_iter()
            .chain(ttl)
            .chain(follow)
            .chain(signed)
            .chain(unsigned)
            .chain(max)
            .chain(max_new)
            .chain(max_log)
            .chain(max_branch)
            .chain(log)
            .chain(new)

    }
}
