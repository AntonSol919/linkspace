// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(dead_code, unused_variables)]
use std::ops::ControlFlow;

use liblinkspace::misc::{ cb};
use liblinkspace::{ lk_process_while };
use linkspace_common::cli::{clap };
use linkspace_common::runtime::handlers::PktStreamHandler;
use linkspace_common::{
    cli::{clap::Parser, opts::CommonOpts, ReadSource, WriteDestSpec},
    core::stamp_fmt::DurationStr,
};

use linkspace_common::prelude::*;

#[derive(Parser, Debug)]
pub struct StatusArgs {
    pub domain: DomainExpr,
    pub group: GroupExpr,
    pub objtype: TypedABE<Vec<u8>>,
    pub instance: Option<TypedABE<Vec<u8>>>,
}
impl StatusArgs {
    pub fn eval(
        self,
        ctx: &EvalCtx<impl Scope>,
    ) -> anyhow::Result<(Domain, GroupID, Vec<u8>, Option<Vec<u8>>)> {
        Ok((
            self.domain.eval(ctx)?,
            self.group.eval(ctx)?,
            self.objtype.eval(ctx)?,
            self.instance.map(|v| v.eval(ctx)).transpose()?,
        ))
    }
}

#[derive(Parser, Debug)]
pub struct SetStatus {
    #[clap(flatten)]
    args: StatusArgs,
    /// the status data.
    #[clap(short,long,default_value = "abe:OK")]
    data: Option<ReadSource>,
    #[clap(short,long)]
    link: Vec<LinkExpr>,
}
pub fn set_status(common: CommonOpts,ss: SetStatus) -> anyhow::Result<()> {
    let SetStatus { args, data, link } = ss;
    let ctx = common.eval_ctx();
    let (domain, group, objtype, instance) = args.eval(&ctx)?;
    use liblinkspace::prelude::*;
    use liblinkspace::conventions::status::*;
    let status = LkStatus {
        domain,
        group,
        objtype: &objtype,
        instance: instance.as_deref().or(Some(b"default")),
    };
    let lk : Linkspace = common.runtime()?.into();
    let c= common.clone();

    let mut data_reader = common.open_read(data.as_ref())?;
    lk_status_set(&lk, status, move |_,domain,group,path,link| {
        let mut buf = vec![];
        let data = data_reader(&c.eval_ctx().dynr(),&mut buf)?;
        lk_linkpoint(domain, group, path, &[link], data, None)
    })?;
    lk_process_while(&lk, Stamp::ZERO,Stamp::ZERO)?;

    return Ok(())
}

#[derive(Parser)]
pub struct PollStatus {
    #[clap(flatten)]
    args: StatusArgs,
    /// wait for this duration (since last request) before returning an error 
    #[clap(short, long, default_value = "5s")]
    timeout: DurationStr,
    #[clap(short, long, default_value = "stdout")]
    write: Vec<WriteDestSpec>,
    /// Output multiple replies (until last_req+duration)
    #[clap(short,long)]
    multi: bool,
    /// only output query before quiting
    #[clap(long)]
    print_query: bool,
    /// only output the request packet
    #[clap(long)]
    write_request: Vec<WriteDestSpec>,
}
pub fn poll_status(mut common: CommonOpts, ps: PollStatus) -> anyhow::Result<()> {
    let PollStatus {
        args,
        timeout,
        write,
        print_query,
        write_request,
        multi,
    } = ps;
    *common.mut_write_private() = Some(true);
    let ctx = common.eval_ctx();
    let (domain, group, objtype, instance) = args.eval(&ctx)?;
    let timeout = timeout;
    use liblinkspace::conventions::status::*;
    let status = LkStatus {
        domain,
        group,
        objtype: &objtype,
        instance: instance.as_deref(),
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
    let lk : liblinkspace::Linkspace= common.runtime()?.into();
    let mut write= common.clone().multi_writer(out);

    let ok = Rc::new(std::cell::Cell::new(false));
    let isokc = ok.clone();

    lk_status_poll(&lk, status, timeout.stamp(), cb(move |pkt,lk| -> ControlFlow<()>{
        if pkt.get_links().is_empty() || pkt.data().is_empty(){
            panic!()
        }
        isokc.set(true);
        write.handle_pkt(&pkt, lk.as_impl())?;
        if multi { ControlFlow::Continue(())}else {ControlFlow::Break(())}
    }))?;
    // We only have a single watch. Will be dropped after recv predicate becomes imposible.
    lk_process_while(&lk, Stamp::ZERO,Stamp::ZERO)?;
    anyhow::ensure!(ok.get(),"no resposne after {:?}",timeout);
    Ok(())
}
