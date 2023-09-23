// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(iterator_try_collect)]

use anyhow::*;
use linkspace_common::{
    anyhow::{self},
    cli::{clap::Parser,  opts::CommonOpts, *, keys::KeyOpts},
    prelude::{* }, protocols::lns::{self, name::{NameExpr}, claim::{ Claim}, PUBKEY_TAG, GROUP_TAG, public_claim::Issue, PUBKEY_AUTH_TAG, lnstag }, identity,  };
use tracing_subscriber::EnvFilter;


#[derive(Parser )]
pub struct Opts {
    #[command(subcommand)]
    cmd: Cmd,
    #[command(flatten)]
    common: CommonOpts,
}

#[derive(Parser )]
pub enum Cmd {
    /// resolve the claim chain for a linkpoint name
    Get{
        name: Option<NameExpr>,
        #[arg(long,default_value="stdout")]
        write: Vec<WriteDestSpec>,
        #[arg(default_value="null")]
        write_signatures: Vec<WriteDestSpec>,
        #[arg(long)]
        chain:bool
    },
    LsPubkey{
        pubkey: Option<PubKeyExpr>
    },
    LsGroup{
        group: Option<GroupExpr>
    },
    Ls{
        name:NameExpr
    },
    Vote{
        name:NameExpr,
        claim: HashExpr,
        #[command(flatten)]
        key: KeyOpts,
        #[arg(long,default_value="stdout")]
        write: Vec<WriteDestSpec>,
    },
    CreateClaim{
        /// name of claim
        name:NameExpr,
        #[arg(long)]
        /// the group id value for [#:NAME]
        group: Option<GroupExpr>,
        #[arg(long)]
        /// the public key to find with [@:NAME] - becomes an authority as well unless --no-auth was set
        pubkey: Option<PubKeyExpr>,
        /// do not give the pubkey/enckey authority status
        #[arg(long)]
        no_auth: bool,
        /// implies pubkey
        #[arg(long,conflicts_with("pubkey"))]
        enckey:Option<String>,
        /// Copy from pubkey
        #[arg(long,conflicts_with_all(["enckey","pubkey"]))]
        copy_from:Option<NameExpr>,

        #[arg(long)]
        /// desired list of authname^:pubkey authorities over [NAME + ':*'] - authname is arbitrary ('^' is inserted automatically)
        auth: Vec<LinkExpr>,
        #[arg(long,default_value="[now:+7D]")]
        /// end date of this claim
        until: StampExpr,
        #[arg(long)]
        allow_empty:bool,

        #[arg(long,default_value="stdout")]
        write: Vec<WriteDestSpec>
    }
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::metadata::LevelFilter::WARN.into())
        .from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
    let Opts { mut common, cmd } = Opts::parse();
    common.mut_write_private().get_or_insert(true);
    match cmd {
        Cmd::Vote { name, claim, key ,write } => {
            let lk= common.runtime()?;
            let ctx = common.eval_ctx();
            let name = name.eval(&ctx)?;
            let mut write = common.open(&write)?;

            let reader =lk.get_reader();
            let hash = claim.eval(&ctx)?;
            let claim_pkt = reader.read(&hash)?.context("cant find claim")?;
            let claim = Claim::from(claim_pkt)?;
            ensure!(claim.name == name);
            let signing = key.identity(&common, true)?;
            let live_parent = lns::lookup_authority_claim(&lk, &name,&mut |_|Ok(()))?.map_err(|_e| anyhow!("only found upto {}",name))?;
            ensure!(live_parent.authorities().any(|p| p == signing.pubkey()),"key is not an authority in {live_parent}");
            let pkt = lns::claim::vote(&claim, signing,&[])?;
            common.write_multi_dest(&mut write, &pkt, None)?;
        }
        Cmd::Get{name, write, write_signatures,chain } => {
            let mut write = common.open(&write)?;
            let mut signatures = common.open(&write_signatures)?;

            let (is_ok,liveclaim) = match name {
                None => (true,lns::public_claim::root_claim()),
                Some(name) => {
                    let name = name.eval(&common.eval_ctx())?;
                    let mut issue_handler = |issue:Issue| {eprintln!("{:?}",issue); Ok(())};
                    let r = lns::lookup_live_chain(&common.runtime()?, &name, &mut issue_handler)?;
                    let is_ok = r.is_ok();
                    let liveclaim = match r{
                        Result::Ok(r) => r,
                        Err(r) => r,
                    };
                    (is_ok,liveclaim)
                },
            };
            let lst = if !chain { vec![&liveclaim]}else { liveclaim.list()};
            for cl in lst.iter().rev(){
                common.write_multi_dest(&mut write,&cl.claim.pkt, None)?;
                if !signatures.is_empty(){
                    for s in &cl.signatures {
                        common.write_multi_dest(&mut signatures,&s, None)?;
                    }
                }
            }
            if !is_ok{ bail!("incomplete chain")}
        },
        Cmd::CreateClaim { name, group, pubkey, auth, until, write,allow_empty, no_auth, enckey,copy_from} => {
            let mut write = common.open(&write)?;
            let ctx = common.eval_ctx();
            let until = until.eval(&ctx)?;

            let as_link = |tag:Tag| move |ptr:LkHash| Link::new(tag,ptr);
            let mut links = vec![];
            let name = name.eval(&ctx)?;
            let mut pubkey = pubkey.map(|e| e.eval(&ctx)).transpose()?;
            let mut data= vec![];
            match enckey {
                Some(k) => {
                    let encpubkey : PubKey= identity::pubkey(&k)?.into();
                    if let Some(pk) = pubkey { ensure!(pk == encpubkey,"pubkey and enckey don't match. pick one")}
                    pubkey = Some(encpubkey);
                    use std::io::Write;
                    writeln!(&mut data,"{}",k)?;
                },
                None => {},
            }
            if let Some(n) = copy_from{
                let alt_name = n.eval(&ctx)?;
                pubkey = lns::lookup_pubkey(&common.runtime()?, &alt_name)?;
            }

            let group = group.map(|e| e.eval(&ctx)).transpose()?;

            links.extend(group.map(as_link(ab(&GROUP_TAG))));
            links.extend(pubkey.map(as_link(ab(&PUBKEY_TAG))));
            links.extend(pubkey.filter(|_| !no_auth).map(as_link(ab(&PUBKEY_AUTH_TAG))));
            for link_e in auth {
                let mut auth_link = link_e.eval(&ctx)?;
                let tag = auth_link.tag.cut_prefix_nulls();
                auth_link.tag = lnstag(Stamp::ZERO,&tag,b'^')?;
                links.push(auth_link)
            }
            if links.is_empty(){
                ensure!(allow_empty,"empty claim");
                links.push(Link::DEFAULT);
            }
            let claim = Claim::new(name, until, &mut links, &data)?;
            common.write_multi_dest(&mut write, &claim.pkt, None)?;
        },
        Cmd::Ls { name } => {
            let ctx = common.eval_ctx();
            let name = name.eval(&ctx)?;
            let lk = common.runtime()?;
            let reader = lk.get_reader();
            let claims = lns::utils::list_all_potential_claims_with_prefix(&reader,&name);
            for c_ok in claims{
                match c_ok {
                    Result::Ok(o) => {
                        println!("{o}")
                    },
                    Err(e) => eprintln!("{e:#?}"),
                }
            }
        },
        Cmd::LsGroup { group } => ls_tag(&common,&GROUP_TAG,group)?,
        Cmd::LsPubkey{ pubkey} => ls_tag(&common,&PUBKEY_TAG,pubkey)?,
    };
    Ok(())
}


fn ls_tag(common:&CommonOpts,tag:&[u8],ptr:Option<PExpr>) -> anyhow::Result<()>{
    let ctx = common.eval_ctx();
    let ptr = ptr.map(|g|g.eval(&ctx)).transpose()?;
    let lk = common.runtime()?;
    let reader = lk.get_reader();
    for c_ok in lns::utils::list_all_reverse_lookups(&reader,tag,ptr){
        for (_,el) in c_ok{
            match el {
                Result::Ok(Some(c)) => {
                    let val = c.links().first_tailmask(tag).map(|l|l.ptr.to_string()).unwrap_or("cleared".to_string());
                    println!("{val} {} {}",c.pkt.hash(),c.name)
                },
                Result::Ok(None) => {
                    eprintln!("Missing claim")
                }
                Err(e) => eprintln!("{e:?}"),
            }
        }
    }
    Ok(())
}

