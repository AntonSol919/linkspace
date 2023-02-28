// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/// WARN - This is a work in progress. Only [local] is currently implemented.
/// Will impl's ABE scope/eval for [#:hello:world], [#@:world:hello], and [/#/hello/world]
/// And reverse lookup for [group/#?] and [[#:hello:world]/#?]
pub mod local;
/*
LNS: lovely name system
LNS is a semi moderated, weighted vote based, public bindings for key<->values.
every authority can create and publish.
Each super authority can overwrite a subauthority

As such:
Bindings are either: proposed, accepted-unfixed,accepted-fixed, accepted-superceded.
Once a proposal is voted it becomes accapeted-unfixed.
If the entire chain of authorities publish sign their log a binding becomes fixed for a specific time range.
If one of them is superceded a binding becomes accepted-superceded

--propose : e.g. lns:[#:pub]:/hello/world

parent : PROPOSAL_PTR - contains the lnsauth@ keys for /hello , the keys required to vote this proposal into effect
XXXXX# : Value evaled with [#:world:hello] ( [#:...] returns the first link pointer with a tag ending in '#' )
XXXXX@ : Value evaled with [@:world:hello] ( [@:...] returns the first link pointer with a tag ending in '@' )
///// XXXXX> : Value evaled with [>:hello:world]
XXXXXXXX_lnsaut@ : PUBKEY  Can authority the bitset XXXXXXXX
[ XXXXXXXX_lnsaut@ : PUBKEY  ] *
-- data
start:STAMP
stop:STAMP

#vote : (Signed by the lnsauth@ of the parent to the proposal)
proposal   : PROPOSAL_PTR
bind       : BIND_PTR that binds the signing key as an lnsauth@ for the parent
prevvote   : a backpointer to the previous vote from this key for this bind_ptr
[ XXXXXXXX_hasvote : VOTE_PTR ] * : other votes for the same proposal.

#bind:
proposal : PROPOSAL_PTR
XXXXXXXX_accept1 : VOTE_PTR
XXXXXXXX_accept1 : VOTE_PTR
XXXXXXXX_accept3 : VOTE_PTR
?overrules : BIND_PTR


Evaluation & interpretation:
A proposal, if enacted, associates a set of links a given lns name between two dates.
That is, a lns group name such [#:hello:com] can get a publicly acknowledged set of links.
The set of links can have arbitrary tags, but some are special.

The first link with a tag ending in '#' is the default value for [#:world:hello]. This is a groupid by convention
The first link with a tag ending in '@' is the default value for [@:world:hello]. This is a public key by convention
One or more link tags  XXXXXXXX_lnsaut@  refer to the public keys that have voting authority for subnames.
The bit OR of all XXXXXXXX must be [a::8:\xff] ( all ones ).

packet's arent acknowledged if their their create stamp is in the future.
All pointers must point to packets with their create stamp in the past of their create stamp.

Parent lnsauth can vote for child proposal's.

Once enacted and bound, a proposal can not be retracted. It will always remain valid between start and stop.
However, there is the concept of primary binding.
voted for by the most top most authorities.
witht the most number of votes.
with the earliest final vote.


TODO:
Can a claim extend beyond its parent?
reverse lookup


The LNS evaluator consists of 2 systems.
- The LNS resolver daemon is a standalone program.
It watches for requests, verifies, and links LNS claims into the [#:0] group.

- The LNS Scope tries to resolve [#:hello:com].

The sequence of events is:
LNS Scope tries to resolve [#:hello:com] for the first time.

It checks if there is a 'alive' linkpoint lns:[#:0]:/#/com/hello, if there is this claim is used.
//By relinking [#:pub] claims into [#:0] when validate, the sccope can skip the 'expensive' validation step.

If it does not exists or is no longer alive 'now' it creates linkpoint lns:[#:0]:/find/com/hello.
By default it will wait for 1 second to see if a value is returned. This can be disabled.

The resolver daemon is watching for /find:** linkpoints.
For each it will attempt to request, veriy, and link validated live claims.
*/

#[derive(Copy, Clone, PartialEq, Eq,Debug)]
#[repr(u8)]
pub enum TagSuffix {
    Group = b'#',
    Pubkey = b'@',
}

