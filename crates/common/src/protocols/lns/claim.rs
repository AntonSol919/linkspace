
use std::fmt::{Display, Debug};

use abe::{ast::{Ctr}, abconf::ABConf};
use anyhow::{ensure, Context};
use either::Either;
use crate::{prelude::*};
use linkspace_core::{prelude::{PRIVATE }, stamp_fmt::delta_stamp };
use linkspace_pkt::{NetPkt, PointExt, SelectLink,  lptr,   reroute::RecvPkt};

use crate::protocols::lns::name::Name;
use super::*;

pub struct LiveClaim {
    pub claim: Claim,
    pub signatures: Vec<RecvPkt<NetPktBox>>,
    pub parent: Option<Box<LiveClaim>>
}
impl LiveClaim{
    pub fn list(&self) -> Vec<&LiveClaim>{
        let mut p : Option<&LiveClaim>= Some(self);
        let mut vec = vec![];
        while let Some(lc) = p {
            vec.push(lc);
            p = lc.parent.as_deref();
        }
        vec
    }
}
impl Debug for LiveClaim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut lst = self.list().into_iter();
        let mut f = f.debug_struct("LiveClaim");
        if let Some(this) = lst.next(){
                f.field("claim", &this.claim)
                .field("signatures", &this.signatures.iter().map(|p|p.get_pubkey()).collect::<Vec<_>>())
                .field("parent", &self.parent);
        }
        for this in lst {
            f.field("name", &this.claim.name)
                .field("claim", &this.claim.pkt.hash_ref())
                .field("signatures", &this.signatures.iter().map(|p|p.get_pubkey()).collect::<Vec<_>>());
        }
        f.finish()
    }
}

pub struct Claim{
    pub pkt:RecvPkt<NetPktBox>,
    pub name:Name,
    pub data: ClaimData
}

impl Display for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let u = self.until();
        let d = delta_stamp(now(), u);
        writeln!(f,"{}\t{}\t{u}\t{d}",self.pkt.hash(),self.name)?;
        for Link{ptr,tag} in self.pkt.get_links(){
            writeln!(f,"{ptr}\t{tag}")?;
        }
        writeln!(f,"{}",self.data)
    }
}
impl Debug for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self,f)
    }
}

pub fn resolve_enckey(r:&ReadTxn,claim_enckey:Either<&str,LkHash>) -> anyhow::Result<(PubKey,String)>{
    match claim_enckey {
        Either::Left(s) => {
            let pubkey = linkspace_argon2_identity::pubkey(s)?;
            Ok((pubkey.into(),s.to_string()))
        },
        Either::Right(p) => resolve_enckey(r,Either::Left(r.read(&p)?.context("Missing pkt")?.get_data_str()?)),
    }
}

impl Claim {
    pub fn new(name:Name,until:Stamp,links:&[Link],misc:Vec<ABList>) -> anyhow::Result<Self>{
        let path = name.claim_ipath();
        let data = ClaimData::new(until, misc).to_vec();
        let group =name.claim_group().unwrap_or(PRIVATE);
        let pkt = linkpoint(group, LNS, &path, links, &data, Stamp::ZERO, ()).as_netbox(); 
        ensure!(*pkt.get_create_stamp() < until);
        Self::from(pkt)
    }
    pub fn read(reader: &ReadTxn,ptr: &LkHash) -> anyhow::Result<Option<Self>>{
        reader.read(ptr)?.map(Claim::from).transpose()
    }
    
    pub fn enckey(&self) -> anyhow::Result<Option<Either<&str,LkHash>>>{
        match self.data.0.has_optional_value(&[b"enckey"]){
            Some(Ok(Some(val))) => Ok(Some(Either::Left(std::str::from_utf8(val)?))),
            _ => match self.links().first_eq(ENCKEY_TAG){
                Some(lnk) => Ok(Some(Either::Right(lnk.ptr))),
                None => Ok(None),
            },
        }
    }
    
