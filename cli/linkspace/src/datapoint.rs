use std::{path::PathBuf, io::{ StdinLock, Cursor, BufReader, BufRead,Read }, fs::File};

use anyhow::{ ensure };
use linkspace::consts::MAX_DATA_SIZE;
use linkspace_common::{cli::{clap::{*,self},  opts::CommonOpts, WriteDestSpec}, abe::TypedABE,prelude::*};
use memmap2::Mmap;


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
}

pub enum ReadSource {
    String(Cursor<String>),
    File(BufReader<File>),
    Mmap(Cursor<Mmap>,File),
    Stdin(StdinLock<'static>)
}
impl ReadSource {
    pub fn bufr(&mut self) ->  &mut dyn BufRead{
        match self {
            ReadSource::String(a) =>  a,
            ReadSource::File(a) => a,
            ReadSource::Mmap(a, _) => a,
            ReadSource::Stdin(a) => a,
        }
    }
    pub fn rewind(&mut self) -> std::io::Result<()>{
        use std::io::Seek;
        match self {
            ReadSource::String(a) => a.rewind(),
            ReadSource::File(a) => a.rewind(),
            ReadSource::Mmap(a, _) => a.rewind(),
            ReadSource::Stdin(_) => panic!("bug - rewind on stdin"),
        }
    }
}

impl ReadOpt {
    pub fn open<'o>(&self,default_stdin:bool) -> anyhow::Result<ReadSource>{
        use std::io::*;
        let r = if let Some(o) = &self.read_str {
            ReadSource::String(Cursor::new(o.clone()))
        }else if let Some(path) = &self.read{
            let file = std::fs::File::open(path)?;
            let meta = file.metadata()?;
            if linkspace_common::protocols::impex::blob::should_mmap(meta.len()) || self.read_cycle{
                let mmap = unsafe { memmap2::Mmap::map(&file)? };
                tracing::info!("new mmap");
                ReadSource::Mmap(Cursor::new(mmap),file)
            } else {
                tracing::info!("new mmap");
                ReadSource::File(BufReader::new(file))
            }
        } else if default_stdin {
            ensure!(!self.read_cycle, "cycle not supported for stdin");
            ReadSource::Stdin(stdin().lock())
        }else {
            ReadSource::String(Cursor::new("".into()))
        };
        Ok(r)
    }
}

pub type Reader = Box<dyn FnMut(&EvalCtx<&dyn Scope>, &mut Vec<u8>) -> anyhow::Result<Option<()>>>;
pub fn into_reader(opts:ReadOpt,default_stdin:bool,ctx:&EvalCtx<impl Scope>) -> anyhow::Result<Reader> {
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
    let mut inp = opts.open(default_stdin)?;
    let reader : Reader = Box::new(move |ctx,buf|{
        if call_limit == 0 { return Ok(None)}
        call_limit -= 1;
        match delim {
            Some(byte) => loop {
                let mut taker = inp.bufr().take(max as u64);
                if taker.read_until(byte,buf)? == 0 {
                    if opts.read_cycle { inp.rewind()?; continue}
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
                let c = inp.bufr().take(max as u64).read_to_end(buf)?;
                if c == 0 {
                    if opts.read_cycle { inp.rewind()?; continue}
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
    let mut reader = into_reader(opts,true,&common.eval_ctx())?;
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
