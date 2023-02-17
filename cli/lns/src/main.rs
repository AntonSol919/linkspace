// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub mod local;

use anyhow::*;
/**
LNS is a keypoint scheme that makes packets 'valid', at a specific path , between two timestamps.
It's resolution is integrated into ABE.
e.g. after registering nl/sol/rs a publickey it can be queried with `lk_eval("{@:rs:sol:nl}")`,
and after registering a nl/sol group it can be queried with `lk_eval("{#:nl:sol}")`

the '<lns' function does reverse lookup. such that {/?:{#:nl:sol}} -> /nl/sol

**/
use linkspace_common::{
    anyhow::{self},
    cli::{clap::Parser, keys::KeyOpts, opts::CommonOpts, *},
    prelude::*,
};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
pub struct Opts {
    #[clap(flatten)]
    common: CommonOpts,
    #[clap(subcommand)]
    cmd: LNSCmd,
}

#[derive(Parser, Debug)]
pub enum LNSCmd {
    LocalGet {
        path: SPathExpr,
    },
    Bootstrap {
        #[clap(default_value = "stdout")]
        write: Vec<WriteDest>,
    },
    Get {
        path: SPathExpr,
    },
    Propose {
        #[clap(flatten)]
        links: RefList,
        name: SPathBuf,
        #[clap(default_value = "{now:+1D}")]
        until: StampExpr,
        #[clap(default_value = "stdout")]
        write: Vec<WriteDest>,
    },
    Vote {
        #[clap(flatten)]
        key: KeyOpts,
        #[clap(short, long)]
        key_authority: Option<PartialHash>,
        hash: PartialHash,
        #[clap(default_value = "stdout")]
        write: Vec<WriteDest>,
    },
}

fn main() -> anyhow::Result<()> {
    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::metadata::LevelFilter::WARN.into())
        .from_env()?;
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();
    let Opts { common, cmd } = Opts::parse();
    match cmd {
        LNSCmd::LocalGet { path } => local::get(common, path)?,
        _ => todo!(), /*
                      LNSCmd::Bootstrap{write} => {
                          *common.mut_write_private() = Some(true);
                          let lst = ROOT_BINDING.clone().into_iter()
                              .chain(Some(ROOT_BINDING.local_binding()))
                              .chain(setup_rootkey_names()?.into_iter().flat_map(|v| [v.local_binding()].into_iter().chain(v.into_iter())))
                              .chain(setup_example_bindings()?.into_iter());
                          for p in lst {
                              common.write_multi_dest(&write, &*p, None)?;
                          }
                      }
                      LNSCmd::Get{spath} => {
                          let env = common.env()?;
                          let spath = spath.eval(&common.eval_ctx())?;
                          let pkt = get_local( &spath,&env.get_reader()?)?;
                          common.write_pkt(std::io::stdout(), &*pkt)?;
                      },
                      LNSCmd::Propose { name :_,links, until:_, write:_ } => {
                          let values = links.try_into_links(&common.eval_ctx())?;
                          ensure!(values .len() > 0,"empty proposal?");
                          todo!()
                              /*
                              let pkt = propose(&name.try_idx()?, &values,until.eval_static_now()?)?;
                          common.write_multi_dest(&dest, &*pkt,None)?;
                              */
                      },
                      LNSCmd::Vote { key, hash ,key_authority, write }=>{
                          let id = key.identity(&common, true)?;
                          let env = common.env()?;
                          let reader = env.get_reader()?;
                          let proposal = reader.uniq_partial(hash).context("Proposal not found")?.map_err(|lst| anyhow!("Multiple Found {:?}",lst))?.as_netbox();
                          let name = split_first_if_eq(proposal.get_spath(),"proposal")?;
                          let parent = name.parent().expect("todo");
                          let auth = match key_authority{
                              Some(h) => {
                                  let binding = reader.uniq_partial(h).context("Auth not found")?.map_err(|lst| anyhow!("Multiple Found {:?}",lst))?.as_netbox();
                                  let auth_name = split_first_if_eq(binding.get_spath(), "binding")?;
                                  ensure!(parent == auth_name,"key authority is not the parent");
                                  todo!()
                                      /*
                                  let proposal_ref= binding.get_links().first_prefix(b"proposal").context("Missing auth proposal?")?;
                                  let proposal = reader.read(&proposal_ref.pointer)?.context("Could not find authirization proposal")?.as_netbox();
                                  NaamStatus{proposal, binding,votes:HashMap::new()}
                                      */
                              },
                              None => {
                                  get_local_naamstatus(&parent,&reader)?
                              },
                          };
                          // todo PROMPT
                          let pkt = vote(&id, &*proposal, &auth)?;
                          common.write_multi_dest(&write, &*pkt,None)?;
                      }
                          */
    }
    Ok(())
}
