use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, StdinLock},
    path::PathBuf,
};

use anyhow::ensure;
use linkspace::consts::MAX_DATA_SIZE;
use linkspace_common::{
    abe::TypedABE,
    cli::{
        clap::{self, *},
        opts::CommonOpts,
        WriteDestSpec,
    },
    prelude::*, protocols::impex::blob::should_mmap,
};
use memmap2::Mmap;

#[derive(Parser, Clone, Debug)]
pub struct ReadOpt {
    #[clap(short, long)]
    read: Option<PathBuf>,
    #[clap(long, conflicts_with("read"))]
    read_str: Option<String>,
    #[clap(long, alias = "rr")]
    read_cycle: bool,
    #[clap(short = 'd', long)]
    read_delim: Option<TypedABE<Vec<u8>>>,
    #[clap(short = 'n', long)]
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
    Mmap(Cursor<Mmap>, File),
    Stdin(StdinLock<'static>),
}

impl ReadSource {
    pub fn bufr(&mut self) -> &mut dyn BufRead {
        match self {
            ReadSource::String(a) => a,
            ReadSource::File(a) => a,
            ReadSource::Mmap(a, _) => a,
            ReadSource::Stdin(a) => a,
        }
    }
    pub fn rewind(&mut self) -> std::io::Result<()> {
        use std::io::Seek;
        match self {
            ReadSource::String(a) => a.rewind(),
            ReadSource::File(a) => a.rewind(),
            ReadSource::Mmap(a, _) => a.rewind(),
            ReadSource::Stdin(_) => panic!("bug - rewind on stdin"),
        }
    }
}

pub type Reader = Box<dyn FnMut(&EvalCtx<&dyn Scope>, &mut Vec<u8>,usize) -> anyhow::Result<Option<()>>>;
impl ReadOpt {
    pub fn open<'o>(&self, default_stdin: bool) -> anyhow::Result<Option<ReadSource>> {
        use std::io::*;
        let r = if let Some(o) = &self.read_str {
            ReadSource::String(Cursor::new(o.clone()))
        } else if let Some(path) = &self.read {
            let file = std::fs::File::open(path)?;
            let meta = file.metadata()?;
            if should_mmap(meta.len()) || self.read_cycle
            {
                let mmap = unsafe { memmap2::Mmap::map(&file)? };
                tracing::info!("new mmap");
                ReadSource::Mmap(Cursor::new(mmap), file)
            } else {
                tracing::info!("new file");
                ReadSource::File(BufReader::new(file))
            }
        } else if default_stdin {
            ensure!(!self.read_cycle, "cycle not supported for stdin");
            ReadSource::Stdin(stdin().lock())
        } else {
            return Ok(None)
        };
        Ok(Some(r))
    }

    pub fn open_reader(
        &self,
        default_stdin: bool,
        ctx: &EvalCtx<impl Scope>,
    ) -> anyhow::Result<Reader> {
        let readmax = self.read_maxsize;
        let delim = match &self.read_delim {
            Some(delim) => {
                let vec = delim.eval(ctx)?;
                ensure!(vec.len() == 1, "delimiter can only be a single byte");
                Some(vec[0])
            }
            None => None,
        };
        let mut call_limit = self.read_maxreads.unwrap_or(usize::MAX);
        let mut inp = self.open(default_stdin)?;
        let cycle = self.read_cycle;
        let read_eval = self.read_eval;
        let reader: Reader = Box::new(move |ctx, buf,mut max| {
            max = max.min(readmax.unwrap_or(max));
            if call_limit == 0 {
                return Ok(None);
            }
            call_limit -= 1;
            let inp = match &mut inp {
                Some(i) =>i,
                None => return Ok(Some(())),
            };
            match delim {
                Some(byte) => loop {
                    let mut taker = inp.bufr().take(max as u64 +1);
                    let c = taker.read_until(byte, buf)?;
                    tracing::debug!(c,"reading got");
                    if c == 0 {
                        if cycle {
                            inp.rewind()?;
                            continue;
                        }
                        return Ok(None);
                    } else {
                        if buf.last().copied() == delim {
                            buf.pop();
                        }
                        if read_eval {
                            *buf = eval(ctx, &parse_abe_b(&buf)?)?.concat();
                        }
                        return Ok(Some(()));
                    }
                },
                None => loop {
                    tracing::debug!(max,"reading");
                    let c = inp.bufr().take(max as u64).read_to_end(buf)?;
                    tracing::debug!(c,"reading got");
                    if c == 0 {
                        if cycle {
                            inp.rewind()?;
                            continue;
                        }
                        return Ok(None);
                    }
                    if read_eval {
                        *buf = eval(ctx, &parse_abe_b(&buf)?)?.concat();
                    }
                    return Ok(Some(()));
                },
            }
        });
        Ok(reader)
    }
}

pub fn write_datapoint(
    write: Vec<WriteDestSpec>,
    common: &CommonOpts,
    opts: ReadOpt,
) -> anyhow::Result<()> {
    let mut buf = Vec::with_capacity(MAX_DATA_SIZE);
    let mut reader = opts.open_reader(true, &common.eval_ctx())?;
    let mut write = common.open(&write)?;
    let ctx = common.eval_ctx();
    let ctx = ctx.dynr();
    while reader(&ctx, &mut buf,MAX_DATA_SIZE)?.is_some() {
        let pkt = datapoint(&buf, ());
        common.write_multi_dest(&mut write, &pkt, None)?;
        buf.clear();
    }
    Ok(())
}
