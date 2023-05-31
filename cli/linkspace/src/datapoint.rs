use linkspace::consts::MAX_DATA_SIZE;
use linkspace_common::{
    cli::{
        opts::CommonOpts,
        WriteDestSpec, read_data::ReadOpt,
    },
    prelude::*,
};

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
    while reader.read_next_data(&ctx, MAX_DATA_SIZE, &mut buf)?.is_some() {
        let pkt = datapoint(&buf, ());
        common.write_multi_dest(&mut write, &pkt, None)?;
        buf.clear();
    }
    Ok(())
}
