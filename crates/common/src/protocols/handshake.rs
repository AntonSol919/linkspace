// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/// basic response-reply proving each side can sign with a key
use std::time::Duration;

use crate::{prelude::*, protocols::unicast_group};
use anyhow::ensure;
pub const HANDSHAKE_D: Domain = ab(b"\xFFhandshake");

pub const ID_SENTINAL_SPACENAME: RootedStaticSpace<17> = rspace1::<8>(b"sentinal");
pub const ANONYMOUSE_SPACENAME: RootedStaticSpace<18> = rspace1::<9>(b"anonymous");

const MAX_DIFF_SECONDS: usize = 15;
pub fn valid_stamp_range(stamp: Stamp, max_diff_sec: Option<usize>) -> anyhow::Result<()> {
    let dur = Duration::from_secs(max_diff_sec.unwrap_or(MAX_DIFF_SECONDS) as u64);
    match stamp_age(stamp) {
        Ok(d) => ensure!(
            d < dur,
            "check your clocks - packet is too old  {d:?} >= {dur:?}"
        ),
        Err(d) => ensure!(
            d < dur,
            "check your clocks - packet is too new {d:?} >= {dur:?}"
        ),
    };
    Ok(())
}

pub struct Phase0(pub NetPktBox);
pub struct Phase1(pub NetPktBox);
pub struct Phase2(pub NetPktBox);
pub struct Phase3(pub NetPktBox);
pub fn phase0_client_init(id: &SigningKey) -> Phase0 {
    tracing::trace!("Build phase0");
    let now = now();
    Phase0(
        keypoint(
            id.pubkey(),
            HANDSHAKE_D,
            &ID_SENTINAL_SPACENAME,
            &[],
            &[],
            now,
            id,
            (),
        )
        .as_netbox(),
    )
}
pub fn phase1_server_signs(
    theirs: &Phase0,
    id: &SigningKey,
    max_diff_sec: Option<usize>,
) -> anyhow::Result<Phase1> {
    tracing::trace!("Build Phase1");
    let theirs = &theirs.0;
    assert!(
        theirs.net_header().hop.get() > 0,
        "the rx channel is not calling net_header.hop() {:?}",
        theirs.net_header()
    );
    let their_key = match theirs.pubkey() {
        Some(their_key) => *their_key,
        None => anyhow::bail!("not signed"),
    };
    valid_stamp_range(*theirs.get_create_stamp(), max_diff_sec)?;
    let our_group = unicast_group(their_key, id.pubkey());
    ensure!(
        our_group != PRIVATE,
        "Connecting to yourself (using the same key) is currently not supported"
    );
    let links = [Link::new("auth", *theirs.hash())];
    Ok(Phase1(
        keypoint(
            our_group,
            HANDSHAKE_D,
            &ID_SENTINAL_SPACENAME,
            &links,
            &[],
            now(),
            id,
            (),
        )
        .as_netbox(),
    ))
}
pub fn phase2_client_signs(
    my_phase0: &Phase0,
    server_reply: &Phase1,
    id: &SigningKey,
    max_diff_sec: Option<usize>,
) -> anyhow::Result<(Phase2, PubKey)> {
    tracing::trace!("Build Phase2");
    ensure!(
        my_phase0.0.pubkey() == Some(&id.pubkey()),
        "identity mismatch"
    );
    let mine_hash = my_phase0.0.hash();
    let theirs = &server_reply.0;
    assert!(
        theirs.net_header().hop.get() > 0,
        "the rx channel is not calling net_header.hop() {:?}",
        theirs.net_header()
    );
    let their_key = match theirs.pubkey() {
        Some(p) => *p,
        None => anyhow::bail!("hello pkt not signed"),
    };
    valid_stamp_range(*theirs.get_create_stamp(), max_diff_sec)?;
    ensure!(
        theirs.get_links().iter().any(|r| r.ptr == mine_hash),
        "did not validate my hash {}",
        mine_hash
    );
    ensure!(
        theirs.domain() == Some(&HANDSHAKE_D),
        "not in the session domain"
    );
    let our_group = unicast_group(id.pubkey(), their_key);
    ensure!(
        theirs.group() == Some(&our_group),
        "not in the right group "
    );
    ensure!(
        theirs.get_rooted_spacename() == ID_SENTINAL_SPACENAME.as_ref(),
        "wrong spacename"
    );
    let now = now();
    let proof = keypoint(
        our_group,
        HANDSHAKE_D,
        &ID_SENTINAL_SPACENAME,
        &[Link::new("signed", *theirs.hash())],
        &[],
        now,
        id,
        (),
    )
    .as_netbox();
    Ok((Phase2(proof), their_key))
}
pub fn phase3_server_verify(
    their_init: &Phase0,
    my_phase1: &Phase1,
    theirs: &Phase2,
    id: &SigningKey,
) -> anyhow::Result<PubKey> {
    ensure!(
        my_phase1.0.pubkey() == Some(&id.pubkey()),
        "your identity mismatch"
    );
    let theirs = theirs.0.as_netbox();
    let mine_hash = my_phase1.0.hash();
    let their_key = match theirs.pubkey() {
        Some(p) => {
            ensure!(theirs.pubkey() == their_init.0.pubkey(), "switched keys");
            *p
        }
        None => anyhow::bail!("hello pkt not signed"),
    };
    let our_group = unicast_group(id.pubkey(), their_key);
    ensure!(
        theirs.get_links().iter().any(|r| r.ptr == mine_hash),
        "did not validate my hash {}",
        mine_hash
    );
    ensure!(
        theirs.get_rooted_spacename() == ID_SENTINAL_SPACENAME.as_ref(),
        "wrong spacename"
    );
    ensure!(
        theirs.domain() == Some(&HANDSHAKE_D),
        "not in the session domain"
    );
    ensure!(theirs.group() == Some(&our_group), "not in the right group");
    Ok(their_key)
}
