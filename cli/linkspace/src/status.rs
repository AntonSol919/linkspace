// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::ops::ControlFlow;
use std::rc::Rc;

use anyhow::Context;
use linkspace::runtime::cb::{ try_cb};
use linkspace::{ lk_process_while };
use linkspace_common::cli::clap::Subcommand;
use linkspace_common::cli::reader::DataReadOpts;
use linkspace_common::cli::{clap };
use linkspace_common::runtime::handlers::PktStreamHandler;
use linkspace_common::{
    cli::{clap::Parser, opts::CommonOpts,  WriteDestSpec},
    core::stamp_fmt::DurationStr,
};

use linkspace_common::prelude::*;

#[derive(Subcommand)]
/// convention - (local) status convention 
pub enum StatusCmd{
    /// watch a status update (or checks for a recent one)
    Watch(StatusWatch),
    /// reply/set a status watch requests
    Set(StatusSet)
}

#[derive(Parser, Debug)]
pub struct StatusArgs {
    pub domain: DomainExpr,
    pub group: GroupExpr,
    pub objtype: TypedABE<Vec<u8>>,
    pub instance: Option<TypedABE<Vec<u8>>>,
}
impl StatusArgs {
    #[allow(clippy::type_complexity)]
    pub fn eval(
        self,
        scope: &dyn Scope,
    ) -> anyhow::Result<(Domain, GroupID, Vec<u8>, Option<Vec<u8>>)> {
        Ok((
            self.domain.eval(scope)?,
            self.group.eval(scope)?,
            self.objtype.eval(scope)?,
            self.instance.map(|v| v.eval(scope)).transpose()?,
        ))
    }
}

#[derive(Parser, Debug)]
pub struct StatusSet {
    #[command(flatten)]
    args: StatusArgs,
    /// the status data.
    #[command(flatten,next_help_heading="Data Options")]
    readopts: DataReadOpts,
    // #[arg(short,long)]
    //link: Vec<LinkExpr>,
}
pub fn status_set(common: CommonOpts,ss: StatusSet) -> anyhow::Result<()> {
    let StatusSet { args, readopts } = ss;
    let scope = common.eval_scope();
    let (domain, group, objtype, instance) = args.eval(&scope)?;
    use linkspace::prelude::*;
    use linkspace::conventions::status::*;
    let status = LkStatus {
        domain,
        group,
        objtype: &objtype,
        instance: instance.as_deref().or(Some(b"default")),
        qid: b"<lk set status>"
    };
    let lk : Linkspace = common.runtime()?.into();
    let c= common.clone();

    let mut reader = readopts.open_reader(false, &scope)?;
    let mut buf = vec![];
    lk_status_set(&lk, status, move |_,domain,group,space,link| {
        buf.clear();
        let freespace : usize = calc_free_space(space, &[link], &[], false).try_into()?;
        reader.read_next_data(&c.eval_scope(),freespace,&mut buf)?.context("no more data")?;
        lk_linkpoint(&buf,domain, group, space, &[link], None)
    })?;
    lk_process_while(&lk,None, Stamp::ZERO)?;

    Ok(())
}

#[derive(Parser)]
pub struct StatusWatch {
    #[command(flatten)]
    args: StatusArgs,
    /// wait for this duration (since last request) before returning an error 
    #[arg(short, long, default_value = "5s")]
    timeout: DurationStr,
    #[arg(short, long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,
    /// Output multiple replies (until last_req+duration)
    #[arg(short,long)]
    multi: bool,
    /// only output query before quiting
    #[arg(long)]
    print_query: bool,
    /// only output the request packet
    #[arg(long)]
    write_request: Vec<WriteDestSpec>,
}
pub fn status_watch(mut common: CommonOpts, ps: StatusWatch) -> anyhow::Result<()> {
    let StatusWatch {
        args,
        timeout,
        write,
        print_query,
        write_request,
        multi,
    } = ps;
    *common.mut_write_private() = Some(true);
    let scope = common.eval_scope();
    let (domain, group, objtype, instance) = args.eval(&scope)?;
    use linkspace::conventions::status::*;
    let status = LkStatus {
        domain,
        group,
        objtype: &objtype,
        instance: instance.as_deref(),
        qid: b"<lk set status>"
    };

    let query : Query= lk_status_overwatch(status, timeout.stamp()).unwrap().into();
    if print_query { println!("{}",query); return Ok(())}
    if !write_request.is_empty(){
        let mut out = common.open(&write_request)?;
        let req = lk_status_request(status)?;
        common.write_multi_dest(&mut out, &req, None)?;
        return Ok(())
    }

    let out = common.open(&write)?;
    let lk : linkspace::Linkspace= common.runtime()?.into();
    let mut write= common.clone().multi_writer(out);

    let ok = Rc::new(std::cell::Cell::new(false));
    let isokc = ok.clone();

    lk_status_watch(&lk,status, timeout.stamp(), try_cb(move |pkt,lk| -> ControlFlow<()>{
        if pkt.get_links().is_empty() || pkt.data().is_empty(){
            panic!()
        }
        isokc.set(true);
        write.handle_pkt(&pkt, lk.as_impl())?;
        if multi { ControlFlow::Continue(())}else {ControlFlow::Break(())}
    }))?;
    // We only have a single watch. Will be dropped after recv predicate becomes imposible.
    lk_process_while(&lk,None, Stamp::ZERO)?;
    anyhow::ensure!(ok.get(),"no resposne after {:?}",timeout);
    Ok(())
}
