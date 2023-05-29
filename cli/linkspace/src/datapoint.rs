use std::{path::PathBuf, io::{BufRead, Stdin, Read}, cell::OnceCell, fs::File};

use anyhow::{ ensure };
use linkspace::consts::MAX_DATA_SIZE;
use linkspace_common::{cli::{clap::{*,self},  opts::CommonOpts, WriteDestSpec}, abe::TypedABE,prelude::*};


#[derive(Parser)]
pub struct ReadOpt{
    #[clap(short, long)]
    read: Option<PathBuf>,
    #[clap(long,conflicts_with("read"))]
    read_str: Option<String>,
    #[clap(long,alias="rr")]
    read_cycle: bool,

    #[clap(short='d', long)]
    read_delim:Option<TypedABE<Vec<u8>>>,
    #[clap(short='n',long)]
    read_maxsize: Option<usize>,
    #[clap(long)]
    read_maxreads: Option<usize>,
    /// Eval is done after reading upto maxsize or delim. It is always an error to produce more than MAX_DATA_SIZE bytes
    #[clap(long)]
    read_eval: bool,
    #[clap(skip)]
    open_file : OnceCell<File>,
    #[clap(skip)]
    lock_stdin: OnceCell<Stdin>
}

pub type Reader<'o> = Box<dyn FnMut(&EvalCtx<&dyn Scope>, &mut Vec<u8>) -> anyhow::Result<Option<()>> + 'o >;
pub fn bufreader<'o>(opts:&'o ReadOpt,default_stdin:bool) -> anyhow::Result<Box<dyn BufRead+'o >>{
    use std::io::*;
    if let Some(o) = &opts.read_str {
        Ok(Box::new(Cursor::new(o.as_bytes())))
    }else if let Some(path) = &opts.read{
        let file = opts.open_file.get_or_try_init(||std::fs::File::open(path))?;
        let meta = file.metadata()?;
        if linkspace_common::protocols::impex::blob::should_mmap(meta.len()) || opts.read_cycle{
            let mmap = unsafe { memmap2::Mmap::map(file)? };
            tracing::info!("new mmap");
            Ok(Box::new(Cursor::new(mmap)))
        } else {
            tracing::info!("new mmap");
            Ok(Box::new(BufReader::new(file)))
        }
    } else if default_stdin {
        ensure!(!opts.read_cycle, "cycle not supported for stdin");
        Ok(Box::new(opts.lock_stdin.get_or_init(stdin).lock()))
    }else {
        Ok(Box::new(Cursor::new(&[])))
    }
}

pub fn into_reader<'o>(opts:&'o ReadOpt,default_stdin:bool,ctx:&EvalCtx<impl Scope>) -> anyhow::Result<Reader<'o>> {
    let mut inp = bufreader(&opts, default_stdin)?;
    let max = opts.read_maxsize.unwrap_or(MAX_DATA_SIZE);
    if max > MAX_DATA_SIZE { anyhow::bail!("Can only read {MAX_DATA_SIZE} bytes per packet")}
    let delim = match &opts.read_delim {
        Some(delim) => {
            let vec = delim.eval(ctx)?;
            ensure!(vec.len() == 1, "delimiter can only be a single byte");
            Some(vec[0])
        },
        None => None,
    };
    let mut call_limit = opts.read_maxreads.unwrap_or(usize::MAX);
    let reader : Reader = Box::new(move |ctx,buf|{
        if call_limit == 0 { return Ok(None)}
        call_limit -= 1;
        match delim {
            Some(byte) => loop {
                let mut taker = (&mut inp).take(max as u64);
                if taker.read_until(byte,buf)? == 0 {
                    if opts.read_cycle { inp = bufreader(&opts,default_stdin)?; continue}
                    return Ok(None)
                }else {
                    if buf.last().copied() == delim { buf.pop();}
                    if opts.read_eval {
                        *buf = eval(ctx,&parse_abe_b(&buf)?)?.concat();
                    }
                    return Ok(Some(()))
                }
            },
            None => loop {
                let c = (&mut inp).take(max as u64).read_to_end(buf)?;
                if c == 0 {
                    if opts.read_cycle { inp = bufreader(&opts,default_stdin)?; continue}
                    return Ok(None)
                }
                if opts.read_eval{
                    *buf = eval(ctx,&parse_abe_b(&buf)?)?.concat();
                }
                return Ok(Some(()))
            },
        }
    });
    Ok(reader)
}


pub fn write_datapoint(write: Vec<WriteDestSpec>,common: &CommonOpts, opts: ReadOpt) -> anyhow::Result<()> {
    let mut buf =Vec::with_capacity(MAX_DATA_SIZE);
    let mut reader = into_reader(&opts,true,&common.eval_ctx())?;
    let mut write = common.open(&write)?;
    let ctx = common.eval_ctx();
    let ctx = ctx.dynr();
    while reader(&ctx,&mut buf)?.is_some(){
        let pkt = datapoint(&buf,());
        common.write_multi_dest(&mut write, &pkt, None)?;
        buf.clear();
    }
    Ok(())
}
