// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub mod keys;
pub mod opts;
use abe::{
    abev,
    ast::{as_bytes, single},
    eval::eval,
    parse_abe, TypedABE,
};
use anyhow::{bail, Context};
pub use clap;
use linkspace_pkt::{pkt_ctx, NetPkt, MAX_DATA_SIZE};
pub use tracing;

use linkspace_core::prelude::{EvalCtx, Scope};
use std::{
    borrow::Cow,
    io::{stderr, stdin, stdout, Read},
    str::FromStr,
};

use self::opts::InOpts;

pub type PreProc = Option<TypedABE<Vec<u8>>>;
pub fn write_pkt2(
    preproc: &PreProc,
    pkt: impl NetPkt,
    ctx: &EvalCtx<impl Scope>,
    mut out: impl std::io::Write,
) -> std::io::Result<()> {
    match preproc {
        None => {
            let bytes = pkt.byte_segments();
            out.write_all_vectored(&mut bytes.io_slices())?;
        }
        Some(e) => {
            let v = e
                .eval(&pkt_ctx(ctx.reref(), &pkt))
                .map_err(|e| std::io::Error::other(e))?;
            out.write_all(&v)?;
        }
    }
    out.flush()
}
pub enum Out {
    Fd(Box<dyn std::io::Write + Send + Sync>),
    Db,
    Buffer,
}
#[derive(Clone)]
pub struct WriteDest<OUT = Out> {
    pub path: TypedABE<Vec<u8>>,
    pub prep: PreProc,
    pub out: OUT,
}

pub type WriteDestSpec = WriteDest<()>;
impl std::str::FromStr for WriteDestSpec {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let abe = parse_abe(s)?;
        let mut it = abe.split(|v| v.is_colon());
        let dest = as_bytes(single(it.next().context("missing dest")?)?)?;
        let (is_expr, dest) = match dest.strip_suffix(b"-expr") {
            Some(d) => (true, d),
            None => (false, dest),
        };
        let path = match dest {
            b"-" if !is_expr => abev!( [0] "stdout"),
            b"stdout" => abev!( [0] "stdout"),
            b"stderr" => abev!( [0] "stderr"),
            b"null" => abev!([0]),
            b"db" => abev!( [0] "db"),
            b"buffer" => abev!([0] "buffer"),
            b"file" if !is_expr => {
                // it is far to suprising for files with ':' to silently use that as a name.
                // So we do this little dance to treat file:/some:thing:[hash:str] differently then file-expr:[/:./some:thing]:[hash:str]
                let v = it.as_slice().clone().to_vec();
                for _i in &mut it {}
                v
            }
            b"file" if is_expr => it.next().context("missing file")?.to_vec(),
            _ => bail!("unrecognized option - expect stdout | stderr | db | buffer | file"),
        };
        let expr = if is_expr {
            it.next().unwrap_or_default().to_vec()
        } else {
            anyhow::ensure!(
                it.next().is_none(),
                "tail expr? use 'file-expr:[/:./my/fancy/fil:name]:The hash is=[hash:str]' "
            );
            vec![]
        };
        let prep = if expr.is_empty() {
            None
        } else {
            Some(expr.into())
        };
        tracing::debug!(?path, ?prep, "parsed");
        Ok(WriteDest {
            path: path.into(),
            prep,
            out: (),
        })
    }
}

impl WriteDestSpec {
    pub fn stdout() -> Self {
        WriteDest {
            path: abev!([0] "stdout").into(),
            prep: None,
            out: (),
        }
    }
    pub fn open(&self, ctx: &EvalCtx<impl Scope>) -> std::io::Result<Option<WriteDest>> {
        let WriteDest { path, prep, out: _ } = self;
        let pathv = path.eval(ctx).map_err(std::io::Error::other)?;
        let mut path_str = "";
        let out: Out = match pathv.strip_prefix(&[0]) {
            Some(b) => match b {
                b"buffer" => Out::Buffer,
                b"db" => Out::Db,
                b"stdout" => Out::Fd(Box::new(stdout())),
                b"stderr" => Out::Fd(Box::new(stderr())),
                &[] | b"null" => return Ok(None),
                _ => return Err(std::io::Error::other("0 byte in name")),
            },
            None => {
                path_str = std::str::from_utf8(&pathv).map_err(std::io::Error::other)?;
                let fd = Box::new(
                    std::fs::File::options()
                        .write(true)
                        .append(true)
                        .create(true)
                        .open(&path_str)?,
                );
                Out::Fd(fd)
            }
        };
        tracing::debug!(?path,?prep,%path_str,"Open");
        Ok(Some(WriteDest {
            path: path.clone(),
            out,
            prep: prep.clone(),
        }))
    }
}

