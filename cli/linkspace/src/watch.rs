// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{clap, clap::Args, opts::CommonOpts, tracing, Out, WriteDestSpec},
    predicate_aliasses::{ExtWatchCLIOpts, WithFiles},
    prelude::{query_mode::Mode, TypedABE, *},
};

#[derive(Debug, Args, Clone)]
#[group(skip)]
pub struct DGPDWatchCLIOpts {
    #[clap(long, short)]
    pub bare: bool,
    #[clap(required_unless_present("bare"))]
    pub dgpd: Option<DGPDExpr>,
    #[clap(flatten)]
    pub watch_opts: WithFiles<ExtWatchCLIOpts>,
}

impl DGPDWatchCLIOpts {
    pub fn watch_predicates(self, ctx: &EvalCtx<impl Scope>) -> anyhow::Result<Query> {
        tracing::trace!("Collecting predicates");
        let mut query = Query::default();
        let dgpd = self
            .dgpd
            .filter(|_| !self.bare)
            .map(|dgpd| dgpd.predicate_exprs());
        let aliasses = self.watch_opts.opts.aliasses.as_predicates();
        let exprs = self.watch_opts.opts.exprs.into_iter();
        let it = dgpd.into_iter().flatten().chain(aliasses).map(Into::into);
        for e in it.chain(exprs){
            tracing::trace!(?e, "add expr");
            let e = e.eval(&ctx)?;
            query.add(vec![e])?;
        }
        for file in self.watch_opts.file.iter() {
            let inp = std::fs::read_to_string(file)?;
            query.parse(inp.as_bytes(), &ctx)?;
        }
        Ok(query)
    }
}

#[derive(Args)]
#[group(skip)]
pub struct CLIQuery {
    /// print effective query in string format
    #[clap(long,short,action = clap::ArgAction::Count)]
    pub print_query: u8,
    #[clap(long, default_value = "default")]
    pub id: TypedABE<Vec<u8>>,
    #[clap(long, default_value = "tree-desc")]
    pub mode: Option<Mode>,
    #[clap(flatten)]
    pub opts: DGPDWatchCLIOpts,
    #[clap(long, default_value = "stdout")]
    pub write: Vec<WriteDestSpec>,
}
impl CLIQuery {
    // FIXME: printing here is confusing
    pub fn into_query(self, common: &CommonOpts) -> anyhow::Result<Option<Query>> {
        let ctx = common.eval_ctx();
        let mut select = self.opts.watch_predicates(&ctx)?;
        let inner_mode = select.mode().transpose()?;
        if inner_mode.is_none() || inner_mode != self.mode {
            let st = self.mode.unwrap_or_default().to_string();
            select.add_option(&KnownOptions::Mode.to_string(), &[st.as_bytes()]);
        }
        let inner_id = select.watch_id().transpose()?;
        let id = self.id.eval(&ctx)?;
        if inner_id != Some(&id) {
            select.add_option(&KnownOptions::Watch.to_string(), &[&id]);
        }
        if self.print_query > 0 {
            crate::print_query(self.print_query, &select.into());
            return Ok(None);
        }
        Ok(Some(select))
    }

    pub fn mode(mut self, mode: Mode) -> CLIQuery {
        self.mode = Some(mode);
        self
    }
}
pub fn watch(common: CommonOpts, cli_query: CLIQuery) -> anyhow::Result<()> {
    let write = common.open(&cli_query.write)?;
    if write.iter().any(|v| matches!(v.out, Out::Db)) {
        anyhow::bail!("db and null dest not supported");
    }
    if let Some(query) = cli_query.into_query(&common)? {
        let rt = common.runtime()?;
        let span = debug_span!("Userland watch");
        let out = common.multi_writer(write);
        rt.watch_query(&query, out, span)?;
        let _ = rt.run_while(None, None);
    };
    Ok(())
}
