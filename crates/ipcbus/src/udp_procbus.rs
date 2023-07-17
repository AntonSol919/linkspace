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
    time::Instant, path::PathBuf,
};

pub use crate::udp_multicast::UdpIPC;
pub use event_listener;
use event_listener::{Event, EventListener};

pub fn get_port(bus_id: u64) -> std::io::Result<u16> {
    let path = std::env::temp_dir().join("procbus.map.lock");
    let mut lockfile = fslock::LockFile::open(&path)
        .map_err(|e| {eprintln!("cant open lock file {path:?}"); e})?;
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
            return Ok(saved_port);
        };
    }
    bytes.extend(bus_id.to_ne_bytes());
    new_port = new_port.saturating_add(1);
    bytes.extend(new_port.to_ne_bytes());
    std::fs::write(&path, bytes).unwrap();
    lockfile.unlock().unwrap();
    Ok(new_port)
}

pub struct ProcBus(Arc<Inner>);
use std::sync::OnceLock;

struct Inner {
    val: AtomicU64,
    // Idealy this would be done within memory, but this is the simplest to implement crossplatform for now
    pid: u32,
    udp: UdpIPC,
    listener: OnceLock<JoinHandle<()>>,
    proc: Event,
    bus_id: u64,
}

impl ProcBus {
    pub fn new(path: &PathBuf) -> std::io::Result<ProcBus> {
        tracing::debug!("using UDP for IPC signals");
        let bus_id = u64::from_be_bytes(std::fs::read(path.join("id")).expect("missing id file").try_into().expect("bad id file"));
        let port = get_port(bus_id)?;
        let pid = std::process::id();
        
        Ok(ProcBus(Arc::new(Inner{
            bus_id,
            pid,
            udp:UdpIPC::new(port),
            val: Default::default(),
            listener:OnceLock::new(),
            proc: Default::default(),
        })))
    }
    
    pub fn emit(&self, val: u64) -> u64 {
        self._emit::<false>(val)
    }
    pub fn _emit<const SKIP_UDP: bool>(&self, val: u64) -> u64 {
        let mut old = self.0.val.load(Ordering::Relaxed);
        loop {
            if old > val {
                return old;
            }
            match self.0
                .val
                .compare_exchange_weak(old, val, Ordering::SeqCst, Ordering::Relaxed)
            {
                Ok(_) => break,
                Err(x) => old = x,
            }
        }
        if !SKIP_UDP {
            let msg = [
                &self.0.bus_id.to_ne_bytes() as &[u8],
                &self.0.pid.to_ne_bytes(),
                &val.to_ne_bytes(),
            ].concat();
            if let Err(e) = self.0.udp.send(&msg) {
                tracing::error!(e=?e,"IPC UDP Bus");
            }
        }
        tracing::trace!(ptr=%format!("{:p}",&self.0.val),"Notify");
        self.0.proc.notify(usize::MAX);
        val
    }
    
    pub fn init(&self){
        self.0.listener.get_or_init(move || {
            let this = ProcBus(self.0.clone());
            self.0.udp.rx_thread(move |b| {
                let (busid, rest) = b.split_at(8);
                let busid = u64::from_ne_bytes(busid.try_into().unwrap());
                if busid != this.0.bus_id {
                    tracing::error!("Wrong bus id !! ( Did you delete the database? )");
                    return;
                }
                let (origin, rest) = rest.split_at(4);
                if origin == this.0.pid.to_ne_bytes() {
                    return;
                }
                let val = u64::from_ne_bytes(rest.try_into().unwrap());
                this._emit::<true>(val);
            })
        });
    }

    pub fn proc_listener(&self) -> EventListener {
        self.0.proc.listen()
    }
    pub fn val(&self) -> u64 {
        self.0.val.load(Ordering::SeqCst)
    }
    pub fn next_d(&self, deadline: Option<Instant>) -> Option<u64> {
        tracing::trace!(ptr=%format!("{:p}",&self.0.val),"Waiting");
        match deadline {
            Some(d) => {
                if !self.0.proc.listen().wait_deadline(d) {
                    tracing::trace!("Timeout");
                    return None;
                }
            }
            None => self.0.proc.listen().wait(),
        };
        tracing::trace!("Wakeup");
        Some(self.0.val.load(Ordering::SeqCst))
    }
    pub async fn next_async(&self) -> u64 {
        tracing::trace!("Async Wait");
        self.0.proc.listen().await;
        tracing::trace!("Async Ok");
        self.0.val.load(Ordering::SeqCst)
    }

    /*
    /// Set the proc bus's bus id.
    pub fn set_bus_id(&mut self, bus_id: u64) {
        self.bus_id = bus_id;
    }
    */
}
