// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_common::{
    cli::{opts::CommonOpts, tracing, WriteDest},
    prelude::*,
};

use crate::watch::CLIQuery;

pub fn select(
    cli_query: CLIQuery,
    mut write_false: Vec<WriteDest>,
    common: CommonOpts,
) -> anyhow::Result<()> {
    let mut write = common.open(&cli_query.write)?;
    if let Some(query) = cli_query.into_query(&common)? {
        tracing::trace!(?query, "Query");
        let mut e = WatchEntry::new(Default::default(), query, 0, (), debug_span!("Select"))?;
        tracing::trace!(?e, "Watching");
        let inp = common.inp_reader()?;
        for pkt in inp {
            tracing::trace!(?pkt, "recv");
            let pkt = pkt?;
            let recv_pkt = RecvPktPtr {
                recv: now(),
                pkt: &pkt,
            };
            let (test_ok, cnt) = e.test(recv_pkt);
            tracing::trace!(test_ok, ?cnt, ?pkt, "Test pkt");
            if test_ok {
                common.write_multi_dest(&mut write, &**pkt, None)?;
            } else {
                common.write_multi_dest(&mut write_false, &**pkt, None)?;
            }
            if cnt.is_break() {
                break;
            }
        }
    };
    Ok(())
}