    pub fn from(pkt: impl NetPkt)-> anyhow::Result<Self>{
        ensure!(pkt.is_linkpoint(),"claim is always a linkpoint");
        let spath = pkt.get_path();
        ensure!(spath.starts_with(&CLAIM_PREFIX));
        ensure!(*pkt.get_domain() == LNS);
        let mut it = spath.iter();
        it.next().unwrap();
        let namep= it.spath();
        let name = Name::from_spath(namep)?;
        ensure!(*pkt.get_group() == name.claim_group().unwrap_or(PRIVATE),"claim point in the wrong group");
        let data = ClaimData::try_from(pkt.data())?;
        Ok(Claim{pkt:RecvPkt::from_dyn(&pkt),data,name})
    }
    pub fn until(&self) -> Stamp {self.data.until()}
    pub fn pubkey(&self) -> Option<&PubKey>{ self.links().first_eq(PUBKEY_TAG).map(lptr)}
    pub fn group(&self) -> Option<&GroupID>{ self.links().first_eq(GROUP_TAG).map(lptr)}
    pub fn authorities(&self) -> impl Iterator<Item=PubKey>+'_{ self.pkt.get_links().iter().filter(|v| v.tag[15] == b'^').map(|v| v.ptr)}

    pub fn links(&self) -> SelectLink{
        SelectLink(self.pkt.get_links())
    }
}
pub fn enckey_pkt(encrypted: &str,private:bool) -> anyhow::Result<([Link;2],NetPktBox)>{
    let key = linkspace_argon2_identity::pubkey(encrypted)?;
    let pkt = if private {
        linkpoint(PRIVATE, LNS, IPath::empty(), &[], encrypted.as_bytes(), Stamp::ZERO, ()).as_netbox()
    } else {
        datapoint(encrypted.as_bytes(), ()).as_netbox()
    };
    let links = [Link::new(PUBKEY_TAG,key),Link::new(ENCKEY_TAG,pkt.hash())];
    Ok((links,pkt))
}

// TODO abdata should be a newtype
pub fn vote(claim: &Claim,key: &SigningKey,abc:ABConf)-> anyhow::Result<NetPktBox>{
    let vote_link = [Link::new("vote",claim.pkt.hash())];
    let data = abc.to_string().into_bytes();
    Ok(keypoint(claim.name.claim_group().unwrap(), LNS, claim.pkt.get_ipath(), &vote_link, &data, now(), key, ()).as_netbox())
}

pub struct ClaimData(ABConf);
impl ClaimData {
    pub fn new(until:Stamp,mut values:Vec<ABList>) -> Self {
        values.splice(0..0, [clist([b"until" as &[u8],&until.0])]);
        ClaimData(ABConf::new(values))
    }

    pub fn conf_data(&self) -> &ABConf{
        &self.0
    }
    pub fn try_from(b:&[u8]) -> anyhow::Result<Self>{
        let abc = ABConf::try_from(b, true, Some(abconf::ABConfFmt::ABCTxt))?;
        let cd = ClaimData(abc);
        cd.try_until().context("expected valid 'until:STAMP' as first item")?;
        Ok(cd)
    }
    pub fn to_vec(&self) -> Vec<u8>{
        self.to_string().into_bytes()
    }

    fn until(&self) -> Stamp{
        self.try_until().unwrap()
    }
    // the newtype should ensure this always succeeds.
    fn try_until(&self) -> Option<Stamp>{
        let abl=  self.0.first()?;
        match abl.as_slice(){
            [(None,until),(Some(Ctr::Colon),b)]  if until == b"until"=> {
                Some(U64(b.as_slice().try_into().ok()?))
            }
            _ => None
        }
    }
}
impl Display for ClaimData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f,"#abtxt")?;
        for abl in self.0.iter(){
             writeln!(f,"{abl}")?;
        }
        Ok(())
    }
}



