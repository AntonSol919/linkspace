// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::handlers::{FollowHandler, PktStreamHandler, SinglePktHandler, StopReason};
use anyhow::{bail, Context};
pub use async_executors::{Timer, TimerExt};
pub use futures::task::{LocalSpawn, LocalSpawnExt};
use linkspace_core::prelude::{*, lmdb::BTreeEnv };
use linkspace_pkt::reroute::ShareArcPkt;
use std::{
    borrow::{Cow},
    cell::{Cell, OnceCell, RefCell},
    ops::{ ControlFlow},
    rc::{Rc },
    time::{Duration, Instant}, path::{Path, PathBuf},
};
use tracing::{warn, Span, debug_span, instrument};

pub type PktStream = Box<dyn PktStreamHandler + 'static>;
pub type Matcher = linkspace_core::matcher::Matcher<PktStream>;
/// [WatchEntry] with an associated callback (Box<dyn [PktStreamHandler]>)
pub type RxEntry = linkspace_core::matcher::WatchEntry<PktStream>;

#[derive(Clone)]
#[must_use = "Linkspace runtime does nothing unless processed"]
pub struct Linkspace(Rc<Inner>);

pub struct Inner {
    exec: Executor,
    files : Option<PathBuf>,
    spawner: OnceCell<Rc<dyn LocalAsync>>,
}


impl std::fmt::Debug for Linkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Linkspace").field("todo", &"todo").finish()
    }
}
enum Pending {
    NewWatch { watch_entry: RxEntry },
    Close { id: QueryID, range: bool },
}


struct Executor {
    env: BTreeEnv,
    written: Cell<bool>,
    callbacks: RefCell<Matcher>,
    pending: RefCell<Vec<Pending>>,
    // This is a fake lifetime
    process_txn: RefCell<Rc<ReadTxn<'static>>>,
    process_upto: Cell<Stamp>,
    is_reading: Cell<usize>,
    is_running: Cell<bool>,
}

