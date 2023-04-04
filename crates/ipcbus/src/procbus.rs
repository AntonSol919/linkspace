// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::Instant,
};

pub use crate::udp_multicast::UdpIPC;
pub use event_listener;
use event_listener::{Event, EventListener};
use socket2::Socket;

pub fn get_port(bus_id: u64) -> u16 {
    let mut lockfile =
        fslock::LockFile::open(&std::env::temp_dir().join("procbus.map.lock")).unwrap();
    lockfile.lock_with_pid().unwrap();
    let path = std::env::temp_dir().join("procbus.map");
    let mut bytes = match std::fs::read(&path) {
        Ok(v) => v,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                vec![]
            } else {
                panic!("{e:?}")
            }
        }
    };
    let mut new_port = 10501;
    for b in bytes.chunks_exact(10) {
        let saved_port = u16::from_ne_bytes([b[8], b[9]]);
        new_port = new_port.max(saved_port);
        if u64::from_ne_bytes(b[0..8].try_into().unwrap()) == bus_id {
            return saved_port;
        };
    }
    bytes.extend(bus_id.to_ne_bytes());
    new_port = new_port.saturating_add(1);
    bytes.extend(new_port.to_ne_bytes());
    std::fs::write(&path, bytes).unwrap();
    lockfile.unlock().unwrap();
    new_port
}

pub struct ProcBus {
    pub val: AtomicU64,
    // Idealy this would be done within memory, but this is the simplest to implement crossplatform for now
    pub ipc: Option<(u32, UdpIPC)>,
    pub proc: Event,
    pub bus_id: u64,
}

impl ProcBus {
    pub fn new(bus_id: u64) -> ProcBus {
        ProcBus {
            bus_id,
            val: Default::default(),
            ipc: None,
            proc: Default::default(),
        }
    }
    pub fn init_udp(self: &mut Arc<Self>) {
        self.init_udp_port(get_port(self.bus_id))
    }
    pub fn init_udp_port(self: &mut Arc<Self>, port: u16) {
        let mut this = Arc::get_mut(self).expect("You must init_udp before cloning a env");
        assert!(this.ipc.is_none());
        let pid = std::process::id();
        this.ipc = Some((pid, UdpIPC::new(port)));
        tracing::debug!(port = port, pid = pid, "Init udp")
    }

    pub fn emit(&self, val: u64) -> u64 {
        self._emit::<false>(val)
    }
    pub fn _emit<const SKIP_UDP: bool>(&self, val: u64) -> u64 {
        let mut old = self.val.load(Ordering::Relaxed);
        loop {
            if old > val {
                return old;
            }
            match self
                .val
                .compare_exchange_weak(old, val, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(x) => old = x,
            }
        }
        if !SKIP_UDP {
            if let Some((pid, ipc)) = &self.ipc {
                let msg = [
                    &self.bus_id.to_ne_bytes() as &[u8],
                    &pid.to_ne_bytes(),
                    &val.to_ne_bytes(),
                ]
                .concat();
                if let Err(e) = ipc.send(&msg) {
                    tracing::error!(e=?e,"IPC UDP Bus");
                }
            }
        }
        tracing::trace!(ptr=%format!("{:p}",&self.val),"Notify");
        self.proc.notify(usize::MAX);
        val
    }
    pub fn raw_recv_socket(&self) -> Option<&Arc<Socket>> {
        self.ipc.as_ref().map(|(_, ipc)| &ipc.rx)
    }
    /// This does not emit. Depending on your usecase you should do it yourself.
    pub fn decode_recv(&self, val: &[u8], ignore_pid: bool) -> Result<u64, &'static str> {
        let (busid, rest) = val.split_at(8);
        let busid = u64::from_ne_bytes(busid.try_into().unwrap());
        if busid != self.bus_id {
            tracing::warn!(expected=?self.bus_id, recv=?busid,"Wrong Busid?");
            return Err("recv from wrong bus");
        };
        let (origin, rest) = rest.split_at(4);
        if !ignore_pid && origin == self.ipc.as_ref().unwrap().0.to_ne_bytes() {
            return Err("Same origin");
        }
        let evid = u64::from_ne_bytes(rest.try_into().unwrap());
        Ok(evid)
    }
    pub fn setup_ipc_thread(self: &Arc<Self>) -> JoinHandle<()> {
        let this = self.clone();
        let ipc = self.ipc.as_ref().unwrap().clone();
        let pid = ipc.0;
        ipc.1.rx_thread(move |b| {
            let (busid, rest) = b.split_at(8);
            let busid = u64::from_ne_bytes(busid.try_into().unwrap());
            if busid != this.bus_id {
                tracing::error!("Wrong bus id !! ( Did you delete the database? )");
                return;
            }
            let (origin, rest) = rest.split_at(4);
            if origin == pid.to_ne_bytes() {
                return;
            }
            let val = u64::from_ne_bytes(rest.try_into().unwrap());
            this._emit::<true>(val);
        })
    }

    pub fn proc_listener(&self) -> EventListener {
        self.proc.listen()
    }
    pub fn val(&self) -> u64 {
        self.val.load(Ordering::SeqCst)
    }
    pub fn next_d(&self, deadline: Option<Instant>) -> Option<u64> {
        tracing::trace!(ptr=%format!("{:p}",&self.val),"Waiting");
        match deadline {
            Some(d) => {
                if !self.proc.listen().wait_deadline(d) {
                    tracing::trace!("Timeout");
                    return None;
                }
            }
            None => self.proc.listen().wait(),
        };
        tracing::trace!("Wakeup");
        Some(self.val.load(Ordering::SeqCst))
    }
    pub async fn next_async(&self) -> u64 {
        tracing::trace!("Async Wait");
        self.proc.listen().await;
        tracing::trace!("Async Ok");
        self.val.load(Ordering::SeqCst)
    }

    /*
    /// Set the proc bus's bus id.
    pub fn set_bus_id(&mut self, bus_id: u64) {
        self.bus_id = bus_id;
    }
    */
}
