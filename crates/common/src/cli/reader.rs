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
pub struct PktReadOpts{
    /// read packets from path
    #[arg(long)]
    pub pkts: Option<PathBuf>,
    /// don't read any packets. 
    #[arg(long,conflicts_with_all(["pkts"]))]
    pub no_pkts: bool,
}
impl PktReadOpts {
    pub fn open(&self) -> std::io::Result<Option<ReadSource>>{
        CommonReadOpts {
            data: self.pkts.clone(),
            data_stdin: false,
            data_str: None,
            no_data: self.no_pkts,
            data_repeat: false
        }.open(true)
    }
    pub fn is_stdin(&self) -> bool { self.pkts.is_none() && !self.no_pkts}
}

pub fn check_stdin(pkt_in:&PktReadOpts,read_in:&DataReadOpts,read_default_in:bool) -> anyhow::Result<()>{
    ensure!( !(pkt_in.is_stdin() && read_in.common.is_stdin(read_default_in)) , "both pkts and read data is claiming stdin - change either (e.g. --no-pkts)");
    Ok(())
}




// Shared between reading packets and reading arbitrary data
#[derive(Parser, Clone, Debug,Default,PartialEq)]
pub struct CommonReadOpts{
    /// open a path to read from
    #[arg(long)]
    pub data: Option<PathBuf>,
    /// default when stdin is not used for packets
    #[arg(long,conflicts_with_all(["data","data_str","no_data","data_repeat"]))]
    pub data_stdin: bool,
    /// read a static string
    #[arg(long, conflicts_with_all(["data","data_stdin","no_data"]))]
    pub data_str: Option<String>,
    /// read nothing
    #[arg(long,conflicts_with_all(["data","data_str","data_stdin"]))]
    pub no_data: bool,
    /// after finishing jump back to start
    #[arg(long, alias = "rr", conflicts_with("data_stdin"))]
    pub data_repeat: bool,
}
impl CommonReadOpts {
    pub fn is_stdin(&self,default_stdin:bool) -> bool{
        self.data_stdin
            || ( !self.no_data && self.data_str.is_none() && self.data.is_none() && default_stdin)
    }
   pub fn open(&self,default_stdin:bool) -> std::io::Result<Option<ReadSource>>{
        use std::io::*;
        if self.no_data {return Ok(None)};
        let r = if let Some(o) = &self.data_str {
            ReadSource::String(Cursor::new(o.clone()))
        } else if let Some(path) = &self.data {
            let file = std::fs::File::open(path)?;
            let meta = file.metadata()?;
            if should_mmap(meta.len()) || self.data_repeat {
                let mmap = unsafe { memmap2::Mmap::map(&file)? };
                tracing::info!("new mmap");
                ReadSource::Mmap(Cursor::new(mmap), file)
            } else {
                tracing::info!("new file");
                ReadSource::File(BufReader::new(file))
            }
        } else if self.data_stdin || default_stdin{
            assert!(!self.data_repeat, "repeat not supported for stdin");
            ReadSource::Stdin(stdin().lock())
        }else {
            return Ok(None)
        };
        Ok(Some(r))
    } 
}

#[derive(Parser, Clone, Debug,Default,PartialEq)]
pub struct DataReadOpts {
    #[command(flatten)]
    pub common : CommonReadOpts,
    /// set a byte to function as a delimiter.
    #[arg(short = 'D', long)]
    pub data_delim: Option<TypedABE<Vec<u8>>>,
    /// maximum amount read per call - defaults to free space in packet
    #[arg(short = 'n', long)]
    pub data_bufsize: Option<usize>,
    /// limit number of 'reads' 
    #[arg(long)]
    pub data_reads: Option<usize>,
    /// set how to deal when the data ( after eval ) exceeds the free space in a packet.
    #[arg(long,value_enum)]
    pub data_overflow : Option<EMode>,
    /// evaluate data as ABE expression.
    #[arg(long)]
    pub data_eval: bool,
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
    pub fn into_read(self) -> Box<dyn Read>{
        match self {
            ReadSource::String(a) => Box::new(a),
            ReadSource::File(a) => Box::new(a),
            ReadSource::Mmap(a, _) => Box::new(a),
            ReadSource::Stdin(a) => Box::new(a),
        }
    }
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
            Some(byte) => self.bufr().take(take.saturating_add(1)).read_until(byte, buf),
            None => self.bufr().take(take).read_to_end(buf)
        }
    }
    
}
impl DataReadOpts{
    #[instrument(skip(ctx))]
    pub fn open_reader(
        &self,
        default_stdin: bool,
        ctx: &EvalCtx<impl Scope>,
    ) -> anyhow::Result<Reader> {
        let delim = match &self.data_delim {
            Some(delim) => {
                let vec = delim.eval(ctx)?;
                ensure!(vec.len() == 1, "delimiter can only be a single byte");
                Some(vec[0])
            }
            None => None,
        };
        let input = self.common.open(default_stdin)?;
        tracing::trace!(?input);
        Ok(Reader {
            next: None,
            call_limit: self.data_reads.unwrap_or(usize::MAX),
            input,
            cycle: self.common.data_repeat ,
            eval: self.data_eval,
            delim,
            take_max: self.data_bufsize,
            on_overflow : self.data_overflow.unwrap_or(EMode::Error)
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
            *buf = eval(ctx, &parse_abe_strict_b(buf)?)?.concat();
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
