use linkspace::consts::MAX_DATA_SIZE;
use linkspace_common::{
    cli::{opts::CommonOpts, reader::DataReadOpts, WriteDestSpec},
    prelude::*,
};

pub fn write_datapoint(
    write: Vec<WriteDestSpec>,
    common: &CommonOpts,
    opts: DataReadOpts,
) -> anyhow::Result<()> {
    let mut buf = Vec::with_capacity(MAX_DATA_SIZE);
    let mut reader = opts.open_reader(true, &common.eval_scope())?;
    let mut write = common.open(&write)?;
    let scope = common.eval_scope();
    while reader
        .read_next_data(&scope, MAX_DATA_SIZE, &mut buf)?
        .is_some()
    {
        let pkt = datapoint(&buf, ());
        common.write_multi_dest(&mut write, &pkt, None)?;
        buf.clear();
    }
    Ok(())
}
