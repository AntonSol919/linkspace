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
    exit_status_error,
    unix_sigpipe
)]
use std::{
    ffi::OsString,
    io::{ Write},
    process::ExitCode, sync::{ LazyLock}, path::PathBuf, 
};

use anyhow::{ensure };
use linkspace::{query::PredicateType };
use linkspace_common::{
    cli::{
        clap,
        clap::Parser,
        keys,
        opts::{CommonOpts, LinkspaceOpts },
        tracing, WriteDestSpec, reader::{DataReadOpts, PktReadOpts},
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
use point::{PointOpts, GenPointOpts};
use tracing_subscriber::EnvFilter;
use watch::{DGPDWatchCLIOpts, CLIQuery};

pub mod collect;
pub mod filter;
pub mod multi_watch;
pub mod point;
pub mod pktf;
pub mod rewrite;
pub mod save;
pub mod status;
pub mod watch;
pub mod get_links;
pub mod datapoint;
pub mod eval;

static QUERY_HELP: LazyLock<String> = LazyLock::new(|| {
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
static PKT_HELP: LazyLock<String> = LazyLock::new(|| {
    let scope = LinkspaceOpts::fake_eval_scope();
    let pscope = (scope,pkt_scope(&*PUBLIC_GROUP_PKT));
    let v = eval(&pscope, &abev!({ "help" })).unwrap().concat();
    String::from_utf8(v).unwrap()
});

pub static BUILD_INFO : &'static str = concat!(
    env!("CARGO_PKG_NAME")," - ",
    env!("CARGO_PKG_VERSION")," - ",
    env!("VERGEN_GIT_BRANCH")," - ",
    env!("VERGEN_GIT_DESCRIBE"), " - ", 
    env!("VERGEN_RUSTC_SEMVER")
);


/**
linkspace-cli exposes most library functions as well as some utility functions.
You should have read the guide to understand the following concepts:

Commands taking a WriteDestSpec can set the destination of the packets they create and can be set multiple times:

lk link :: --write db --write stdout --write stderr --write file:./file

Most commands are used in a pipeline and read packets from stdin.
**/

#[derive(Parser)]
#[command(author, about,version,long_version=BUILD_INFO)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    common: CommonOpts,
}

#[derive(Parser)]
enum Command {
    
    /// points - create a new datapoint
    #[command(alias = "d", alias = "data")]
    Datapoint{
        #[arg(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten,next_help_heading="Data Options")]
        read_opts: DataReadOpts
    },
    /** points - create a new linkpoint

    Use --no-data by default
    */
    #[command(alias = "l", alias = "link")]
    Linkpoint {
        #[arg(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        multi:point::MultiOpts,
        #[command(flatten)]
        link: point::PointOpts,
    },
    /** points - create a new keypoint

    Use --no-data by default
    */
    #[command(alias = "keyp")]
    Keypoint {
        #[arg(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        multi:point::MultiOpts,
        #[command(flatten)]
        link: point::PointOpts,
    },
    /// points - create a new point - detect what kind of point - prefer to be exact by using 'data', 'link', or 'keyp' 
    Point{
        #[arg(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        point: point::GenPointOpts,
    },
    #[command(alias = "e")]
    /** abe - eval ABE expression

    The abe syntax can be found in the guide (https://www.linkspace.dev/guide/index.html#ABE)
    Use "[help]" for a list of functions.
    */
    Eval(eval::EvalOpts),
    /** abe   - eval expression for each pkt from stdin 

    The abe syntax can be found in the guide (https://www.linkspace.dev/guide/index.html#ABE)
    */
    #[command(alias="p",before_long_help=&*PKT_HELP)]
    Pktf(pktf::PktFmtOpts),
    /// abe   - encode input into abe
    #[command(alias = "n")]
    Encode {
        #[arg(short,long,action=clap::ArgAction::Count)]
        ignore_err: u8,
        /// a set of '/' delimited options
        #[arg(default_value = "@/#/b:32:64/")]
        opts: String,
    },

    /// query - print full query from common aliases
    #[command(alias="q",alias="print-predicate",before_help=&*QUERY_HELP)]
    PrintQuery {
        #[command(flatten)]
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
        #[arg(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        watch: watch::CLIQuery,
    },
    /// runtime - watch all packets with the same space prefix. alias for: watch --mode tree-desc 'dom:grp:space:**'
    WatchTree {
        #[arg(short, long)]
        asc: bool,
        #[arg(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        query: watch::CLIQuery,
    },
    /// runtime - alias for: watch --mode hash-asc -- hash:=:HASH
    WatchHash {
        hash: HashExpr,
        #[arg(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        rest : ExtWatchCLIOpts 
    },
    /// runtime - alias for: watch --mode log-desc
    WatchLog {
        #[arg(short, long)]
        asc: bool,
        #[arg(long, short, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        query: watch::CLIQuery,
    },
    /// runtime - read a stream of queries
    MultiWatch(multi_watch::MultiWatch),
    /// convention - generate / print a signing key
    Key(keys::KeyGenOpts),
    /// convention - create a pull request
    Pull {
        #[arg(short, long, default_value = "db")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        watch: DGPDWatchCLIOpts,
    },
    
    Status{#[command(subcommand)] cmd: status::StatusCmd},

    /// rewrite packets
    Rewrite(rewrite::Rewrite),
    /// create a linkpoint with links to incoming packets
    Collect(collect::Collect),

    /// filter a stream of packets based on a query
    Filter(filter::Filter) ,
    /// alias to 'filter' with the --write and --write-false argument swapped.
    Ignore(filter::Filter) ,

    /// deduplicate packets based on hash
    Dedup {
        #[arg(long, default_value_t = 256)]
        capacity: usize,
        #[arg(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[command(flatten)]
        pkt_in: PktReadOpts
    },
    /// mutate the netheader of packets
    Route {
        #[command(flatten)]
        pkt_in: PktReadOpts,
        field_mut: Vec<MutFieldExpr> ,
    },
    /// Get and write all link.ptr points
    GetLinks(get_links::GetLinks),
    /// queue datapackets until a linkpoint with a matching link is received
    DataFilter {
        #[arg(short, long, default_value = "4090")]
        buffer_size: usize,
        #[arg(short, long, default_value = "stdout")]
        write: Vec<WriteDestSpec>,
        #[arg(short, long, default_value = "null")]
        dropped: Vec<WriteDestSpec>,
    },
    DbCheck,
    DbImport {
        file: PathBuf
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
        let args = std::env::args();
        tracing::trace!(?args);
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
        Command::Pktf(opts) => {
            crate::pktf::pkt_info(common, opts)?;
        }
        Command::Encode { opts, ignore_err } => {
            use std::io::Read;
            let mut bytes = vec![];
            std::io::stdin().read_to_end(&mut bytes)?;
            tracing::trace!(?bytes);
            let scope = common.eval_scope();
            let r = linkspace_common::abe::eval::encode(&scope, &bytes, &opts,ignore_err > 0 );
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
        Command::Point { write, mut point} =>{
            match point.dgs{
                None => {
                    let data = std::mem::take(&mut point.read);
                    crate::datapoint::write_datapoint(write, &common, data)?;
                },
                Some(dgs) =>{
                    let mut write = common.open(&write)?;
                    let GenPointOpts{ create, create_int, sign, key, read, dgs:_, link } = point;
                    let p = PointOpts{ create, create_int, sign, key, read, dgs, link};
                    point::linkpoint(common, p,Default::default(), &mut write)?;
                },
            }
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
            let mut cquery = CLIQuery{
                mode : Some(Mode::HASH_ASC),
                ..CLIQuery::default()
            };
            cquery.opts.watch_opts = rest;
            let hpred = abev!( "hash" : "=" : +(hash.0));
            cquery.opts.watch_opts.exprs.push( hpred.into());
            watch::watch(common, cquery, write)?;
        }
        Command::WatchTree { query, asc, write } => {
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
        Command::Ignore(mut filter) => {
            std::mem::swap(&mut filter.write_false, &mut filter.write);
            filter::select(common, filter)?
        },
        Command::Eval(eval_opts) => eval::eval_cmd(common, eval_opts)?,
        Command::MultiWatch(mv) => multi_watch::multi_watch(common, mv)?,
        Command::Route { field_mut , pkt_in} => {
            let muth = NetHeaderMutate::from_lst(&field_mut, &common.eval_scope())?;
            common.enable_private_group();
            common.io.inp.skip_hash = true;
            let inp = common.inp_reader(&pkt_in)?;
            let mut out = WriteDestSpec::stdout().open(&common.eval_scope())?.unwrap();
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
            let scope = common.eval_scope();
            watch.watch_opts.aliases.watch = true;
            ensure!(watch.dgpd.is_some(), "DGSD required for pull request");
            let query = watch.into_query(&scope)?;
            let req = linkspace::conventions::pull::lk_pull_point(&query.into())?;
            std::mem::drop(scope);
            *common.mut_write_private() = Some(true);
            let mut write = common.open(&write)?;
            common.write_multi_dest(&mut write, &req, None)?;
        }
        Command::Status{cmd: status::StatusCmd::Watch(w)} => status::status_watch(common, w)?,
        Command::Status{cmd: status::StatusCmd::Set(w)} => status::status_set(common, w)?,
        Command::Init => {
            common.linkspace.init = true;
            let lk = common.runtime()?.into();
            let x = linkspace::runtime::lk_info(&lk);
            println!("{:?}",x);
        },
        Command::DbCheck => {
            let lk = common.runtime()?;
            let env = lk.env();
            println!("{:?}",env.dir());
            println!("{:#?}",env.lmdb_version());
            println!("{:#?}",env.env_info());
            println!("{:#?}",env.db_info());
            println!("real disk size: {:#?}",env.real_disk_size());
            let e = env.linkspace_info();
            if let Err(e) = &e{
                eprintln!("{}",e);
            }else {
                println!("everything ok");
            }
            return e;
        }
        Command::DbImport { file } => {
            let file = std::fs::File::open(file)?;
            let mmap = unsafe { memmap2::Mmap::map(&file)? };
            let lk = common.runtime()?;
            let env = lk.env();
            let mut bytes = mmap.as_ref();
            let mut pkts : Vec<(&NetPktPtr,SaveState)> = vec![];
            while !bytes.is_empty(){
                match read::read_pkt(bytes, common.io.inp.skip_hash)?{
                    std::borrow::Cow::Borrowed(o) => {
                        bytes =&bytes[o.size() as usize..];
                        pkts.push((o,SaveState::Pending));
                    },
                    std::borrow::Cow::Owned(_) => todo!(),
                }
            }
            let i = env.save_ptr( &mut pkts)?;
            for (pkt,state) in &pkts {
                println!("{} {}",pkt.hash_ref(),state)
            }
            println!("read {} - {i:?}",pkts.len());
        },
    }
    Ok(())
}
