// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
pub type ProcBus = multi::ProcBus;

/// A thread watches using inotify and notifies multiple other thread through event-listener
pub mod multi {
    use std::{
        fs::FileTimes,
        io::{self, Read},
        os::fd::{AsRawFd, FromRawFd},
        path::Path,
        sync::atomic::{AtomicU64, Ordering},
        time::SystemTime,
    };

    use event_listener::{Event, EventListener};
    use memmap2::MmapMut;
    use nix::sys::inotify::{AddWatchFlags, InitFlags, Inotify};
    use std::fs::File;
    use std::{path::PathBuf, sync::Arc, thread::JoinHandle, time::Instant};

    pub struct ProcBus(Arc<Inner>);

    pub struct Inner {
        pub watch_thread: std::sync::OnceLock<JoinHandle<()>>,
        pub path: PathBuf,
        file: File,
        pub proc: Event,
        map: MmapMut,
    }
    impl ProcBus {
        pub fn new(path: &Path) -> io::Result<ProcBus> {
            tracing::debug!("using inotify for IPC signals");
            let path = path.join("ipc.inotify");
            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .read(true)
                .open(&path)?;
            file.set_len(16)?;
            let map = unsafe { memmap2::MmapOptions::new().map_mut(&file)? };

            Ok(ProcBus(Arc::new(Inner {
                watch_thread: Default::default(),
                proc: Default::default(),
                path,
                file,
                map,
            })))
        }
    }

    impl ProcBus {
        pub fn val_ptr(&self) -> &AtomicU64 {
            unsafe { &*self.0.map.as_ptr().cast() }
        }
        pub fn val(&self) -> u64 {
            self.val_ptr().load(Ordering::Relaxed)
        }

        pub fn emit(&self, val: u64) -> u64 {
            self._emit::<false>(val)
        }
        pub fn _emit<const SKIP_NOTIFY: bool>(&self, val: u64) -> u64 {
            let at = self.val_ptr().fetch_max(val, Ordering::Relaxed);
            if at == val {
                return at;
            }
            if !SKIP_NOTIFY {
                if let Err(e) = self
                    .0
                    .file
                    .set_times(FileTimes::new().set_accessed(SystemTime::now()))
                {
                    tracing::warn!(?e, "can't notify");
                }
            }
            tracing::trace!(ptr=%format!("{:p}",&self.val_ptr()),"Notify");
            self.0.proc.notify(usize::MAX);
            val
        }

        pub fn init(&self) {
            self.setup_ipc_thread()
        }

        pub fn setup_ipc_thread(&self) {
            let this = ProcBus(self.0.clone());
            self.0.watch_thread.get_or_init(move || {
                std::thread::spawn(move || {
                    let instance = Inotify::init(InitFlags::empty()).unwrap();
                    let _wd = instance
                        .add_watch(&this.0.path, AddWatchFlags::IN_ACCESS)
                        .unwrap();
                    let mut file = unsafe { File::from_raw_fd(instance.as_raw_fd()) };
                    let mut buf = [0; 32];
                    loop {
                        file.read(&mut buf).unwrap();
                        this._emit::<true>(0);
                    }
                })
            });
        }

        pub fn proc_listener(&self) -> EventListener {
            self.0.proc.listen()
        }

        pub fn next_d(&self, deadline: Option<Instant>) -> Option<u64> {
            match deadline {
                Some(d) => {
                    if !self.0.proc.listen().wait_deadline(d) {
                        return None;
                    }
                }
                None => self.0.proc.listen().wait(),
            };
            Some(self.val_ptr().load(Ordering::Relaxed))
        }
        pub async fn next_async(&self) -> u64 {
            self.0.proc.listen().await;
            self.val_ptr().load(Ordering::Relaxed)
        }
    }
}

//// each thread uses poll to watch for changes
mod solo {
    // let mut events = unsafe{ std::fs::File::from_raw_fd(instance.as_raw_fd())};
    // let mut set = [poll::PollFd::new(instance.as_raw_fd(),poll::PollFlags::POLLIN)];
    // let p = poll::ppoll(&mut set,None, None).unwrap();
}
