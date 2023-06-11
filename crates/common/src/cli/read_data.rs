use std::{
    fs::File,
    io::{BufRead, BufReader, Cursor, Read, StdinLock},
    path::PathBuf,
};

use anyhow::ensure;
use tracing::instrument;
use crate::{
    abe::TypedABE,
    cli::{
        clap::{self, *},
    },
    prelude::*,
    protocols::impex::blob::should_mmap,
};
use memmap2::Mmap;



#[derive(Parser, Clone, Debug,Default,PartialEq)]
pub struct ReadOpt {
    #[clap(short, long)]
    pub read: Option<PathBuf>,
    /// default when stdin is not used for packets
    #[clap(long,conflicts_with_all(["read","read_str","read_empty","read_repeat"]))]
    pub read_stdin: bool,
    #[clap(long, conflicts_with_all(["read","read_stdin","read_empty"]))]
    pub read_str: Option<String>,
    #[clap(long,conflicts_with_all(["read","read_str","read_stdin"]))]
    pub read_empty: bool,
    #[clap(long, alias = "rr")]
    pub read_repeat: bool,
    #[clap(short = 'D', long)]
    pub read_delim: Option<TypedABE<Vec<u8>>>,
    /// Maximum amount read per call - defaults to free space in packet
    #[clap(short = 'n', long)]
    pub read_bufsize: Option<usize>,
    /// Set how to deal when the read result ( after eval ) exceeds the free space.
    #[arg(long,value_enum)]
    pub read_overflow : Option<EMode>,
    #[clap(long)]
    pub read_calllimit: Option<usize>,
    /// Eval is done after reading upto maxsize or delim. 
    #[clap(long)]
    pub read_eval: bool,
}
#[derive(Default,Copy,Clone,ValueEnum,Debug,PartialEq)]
pub enum EMode{
    #[default]
    Error,
    Split,
    Carry,
    Trim
}

#[derive(Debug)]
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
    pub fn read(&mut self, take: u64, delim:Option<u8>,buf:&mut Vec<u8>) -> std::io::Result<usize>{
        match delim {
            Some(byte) => self.bufr().take(take+ 1).read_until(byte, buf),
            None => self.bufr().take(take).read_to_end(buf)
        }
    }
}

impl ReadOpt {
    pub fn open<'o>(&self, default_stdin: bool) -> anyhow::Result<Option<ReadSource>> {
        use std::io::*;
        if self.read_empty {return Ok(None)};
        let r = if let Some(o) = &self.read_str {
            ReadSource::String(Cursor::new(o.clone()))
        } else if let Some(path) = &self.read {
            let file = std::fs::File::open(path)?;
            let meta = file.metadata()?;
            if should_mmap(meta.len()) || self.read_repeat {
                let mmap = unsafe { memmap2::Mmap::map(&file)? };
                tracing::info!("new mmap");
                ReadSource::Mmap(Cursor::new(mmap), file)
            } else {
                tracing::info!("new file");
                ReadSource::File(BufReader::new(file))
            }
        } else if self.read_stdin || default_stdin{
            ensure!(!self.read_repeat, "repeat not supported for stdin");
            ReadSource::Stdin(stdin().lock())
        }else {
            return Ok(None)
        };
        Ok(Some(r))
    }

    #[instrument(skip(ctx))]
    pub fn open_reader(
        &self,
        default_stdin: bool,
        ctx: &EvalCtx<impl Scope>,
    ) -> anyhow::Result<Reader> {
        let delim = match &self.read_delim {
            Some(delim) => {
                let vec = delim.eval(ctx)?;
                ensure!(vec.len() == 1, "delimiter can only be a single byte");
                Some(vec[0])
            }
            None => None,
        };
        let input = self.open(default_stdin)?;
        tracing::trace!(?input);
        Ok(Reader {
            next: None,
            call_limit: self.read_calllimit.unwrap_or(usize::MAX),
            input,
            cycle: self.read_repeat ,
            eval: self.read_eval,
            delim,
            take_max: self.read_bufsize,
            on_overflow : self.read_overflow.unwrap_or(EMode::Error)
        })
    }
}
pub struct Reader {
    next: Option<Vec<u8>>,
    take_max : Option<usize>,
    call_limit: usize,
    input: Option<ReadSource>,
    cycle: bool,
    eval: bool,
    delim: Option<u8>,
    on_overflow:EMode
}
impl Reader {
    /// pull one data block from the source, returns Some to continue, None to stop.
    #[instrument(skip(self,ctx))]
    pub fn read_next_data(
        &mut self,
        ctx: &EvalCtx<&dyn Scope>,
        freespace: usize,
        buf: &mut Vec<u8>,
    ) -> anyhow::Result<Option<()>> {
        if self.call_limit == 0 {
            return Ok(None);
        }

        if let Some(mut b) = self.next.take(){
            if self.on_overflow == EMode::Carry {
                self.read_next_data(ctx, freespace, buf)?;
                b.extend_from_slice(buf);
                *buf = b;
            } else { 
                *buf = b;
            }
            self.check_size(buf, freespace)?;
            return Ok(Some(()));
        }
        let take = self.take_max.unwrap_or(freespace) as u64;

        self.call_limit -= 1;

        tracing::debug!("next data (input={:?})",self.input);
        let inp = match &mut self.input{
            Some(i) => i,
            None => return Ok(Some(())),
        };

        if inp.read(take,self.delim,buf)? == 0{
            if !self.cycle{ return Ok(None)}
            inp.rewind()?;
            if inp.read(take,self.delim,buf)? == 0{
                self.input = None;
                self.call_limit =0;// make sure we None on next call as well
                return Ok(None);
            }
        }

        if self.delim.is_some() && self.delim == buf.last().copied() { buf.pop(); }
        if self.eval {
            *buf = eval(ctx, &parse_abe_b(buf)?)?.concat();
        }
        self.check_size(buf, freespace)?;
        Ok(Some(()))
    }
    fn check_size(&mut self, buf:&mut Vec<u8>,freespace:usize) -> anyhow::Result<()>{
        if self.on_overflow == EMode::Carry {
            if let Some(mut carry) = self.next.take(){
                carry.extend_from_slice(buf);
                *buf = carry;
            }
        }
        if freespace < buf.len(){
            match self.on_overflow{
                EMode::Error => anyhow::bail!("read {} but packet only has {freespace} free space - ",buf.len()),
                EMode::Trim => {
                    buf.truncate(freespace);
                },
                EMode::Split | EMode::Carry => {
                    self.next = Some(buf.split_off(freespace));
                },
            }
        } 
        Ok(())
    }
}
