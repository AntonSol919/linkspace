use anyhow::Context;
use linkspace::{
    point::{lk_datapoint_ref, lk_linkpoint_ref},
    prelude::*,
    query::lk_query_compile,
    runtime::{lk_get_all, lk_save_all_ext},
};

use tracing_test::traced_test;

fn init_lk() -> Linkspace {
    std::env::set_var("LK_FORCE_EMPTY", "true");
    let _ = std::fs::remove_dir_all("/tmp/lktests");
    lk_open(Some("/tmp/lktests".as_ref()), true).unwrap()
}

#[test]
#[traced_test]
fn lk_watch_checks_recv_stamp() -> LkResult<()> {
    let lk = init_lk();
    let pkts = &[
        &lk_datapoint_ref(b"1")? as &dyn NetPkt, // tykMj7QFUs9PwFvZN4C-Vd06puqsvwO80VTDxSjTjR0
        &lk_datapoint_ref(b"2")?,                // ef6IfBb6szkE-MIENvuiQo5AZqz9o2cjWLkTfjI3SeM
        &lk_datapoint_ref(b"3")?,                // Zsu1AIcF7LrGWRbTgA3AdwtObQB0pXIcC3-mv_eeXLc
    ];
    let (range_start, range_end) = lk_save_all_ext(&lk, pkts, false)?;
    assert_eq!(range_end.get() - range_start.get(), 3);

    tracing::warn!(%range_start,%range_end,"save ok");
    let q = lk_query_push(lk_query(&Q), "recv", ">", &range_start.0)?;

    let q = lk_query_push(q, "", "mode", b"log-asc")?;
    eprintln!("QUERY = {q}");
    let frst_match = lk_get(&lk, &q)?.context("expected some match")?;
    assert_eq!(frst_match.data(), b"2");

    let q = lk_query_push(q, "", "mode", b"hash-asc")?;
    eprintln!("QUERY = {q}");
    let mut lst = vec![];
    let total = lk_get_all(&lk, &q, &mut |p| {
        lst.push(p.as_netbox());
        false
    })?;
    assert_eq!(total, -2);
    assert_eq!(lst[0].data(), b"3"); // base64 'Zsu...' < 'ef6...'
    assert_eq!(lst[1].data(), b"2");

    // The tree index will order by create stamp. We're interested in correctly checking recv stamp
    let pkts = &[
        &lk_linkpoint_ref(
            b"3",
            ab(b""),
            PRIVATE,
            RootedSpace::empty(),
            &[],
            Some(3.into()),
        )? as &dyn NetPkt,
        &lk_linkpoint_ref(
            b"2",
            ab(b""),
            PRIVATE,
            RootedSpace::empty(),
            &[],
            Some(2.into()),
        )?,
        &lk_linkpoint_ref(
            b"1",
            ab(b""),
            PRIVATE,
            RootedSpace::empty(),
            &[],
            Some(1.into()),
        )?,
    ];
    let (start, _) = lk_save_all_ext(&lk, pkts, false)?;
    let q = lk_query_push(q, "", "mode", b"tree-asc")?;
    let q = lk_query_push(q, "recv", ">", &start.0)?;
    eprintln!("TREE QUERY = {q}");
    let mut lst = vec![];
    let total = lk_get_all(&lk, &q, &mut |p| {
        lst.push(p.as_netbox());
        false
    })?;
    assert_eq!(total.abs(), 2);
    assert_eq!(lst[0].data(), b"1");
    assert_eq!(lst[1].data(), b"2");

    Ok(())
}

#[test]
fn query_compile() -> LkResult<()> {
    let pkts = [
        &lk_datapoint(b"1")?, // tykMj7QFUs9PwFvZN4C-Vd06puqsvwO80VTDxSjTjR0
        &lk_datapoint(b"2")?, // ef6IfBb6szkE-MIENvuiQo5AZqz9o2cjWLkTfjI3SeM
        &lk_datapoint(b"3")?, // Zsu1AIcF7LrGWRbTgA3AdwtObQB0pXIcC3-mv_eeXLc
    ];

    let q = lk_query_parse(Q.clone(), &["hash:=*:[b:ef6I]"], ())?;

    let mut func = lk_query_compile(q.clone())?;
    let matches = pkts.map(|p| func(p).0);
    assert_eq!(matches, [false, true, false]);

    Ok(())
}
