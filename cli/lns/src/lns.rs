// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub enum PktRef {
    Hash(PointHash),
    Pkt(NetPktBox)
}

impl PktRef {
    pub fn pkt(&mut self, reader: &mut ReadTxn) -> anyhow::Result<&NetPktBox>{
        if let PktRef::Hash(h) = self{
            let h = *h;
            *self = PktRef::Pkt(reader.read(&h)?.context(h)?.as_netbox());
        }
        match self {
            PktRef::Pkt(ref mut p) => Ok(p),
            _ => unreachable!()
        }
    }
}

#[derive(Clone)]
pub struct NaamStatus {
    pub proposal : NetPktBox,
    pub votes: HashMap<PubKey,NetPktBox>,
    pub binding: NetPktBox
}
impl NaamStatus {

    pub fn into_iter(self) -> impl Iterator<Item=NetPktBox>{
        use std::iter::once;
        once(self.proposal)
            .chain(once(self.binding))
            .chain(self.votes.into_values())
    }

    pub fn local_binding(&self) -> NetPktBox{
        let name = self.proposal.get_spath().split_first().unwrap().1;
        let name = name.to_owned().try_idx().unwrap();
        let mut links = self.proposal.get_links().to_vec();
        links.push(Link::new("bind", *self.binding.hash()));
        links.push(Link::new("proposal", *self.proposal.hash()));
        linkpoint(PRIVATE, NAAM, &name, &links, *self.proposal.get_create_stamp(),&[], ()).as_netbox()
    }
}
fn vote_by_rootkeys(proposal:NetPktBox) -> anyhow::Result<NaamStatus>{
    let votes = AUTH_KEYS.iter()
        .map(|k|
             Ok((k.pubkey().into(),vote(k,&*proposal,&*ROOT_BINDING)?))
        ).collect::<anyhow::Result<HashMap<_,_>>>()?;
    let binding = bind(&proposal,votes.values().map(|v|&**v).collect::<Vec<&NetPktPtr>>())?;
    Ok(NaamStatus{ proposal,votes,binding})
}

pub fn setup_rootkey_names() -> anyhow::Result<Vec<NaamStatus>>{
    let mut key_bindings = vec![];
    for (i,key) in AUTH_KEYS.iter().enumerate() {
        let name = ipath_buf(&[format!("root{}",i).as_bytes()]);
        let proposal = propose(&name, &[Link::new(&*VETO_KEY_TAG,key.pubkey())], "{now:+300Y}".parse::<StampExpr>()?.eval(&std_ctx())?)?;
        key_bindings.push(vote_by_rootkeys(proposal)?);
    }
    Ok(key_bindings)
}

fn setup_example_bindings() -> anyhow::Result<Vec<NetPktBox>>{
    let links :Vec<_>= EXAMPLE_KEYS.values().map(|k| Link::new(key_weight_tag(0.4),k.pubkey())).collect();
    let proposal = propose(&ipath_buf(&[b"example"]), &links, "{now:+50Y}".parse::<StampExpr>()?.eval(&std_ctx())?)?;
    let example_bind = vote_by_rootkeys(proposal)?;
    let mut lst = example_bind.clone().into_iter().collect::<Vec<_>>();
    for (name,key) in &*EXAMPLE_KEYS{
        let data = liblinkspace::identity::encrypt(key.clone(), &[], Some((8,1)));
        let identity = datapoint(data.as_bytes(), ()).as_netbox();
        let links = vec![
            Link::new(&*VETO_KEY_TAG,key.pubkey()),
            Link::new("identity",*identity.hash())
        ];
        let proposal = propose(&ipath_buf(&[b"example",name.as_bytes()]), &links, "{now:+50Y}".parse::<StampExpr>()?.eval(&std_ctx())?)?;
        let votes = EXAMPLE_KEYS.values()
            .map(|k|
                 Ok((k.pubkey().into(),vote(k,&*proposal,&example_bind)?))
            ).collect::<anyhow::Result<HashMap<_,_>>>()?;
        let binding = bind(&proposal,votes.values().map(|v|&**v).collect::<Vec<_>>())?;
        let name_binding = NaamStatus{ proposal,votes,binding};
        lst.push(identity);
        lst.push(name_binding.local_binding());
        lst.extend(name_binding.into_iter());
    }
    Ok(lst)
}



lazy_static::lazy_static!{
    pub static ref PROPOSAL: IPathBuf = ipath_buf(&[b"proposal"]);
    pub static ref VOTE: IPathBuf = ipath_buf(&[b"vote"]);
    pub static ref NAAM_SP: IPathBuf = ipath_buf(&[b"naam"]);
}
//spath!(pub const PROPOSAL= [b"proposal"]);
//spath!(pub const VOTE= [b"vote"]);
//spath!(pub const NAAM_SP= [b"naam"]);

fn split_first_if_eq<'o>(sp:&'o SPath,expect:&str) -> anyhow::Result<&'o SPath>{
    let (kind,name) = sp.split_first().context("Empty SPath")?;
    ensure!(kind == expect.as_bytes(),"expected {}",expect);
    ensure!(name.iter().all(|v| !v.is_empty()),"empty name segment not allowed");
    Ok(name)
}


pub const NAAM : Domain = as_domain(b"naam");
fn propose(name: &SPath,links:&[Link],until:Stamp) -> anyhow::Result<NetPktBox>{
    let spath = PROPOSAL.concat(name)?.try_idx()?;
    Ok(linkpoint(PUBLIC, NAAM, &spath, links, until,&[], ()).as_netbox())
}

