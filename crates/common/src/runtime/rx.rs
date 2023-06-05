// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::handlers::{FollowHandler, PktStreamHandler, SinglePktHandler, StopReason};
use anyhow::{bail, Context};
pub use async_executors::{Timer, TimerExt};
use futures::future::Either;
pub use futures::task::{LocalSpawn, LocalSpawnExt};
use linkspace_core::prelude::*;
use linkspace_pkt::reroute::ShareArcPkt;
use std::{
    borrow::{Borrow, Cow},
    cell::{Cell, OnceCell, RefCell},
    ops::{ ControlFlow},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};
use tracing::{warn, Span, debug_span, instrument};

pub type PktStream = Box<dyn PktStreamHandler + 'static>;
pub type Matcher = linkspace_core::matcher::Matcher<PktStream>;
/// [WatchEntry] with an associated callback (Box<dyn [PktStreamHandler]>)
pub type RxEntry = linkspace_core::matcher::WatchEntry<PktStream>;
pub type PostTxnHandler = Box<dyn FnMut(Stamp, &Linkspace) -> ControlFlow<()>>;
pub type PostTxnList = Vec<(PostTxnHandler, Span)>;

#[derive(Clone)]
#[must_use = "Linkspace runtime does nothing unless processed"]
pub struct Linkspace {
    pub(crate) exec: Rc<Executor>,
}
impl std::fmt::Debug for Linkspace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Runtime").field("exec", &"todo").finish()
    }
}
enum Pending {
    PostWatch { cb: PostTxnHandler, span: Span },
    NewWatch { watch_entry: RxEntry },
    Close { id: QueryID, range: bool },
}

struct _ExecutorTxn {
    last: Stamp,
    txn: Either<Rc<ReadTxn>, Weak<ReadTxn>>,
}

impl Borrow<BTreeEnv> for Linkspace {
    fn borrow(&self) -> &BTreeEnv {
        self.env()
    }
}

pub(crate) struct Executor {
    env: BTreeEnv,
    written: Cell<bool>,
    cbs: RefCell<(Matcher, PostTxnList)>,
    pending: RefCell<Vec<Pending>>,
    process_txn: RefCell<Rc<ReadTxn>>,
    process_upto: Cell<Stamp>,
    is_running: Cell<bool>,
    pub spawner: OnceCell<Rc<dyn LocalAsync>>,
}

impl Linkspace {
    pub fn get_reader(&self) -> Rc<ReadTxn> {
        //Rc::new(self.exec.env.get_reader().unwrap())
        self.exec.process_txn.borrow().clone()
    }
    pub fn get_writer(&self) -> WriteTxn2 {
        if self.exec.written.get() == false {
            tracing::trace!("Set Written true")
        }
        self.exec.written.set(true);
        self.exec.env.get_writer().unwrap()
    }
    pub fn env(&self) -> &BTreeEnv {
        &self.exec.env
    }
    pub fn spawner(&self) -> &OnceCell<Rc<dyn LocalAsync>> {
        &self.exec.spawner
    }
}

impl Linkspace {
    fn rt_log_head(&self) -> Stamp {
        self.exec.process_upto.get()
    }
    /** return when next to process.
     Ok(None) means immediatly, Ok(Some(stamp)) means at stamp a watch can be dropped, Err means no current watches
    **/
    #[instrument(skip(self),ret)]
    fn next_work(&self) -> Result<Option<Stamp>, ()> {
        if self.exec.is_running.get() {
            tracing::warn!("has_work called during work");
            panic!();
            //return true;
        }
        if !self.exec.pending.borrow().is_empty() {
            return Ok(None);
        };
        self.exec.cbs.borrow_mut().0.gc(now()).ok_or(()).map(Some)
    }
    #[must_use]
    pub fn new(env: BTreeEnv, spawner: Rc<dyn LocalAsync>) -> Linkspace {
        Self::new_opt_rt(env, OnceCell::from(spawner))
    }

