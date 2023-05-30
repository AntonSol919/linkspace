// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{clap, clap::Args, opts::CommonOpts, tracing, Out, WriteDestSpec},
    predicate_aliases::{ExtWatchCLIOpts },
    prelude::{query_mode::Mode,  *},
};

#[derive(Debug, Args, Clone,Default)]
#[group(skip)]
pub struct DGPDWatchCLIOpts {
    #[clap(long, short)]
    pub bare: bool,
    #[clap(required_unless_present("bare"))]
    pub dgpd: Option<DGPDExpr>,
    #[clap(flatten)]
    pub watch_opts: ExtWatchCLIOpts,
}

impl DGPDWatchCLIOpts {
    pub fn iter_statments(self) -> anyhow::Result<Vec<TypedABE<ABList>>>{
        tracing::trace!("Collecting predicates");
        let dgpd = self
            .dgpd
            .filter(|_| !self.bare)
            .map(|dgpd| dgpd.predicate_exprs()).transpose()?;
        let aliases = self.watch_opts.aliases.as_predicates();
        let exprs = self.watch_opts.exprs.into_iter();
        let it = dgpd.into_iter().flatten().chain(aliases).map(Into::into);
        Ok(it.chain(exprs).collect())
    }
    pub fn into_query(self, ctx: &EvalCtx<impl Scope>) -> anyhow::Result<Query> {
        statements2query(&self.iter_statments()?, ctx)
    }
}
pub fn statements2query(it: &[TypedABE<ABList>], ctx: &EvalCtx<impl Scope>) -> anyhow::Result<Query> {
    let mut query = Query::default();
    for e in it{
        tracing::trace!(?e, "add expr");
        let e = e.eval(&ctx)?;
        query.add_stmt(e)?;
    }
    Ok(query)
}


#[derive(Args,Clone,Copy,Default)]
#[group(skip)]
pub struct PrintABE{
    /// print the query
    #[clap(short,long,alias="print",short)]
    pub print_expr: bool,
    /// print in ascii-byte-text format (ABE without '{..}' expressions)
    #[clap(long,alias="text",conflicts_with="print_expr")]
    pub print_text: bool,
}
impl PrintABE {
    pub fn do_print(&self) -> bool { self.print_expr || self.print_text}
    pub fn print_query(&self, query:&Query, out : &mut dyn std::io::Write) -> std::io::Result<()>{
        if self.print_expr {
            writeln!(out, "{}", query.to_str(true))?
        }else if self.print_text {
            writeln!(out,"{}", query.to_str(false))?
        }
        Ok(())
    }
}

#[derive(Args,Default)]
#[group(skip)]
pub struct CLIQuery {
    #[clap(flatten)]
    pub print: PrintABE,
    #[clap(long, default_value = "tree-desc")]
    pub mode: Option<Mode>,
    #[clap(flatten)]
    pub opts: DGPDWatchCLIOpts,
}
impl CLIQuery {
    // FIXME: printing here is confusing
    pub fn into_query(self, common: &CommonOpts) -> anyhow::Result<Option<Query>> {
        let ctx = common.eval_ctx();
        let mut select = self.opts.into_query(&ctx)?;
        let inner_mode = select.mode().transpose()?;
        if inner_mode.is_none() || inner_mode != self.mode {
            let st = self.mode.unwrap_or_default().to_string();
            select.add_option(&KnownOptions::Mode.to_string(), &[st.as_bytes()]);
        }
        if self.print.do_print() {
            self.print.print_query(&select.into(), &mut std::io::stdout())?;
            return Ok(None);
        }
        Ok(Some(select))
    }

    pub fn mode(mut self, mode: Mode) -> CLIQuery {
        self.mode = Some(mode);
        self
    }
}
pub fn watch(common: CommonOpts, cli_query: CLIQuery,write:Vec<WriteDestSpec>) -> anyhow::Result<()> {
    let write = common.open(&write)?;
    if write.iter().any(|v| matches!(v.out, Out::Db)) {
        anyhow::bail!("db and null dest not supported");
    }
    if let Some(mut query) = cli_query.into_query(&common)? {
        query.add_option("qid", &[b"<cli>"]);
        let rt = common.runtime()?;
        let span = debug_span!("linkspace-cli watch");
        let out = common.multi_writer(write);
        rt.watch_query(&query, out, span)?;
        let _ = rt.run_while(None, None);
    };
    Ok(())
}

