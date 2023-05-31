// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    once_cell_try,
    iterator_try_collect,
    write_all_vectored,
    can_vector,
    lazy_cell,
    control_flow_enum,
    type_alias_impl_trait,
    io_error_other,
    exit_status_error,
    unix_sigpipe
)]
use std::{
    cell::LazyCell,
    ffi::OsString,
    io::{ Write},
    process::ExitCode, 
};

use anyhow::{ensure };
use linkspace::query::PredicateType;
use linkspace_common::{
    cli::{
        clap,
        clap::Parser,
        keys,
        opts::{CommonOpts, LinkspaceOpts, PktIn},
        tracing, WriteDestSpec, read_data::ReadOpt,
    },
    core::{
        mut_header::{MutFieldExpr, NetHeaderMutate},
    },
    prelude::{
        predicate_type::PredInfo,
        query_mode::{Mode, Order, Table},
        *,
    }, predicate_aliases::ExtWatchCLIOpts,
};
use tracing_subscriber::EnvFilter;
use watch::{DGPDWatchCLIOpts, CLIQuery};

pub mod collect;
pub mod filter;
pub mod multi_watch;
pub mod point;
pub mod printf;
pub mod rewrite;
pub mod save;
pub mod status;
pub mod watch;
pub mod get_links;
pub mod datapoint;

// const is wrong but who cares.
const QUERY_HELP: LazyCell<String> = LazyCell::new(|| {
    use std::fmt::Write;
    let mut st: String = "\n".into();
    for f in PredicateType::ALL {
        let PredInfo {
            name,
            help,
            example,
            implies,
        } = f.info();
        let _ = write!(st, "{name: <12} - {help} e.g. {example}");
        if implies.is_empty() {
            //let _ = write!(st, " [{implies}]");
        }
        let _ = writeln!(st, "");
    }
    st.push_str("\nThe following options are available\n\n");
    for f in KnownOptions::iter_all() {
        let _ = writeln!(st, "\t:{f}");
    }
    st
});
const PKT_HELP: LazyCell<String> = LazyCell::new(|| {
    let ctx = LinkspaceOpts::fake_eval_ctx();
    let pctx = pkt_ctx(ctx, &*PUBLIC_GROUP_PKT);
    let v = eval(&pctx, &abev!({ "help" })).unwrap().concat();
    String::from_utf8(v).unwrap()
});

/**
linkspace-cli exposes most library functions as well as some utility functions.
You should have read the guide to understand the following concepts:

Commands taking a WriteDestSpec can set the destination of the packets they create and can be set multiple times:

lk link :: --write db --write stdout --write stderr --write file:./file

Most commands are used in a pipeline and read packets from stdin.
**/
#[derive(Parser)]
#[clap(author, about,version)]
struct Cli {
    #[clap(flatten)]
    common: CommonOpts,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Parser)]
enum Command {
    /// points - create a new datapoint
    #[clap(alias = "d", alias = "data")]
    Datapoint{
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        read_opts: ReadOpt
    },
    /** points - create a new linkpoint

    Use --read-empty by default
    */
    #[clap(alias = "l", alias = "link")]
    Linkpoint {
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        multi:point::MultiOpts,
        #[clap(flatten)]
        link: point::PointOpts,
    },
    /** points - create a new keypoint

    Use --read-empty by default
    */
    #[clap(alias = "keyp")]
    Keypoint {
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        multi:point::MultiOpts,
        #[clap(flatten)]
        link: point::PointOpts,
    },

    #[clap(alias = "e")]
    /** abe - eval ABE expression

    The abe syntax can be found in the guide (https://www.linkspace.dev/docs/guide/index.html#ABE)
    Use "[help]" for a list of functions.
    */
    Eval {
        /// output json ABList format 
        #[clap(long)]
        json: bool,
        abe: String,
        /// Read stdin as argv [0]
        #[clap(long,default_value_t)]
        stdin: bool
    },
    /** abe   - eval expression for each pkt from stdin

    The abe syntax can be found in the guide (https://www.linkspace.dev/docs/guide/index.html#ABE)
    */
    #[clap(alias="p",before_long_help=PKT_HELP.to_string())]
    Printf(printf::PrintFmtOpts),
    /// abe   - encode input into abe
    #[clap(alias = "n")]
    Encode {
        #[clap(short,long,action=clap::ArgAction::Count)]
        ignore_err: u8,
        /// a set of '/' delimited options
        #[clap(default_value = "@/#/b:32:64/")]
        opts: String,
    },

    /// query - print full query from common aliases
    #[clap(alias="pq",alias="print-predicate",before_help=QUERY_HELP.to_string())]
    PrintQuery {
        #[clap(flatten)]
        opts: CLIQuery,
    },

    /// runtime - create a new instance (alternative to the '--init' argument)
    Init,

