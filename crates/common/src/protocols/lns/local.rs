// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/// Impl's ABE scope/eval for {+:hello:world}, {+@:world:hello}, and {/+/hello/world}
use std::fmt::Debug;

use linkspace_core::{
    eval,
    prelude::{query_mode::Order, *},
};

use crate::runtime::Linkspace;

use super::TagSuffix;
pub const LNS: Domain = ab(b"lns");
spath!(pub const LOCAL_CLAIM_PREFIX = [b"local"]);

spath!(pub const LOCAL_CLAIM_LOOKUP_PREFIX = [b"lookup",b"local"]);

#[derive(Debug, Clone, Copy)]
pub struct LocalLNS<R> {
    pub rt: R,
}
/**
create a linkpoint claim lns:{#:0}:('local'/..path) with `links` and `data`
for every link.tag ending with @ or # create a lookup entry under
lns:{#:0}:/lookup/local/@/{link.ptr} [("origin",{hash of claim})]
and
lns:{#:0}:/lookup/local/#/{link.ptr} [("origin",{hash of claim})]
respectivly
**/
pub fn build_local_lns_points(path: &SPath, links: &[Link], data: &[u8]) -> Vec<NetPktBox> {
    let path = LOCAL_CLAIM_PREFIX.idx().try_join(path).unwrap();
    let claim = linkpoint(LOCAL_ONLY_GROUP, LNS, &*path, links, data, now(), ()).as_netbox();
    let lookup_link = [Link::new("origin", claim.hash())];
    let lookup_path_data: Vec<u8> = claim.get_ipath().ipath_bytes().to_vec();
    let mut pkts = vec![claim];
    let lookups = TagSuffix::ALL
        .map(|tp| (tp, tp.select(links)))
        .into_iter()
        .flat_map(move |(tp, links): (TagSuffix, _)| {
            let path = LOCAL_CLAIM_LOOKUP_PREFIX.idx().append(tp.bslice());
            let v = lookup_path_data.clone();
            links.map(move |link| {
                let path = path.clone().append(&*link.ptr);
                linkpoint(LOCAL_ONLY_GROUP, LNS, &path, &lookup_link, &v, now(), ()).as_netbox()
            })
        });
    pkts.extend(lookups);
    pkts
}

pub fn get_local_claim<'o>(
    reader: &'o IReadTxn<impl db::Cursors>,
    path: &SPath,
) -> anyhow::Result<Option<RecvPktPtr<'o>>> {
    Ok(get_local_claims(reader, path)?.next())
}
pub fn get_local_claims<'o>(
    reader: &'o IReadTxn<impl db::Cursors>,
    path: &SPath,
) -> anyhow::Result<impl Iterator<Item = RecvPktPtr<'o>>> {
    let path = LOCAL_CLAIM_PREFIX.try_join(path)?;
    let q = PktPredicates::from_gdp(LOCAL_ONLY_GROUP, LNS, &path).create_before(now())?;
    Ok(reader.query_tree(Order::Desc, &q))
}
pub fn get_lookup_entry<'o>(
    reader: &IReadTxn<impl db::Cursors>,
    kind: TagSuffix,
    bytes: [u8; 32],
) -> anyhow::Result<Option<NetPktBox>> {
    let path = LOCAL_CLAIM_LOOKUP_PREFIX
        .to_owned()
        .extend_from_iter(&[&[kind as u8] as &[u8], &bytes])?;
    let q = PktPredicates::from_gdp(LOCAL_ONLY_GROUP, LNS, &path).create_before(now())?;
    Ok(reader
        .query_tree(Order::Desc, &q)
        .next()
        .map(|v| v.as_netbox()))
}
pub fn get_lookup_entries<'o>(
    reader: &IReadTxn<impl db::Cursors>,
    kind: TagSuffix,
    bytes: [u8; 32],
) -> impl Iterator<Item = RecvPktPtr> {
    let path = LOCAL_CLAIM_LOOKUP_PREFIX
        .to_owned()
        .extend_from_iter(&[&[kind as u8] as &[u8], &bytes])
        .unwrap();
    let q = PktPredicates::from_gdp(LOCAL_ONLY_GROUP, LNS, &path)
        .create_before(now())
        .unwrap();
    reader.query_tree(Order::Desc, &q)
}

