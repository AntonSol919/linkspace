// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub mod keys;
pub mod opts;
pub mod reader;
use abe::{
    abev,
    ast::{as_bytes, single},
    parse_abe_strict_b, TypedABE,
};
use anyhow::{bail, Context};
pub use clap;
use linkspace_pkt::NetPkt;
pub use tracing;

use linkspace_core::prelude::{pkt_scope, Scope};
use std::io::{stderr, stdout};

pub type PreProc = Option<TypedABE<Vec<u8>>>;
pub fn write_pkt2(
    preproc: &PreProc,
    pkt: impl NetPkt,
    scope: &dyn Scope,
    mut out: impl std::io::Write,
) -> std::io::Result<()> {
    match preproc {
        None => {
            let bytes = pkt.byte_segments();
            out.write_all_vectored(&mut bytes.io_slices())?;
        }
        Some(expr) => {
            let v = expr.eval(&(scope, pkt_scope(&pkt)));
            if let Err(err) = &v {
                tracing::warn!(?err, ?expr, "error pre-processing with expression");
            }
            let v = v.map_err(std::io::Error::other)?;
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
        let abe = parse_abe_strict_b(s.as_bytes())?;
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
                // it is far too suprising for files with ':' to silently use that as a name.
                // So we do this little dance to treat file:/some:thing:[hash:str] differently then file-expr:[/:./some:thing]:[hash:str]
                let v = it.as_slice().to_vec();
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
    pub fn open(&self, scope: &dyn Scope) -> std::io::Result<Option<WriteDest>> {
        let WriteDest { path, prep, out: _ } = self;
        let pathv = path.eval(scope).map_err(std::io::Error::other)?;
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
                        .open(path_str)?,
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