fn vote(key: &SigningKey, proposal: &(impl NetPkt+?Sized),key_bind_authority:&NaamStatus) -> anyhow::Result<NetPktBox>{
    let sp = proposal.point().get_spath();
    let name = split_first_if_eq(sp, "proposal")?;
    let vote_spath = VOTE.concat(name).unwrap().try_idx().unwrap();
    ensure!(proposal.point().pubkey().is_none(),"proposals should not be signed signed");
    let links = vec![
        Link::new("proposal",proposal.hash()),
        Link::new("auth_proposal",key_bind_authority.proposal.hash()),
        Link::new("auth_bind",key_bind_authority.binding.hash())
    ];
    Ok(keypoint(&key, PUBLIC, NAAM, &vote_spath, &links, now(),&[], ()).as_netbox())
}

fn bind(proposal : &NetPktPtr,mut votes: Vec<&(impl NetPkt+?Sized)>) -> anyhow::Result<NetPktBox>{
    let pname = split_first_if_eq(proposal.point().get_spath(), "proposal")?;
    for v in &votes {
        let vname = split_first_if_eq(v.point().get_spath(), "vote")?;
        ensure!(pname == vname);
        ensure!(v.point().is_keypoint());
        ensure!(v.point().get_links().get(0) == Some(&Link::new("proposal",proposal.hash())));
    }
    votes.sort_by_key(|v| *v.hash());
    let links = votes.iter().map(|p| Link::new("vote",p.hash()))
        .chain(Some(Link::new("proposal",proposal.hash())))
        .collect::<Vec<_>>();
    let stamp = votes.iter().map(|p| *p.point().get_create_stamp()).max_by_key(|v| v.get()).ok_or(anyhow!("TODO no stamp"))?;
    let name = split_first_if_eq(proposal.get_spath(), "proposal")?;
    let bind_spath = NAAM_SP.concat(name).unwrap().try_idx()?;
    Ok(linkpoint(PUBLIC, NAAM, &bind_spath, &links, stamp,&[], ()).as_netbox())
}

fn get_local(name: &SPath,r:&ReadTxn) -> anyhow::Result<NetPktBox>{
    todo!()
        /*
    r.tree(TreeQuery::local(NAAM).spath(name))
        .next().context("Not Found").map(|v| v.as_netbox())
        */
}
fn get_local_naamstatus(name: &SPath, r:&ReadTxn) -> anyhow::Result<NaamStatus>{
    let local = get_local(name, r)?;
    let links = local.get_links();
    let (proposal_ref,rest) =  links.split_last().context("A")?;
    ensure!(proposal_ref.tag == as_tag(b"proposal"));
    let (bind_ref, _rest) = rest.split_first().context("B")?;
    ensure!(bind_ref.tag == as_tag(b"bind"));
    let binding = r.read(&bind_ref.pointer)?.context("Missing bind")?.as_netbox();
    let proposal = r.read(&proposal_ref.pointer)?.context("Proposal missing")?.as_netbox();
    Ok(NaamStatus { proposal, votes:HashMap::new(), binding })
}


const STATIC_WEIGHT: u64 = 0u64.wrapping_sub(1);
const VOTE_THRESHOLD : u64 = 1u64 << 60;
fn key_weight_tag(frac:f64)-> Tag{
    assert!(frac > 0.0 && frac <= 1.0);
    let u64_weight = (VOTE_THRESHOLD as f64 * frac) as u64;
    let mut tag = as_tag(b"key");
    tag[8..].copy_from_slice(&u64_weight.to_be_bytes());
    tag
}
lazy_static::lazy_static!{
    pub static ref AUTH_KEYS : [SigningKey;11] = {
        use rand::SeedableRng;
        let mut gen = rand_chacha::ChaCha8Rng::seed_from_u64(50010);
        [0;11].map(|_| SigningKey::generate_with(&mut gen))
    };
    pub static ref EXAMPLE_KEYS : BTreeMap<&'static str,SigningKey> = {
        use rand::SeedableRng;
        let mut gen = rand_chacha::ChaCha8Rng::seed_from_u64(5010);
        ["alice","bob","charlie"]
            .into_iter().map(|name| (name,SigningKey::generate_with(&mut gen)))
            .collect::<BTreeMap<_,_>>()
    };
    pub static ref VETO_KEY_TAG : Tag = {
        let mut tag = as_tag(b"key");
        tag.0[8..].copy_from_slice(&STATIC_WEIGHT.to_be_bytes());
        tag
    };
    pub static ref PUBLIC_ROOT : Vec<Link> =
        vec![Link::new("group",PUBLIC),Link::new("pkt",PUBLIC)].into_iter().chain(
        AUTH_KEYS.iter().map(|s| Link{tag: *VETO_KEY_TAG,pointer:s.pubkey().into()})).collect();
    pub static ref ROOT_BINDING : NaamStatus = {
        let proposal = propose(SPath::empty(), &*PUBLIC_ROOT,Stamp::MAX).unwrap();
        let votes = AUTH_KEYS.iter()
            .map(|k| {
                let links = vec![
                    Link::new("proposal",proposal.hash()),
                    Link::new("auth_proposal",proposal.hash()),
                ];
                (k.pubkey().into(),keypoint(&k, PUBLIC, NAAM, &*VOTE, &links, now(),&[], ()).as_netbox())
            })
            .collect::<HashMap<PubKey,_>>();
        let binding = bind(&proposal,votes.values().map(|v|&**v).collect::<Vec<_>>()).unwrap();
        NaamStatus {
            proposal,
            votes,
            binding,
        }
    };
}