impl<R: Fn() -> std::io::Result<Linkspace>> LocalLNS<R> {
    fn lookup_claim(&self, kind: TagSuffix, bytes: &[u8]) -> ApplyResult {
        let hash = LkHash::try_fit_bytes_or_b64(bytes).ok()?;
        let reader = (self.rt)()?.env().get_reader()?;
        let lookup_lp = get_lookup_entry(&reader, kind, hash.0)??;
        if let Some(Link { tag, ptr: pointer }) = lookup_lp.get_links().first() {
            if tag != "origin" {
                todo!()
            }
            let claim = reader.read(pointer).ok()??;
            let path = claim.get_ipath();
            let mut it = path.iter();
            let _local_comp = it.next();
            ApplyResult::Ok(format!("{{@local:{}}}", abl(it)).into_bytes())
        } else {
            ApplyResult::None
        }
    }
    fn local_pkt(&self, args: &[&[u8]], scope: &dyn Scope) -> Result<Vec<u8>, ApplyErr> {
        let empty = args.iter().position(|v| v.is_empty());
        let (path_b, rest) = if let Some(i) = empty {
            (&args[..i], &args[i + 1..])
        } else {
            (args, &[] as &[&[u8]])
        };
        let reader = (self.rt)()?.env().get_reader()?;
        let path = SPathBuf::try_from_iter(path_b.iter())?;
        let pkt = get_local_claim(&reader, &path)?
            .ok_or_else(|| format!("no local claim found for {}", path))?;
        let (id, args) = rest.split_first().unwrap_or((&{ b"pkt" as &[u8] }, &[]));
        tracing::trace!(or=?rest,id=%AB(*id),args=?args,"subcall netpkteval");
        let r = pkt_scope(&*pkt.pkt)
            .lookup_apply(id, args, true, scope)
            .into_opt()
            .unwrap_or_else(|| {
                Err(format!(
                    "no such function found for pkt : {} - {}",
                    AB(*id),
                    abl(args)
                )
                .into())
            });
        drop(reader);
        r
    }
    fn local_tag_ptr(&self, args: &[&[u8]], tag: TagSuffix) -> Result<Vec<u8>, ApplyErr> {
        let empty = args.iter().position(|v| v.is_empty());
        let (path_b, rest) = if let Some(i) = empty {
            (&args[..i], &args[i + 1..])
        } else {
            (args, &[] as &[&[u8]])
        };
        let path = SPathBuf::try_from_iter(path_b.iter().rev())?;
        if !rest.is_empty() {
            todo!("add options to select other links");
        }
        let reader = (self.rt)()?.env().get_reader()?;
        let pkt = get_local_claim(&reader, &path)?
            .ok_or_else(|| format!("no local claim found for {}", path))?;
        let link = tag.select(pkt.get_links()).next().ok_or_else(|| {
            format!(
                "missing '{}' tag in \n {}",
                tag as u8 as char,
                pkt_fmt(&*pkt)
            )
        })?;
        Ok(link.ptr.to_vec())
    }
}
impl<R: Fn() -> std::io::Result<Linkspace>> eval::EvalScopeImpl for LocalLNS<R> {
    fn about(&self) -> (String, String) {
        ("local-lns".into(), "".into())
    }
    fn list_funcs(&self) -> &[linkspace_core::prelude::ScopeFunc<&Self>] {
        &[
            fnc!( @C "local",1..=16,Some(true),"[namecomp*] - get the entire local lns packet and fowards arg to pkt scope",
                   |this:&Self,args:&[&[u8]],_,scope| this.local_pkt(args,scope),none),
            fnc!(
                "#local",
                1..=7,
                Some(true),
                "[namecomp*] - get the associated local lns group name",
                |this: &Self, args: &[&[u8]]| this.local_tag_ptr(args, TagSuffix::Group),
                |this: &Self, phash: &[u8], _| this.lookup_claim(TagSuffix::Group, phash)
            ),
            fnc!(
                "@local",
                1..=7,
                Some(true),
                "[namecomp*] - get the associated local lns group name",
                |this: &Self, args: &[&[u8]]| this.local_tag_ptr(args, TagSuffix::Pubkey),
                |this: &Self, phash: &[u8], _| this.lookup_claim(TagSuffix::Pubkey, phash)
            ),
        ]
    }

    fn list_eval(&self) -> &[ScopeEval<&Self>] {
        &[eval_fnc!(
            "local",
            "namecmp*::{EXPR} evaluate expr with pkt scope of local",
            |this: &Self, abe: &[ABE], scope: &dyn Scope| {
                let brk = abe.iter().position(|v| v.is_colon());
                let default = linkspace_core::eval::abev!({ "pkt" });
                let (path, expr) = if let Some(i) = brk {
                    (&abe[..i], abe.get(i + 1..).ok_or("Missing expr after ':'")?)
                } else {
                    (abe, default.as_slice())
                };
                let ctx = EvalCtx { scope };
                tracing::trace!(?path, "eval path");
                let path = eval::eval(&ctx, path)?;
                let path = SPathBuf::try_from_ablist(path)?;
                let reader = (this.rt)()?.env().get_reader()?;
                let pkt = get_local_claim(&reader, &path)?
                    .ok_or(format!("No local claim found for {}", path))?;
                tracing::trace!(?expr, "eval in pkt ctx");
                let ctx = pkt_ctx(ctx, &*pkt);
                let bytes = eval::eval(&ctx, expr)?;
                Ok(bytes.concat())
            }
        )]
    }
}

pub fn setup_local_key(
    lk: &Linkspace,
    id: &str,
    enckey: &str,
    data: &[u8],
) -> std::io::Result<PubKey> {
    let pubkey =
        crate::identity::pubkey(enckey).map_err(|_e| std::io::Error::other("bad enckey"))?;
    let path = spath_buf(&[id.as_bytes()]);
    let encrypted_private_key = datapoint(enckey.as_bytes(), ()).as_netbox();
    let links = [
        Link::new("pubkey@", pubkey),
        Link::new("enckey", encrypted_private_key.hash()),
    ];

    let mut pkts = build_local_lns_points(&path, &links, data);
    pkts.insert(0, encrypted_private_key);

    let writer = lk.env().get_writer()?;
    save_pkts(writer, &pkts)?;
    tracing::debug!(?pkts, "Written local lns claim");
    Ok(pubkey.into())
}
