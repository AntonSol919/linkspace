// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use linkspace_pkt::{NetPkt, NetPktBox, Stamp};
use std::io;

use super::write_result::WriteResult;

pub fn save_pkt(mut writer: impl SWrite, pkt: impl NetPkt) -> io::Result<bool> {
    let (count, _) = writer.write_many_state(&mut std::iter::once(&pkt as &dyn NetPkt), None)?;
    Ok(count > 0)
}
pub fn save_pkts(
    mut writer: impl SWrite,
    pkts: &[NetPktBox],
) -> io::Result<(usize, Option<Stamp>)> {
    let mut it = pkts
        .iter()
        .map(|p| p as &linkspace_pkt::NetPktPtr as &dyn NetPkt);
    writer.write_many_state(&mut it, None)
}
pub trait SWrite {
    fn write_many_state<'o>(
        &mut self,
        pkts: &'o mut dyn Iterator<Item = &'o dyn NetPkt>,
        out: Option<&'o mut dyn FnMut(&'o dyn NetPkt, bool) -> Result<bool, ()>>,
    ) -> io::Result<(usize, Option<Stamp>)>;
}
impl<X: SWrite> SWrite for &mut X {
    fn write_many_state<'o>(
        &mut self,
        pkts: &'o mut dyn Iterator<Item = &'o dyn NetPkt>,
        out: Option<&'o mut dyn FnMut(&'o dyn NetPkt, bool) -> Result<bool, ()>>,
    ) -> io::Result<(usize, Option<Stamp>)> {
        (**self).write_many_state(pkts, out)
    }
}
#[derive(Debug)]
pub enum WriteState {
    Pending,
    Processing,
    Error(std::io::Error),
    Finished { is_new: bool },
}
impl WriteState {
    pub fn into_result(self) -> io::Result<WriteResult> {
        use std::io::*;
        match self {
            WriteState::Pending => Err(Error::other("Write still pending")),
            WriteState::Processing => Err(Error::other("Write still processing")),
            WriteState::Finished { is_new } => Ok(WriteResult::from(is_new, ())),
            WriteState::Error(e) => Err(e),
        }
    }
}
