

use linkspace_core::prelude::query_mode::Mode;
use linkspace_pkt::reroute::RecvPkt;
use linkspace_pkt::utils::LkHashMap;
use thiserror::Error;
use tracing::instrument;
use crate::prelude::*;
use crate::protocols::lns::{LNS, VOTE_TAG};

use super::claim::{LiveClaim, ClaimData, Claim };
use super::name::Name;


pub fn root_claim() -> LiveClaim{
    let mut it = lns_root_claims();
    let claim_pkt = it.next().unwrap();
    let nsigns = claim_pkt.get_links().iter().filter(|v| v.tag.0[15] == b'^').count();
    let data = ClaimData::try_from(claim_pkt.data()).unwrap();
    let claim = Claim{pkt:RecvPkt::from_dyn(&claim_pkt),data,name:Name::root()};
    let signatures = it.take(nsigns).map(|p| RecvPkt::from_dyn(&p)).collect();
    LiveClaim { claim, signatures, parent: None }
}

pub fn lns_root_claims() -> impl Iterator<Item=NetPktBox>{
    crate::pkt_reader::NetPktDecoder::new(linkspace_core::LNS_ROOTS).map(|v| v.unwrap().as_netbox())
}

pub type IssueHandler<'o> = &'o mut dyn FnMut(Issue) -> anyhow::Result<()>;

pub type Voteing = (LkHash,(Option<Claim>,Vec<RecvPkt>));

#[derive(Error,Debug)]
pub enum Issue{
    /// Not an error - No known vote for this authority.
    #[error("we do not have the vote for this authority")]
    NoVote{auth:PubKey,sub_claim: LkHash},
    #[error("could not interpret this vote: {0:?}")]
    UnknownVoteFmt(RecvPkt),
    #[error("a claim we don't know about has been voted live: {0:?}")]
    UnknownClaimIsLive(LkHash),
    #[error("the vote {vote} is pointing to an invalid claim {claim:?} because {error:?}")]
    BadClaimInVote{claim: RecvPkt,vote: LkHash,error: anyhow::Error},
    #[error("a claim we don't know was voted {0}")]
    MissingClaim(LkHash),
    #[error("a tied between claims")]
    Tie(Vec<Voteing>)
}



#[instrument(ret,skip(reader,issue_handler))]
pub fn walk_live_claims(reader: &ReadTxn, parent:LiveClaim, name_comps: &mut &SPathIter, issue_handler:IssueHandler) -> anyhow::Result<Result<LiveClaim,LiveClaim>>{
    let sub = match name_comps.next(){
        Some(v) => v,
        None => return Ok(Ok(parent)),
    };
    let ipath = parent.claim.pkt.get_ipath().into_ipathbuf().append(sub);
    let mut predicates = Query::dgpk(LNS, *parent.claim.pkt.get_group(), ipath,B64([255;32])).predicates;
    predicates.path_len.add(crate::core::prelude::TestOp::Equal, 2);
    let mut claim_votes : LkHashMap<(Option<Claim>,Vec<RecvPkt>)>= Default::default();
    let mut _count = 0;
    let max_required_votes = (parent.claim.authorities().count()+1)/2;
    for auth in parent.claim.authorities(){
        _count = 0;
        predicates.pubkey = TestSet::new_eq(auth.into());
        match reader.query(Mode::TREE_DESC,&predicates,&mut _count)?.next(){
            Some(vote) => {
                match vote.get_links().first(){
                    Some(l) if l.tag == VOTE_TAG =>{
                        match claim_votes.entry(l.ptr){
                            std::collections::hash_map::Entry::Occupied(mut o) => {
                                o.get_mut().1.push(vote.owned());
                                if o.get().1.len() >= max_required_votes { break};
                            },
                            std::collections::hash_map::Entry::Vacant(v) =>{
                                let o_claim = reader.read(&l.ptr)?;
                                let claim = match o_claim {
                                    Some(claim_pkt) => match Claim::from(claim_pkt){
                                        Ok(o) => Some(o),
                                        Err(error) => {
                                            issue_handler(Issue::BadClaimInVote{ claim:claim_pkt.owned(), vote: l.ptr ,error})?;
                                            None
                                        },
                                    },
                                    None => {
                                        issue_handler(Issue::MissingClaim(l.ptr))?;
                                        None
                                    },
                                };
                                v.insert((claim,vec![vote.owned()]));

                            },
                        }
                    },
                    _ => {
                        issue_handler(Issue::UnknownVoteFmt(vote.owned()))?;
                    }
                }
            },
            None => issue_handler(Issue::NoVote { auth, sub_claim: parent.claim.pkt.hash() })?,
        }
    }
    let mut votes_by_claim :Vec<Voteing>= claim_votes.into_iter().filter(|(_h,(_c,votes))| !votes.is_empty()).collect();
    if votes_by_claim.is_empty() { return Ok(Err(parent));}

    //pick the one with the most votes, or first to reach the max votes, or error.
    pub type Order = (usize,u64);
    let order = |(_h,(_c,sigs)) : &(_,(_,Vec<RecvPkt>))| -> Order {
        (sigs.len(),
         // this only comes into play if both have the same number of sigs.
         // we want the max create_stamp from the vote set, to be bigger then those of the other sets.
         Stamp::MAX.get() - sigs.iter().map(|v| v.get_create_stamp()).max().unwrap().get(),
        )};

    votes_by_claim.sort_by_key(order);
    let live = votes_by_claim.pop().unwrap();
    let mut ties : Vec<_>= votes_by_claim.into_iter().take_while(|p| order(&live) == order(p)).collect();
    if !ties.is_empty(){
        ties.push(live);
        issue_handler(Issue::Tie(ties))?;
        return Ok(Err(parent))
    }

    let (claim_hash,(claim,signatures)) = live;
    match claim {
        Some(claim) => {
            let new_parent = LiveClaim { parent:Some(Box::new(parent)),claim,signatures};
            walk_live_claims(reader, new_parent, name_comps, issue_handler)
        },
        None => {
            issue_handler(Issue::UnknownClaimIsLive(claim_hash))?;
            Ok(Err(parent))
        }
    }
}