/// --data stdin
/// --data stdin:pkt
/// --data stdin:pkt:[data]
/// --data "stdin:pkt:[data] and [create:str]"
/// --data stdin-live:pkt
/// --data "abe:The time at start [now]"
/// --data "abe-live:The time is [now]"
/// --data file:[/:./some/path]  read once
/// --data file-live:[/:./some/path] read every time
/// --data file-live:[/:./some/path]:pkt:The hash [hash:str]" read every time
#[derive(Clone, Debug, Default)]
pub enum ReadAs {
    #[default]
    Raw,
    Pkt(PreProc),
}
#[derive(Clone, Debug)]
pub struct ReadSource {
    pub live: bool,
    pub source: DataSource,
    pub read_as: ReadAs,
}
impl FromStr for ReadSource {
    type Err = anyhow::Error;

    fn from_str(st: &str) -> Result<Self, Self::Err> {
        use abe::ast::*;
        let (kind,expr) = st.split_once(':').unwrap_or((st,""));
        let (live, kind) = match kind.strip_suffix("-live") {
            Some(kind) => (true,kind),
            None => (false, kind),
        };
        let mut read_as = ReadAs::Raw;
        let abe = parse_abe(expr)?;
        let mut it = abe.split(|v| v.is_colon());
        let source = match kind {
            "abe" => {
                let expr = it.as_slice().to_vec();
                tracing::debug!(live, ?expr, ?read_as, "setup source");
                return Ok(ReadSource {
                    live,
                    source: DataSource::ABE(expr.into()),
                    read_as,
                });
            }
            "-" if !live => DataSource::Stdin,
            "stdout" => DataSource::Stdin,
            "file" => {
                let name = it.next().context("missing filename")?.to_vec();
                DataSource::File(name.into())
            }
            _ => anyhow::bail!("unknown type - accepts stdout | file | abe )"),
        };
        if let Some(n) = it.next() {
            anyhow::ensure!(
                as_bytes(single(n)?)? == b"pkt",
                "unknown options - accepts :pkt[:..expr] got {n:?}"
            );
            read_as = ReadAs::Pkt(None);
            let rest = it.as_slice();
            if !rest.is_empty() {
                read_as = ReadAs::Pkt(Some(rest.to_vec().into()));
            }
        }
        tracing::debug!(live, ?source, ?read_as, "setup source");
        Ok(ReadSource {
            live,
            source,
            read_as,
        })
    }
}

#[derive(Clone, Debug)]
pub enum DataSource {
    Stdin,
    File(TypedABE<Vec<u8>>),
    ABE(TypedABE<Vec<u8>>),
}
/// Fill buffer with whatever is being read ( live or otherwise )
/// Returns entire buffer - caller is responsible for cleaning
// should prob be &mut dyn Write
pub type Reader = Box<
    dyn for<'u> FnMut(&EvalCtx<&dyn Scope>, &'u mut Vec<u8>) -> anyhow::Result<&'u [u8]>
        + Send
        + Sync,
>;
pub fn no_reader() -> Reader{
    Box::new(|_,v| Ok(&*v))
}
pub fn read2vec(reader: &mut Reader,ctx: &EvalCtx<&dyn Scope>) -> anyhow::Result<Vec<u8>>{
    let mut buf = vec![];
    (reader)(ctx,&mut buf)?;
    Ok(buf)
}


