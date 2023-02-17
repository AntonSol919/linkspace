// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use anyhow::Context;
use linkspace_common::{
    cli::{clap, clap::Args, opts::CommonOpts, tracing, Out, WriteDestSpec},
    predicate_aliasses::{ExtViewCLIOpts, WithFiles},
    prelude::{query_mode::Mode, TypedABE, *},
};

#[derive(Debug, Args, Clone)]
#[group(skip)]
pub struct DGPDViewCLIOpts {
    #[clap(long, short)]
    pub bare: bool,
    #[clap(required_unless_present("bare"))]
    pub dgpd: Option<DGPDExpr>,
    #[clap(flatten)]
    pub view_opts: WithFiles<ExtViewCLIOpts>,
}

impl DGPDViewCLIOpts {
    pub fn view_predicates(&self, ctx: &EvalCtx<impl Scope>) -> anyhow::Result<Query> {
        tracing::trace!("Collecting predicates");
        let mut query = Query::default();
        let predicates = self
            .dgpd
            .as_ref()
            .filter(|_| !self.bare)
            .map(|dgpd| dgpd.predicate_exprs())
            .into_iter()
            .flatten()
            .chain(self.view_opts.opts.aliasses.as_predicates());
        for pexpr in predicates {
            let p = pexpr.eval(&ctx);
            tracing::trace!(?pexpr, ?p, "eval predicate");
            let p = p.with_context(|| format!("{:?}", pexpr))?;
            query.predicates.add_ext_predicate(p)?;
        }
        let exprs = self.view_opts.opts.exprs.iter();
        for e in exprs {
            tracing::trace!(?e, "add expr");
            let e = e.eval(&ctx)?;
            query.add(vec![e])?;
        }
        for file in self.view_opts.file.iter() {
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
    pub opts: DGPDViewCLIOpts,
    #[clap(long, default_value = "stdout")]
    pub write: Vec<WriteDestSpec>,
}
impl CLIQuery {
    // FIXME: printing here is confusing
    pub fn into_query(&self, common: &CommonOpts) -> anyhow::Result<Option<Query>> {
        let ctx = common.eval_ctx();
        let mut select = self.opts.view_predicates(&ctx)?;
        let inner_mode = select.mode().transpose()?;
        if inner_mode.is_none() || inner_mode != self.mode {
            let st = self.mode.unwrap_or_default().to_string();
            select.add_option("mode", &[st.as_bytes()]);
        }
        let inner_id = select.id().transpose()?;
        let id = self.id.eval(&ctx)?;
        if inner_id != Some(&id) {
            select.add_option("id", &[&id]);
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
pub fn view(common: CommonOpts, cli_query: CLIQuery) -> anyhow::Result<()> {
    let write = common.open(&cli_query.write)?;
    if write.iter().any(|v| matches!(v.out, Out::Db)) {
        anyhow::bail!("db and null dest not supported");
    }
    if let Some(query) = cli_query.into_query(&common)? {
        let rt = common.runtime()?;
        let span = debug_span!("Userland view");
        let out = common.multi_writer(write);
        rt.view_query(&query, out, span)?;
        let _ = rt.run_while(None, None);
    };
    Ok(())
}