    #[must_use]
    pub fn new_opt_rt(env: BTreeEnv, spawner: OnceCell<Rc<dyn LocalAsync>>) -> Linkspace {
        let reader = env.get_reader().unwrap();
        let at = reader.log_head();
        Linkspace {
            exec: Rc::new(Executor {
                env,
                written: Cell::new(false),
                cbs: Default::default(),
                pending: Default::default(),
                process_txn: RefCell::new(Rc::new(reader)),
                process_upto: at.into(),
                is_running: Cell::new(false),
                spawner,
                //subroutines:RefCell::new(Registry::new())
            }),
        }
    }

    pub fn insert_watch(&self, watch_entry: RxEntry) {
        match self.exec.cbs.try_borrow_mut() {
            Ok(mut lk) => {
                if let Some(w) = lk.0.register(watch_entry) {
                    drop_watch(w, &self, StopReason::Replaced)
                }
            }
            Err(_) => self
                .exec
                .pending
                .borrow_mut()
                .push(Pending::NewWatch { watch_entry }),
        };
    }
    pub fn drain_pending(&self, lk: &mut (Matcher, PostTxnList)) {
        for cmd in self.exec.pending.borrow_mut().drain(..) {
            match cmd {
                Pending::NewWatch { watch_entry } => {
                    if let Some(w) = lk.0.register(watch_entry) {
                        drop_watch(w, &self, StopReason::Replaced)
                    }
                }
                Pending::Close { id, range } => {
                    lk.0.deregister(&id, range, |w| drop_watch(w, &self, StopReason::Closed));
                }
                Pending::PostWatch { cb, span } => lk.1.push((cb, span)),
            }
        }
    }
    pub async fn poll(&self) -> Stamp {
        loop {
            self.process();
            let rt_head = self.rt_log_head();
            let env_head = self.inner().log_head().await;
            if env_head == rt_head {
                return env_head;
            }
        }
    }
    pub fn run(&self) -> ! {
        loop {
            if let Some(v) = self.inner().log_head.next_d(None) {
                if self.rt_log_head().get() < v {
                    self.process();
                }
            }
        }
    }