    /** runtime - save packets from stdin to database

    WARN : By default this will increment hop.
    This indicate it was not created locally.
    Usually this means packets are excluded from "all locally created data"
    when syncing.

    Set --hop false or use the '--write db' destination when creating the packets .
     **/
    Save(save::SaveForward),
    /// runtime - get packets matching query
    Watch {
        #[clap(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        watch: watch::CLIQuery,
    },
    /// runtime - watch all packets with the same locaiton prefix. alias for: watch --mode tree-desc 'dom:grp:path:**'
    WatchTree {
        #[clap(short, long)]
        asc: bool,
        #[clap(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        query: watch::CLIQuery,
    },
    /// runtime - alias for: watch --mode hash-asc -- hash:=:HASH
    WatchHash {
        hash: HashExpr,
        #[clap(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        rest : ExtWatchCLIOpts 
    },
    /// runtime - alias for: watch --mode log-desc
    WatchLog {
        #[clap(short, long)]
        asc: bool,
        #[clap(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        query: watch::CLIQuery,
    },
    /// runtime - read a stream of queries
    MultiWatch(multi_watch::MultiWatch),
    /// convention - generate / print a signing key
    Key(keys::KeyGenOpts),
    /// convention - create a pull request
    Pull {
        #[clap(short, long, default_value = "db")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        watch: DGPDWatchCLIOpts,
    },
    /// convention - creates a status update request (or checks for a recent one)
    PollStatus(status::PollStatus),
    /// convention - reply to status update requests
    SetStatus(status::SetStatus),

    /// rewrite packets
    Rewrite(rewrite::Rewrite),
    /// create a linkpoint with links to incoming packets
    Collect(collect::Collect),

