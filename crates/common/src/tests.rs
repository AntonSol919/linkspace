// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/*
use crate::{protocols::impex::blob::checkin_bytes, prelude::*};
fn btree_sanity<S:SyncDB>(store: BTreeStore<S>) -> anyhow::Result<()>{
    routing_bits();
    assert!(store.get_reader().partial_matches(PartialHash::min()).next().is_none(),"is_empty");
    {
        let domain1 = as_domain(b"a");
        let spath = spath_str("/hello").expect("parse spath");
        let a1 = spoint_ref(PUBLIC,domain1,&spath,&[],Stamp::new(10),());
        let a2 = spoint_ref(PUBLIC,domain1,&spath,&[],Stamp::new(11),());
        store.get_writer().write_many_iter([a1,a2])?;
    }
    assert_eq!(store.get_reader().dump().count(),2," has two spoints");
    {
        let domain1 = as_domain(b"b");
        let spath = spath_str("/hello").expect("parse spath");
        let a1 = spoint_ref(PUBLIC,domain1,&spath,&[],Stamp::new(10),());
        let a2 = spoint_ref(PUBLIC,domain1,&spath,&[],Stamp::new(11),());
        store.get_writer().write_many_iter([a1,a2])?;
    }

    let reader = store.get_reader();
    assert_eq!(reader.dump().count(),4," has four spoints");
    assert_eq!(reader.tree(TreeQuery::new().domain("a").max_depth(255)).count(),2," Two in 'a' domain");
    assert_eq!(reader.tree(TreeQuery::new().domain("b").max_depth(255)).count(),2," Two in 'b' domain");
    assert_eq!(reader.tree(TreeQuery::new().domain("a").max_depth(255).rev_range()).count(),2," Two in 'a' domain");
    assert_eq!(reader.tree(TreeQuery::new().domain("b").max_depth(255).rev_range()).count(),2," Two in 'b' domain");
    ::std::mem::drop(reader);
    {
      // let data = unsafe{memmap2::Mmap::map(&file)}.unwrap();
        let mut bytes = vec![0;170*MAX_PKT_SIZE];
        rand::rngs::mock::StepRng::new(1, 1).fill_bytes(&mut bytes);
        let ((hash,_kind),_is_new) = checkin_bytes(&mut store.get_writer(), &*bytes, ().into(), Some((PUBLIC,as_domain(b"domaintest")))).unwrap().unwrap();
        let reader = store.get_reader();
        let pkt = reader.read(&hash).unwrap().expect("HashJust written");
        for r in pkt.body().get_links(){
            reader.read(&r.hash).unwrap().expect("Ref to have been written");
        }
    }
    store.validate().expect("Valid store");
    Ok(())
}

use rand::RngCore;
use tracing_test::traced_test;
#[traced_test]
#[test]
fn inmem(){
    let store = crate::init_db::inmem();
    btree_sanity(store).expect("OK");
}
#[cfg(feature="lmdb")]
#[traced_test]
#[test]
fn lmdb(){
    let p = format!("/tmp/{}/",now().get());
    let p = std::path::Path::new(&p);
    std::fs::create_dir(&p).unwrap();
    let store = crate::init_db::lmdb(&p);
    btree_sanity(store).expect("OK");
}
*/

#[test]
fn serde_abtxt() {
    use crate::prelude::*;
    let null: Domain = r#"a\0b"#.parse().unwrap();
    let escaped: Domain = r#"a\\0b"#.parse().unwrap();
    assert_ne!(null, escaped);
    assert_eq!(null.cut_ending_nulls(), b"a\0b");
    assert_eq!(format!("{}", escaped), r#"a\\0b"#);
    let null_js = serde_json::to_string(&null).unwrap();
    let escaped_js = serde_json::to_string(&escaped).unwrap();
    assert_ne!(null_js, escaped_js);
}