impl Linkspace {
    pub fn get_reader<'env:'txn,'txn>(&'env self) -> Rc<ReadTxn<'txn>> {
        self.0.exec.process_txn.borrow().clone()
    }
    
    pub fn env(&self) -> &BTreeEnv {
        &self.0.exec.env
    }
    pub fn spawner(&self) -> &OnceCell<Rc<dyn LocalAsync>> {
        &self.0.spawner
    }
    pub fn files(&self) -> Option<&Path>{
        self.0.files.as_deref()
    }

    
    
    pub fn new(env: BTreeEnv, spawner: Rc<dyn LocalAsync>) -> Linkspace {
        Self::new_opt_rt(env, OnceCell::from(spawner))
    }

    pub fn new_opt_rt(env: BTreeEnv, spawner: OnceCell<Rc<dyn LocalAsync>>) -> Linkspace {
        let reader : ReadTxn<'_>= env.new_read_txn().unwrap();
        let reader : ReadTxn<'static> = unsafe { std::mem::transmute(reader)};
        let at = reader.log_head();
        // TODO make this an option
        let files = Some(env.location().join("files"));

        Linkspace(Rc::new(Inner{

            spawner,
            files,
            exec: Executor {
                env,
                written: Cell::new(false),
                callbacks: Default::default(),
                pending: Default::default(),
                process_txn: RefCell::new(Rc::new(reader)),
                process_upto: at.into(),
                is_running: Cell::new(false),
                is_reading: Cell::new(0),
                //subroutines:RefCell::new(Registry::new())
            }}))
    }

    fn insert_watch(&self, watch_entry: RxEntry) {
        match self.0.exec.callbacks.try_borrow_mut() {
            Ok(mut lk) => {
                if let Some(w) = lk.register(watch_entry) {
                    drop_watch(w, self, StopReason::Replaced)
                }
            }
            Err(_) => self
                .0
                .exec
                .pending
                .borrow_mut()
                .push(Pending::NewWatch { watch_entry }),
        };
    }
    pub fn drain_pending(&self, lk: &mut Matcher) {
        for cmd in self.0.exec.pending.borrow_mut().drain(..) {
            match cmd {
                Pending::NewWatch { watch_entry } => {
                    if let Some(w) = lk.register(watch_entry) {
                        drop_watch(w, self, StopReason::Replaced)
                    }
                }
                Pending::Close { id, range } => {
                    lk.deregister(&id, range, |w| drop_watch(w, self, StopReason::Closed));
                }
            }
        }
    }
    pub async fn poll(&self) -> Stamp {
        loop {
            self.process();
            let rt_head = self.0.exec.process_upto.get();
            let env_head = self.env().log_head().await;
            if env_head == rt_head {
                return env_head;
            }
        }
    }

    fn watch_status(&self,id:&QueryIDRef) -> Option<WatchStatus>{
        let cbs = self.0.exec.callbacks.borrow();
        Some(cbs.get(id)?.status())
    }

    /**
    continuously process callbacks until:
    - now > last_step => returns 0
    - qid = Some and qid is matched => if removed 1, if waiting for more -1
    - qid = None => no more callbacks (1) 
     **/
    #[instrument(skip(self))]
    pub fn run_while(
        &self,
        last_step: Option<Instant>,
        user_qid: Option<&QueryIDRef>
    ) -> anyhow::Result<isize> {
        let exec = &self.0.exec;
        if exec.is_running.get() {
            bail!("already running")
        }
        if exec.is_reading.get() > 0  {
            tracing::warn!("Using a process_while nested in read txn can eat up memory. it might become an error in the future");
        }
        tracing::trace!(
            last_step_in=?last_step.map(|i| i-Instant::now()),
            "run while");
        let last_new_pkt = Instant::now();
        let current_state = match user_qid{
            Some(id) => Some((id,self.watch_status(id).with_context(||anyhow::anyhow!("watch id '{}' not found",AB(id)))?)),
            _ => None
        };
        // check the break conditions, and update 'next_check' as required for next check
        loop {
            let _log_head = self.process();
            if let Some((user_qid,old_status)) = current_state{
                let status = match self.watch_status(user_qid){
                    Some(v) => v,
                    None => {
                        tracing::debug!("watch was dropped");
                        return Ok(1);
                    }
                };
                if status.watch_id != old_status.watch_id { tracing::debug!("Watch was overwritten"); return Ok(1);}
                if status.nth_query != old_status.nth_query { tracing::debug!("Watch was triggered (is_done=false)"); return Ok(-1);}
            }
            let mut next_check = last_new_pkt + Duration::from_micros(Stamp::MAX.get());
            let newtime = Instant::now();
            let d = |i| i-newtime;

            if let Some(term) = last_step {
                let wait_dur = match term.checked_duration_since(newtime) {
                    Some(v) => v,
                    None => {
                        tracing::debug!("last_step reached");
                        return Ok(0);
                    }
                };
                let last_step_constraint = newtime+wait_dur;
                tracing::trace!(
                    next_check=?d(next_check),
                    last_step_constraint=?d(last_step_constraint),
                    "set Until constraining");
                next_check = next_check.min(last_step_constraint);
            }

            match exec.next_work() {
                Ok(Some(next_oob)) => {
                    if next_oob != Stamp::MAX {
                        match next_oob.get().checked_sub(now().get()) {
                            Some(micros) => {
                                let next_recv_oob = newtime + Duration::from_micros(micros);
                                tracing::trace!(
                                    next_check=?d(next_check),
                                    next_recv_oob=?d(next_recv_oob),
                                    "set packet recv OOB constraint");

                                next_check = next_check.min(next_recv_oob)
                            }
                            None => continue,
                        }
                    }
                }
                Ok(None) => {
                    continue;
                }
                Err(_) => {
                    tracing::debug!("no more callbacks");
                    return Ok(1);
                }
            };

            tracing::debug!(wakeup=?d(next_check), "waiting for new event");
            self.env().next_deadline(Some(next_check));
        }
    }

    /// check the log for new packets and execute callbacks
    #[instrument(skip(self))]
    pub fn process(&self) -> Stamp {
        let exec = &self.0.exec;
        exec.written.set(false);
        let (txn, from, upto): (Rc<ReadTxn>, Stamp, Stamp) = {
            let mut txn = exec.process_txn.borrow_mut();
            let rx_last = exec.process_upto.get();
            match Rc::get_mut(&mut txn) {
                Some(txn) => {
                    tracing::trace!(?rx_last, "refresh txn");
                    txn.refresh();
                }
                None => {
                    tracing::warn!("holding a read txn across callbacks!");
                    // the transmute sets the lifetime which is correct. 
                    // the real danger is that the open txn eats up memory
                    *txn = Rc::new(unsafe{std::mem::transmute(exec.env.new_read_txn().unwrap())});
                }
            }
            let txn_last = txn.log_head();
            if rx_last >= txn_last {
                tracing::debug!(?txn_last, ?rx_last, "Already processed");
                return rx_last;
            }
            (txn.clone(), rx_last, txn_last)
        };
        let _g = tracing::error_span!("Processing txn", ?from, ?upto).entered();
        tracing::trace!("Lock cbs");
        let mut lock = exec.callbacks.borrow_mut();
        self.drain_pending(&mut lock);
        exec.is_running.set(true);
        let i = 0;
        let mut count = Rc::strong_count(&txn);
        let mut validate = || {
            if Rc::strong_count(&txn) != count {
                warn!("holding txn");
            }
            count = Rc::strong_count(&txn)
        };
        for pkt in txn.pkts_after(from) {
            if pkt.net_header().flags.contains(NetFlags::SILENT) {
                tracing::trace!("(not) skipping silent pkt - TODO make this a option");
            }
            let _g = tracing::error_span!("Matching",pkt=%PktFmtDebug(&pkt)).entered();
            tracing::debug!("Testing New Pkt");

            let pkt = ShareArcPkt {
                arc: OnceCell::new(),
                pkt,
            };

            lock.trigger(
                *pkt,
                |p| {
                    let r = p.handle_pkt(&pkt, self);
                    validate();
                    r
                },
                |w| {
                    let reason = if w.last_test.1.is_break() {
                        StopReason::Break
                    } else {
                        StopReason::Finish
                    };
                    drop_watch(w, self, reason)
                },
            );
        }
        tracing::debug!(npkts = i, "Updated Finished");
        self.drain_pending(&mut lock);
        
        exec.is_running.set(false);
        exec.process_upto.set(upto);
        std::mem::drop((txn, lock));
        if !exec.written.get() { return upto};

        tracing::trace!("Written true");
        self.process()
    }

    pub fn read<F>(&self, hash: LkHash, rx: F, watchid: QueryID, span: Span) -> anyhow::Result<()>
    where
        F: FnOnce(&dyn NetPkt, &Linkspace) + 'static,
    {
        let reader = self.get_reader();
        if let Some(dbp) = reader.read(&hash)? {
            let _g = tracing::debug_span!(parent: &span, "Local").entered();
            rx(&dbp, self);
            return Ok(());
        }
        if !watchid.is_empty() {
            let e = RxEntry::new(
                watchid,
                Query::hash_eq(hash),
                0,
                Box::new(SinglePktHandler(Some(rx))),
                span,
            )?;
            self.insert_watch(e);
        }
        Ok(())
    }
    /// automatically handle the options 'follow', 'start', 'mode', and 'qid'
    pub fn watch_query(
        &self,
        query: &Query,
        onmatch: impl PktStreamHandler + 'static,
        span: Span,
    ) -> anyhow::Result<i32> {
        let mode = query.get_mode()?;
        let id :&[u8]= query.qid()?.flatten().expect("watch always requires a :qid:...  option");
        let follow = query.get_known_opt(KnownOptions::Follow)?;
        let start = None; //query.get_known_opt(KnownOptions::Start).map(|v| Ptr::try_from(v.clone())).transpose()?;
                          // TODO span should already have these fields.
        let span = tracing::debug_span!(parent: &span, "with_opts", id=?AB(id), ?follow, ?mode, ?start);
        match follow.is_some() {
            true => {
                let onmatch = FollowHandler { inner: onmatch };
                Ok(self.watch(
                    id,
                    mode,
                    Cow::Borrowed(query),
                    onmatch,
                    start,
                    span,
                )?)
            }
            false => Ok(self.watch(
                id,
                mode,
                Cow::Borrowed(query),
                onmatch,
                start,
                span,
            )?),
        }
    }
    /// only checks predicates, does not handle any options.
    pub fn watch(
        &self,
        watch_id: &QueryIDRef,
        mode: query_mode::Mode,
        q: Cow<Query>,
        mut onmatch: impl PktStreamHandler + 'static,
        start: Option<LkHash>,
        span: Span,
    ) -> anyhow::Result<i32> {
        let exec = &self.0.exec;
        if start.is_some() {
            panic!("todo")
        }
        let span = debug_span!(parent:&span,"query", preds=%q.predicates);
        let mut counter = 0;
        let check_db = q.predicates.state.check_db();
        self.close(watch_id); // this is not ideal. But other close semantics seem worse.
        if check_db {
            let local_span = tracing::debug_span!(parent: &span, "DB Callback").entered();
            tracing::trace!(?mode);
            let reader = self.get_reader();
            exec.is_reading.update(|i| i+1);
            let r = reader
                .query(mode, &q.predicates, &mut counter)?
                .try_for_each(|dbp| {
                    let _g = local_span.enter();
                    tracing::debug!(pkt=%PktFmtDebug(&dbp.pkt), recv=%dbp.recv().unwrap(),"Match");
                    onmatch.handle_pkt(&dbp, self)
                });

            let strong_count = Rc::strong_count(&reader);
            if strong_count > 2{
                if exec.is_reading.get() > 0 || exec.is_running.get(){ tracing::debug!("Assuming open txn is on purpose")}
                else {warn!(strong_count,"Holding txn open")};
            }
            exec.is_reading.update(|i|i-1);
            tracing::debug!(?r,"Done with DB");
            if matches!(r, ControlFlow::Break(_)) {
                return Ok(crate::saturating_cast(counter))
            }
        }
        match RxEntry::new(watch_id.to_vec(), q.into_owned(), counter, Box::new(onmatch), span) {
            Ok(e) => {
                tracing::debug!("Setup Watch");
                self.insert_watch(e)
            }
            Err(r) => tracing::info!(e=?r,"Did not register"),
        }
        Ok(crate::saturating_neg_cast(counter))
    }

    pub fn close(&self, id: impl AsRef<QueryIDRef>) {
        match self.0.exec.callbacks.try_borrow_mut() {
            Ok(mut lk) => {
                lk.deregister(id.as_ref(), false, |w| {
                    drop_watch(w, self, StopReason::Closed)
                });
            }
            Err(_) => self.0.exec.pending.borrow_mut().push(Pending::Close {
                id: id.as_ref().to_vec(),
                range: false,
            }),
        }
    }
    pub fn close_range(&self, prefix: impl AsRef<QueryIDRef>) {
        match self.0.exec.callbacks.try_borrow_mut() {
            Ok(mut lk) => {
                lk.deregister(prefix.as_ref(), true, |w| {
                    drop_watch(w, self, StopReason::Closed)
                });
            }
            Err(_) => self.0.exec.pending.borrow_mut().push(Pending::Close {
                id: prefix.as_ref().to_vec(),
                range: true,
            }),
        }
    }
    /// Will panic if called during execution
    pub fn dbg_watches(&self) -> std::cell::Ref<Matcher> {
        self.0.exec.callbacks.borrow()
    }
}
impl Executor{
    /** return when next to process.
    Ok(None) means immediatly, Ok(Some(stamp)) means at stamp a watch can be dropped, Err means no current watches
     **/
    #[instrument(skip(self),ret)]
    fn next_work(&self) -> Result<Option<Stamp>, ()> {
        if self.is_running.get() {
            tracing::warn!("has_work called during work");
            panic!();
            //return true;
        }
        if !self.pending.borrow().is_empty() {
            return Ok(None);
        };
        self.callbacks.borrow_mut().gc(now()).ok_or(()).map(Some)
    }
}


fn drop_watch(w: RxEntry, rt: &Linkspace, reason: StopReason) {
    let (mut handler, entry) = w.map(());
    handler.stopped(entry, rt, reason)
}

pub trait LocalAsync
where
    Self: LocalSpawn + Timer,
{
}
impl<X: LocalSpawn + Timer> LocalAsync for X {}

impl LocalSpawn for Linkspace {
    fn spawn_local_obj(
        &self,
        future: futures::future::LocalFutureObj<'static, ()>,
    ) -> Result<(), futures::task::SpawnError> {
        self.0
            .spawner
            .get()
            .expect("No Spawner Set")
            .spawn_local_obj(future)
    }

    fn status_local(&self) -> Result<(), futures::task::SpawnError> {
        self.0
            .spawner
            .get()
            .expect("No Spawner set")
            .status_local()
    }
}
impl Timer for Linkspace {
    fn sleep(&self, dur: std::time::Duration) -> futures::future::BoxFuture<'static, ()> {
        self.0.spawner.get().expect("No Spawner Set").sleep(dur)
    }
}
