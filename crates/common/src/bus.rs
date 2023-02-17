// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/// wrapper around ipcbus to notify processes of new packets. 

use std::thread::JoinHandle;
pub use ipcbus;
use ipcbus::UdpIPC;
use linkspace_core::{ prelude::*};

#[derive(Clone)]
pub struct LinkspaceBUS { ipc: UdpIPC, pid: u32}
impl LinkspaceBUS {

    pub fn setup_udp(port: u16) -> LinkspaceBUS {
        let ipc = UdpIPC::new(port);
        let pid = std::process::id();
        LinkspaceBUS{ipc,pid}
    }
    pub fn emit(&self, ev: LocalLogPtr) -> std::io::Result<()> {
        tracing::trace!(ev=?ev,"Writing to bus");
        let msg= [&self.pid.to_ne_bytes() as &[u8],&ev.into_bytes(),&routing_bits()].concat();
        self.ipc.send(&msg)
    }
    pub fn notify_env_thread(&self, env: BTreeEnv)-> JoinHandle<()>{
        if BUS.get().is_some() { todo!()}
        let chan = match env.raw_log_head(){
            either::Either::Left(brd) => brd.clone(),
            either::Either::Right(_) => todo!(),
        };
        self.rx_thread(move |ev| {tracing::trace!(ev=?ev,"Received");let _ = chan.send(ev);})
    }


    pub fn raw_rx_socket(&self) -> &Arc<ipcbus::udp_multicast::Socket> {
        &self.ipc.rx
    }

    pub fn rx_thread<RX: Fn(LocalLogPtr) + Send + 'static>(&self, recv: RX) -> JoinHandle<()>{
        let pid = self.pid.to_ne_bytes();
        self.ipc.rx_thread(move |bytes| {
            let (ev_pid,rest) = bytes.split_at(4);
            if ev_pid == pid { return tracing::trace!("Ignoring bounced ev");}
            let (ev,rest) = rest.split_at(8);
            let lp = match <[u8;8]>::try_from(ev){
                Ok(lp) => LocalLogPtr::from(lp),
                Err(_) => todo!(),
            };
            tracing::trace!(lp=?lp,"Recv");
            if &routing_bits() != rest {
                tracing::warn!("Multiple databases on same socket");
                std::process::exit(0);
            }
            recv(lp)
        })
    }
}