impl ReadSource {
    pub fn into_reader(
        opt: Option<&Self>,
        opts: InOpts,
        ctx: &EvalCtx<impl Scope>,
    ) -> anyhow::Result<Reader> {
        match opt {
            Some(v) => v.reader(opts, ctx),
            None => Ok(Box::new(|_, v| Ok(v))),
        }
    }
    pub fn reader(&self, opts: InOpts, ctx: &EvalCtx<impl Scope>) -> anyhow::Result<Reader> {
        Ok(match (&self.source, &self.read_as, &self.live) {
            (DataSource::Stdin, ReadAs::Raw, true) => {
                let mut buf = [0; MAX_DATA_SIZE];
                Box::new(move |_ctx, out| {
                    let len = stdin().read(&mut buf)?;
                    out.extend_from_slice(&buf[..len]);
                    Ok(out)
                })
            }
            (DataSource::Stdin, ReadAs::Raw, false) => {
                let mut buf = vec![];
                stdin().read_to_end(&mut buf)?;
                val_reader(buf)
            }
            (DataSource::Stdin, ReadAs::Pkt(p), true) => {
                let prep = p.clone();
                let mut reader = opts.pkt_reader(stdin());
                Box::new(move |ctx, out| {
                    let pkt = reader.next().context("no more pkts for reader")??;
                    let val = read_prep(&pkt, &ctx, &prep)?;
                    out.extend_from_slice(&val);
                    Ok(out)
                })
            }
            (DataSource::Stdin, ReadAs::Pkt(prep), false) => {
                let pkt = opts.pkt_reader(stdin()).next().context("expected pkt")??;
                let val = read_prep(&pkt, ctx, prep)?.into_owned();
                val_reader(val)
            }
            (DataSource::File(p), ReadAs::Raw, true) => {
                let p = p.clone();
                Box::new(move |ctx, out| {
                    let pathb = p.eval(&ctx)?;
                    let path: String = String::from_utf8(pathb)?;
                    let mut file = std::fs::File::open(path)?;
                    file.read_to_end(out)?;
                    Ok(out)
                })
            }
            (DataSource::File(p), ReadAs::Raw, false) => {
                let path: String = String::from_utf8(p.eval(&ctx)?)?;
                val_reader(std::fs::read(path)?)
            }
            (DataSource::File(file), ReadAs::Pkt(prep), true) => {
                let file_expr = file.clone();
                let prep = prep.clone();
                let mut path = file_expr.eval(&ctx)?;
                let mut file = None;
                Box::new(move |ctx, out| {
                    let old = std::mem::replace(&mut path, file_expr.eval(&ctx)?);
                    if file.is_none() || old != path {
                        let st = std::str::from_utf8(&path)?;
                        file = Some(opts.pkt_reader(std::fs::File::open(st)?));
                    }
                    let pkt = file
                        .as_mut()
                        .unwrap()
                        .next()
                        .with_context(|| format!("no more pkts ({path:?})"))??;
                    let val = read_prep(&pkt, &ctx, &prep)?.into_owned();
                    out.copy_from_slice(&val);
                    Ok(out)
                })
            }
            (DataSource::File(file), ReadAs::Pkt(prep), false) => {
                let path: String = String::from_utf8(file.eval(&ctx)?)?;
                let read = std::fs::File::open(path)?;
                let pkt = opts.pkt_reader(read).next().context("expected pkt")??;
                let val = read_prep(&pkt, ctx, prep)?.into_owned();
                val_reader(val)
            }
            (DataSource::ABE(expr), ReadAs::Raw, true) => {
                let expr = expr.clone();
                Box::new(move |ctx, out| {
                    let b = eval(&ctx, &expr).map_err(std::io::Error::other)?.concat();
                    out.extend_from_slice(&b);
                    Ok(out)
                })
            }
            (DataSource::ABE(expr), ReadAs::Raw, false) => val_reader(eval(ctx, &expr)?.concat()),
            // dont see a usecase but could be done if we change parsing logic
            (DataSource::ABE(_), ReadAs::Pkt(_), _) => unreachable!(),
        })
    }
}
fn val_reader(val: Vec<u8>) -> Reader {
    Box::new(move |_, out| {
        out.extend_from_slice(&val);
        Ok(out)
    })
}
fn read_prep<'p>(
    pkt: &'p impl NetPkt,
    ctx: &EvalCtx<impl Scope>,
    prep: &Option<TypedABE<Vec<u8>>>,
) -> anyhow::Result<Cow<'p, [u8]>> {
    match prep {
        Some(e) => Ok(e.eval(&pkt_ctx(ctx.reref(), pkt))?.into()),
        None => Ok(pkt.as_point().data().into()),
    }
}