impl TagSuffix {
    pub fn bslice(self) -> &'static [u8] {
        match self {
            TagSuffix::Group => &[b'#'],
            TagSuffix::Pubkey => &[b'@'],
        }
    }
    pub const ALL: [TagSuffix; 2] = [TagSuffix::Group, TagSuffix::Pubkey];
    pub fn select(self, links: &[Link]) -> impl Iterator<Item = &Link> {
        links.iter().filter(move |v| v.tag[15] == self as u8)
    }
}

use std::{fmt::Debug, time::Duration};

use linkspace_core::prelude::*;

use crate::runtime::Linkspace;

use self::local::LocalLNS;
pub const LNS: Domain = ab(b"lns");
spath!(pub const LOCAL_CLAIM_PREFIX = [b"#"]);

#[derive(Debug, Clone, Copy)]
pub struct LNS<R> {
    pub rt: R,
    pub timeout: Duration,
}

pub fn get_claim<'o>(
    _rt: Linkspace,
    _timeout: Duration,
    _path: &SPath,
) -> anyhow::Result<Option<NetPktBox>> {
    Ok(None)
}
pub fn reverse_lookup(i: &[&[u8]], _mode: Option<TagSuffix>) -> Result<Vec<u8>, ApplyErr> {
    let hash: B64<[u8; 32]> = B64::try_fit_slice(i[0])?;
    Ok(hash.to_abe_str().into_bytes())
}

impl<R: Fn() -> anyhow::Result<Linkspace>> LNS<R> {

    fn tag_ptr(&self, args: &[&[u8]], tag: TagSuffix) -> Result<Vec<u8>, ApplyErr> {
        tracing::trace!(args=?ab_slice(args),?tag,"LNS");
        match args.last().copied(){
            Some(b"local") => LocalLNS{rt:&self.rt}.local_tag_ptr(args.split_last().unwrap().1, tag),
            _ => Err("name not found".into())
        }
    }
    fn lookup_claim(&self, kind: TagSuffix, bytes: &[u8]) -> ApplyResult {

        // fallback to local
        LocalLNS{rt:&self.rt}.lookup_claim(kind,bytes)
    }
}

impl<R: Fn() -> anyhow::Result<Linkspace>> EvalScopeImpl for LNS<R> {
    fn about(&self) -> (String, String) {
        ("lns".into(), "".into())
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        fncs!([
            (
                "#",
                1..=7,
                Some(true),
                "(namecomp)* - get the associated lns group name",
                |this: &Self, args: &[&[u8]]| this.tag_ptr(args, TagSuffix::Group),
                |this: &Self, phash: &[u8], _| this.lookup_claim(TagSuffix::Group, phash)
            ),
            (
                "@",
                1..=7,
                Some(true),
                "(namecomp)* - get the associated lns group name",
                |this: &Self, args: &[&[u8]]| this.tag_ptr(args, TagSuffix::Pubkey),
                |this: &Self, phash: &[u8], _| this.lookup_claim(TagSuffix::Pubkey, phash)
            ),
            ("lns", 1..=1, "rev lookup", |_, i: &[&[u8]]| reverse_lookup(
                i, None
            )),
            ("?@", 1..=1, "rev lookup", |_, i: &[&[u8]]| reverse_lookup(
                i,
                Some(TagSuffix::Pubkey)
            )),
            ("?#", 1..=1, "rev lookup", |_, i: &[&[u8]]| reverse_lookup(
                i,
                Some(TagSuffix::Group)
            ))
        ])
    }

    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[]
        /*
                if id != b"+" { return ApplyResult::None;}
                let brk = abe.iter().position(|v| v.is_colon());
            // FIXME: #[thread_local] static
                let default = linkspace_core::eval::abev!( { "pkt" });
                let (path,expr) = if let Some(i) = brk {
                (&abe[..i],abe.get(i+1..).ok_or("Missing expr after ':'")?)
        }else { (abe,default.as_slice())};
                let ctx = EvalCtx{scope,reval:evals};
                tracing::trace!(?path,"eval path");
                let path = eval::eval(&ctx, path)?;
                let path = SPathBuf::try_from_ablist(path)?;
                let pkt = get_claim((self.rt)()?,self.timeout, &path)?.ok_or(format!("No local claim found for {}",path))?;
                tracing::trace!(?expr,"eval in pkt ctx");
                let ctx = pkt_ctx(ctx, &**pkt);
                let bytes = eval::eval(&ctx,expr)?;
                ApplyResult::Ok(bytes.concat())
                */
    }
}
