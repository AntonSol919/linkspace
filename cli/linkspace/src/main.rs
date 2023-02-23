// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    iterator_try_collect,
    write_all_vectored,
    can_vector,
    once_cell,
    control_flow_enum,
    type_alias_impl_trait,
    io_error_other,
    exit_status_error,
    unix_sigpipe
)]
use std::{
    cell::LazyCell,
    ffi::OsString,
    io::{stdin, Write},
    process::ExitCode,
};

use anyhow::{ensure, Context};
use liblinkspace::query::PredicateType;
use linkspace_common::{
    cli::{
        clap,
        clap::Parser,
        keys,
        opts::{CommonOpts, LinkspaceOpts},
        tracing, WriteDestSpec,
    },
    core::{
        mut_header::{MutFieldExpr, NetHeaderMutate},
    },
    prelude::{
        predicate_type::PredInfo,
        query_mode::{Mode, Order, Table},
        *,
    },
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
            let _ = write!(st, " [{implies}]");
        }
        let _ = writeln!(st, "");
    }
    st.push_str("\nThe following options are available\n\n");
    for f in KnownOptions::iter_all() {
        let _ = writeln!(st, "\t/{f}");
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
#[clap(author, about)]
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
    },
    /// points - create a new linkpoint
    #[clap(alias = "l", alias = "link")]
    Linkpoint {
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        link: point::PointOpts,
    },
    /// points - create a new keypoint
    #[clap(alias = "keyp")]
    Keypoint {
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[clap(flatten)]
        link: point::PointOpts,
    },

    #[clap(alias = "e")]
    /// abe   - eval ABE expression
    Eval {
        #[clap(long)]
        json: bool,
        abe: String,
    },
    /// abe   - eval expression for each pkt from stdin
    #[clap(alias="p",before_long_help=PKT_HELP.to_string())]
    Printf(printf::PrintFmtOpts),
    /// abe   - encode input into abe
    #[clap(alias = "n")]
    Encode {
        /// a set of '/' delimited options
        #[clap(default_value = "@/#/@local/#local/b:32:64")]
        opts: String,
    },

    /// query - print full query from common aliases
    #[clap(alias="pq",alias="print-predicate",before_help=QUERY_HELP.to_string())]
    PrintQuery {
        #[clap(flatten)]
        opts: CLIQuery,
    },

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
        #[clap(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        hash: HashExpr,
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
    Filter {
        #[clap(flatten)]
        query: watch::CLIQuery,
        #[clap(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        /// destination for filtered packets
        #[clap(short = 'f', long, default_value = "null")]
        write_false: Vec<WriteDestSpec>,
    },
    /// deduplicate packets based on hash
    Dedup {
        #[clap(long, default_value_t = 256)]
        capacity: usize,
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
    },
    /// mutate the netheader of packets
    Route { field_mut: Vec<MutFieldExpr> },
    /// Output known link.ptr packets ( this is not the same as setting :follow )
    GetLinks {
        /// writedest of stdin packets
        #[clap(short, long, default_value = "stdout")]
        forward: Vec<WriteDestSpec>,
        /// writedest of linked packets
        #[clap(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
    },
    /// queue datapackets untill a linkpoint with a matching link is received
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
            eprintln!("{:?}", e);
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
        Command::Datapoint{write} => {
            let mut write = common.open(&write)?;
            let inp = stdin();
            linkspace_common::protocols::impex::chunk_reader_try_fold::<_, _, _, MAX_DATA_SIZE>(
                inp,
                (),
                |(), buf| {
                    let pkt = datapoint(&buf, ());
                    common.write_multi_dest(&mut write, &pkt, None)
                },
            )?;
        }
        Command::Save(opts) => {
            crate::save::save(opts, common)?;
        }
        Command::Printf(opts) => {
            crate::printf::pkt_info(common, opts)?;
        }
        Command::Encode { opts } => {
            use std::io::Read;
            let mut bytes = vec![];
            std::io::stdin().read_to_end(&mut bytes)?;
            let ctx = common.eval_ctx();
            let r = linkspace_common::abe::eval::encode(&ctx, &bytes, &opts)?;
            std::io::stdout().write_all(r.as_bytes())?;
        }
        Command::Linkpoint { write, link } => {
            let mut write = common.open(&write)?;
            point::linkpoint(common, link, &mut write)?;
        }
        Command::Keypoint { write, mut link } => {
            link.sign = true;
            let mut write = common.open(&write)?;
            point::linkpoint(common, link, &mut write)?
        }
        Command::Key(opts) => keys::keygen(&common, opts)?,
        Command::DataFilter { .. } => {
            todo!("Use before/after link NetFlags");
            /*
            let inp = common.reader()?;
            let mut out = stdout();
            let mut buffer = liblinkspace::databuffer::Buffer::default();
            for p in inp {
                let released = buffer.push(p?);
                for pkt in released {
                    common.write(&mut out, pkt)?;
                }
            }
            */
        }
        Command::Dedup { capacity, write } => {
            common.enable_private();
            let inp = common.inp_reader()?;
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
        Command::GetLinks { forward, write } => {
            let store = common.env()?;
            let inp = common.inp_reader()?;
            // FIXME : set proper Linked Futer Packets
            //let netflag = if prepend_links { NetFlag::LinkedInFuturePkt}else { NetFlag::LinkedInPreviousPkt}.into();
            let mut write = common.open(&write)?;
            let mut forward = common.open(&forward)?;
            let mut buffer = vec![];
            for pkt in inp {
                let pkt = pkt?;
                if !pkt.get_links().is_empty() {
                    let reader = store.get_reader()?;
                    for Link {
                        tag: _,
                        ptr: pointer,
                    } in pkt.get_links()
                    {
                        if let Some(pkt) = reader.read(&pointer)? {
                            common.write_multi_dest(&mut write, &pkt.pkt, Some(&mut buffer))?;
                        }
                    }
                }
                common.write_multi_dest(&mut forward, &**pkt, Some(&mut buffer))?;
                if !buffer.is_empty() {
                    let mut out = std::io::stdout();
                    out.write_all(&mut buffer)?;
                    out.flush()?;
                    buffer.clear();
                }
            }
        }
        Command::WatchHash { hash, write } => {
            let env = common.env()?;
            let hash = hash.eval(&common.eval_ctx())?;
            let r = env.get_reader()?;
            let pkt = r.read(&hash)?.context("Pkt not in db")?;
            common.write_multi_dest(&mut common.open(&write)?, &pkt.pkt, None)?;
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
        Command::Filter { query, write_false, write } => {
            filter::select(query, write,write_false, common)?
        }
        Command::Eval { json, abe } => {
            let abe = parse_abe(&abe)?;
            let ctx = common.eval_ctx();
            let val = eval(&ctx, &abe)?;
            let mut out = std::io::stdout();
            if json {
                serde_json::ser::to_writer(&mut out, &val)?;
            } else {
                out.write_all(&val.concat())?;
            }
            out.flush()?;
        }
        Command::MultiWatch(mv) => multi_watch::multi_watch(common, mv)?,
        Command::Route { field_mut } => {
            let muth = NetHeaderMutate::from_lst(&field_mut, &common.eval_ctx())?;
            common.enable_private();
            common.io.inp.no_check = true;
            let inp = common.inp_reader()?;
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
            watch.watch_opts.opts.aliases.watch = true;
            ensure!(watch.dgpd.is_some(), "DGSD required for pull request");
            let query = watch.watch_predicates(&ctx)?;
            let req = liblinkspace::conventions::lk_pull_req(&query.into())?;
            *common.mut_write_private() = Some(true);
            let mut write = common.open(&write)?;
            common.write_multi_dest(&mut write, &req, None)?;
        }
        Command::PollStatus(ps) => status::poll_status(common, ps)?,
        Command::SetStatus(ss) => status::set_status(common, ss)?,
    }
    Ok(())
}


