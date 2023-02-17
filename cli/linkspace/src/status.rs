// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![allow(dead_code, unused_variables)]
use linkspace_common::cli::clap;
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
    #[clap(default_value = "abe:OK")]
    data: Option<ReadSource>,
}
pub fn set_status(_common: CommonOpts, _ss: SetStatus) -> anyhow::Result<()> {
    /*
    use liblinkspace::conventions::status::*;
    let ctx = common.eval_ctx();
    let SetStatus { args, data } = ss;
    let (domain,group,objtype,instance) = args.eval(&ctx)?;
    let status = LkStatus { domain, group, objtype: &objtype, instance: instance.as_deref() };
    let instance_path = lk_status_path(&status)?;
    let obj_path = lk_status_path(&LkStatus { instance:None,..status})?;

    let init = lk_linkpoint_ref(domain, LOCAL_ONLY_GROUP, &path, &[], &value, None)?;
    let init = lk_linkpoint_ref(domain, LOCAL_ONLY_GROUP, &instance_path, &[], data, None)?;
    */

    Ok(())
}

#[derive(Parser)]
pub struct PollStatus {
    #[clap(flatten)]
    args: StatusArgs,
    /// wait for this duration (since last request) before returning an error
    #[clap(short, long, default_value = "5s")]
    max_duration: DurationStr,
    #[clap(short, long, default_value = "db")]
    write: Vec<WriteDestSpec>,
    /// only output query before quiting
    #[clap(long)]
    print_query: bool,
    /// only output the request packet
    #[clap(long)]
    print_request: bool,
}
pub fn poll_status(common: CommonOpts, ps: PollStatus) -> anyhow::Result<()> {
    let PollStatus {
        args,
        max_duration,
        write,
        print_query,
        print_request,
    } = ps;
    let ctx = common.eval_ctx();
    let (domain, group, objtype, instance) = args.eval(&ctx)?;
    let max_duration = max_duration;
    use liblinkspace::conventions::status::*;
    let status = LkStatus {
        domain,
        group,
        objtype: &objtype,
        instance: instance.as_deref(),
    };
    todo!()

    /*
    let mut query :Query= lk_status_query(&status, max_duration.stamp()).unwrap().into();
    if print_query { println!("{}",query); return Ok(())}
    if print_request{
        let req = lk_status_request(&status).unwrap();
        common.write_multi_dest(&write, &req, None)?;
        return Ok(())
    }
    let lk = common.runtime()?;
    let mut ok = false;
    let mut has_request =false;
    {
        let r = lk.get_reader();
        let it = r.query_tree(Order::Asc, &query.predicates);

        for p in it {
            if p.get_links().is_empty() && p.data().is_empty() {
                has_request = true; continue;
            }
            common.write_multi_dest(&write, &p, None)?;
            ok = true;
        }
        if ok { return Ok(())}
    }
    if !has_request{
        let req = lk_status_request(&status).unwrap();
        common.write_dest(&WriteDest::Db, &req, &mut None)?;
    }
    query.predicates.add_predicate(&Predicate { kind: FieldEnum::DataSizeF.into(), op: TestOp::Greater, val: U16::ZERO.into() })?;
    query.predicates.add_predicate(&Predicate { kind: FieldEnum::LinksLenF.into(), op: TestOp::Greater, val: U16::ZERO.into() })?;
    let mut w = common.multi_writer_dyn(write);
    let ok = Rc::new(std::cell::Cell::new(false));
    let isokc = ok.clone();
    lk.view_query(&query, move |pkt:&dyn NetPkt,lk:&Linkspace| {
        isokc.set(true);
        w.handle_pkt(pkt, lk)
    }, debug_span!("await status"))?;
    lk.run_while(None,None)?;
    ensure!(ok.get(),"no resposne after {:?}",max_duration);
    Ok(())
        */
}