    fn watch_status(&self,id:&QueryIDRef) -> Option<WatchStatus>{
        let cbs = self.exec.cbs.borrow();
        Some(cbs.0.get(id)?.status())
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
        if self.exec.is_running.get() {
            bail!("already running")
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

            match self.next_work() {
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
            self.inner().log_head.next_d(Some(next_check));
        }
    }

    /// check the log for new packets and execute callbacks
    #[instrument(skip(self))]
    pub fn process(&self) -> Stamp {
        self.exec.written.set(false);
        let this = self.clone();
        let (txn, from, upto): (Rc<ReadTxn>, Stamp, Stamp) = {
            let mut txn = self.exec.process_txn.borrow_mut();
            let rx_last = self.exec.process_upto.get();
            match Rc::get_mut(&mut txn) {
                Some(txn) => {
                    tracing::trace!(?rx_last, "refresh txn");
                    Refreshable::refresh(txn);
                }
                None => {
                    tracing::warn!("holding a txn across callbacks!");
                    *txn = Rc::new(self.exec.env.get_reader().unwrap());
                }
            }
            let txn_last = txn.log_head();
            if rx_last > txn_last {
                tracing::debug!(?txn_last, ?rx_last, "Already processed");
                return rx_last;
            }
            (txn.clone(), rx_last, txn_last)
        };
        let _g = tracing::error_span!("Processing txn", ?from, ?upto).entered();
        let txn = txn;
        tracing::trace!("Lock cbs");
        let mut lock = self.exec.cbs.borrow_mut();
        self.drain_pending(&mut lock);
        self.exec.is_running.set(true);
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
            let _g = tracing::error_span!("Matching",logptr=?pkt.recv,pkt=%pkt_fmt(&pkt.pkt)).entered();
            tracing::debug!("Testing New Pkt");

            let pkt = ShareArcPkt {
                arc: OnceCell::new(),
                pkt,
            };

            lock.0.trigger(
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
        // We don't do any setup in post_funcs
        lock.1.drain_filter(|(func, span)| {
            let _ = span.enter();
            {
                let cont = func(from, &this);
                tracing::info!(?cont, "PostTxn");
                validate();
                cont.is_break()
            }
        });
        self.exec.is_running.set(false);
        self.exec.process_upto.set(upto);
        std::mem::drop((txn, lock));
        if self.exec.written.get() {
            tracing::trace!("Written true");
            return self.process();
        } else {
            upto
        }
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
    /// automatically handle the options 'follow', 'start', 'mode', and 'id'
    pub fn watch_query(
        &self,
        query: &Query,
        onmatch: impl PktStreamHandler + 'static,
        span: Span,
    ) -> anyhow::Result<i32> {
        let mode = query.get_mode()?;
        let id = query.qid().transpose()?.context("watch always requires the :id option")?;
        let follow = query.get_known_opt(KnownOptions::Follow);
        let start = None; //query.get_known_opt(KnownOptions::Start).map(|v| Ptr::try_from(v.clone())).transpose()?;
                          // TODO span should already have these fields.
        let span = tracing::debug_span!(parent: &span, "with_opts", id=?AB(id), ?follow, ?mode, ?start);
        match follow {
            Some(_p) => {
                let onmatch = FollowHandler { inner: onmatch };
                Ok(self.watch(
                    id,
                    mode,
                    Cow::Borrowed(&query),
                    onmatch,
                    start,
                    span,
                )?)
            }
            None => Ok(self.watch(
                id,
                mode,
                Cow::Borrowed(&query),
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
            let r = reader
                .query(mode, &q.predicates, &mut counter)?
                .try_for_each(|dbp| {
                    let _g = local_span.enter();
                    tracing::debug!(pkt=%pkt_fmt(&dbp.pkt),"Match");
                    onmatch.handle_pkt(&dbp, self)
                });
            if Rc::strong_count(&reader) > 2 {
                warn!("Holding txn open");
            }
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

    /// Add a function to be run after a txn.
    /// Will run during _this_ transaction.
    /// But any further mutations are ignored this transaction.
    pub fn add_post_txn(&self, cb: PostTxnHandler, span: Span) {
        match self.exec.cbs.try_borrow_mut() {
            Ok(mut lk) => lk.1.push((cb, span)),
            Err(_) => self
                .exec
                .pending
                .borrow_mut()
                .push(Pending::PostWatch { cb, span }),
        }
    }
    pub fn inner(&self) -> &BTreeEnv {
        &self.exec.env
    }
    pub fn close(&self, id: impl AsRef<QueryIDRef>) {
        match self.exec.cbs.try_borrow_mut() {
            Ok(mut lk) => {
                lk.0.deregister(id.as_ref(), false, |w| {
                    drop_watch(w, &self, StopReason::Closed)
                });
            }
            Err(_) => self.exec.pending.borrow_mut().push(Pending::Close {
                id: id.as_ref().to_vec(),
                range: false,
            }),
        }
    }
    pub fn close_range(&self, prefix: impl AsRef<QueryIDRef>) {
        match self.exec.cbs.try_borrow_mut() {
            Ok(mut lk) => {
                lk.0.deregister(prefix.as_ref(), true, |w| {
                    drop_watch(w, &self, StopReason::Closed)
                });
            }
            Err(_) => self.exec.pending.borrow_mut().push(Pending::Close {
                id: prefix.as_ref().to_vec(),
                range: true,
            }),
        }
    }
    /// Will panic if called during execution
    pub fn dbg_watches(&self) -> std::cell::Ref<(Matcher, PostTxnList)> {
        self.exec.cbs.borrow()
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
        self.exec
            .spawner
            .get()
            .expect("No Spawner Set")
            .spawn_local_obj(future)
    }

    fn status_local(&self) -> Result<(), futures::task::SpawnError> {
        self.exec
            .spawner
            .get()
            .expect("No Spawner set")
            .status_local()
    }
}
impl Timer for Linkspace {
    fn sleep(&self, dur: std::time::Duration) -> futures::future::BoxFuture<'static, ()> {
        self.exec.spawner.get().expect("No Spawner Set").sleep(dur)
    }
}
