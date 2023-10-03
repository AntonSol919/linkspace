// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use abe::convert::AnyABE;
use clap::Args;
use linkspace_core::{
    predicate::exprs::QScope,
    prelude::{predicate_type::PredicateType, *},
};

#[derive(Debug, Clone, Default, Args)]
#[group(skip)]
pub struct ExtWatchCLIOpts {
    #[command(flatten)]
    pub aliases: PredicateAliases,
    #[arg(last = true)]
    pub exprs: Vec<AnyABE>,
}

#[derive(Debug, Clone, Default, Args)]
/// aliases for a set of common predicates
pub struct PredicateAliases {
    /// only match locally indexed pkts           | `i_new:=:[u32:0]`
    #[arg(long, alias = "no-new")]
    pub db_only: bool,
    /// only match new unindexed pkts             | `i_db:=:[u32:0]`
    #[arg(long, alias = "no-index")]
    pub new_only: bool,

    /// match upto max packets.                   | `i:<:[u32:max]`
    #[arg(long)]
    pub max: Option<u32>,

    /// match upto max per (dm,grp,space,key) key | `i_branch:<:[u32:max_branch]`
    #[arg(long)]
    pub max_branch: Option<u32>,
    /// match upto max from local index           | `i_db:<:[u32:max_index]`
    #[arg(long)]
    pub max_index: Option<u32>,
    /// match upto max unindexed pkts             | `i_new:<:[u32:max_new]`
    #[arg(long)]
    pub max_new: Option<u32>,

    /// match only signed pkts                    | `pubkey:>:[@:none]`
    #[arg(long, conflicts_with = "unsigned")]
    pub signed: bool,

    /// match only unsigned pkts                  | `pubkey:=:[@:none]`
    #[arg(long)]
    pub unsigned: bool,

    #[arg(long)]
    /// Add :qid option (generates qid)
    pub watch: bool,
    #[arg(long)]
    /// set :qid option (implies --watch)
    pub qid: Option<AnyABE>,
    #[arg(long)]
    /// Add :follow option
    pub follow: bool,

    #[arg(long)]
    /// add `recv:<:[us:INIT:+{until}]` where INIT is set at start
    pub until: Option<String>,
}

impl PredicateAliases {
    // TODO use PredicateType instead of RuleType
    pub fn as_predicates(self) -> impl Iterator<Item = Vec<ABE>> {
        let PredicateAliases {
            max,
            signed,
            unsigned,
            db_only,
            new_only,
            max_branch,
            max_index,
            max_new,
            watch,
            qid,
            follow,
            until,
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

        let new =
            new_only.then(|| abev!( (QScope::Index.to_string()) : "<" : +(U32::ZERO.to_abe())));
        let log = db_only.then(|| abev!( (QScope::New.to_string()) : "<" : +(U32::ZERO.to_abe())));

        let watch = qid
            .map(|v| v.unwrap())
            .or(watch.then(|| abev!("default")))
            .map(|v| abev!( : (KnownOptions::Qid.to_string()) : +(v)));

        let now = now().0.to_vec();
        let ttl = until
            .map(|v| abev!( (PredicateType::Recv.to_string()) : "<" :  { : now / "us" : "+" v}));
        let follow = follow.then(|| abev!(: (KnownOptions::Follow.to_string())));

        watch
            .into_iter()
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
