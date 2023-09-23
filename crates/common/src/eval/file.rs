use std::path::Path;

use abe::eval::{none, EvalScopeImpl, ScopeFunc, ScopeFuncInfo};
use anyhow::Context;
use byte_fmt::AB;
use tracing::instrument;

use crate::runtime::Linkspace;

use super::LKS;

#[derive(Copy, Clone)]
pub struct FileEnv<R>(pub(crate) R);

impl<R: LKS> EvalScopeImpl for FileEnv<R> {
    fn about(&self) -> (String, String) {
        let info = match self.0.lk(){
            Ok(o) => match o.files(){
                Some(o) => format!("Reading files from {o:?}"),
                None => format!("no abe files directory set"),
            }
            Err(e) => e.to_string(),
        };
        (
            "filesystem env".into(),
           info 
        )
    }
    fn list_funcs(&self) -> &[ScopeFunc<&Self>] {
        &[ScopeFunc {
            apply: |this: &Self, inp: &[&[u8]], _, _scope| {
                let p = std::str::from_utf8(inp[0])?;
                Ok(this.0.lk()?.files_data(p.as_ref(), true)?.unwrap()).into()
            },
            info: ScopeFuncInfo {
                id: "files",
                init_eq: None,
                argc: 1..=1,
                to_abe: false,
                help: "read a file from the LK_DIR/files directory",
            },
            to_abe: none,
        }]
    }
}

impl Linkspace {
    #[instrument(ret, skip(bytes))]
    pub fn set_files_data(&self, path: &Path, bytes: &[u8], overwrite: bool) -> anyhow::Result<()> {

        tracing::trace!(bytes=%AB(bytes));
        let path = self.files().context("no files location set")?.join(check_path(path)?);
        let r: anyhow::Result<()> = try {
            std::fs::create_dir_all(path.parent().unwrap())?;
            let mut file = if overwrite {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&path)?
            } else {
                std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)?
            };
            std::io::Write::write_all(&mut file, bytes)?;
        };
        r.with_context(|| anyhow::anyhow!("Target {}", path.to_string_lossy()))
    }
    #[instrument(ret)]
    // notfound_err simplifies context errors
    pub fn files_data(
        &self,
        path: &Path,
        notfound_err: bool,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        let path = self.files().context("no files location set")?.join(check_path(path)?);
        use std::io::ErrorKind::*;
        match std::fs::read(&path) {
            Ok(k) => Ok(Some(k)),
            Err(e) if !notfound_err && e.kind() == NotFound => Ok(None),
            Err(e) => {
                Err(e).with_context(|| anyhow::anyhow!("could not open {}", path.to_string_lossy()))
            }
        }
    }

    
}
pub fn check_path(path: &Path) -> anyhow::Result<&Path> {
    if let Some(c) = path
        .components()
        .find(|v| !matches!(v, std::path::Component::Normal(_)))
    {
        anyhow::bail!("path can not contain a {c:?} component")
    }
    Ok(path)
}