    /// filter a stream of packets based on a query
    Filter(filter::Filter) ,
    /// deduplicate packets based on hash
    Dedup {
        #[clap(long, default_value_t = 256)]
        capacity: usize,
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        pkt_in: PktIn
    },
    /// mutate the netheader of packets
    Route {
        #[clap(flatten)]
        pkt_in: PktIn,
        field_mut: Vec<MutFieldExpr> ,
    },
    /// Output known link.ptr packets ( this is not the same as setting :follow )
    GetLinks(get_links::GetLinks),
    /// queue datapackets until a linkpoint with a matching link is received
    DataFilter {
        #[clap(short, long, default_value = "4090")]
        buffer_size: usize,
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(short, long, default_value = "null")]
        dropped: Vec<WriteDestSpec>,
    },
    #[cfg(target_family = "unix")]
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[unix_sigpipe = "sig_dfl"]
fn main() -> std::process::ExitCode {
    let r = std::panic::catch_unwind(|| {
        let env_filter = EnvFilter::builder()
            .with_default_directive(tracing::metadata::LevelFilter::WARN.into())
            .from_env()?;
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr)
            .init();
        let Cli { common, command } = Cli::parse();
        run(command, common)
    });
    match r {
        Ok(Ok(())) => ExitCode::SUCCESS,
        Ok(Err(e)) => {
            let args = std::env::args();
            if let Some(io) = e.downcast_ref::<std::io::Error>() {
                if io.kind() == std::io::ErrorKind::BrokenPipe {
                    eprintln!("pipe closed ({:?})", args);
                    return ExitCode::SUCCESS;
                }
            }
            eprintln!("error: {:?}", args);
            eprintln!("{:#?}", e);
            ExitCode::FAILURE
        }
        Err(e) => {
            let args = std::env::args();
            eprintln!("Panic! :Original {:#?}", args);
            std::panic::resume_unwind(e);
        }
    }
}
fn run(command: Command, mut common: CommonOpts) -> anyhow::Result<()> {
    match command {
        Command::Datapoint{write,read_opts} => {
            crate::datapoint::write_datapoint(write, &common, read_opts)?;
        }
        Command::Save(opts) => {
            crate::save::save(opts, common)?;
        }
        Command::Printf(opts) => {
            crate::printf::pkt_info(common, opts)?;
        }
        Command::Encode { opts, ignore_err } => {
            use std::io::Read;
            let mut bytes = vec![];
            std::io::stdin().read_to_end(&mut bytes)?;
            tracing::trace!(?bytes);
            let ctx = common.eval_ctx();
            let r = linkspace_common::abe::eval::encode(&ctx, &bytes, &opts,ignore_err > 0 );
            if ignore_err > 1 && r.is_err() {
                std::io::stdout().write_all(abtxt::as_abtxt(&bytes).as_bytes())?;
            }else {
                let r = r?;
                std::io::stdout().write_all(r.as_bytes())?;
            }
        }
        Command::Linkpoint { write, link, multi } => {
            let mut write = common.open(&write)?;
            point::linkpoint(common, link,multi, &mut write)?;
        }
        Command::Keypoint { write, mut link, multi } => {
            link.sign = true;
            let mut write = common.open(&write)?;
            point::linkpoint(common, link, multi, &mut write)?
        }
        Command::Key(opts) => keys::keygen(&common, opts)?,
        Command::DataFilter { .. } => {
            todo!("Use before/after link NetFlags");
            /*
            let inp = common.reader()?;
            let mut out = stdout();
            let mut buffer = linkspace::databuffer::Buffer::default();
            for p in inp {
                let released = buffer.push(p?);
                for pkt in released {
                    common.write(&mut out, pkt)?;
                }
            }
            */
        }
        Command::Dedup { capacity, write, pkt_in } => {
            common.enable_private_group();
            let inp = common.inp_reader(&pkt_in)?;
            let mut deduper = linkspace_common::pkt_stream_utils::QuickDedup::new(capacity);
            let mut dest = common.open(&write)?;
            for p in inp {
                let p = p?;
                let is_old = deduper.probable_contains(p.hash());
                if !is_old {
                    common.write_multi_dest(&mut dest, &**p, None)?;
                }
            }
        }
        Command::Collect(cmd) => collect::collect(&common, cmd)?,
        Command::Rewrite(cmd) => rewrite::rewrite(&common, cmd)?,
        Command::GetLinks(cmd) => get_links::exec(common,cmd)?,
        Command::WatchHash { hash, write, rest } => {
            let mut cquery = CLIQuery::default();
            cquery.mode = Some(Mode::HASH_ASC);
            cquery.opts.watch_opts = rest;
            let hpred = abev!( "hash" : "=" : +(hash.0.clone()));
            cquery.opts.watch_opts.exprs.push( hpred.into());
            watch::watch(common, cquery, write)?;
        }
        Command::WatchTree { mut query, asc, write } => {
            if let Some(dgpd) = &mut query.opts.dgpd {
                if dgpd.subsegment_limit == 0 {
                    dgpd.subsegment_limit = 255;
                }
            }
            watch::watch(
                common,
                query.mode(Mode {
                    table: Table::Tree,
                    order: Order::asc(asc),
                }),
                write
            )?
        }
        Command::WatchLog { query, asc, write } => watch::watch(
            common,
            query.mode(Mode {
                table: Table::Log,
                order: Order::asc(asc),
            }),
            write
        )?,
        Command::Watch { watch, write } => watch::watch(common, watch,write)?,
        Command::Filter(filter) => filter::select(common, filter)?,
        Command::Eval { json, abe ,stdin} => {
            let abe = parse_abe(&abe)?;

            use std::io::Read;
            let mut argv = vec![];
            let mut bytes = vec![];
            if stdin{
                std::io::stdin().read_to_end(&mut bytes)?;
                tracing::trace!(?bytes);
                argv.push(bytes.as_slice());
            }
            let argv = ArgV::try_fit(&argv).unwrap();

            let ctx = common.eval_ctx();
            let ctx = ctx.scope(EScope(argv));
            let val = eval(&ctx, &abe)?;
            let mut out = std::io::stdout();
            if json {
                use serde_json::{to_value,value::Value};
                let mut lst = val.inner().iter()
                    .map(|(b,v)| (String::from_utf8(b.clone()).map(Value::String)
                                  .unwrap_or_else(|_|to_value(b).unwrap()),v))
                    .map(to_value);
                let vec = Value::Array(lst.try_collect()?);
                println!("{vec}");
            } else {
                out.write_all(&val.concat())?;
            }
            out.flush()?;
        }
        Command::MultiWatch(mv) => multi_watch::multi_watch(common, mv)?,
        Command::Route { field_mut , pkt_in} => {
            let muth = NetHeaderMutate::from_lst(&field_mut, &common.eval_ctx())?;
            common.enable_private_group();
            common.io.inp.no_check = true;
            let inp = common.inp_reader(&pkt_in)?;
            let mut out = WriteDestSpec::stdout().open(&common.eval_ctx())?.unwrap();
            for p in inp {
                let mut p = p?;
                muth.mutate(&mut p._net_header);
                common.write_dest(&mut out, &**p, &mut None)?;
            }
        }

        Command::PrintQuery { mut opts } => {
            if !opts.print.do_print(){ opts.print.print_expr = true;}
            let _ = opts.into_query(&common)?;
        }
        Command::External(args) => {
            let name = format!("linkspace-{}", args[0].to_str().unwrap());
            let mut cmd = std::process::Command::new(&name);
            cmd.args(&args[1..]);
            #[cfg(target_family = "unix")]
            if true {
                use std::os::unix::process::CommandExt;
                tracing::info!("Calling {:?} - {:?}", name, &args[1..]);
                let e = cmd.exec();
                anyhow::bail!("External arg failure {name} not found - {:?}", e);
            }
            println!("exec not supported")
        }
        Command::Pull { write, mut watch } => {
            let ctx = common.eval_ctx();
            watch.watch_opts.aliases.watch = true;
            ensure!(watch.dgpd.is_some(), "DGSD required for pull request");
            let query = watch.into_query(&ctx)?;
            let req = linkspace::conventions::lk_pull_req(&query.into())?;
            *common.mut_write_private() = Some(true);
            let mut write = common.open(&write)?;
            common.write_multi_dest(&mut write, &req, None)?;
        }
        Command::PollStatus(ps) => status::poll_status(common, ps)?,
        Command::SetStatus(ss) => status::set_status(common, ss)?,
        Command::Init => {
            common.linkspace.init = true;
            let lk = common.runtime()?.into();
            let x = linkspace::runtime::lk_info(&lk);
            println!("{:?}",x);
        },
    }
    Ok(())
}
