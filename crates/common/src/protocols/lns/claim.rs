use std::fmt::{Debug, Display};

use crate::prelude::*;
use anyhow::{ensure, Context};
use linkspace_core::{prelude::PRIVATE, stamp_fmt::delta_stamp};
use linkspace_pkt::{lptr, reroute::RecvPkt, NetPkt, PointExt, SelectLink};

use super::*;
use crate::protocols::lns::name::Name;

pub struct LiveClaim {
    pub claim: Claim,
    pub signatures: Vec<RecvPkt<NetPktBox>>,
    pub parent: Option<Box<LiveClaim>>,
}
impl LiveClaim {
    pub fn list(&self) -> Vec<&LiveClaim> {
        let mut p: Option<&LiveClaim> = Some(self);
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
        if let Some(this) = lst.next() {
            f.field("claim", &this.claim)
                .field(
                    "signatures",
                    &this
                        .signatures
                        .iter()
                        .map(|p| p.get_pubkey())
                        .collect::<Vec<_>>(),
                )
                .field("parent", &self.parent);
        }
        for this in lst {
            f.field("name", &this.claim.name)
                .field("claim", &this.claim.pkt.hash_ref())
                .field(
                    "signatures",
                    &this
                        .signatures
                        .iter()
                        .map(|p| p.get_pubkey())
                        .collect::<Vec<_>>(),
                );
        }
        f.finish()
    }
}

pub struct Claim {
    pub pkt: RecvPkt<NetPktBox>,
    pub name: Name,
}

impl Display for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let u = self.until();
        let d = delta_stamp(now(), u);
        writeln!(f, "{}\t{}\t{u}\t{d}", self.pkt.hash(), self.name)?;
        for Link { ptr, tag } in self.pkt.get_links() {
            writeln!(f, "{ptr}\t{tag}")?;
        }
        Ok(())
    }
}
impl Debug for Claim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

pub fn read_enckey(data: &[u8]) -> anyhow::Result<(PubKey, String)> {
    let first_line = data[..data.len().min(130)]
        .split(|i| *i == b'\n')
        .next()
        .context("empty data")?;
    let st = std::str::from_utf8(first_line)?;
    let pubkey = linkspace_argon2_identity::pubkey(st)?;
    Ok((pubkey.into(), st.to_string()))
}

impl Claim {
    pub fn new(name: Name, until: Stamp, links: &mut [Link], data: &[u8]) -> anyhow::Result<Self> {
        ensure!(!links.is_empty(), "requires at least one link");
        let link_stamp = as_stamp_tag(links[0].tag).0;
        ensure!(
            link_stamp == Stamp::ZERO || link_stamp == until,
            "links[0].tag[..8] is used for until - currently holds {} (wants {})",
            links[0],
            AB(until.0)
        );
        links[0].tag[0..8].copy_from_slice(&until.0);
        let path = name.claim_space();
        let group = name.claim_group();
        let pkt = linkpoint(group, LNS, &path, links, data, Stamp::ZERO, ()).as_netbox();
        ensure!(*pkt.get_create_stamp() < until);
        Self::from(pkt)
    }
    pub fn read(reader: &ReadTxn, ptr: &LkHash) -> anyhow::Result<Option<Self>> {
        reader.read(ptr)?.map(Claim::from).transpose()
    }

    pub fn enckey(&self) -> Result<(PubKey, String), Option<&Link>> {
        read_enckey(self.pkt.data()).map_err(|_| self.links().first_tailmask(&ENCKEY_TAG))
    }

    pub fn from(pkt: impl NetPkt) -> anyhow::Result<Self> {
        tracing::trace!(p=%PktFmtDebug(&pkt),"reading claim");
        ensure!(pkt.is_linkpoint(), "claim is always a linkpoint");
        let spacename = pkt.get_spacename();
        ensure!(spacename.starts_with(&CLAIM_PREFIX));
        ensure!(*pkt.get_domain() == LNS);
        let mut it = spacename.iter();
        it.next().unwrap();
        let namep = it.space();
        let name = Name::from_space(namep)?;
        ensure!(
            *pkt.get_group() == name.claim_group(),
            "claim point {name} ({:?}) in the wrong group ({})",
            name.claim_group(),
            pkt.get_group()
        );
        ensure!(!pkt.get_links().is_empty(), "no links?");
        Ok(Claim {
            pkt: RecvPkt::from_dyn(&pkt),
            name,
        })
    }
    pub fn until(&self) -> Stamp {
        as_stamp_tag(self.pkt.get_links()[0].tag).0
    }
    pub fn pubkey(&self) -> Option<&PubKey> {
        self.links().first_tailmask(&PUBKEY_TAG).map(lptr)
    }
    pub fn group(&self) -> Option<&GroupID> {
        self.links().first_tailmask(&GROUP_TAG).map(lptr)
    }
    pub fn authorities(&self) -> impl Iterator<Item = PubKey> + '_ {
        self.pkt
            .get_links()
            .iter()
            .filter(|v| v.tag[15] == b'^')
            .map(|v| v.ptr)
    }

    pub fn links(&self) -> SelectLink {
        SelectLink(self.pkt.get_links())
    }
}
pub fn enckey_pkt(encrypted: &str, private: bool) -> anyhow::Result<([Link; 2], NetPktBox)> {
    let key = linkspace_argon2_identity::pubkey(encrypted)?;
    let pkt = if private {
        linkpoint(
            PRIVATE,
            LNS,
            RootedSpace::empty(),
            &[],
            encrypted.as_bytes(),
            Stamp::ZERO,
            (),
        )
        .as_netbox()
    } else {
        datapoint(encrypted.as_bytes(), ()).as_netbox()
    };
    let links = [
        Link::new(PUBKEY_TAG, key),
        Link::new(ENCKEY_TAG, pkt.hash()),
    ];
    Ok((links, pkt))
}

pub fn vote(claim: &Claim, key: &SigningKey, data: &[u8]) -> anyhow::Result<NetPktBox> {
    let vote_link = [Link::new("vote", claim.pkt.hash())];
    Ok(keypoint(
        claim.name.claim_group(),
        LNS,
        claim.pkt.get_rooted_spacename(),
        &vote_link,
        data,
        now(),
        key,
        (),
    )
    .as_netbox())
}
