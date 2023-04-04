// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use futures::channel::mpsc;
use futures::StreamExt;
use std::{cell::RefCell, rc::Rc, time::Duration};
#[derive(Clone)]
pub struct ProcBus(Rc<Inner>);
struct Inner {
    tx: mpsc::UnboundedSender<u64>,
    rx: RefCell<mpsc::UnboundedReceiver<u64>>,
    val: RefCell<u64>,
}

impl ProcBus {
    pub fn new(_bus_id: u64) -> ProcBus {
        let (tx, rx) = mpsc::unbounded();
        ProcBus(Rc::new(Inner {
            tx,
            rx: RefCell::new(rx),
            val: RefCell::default(),
        }))
    }
    pub fn emit(&self, val: u64) -> u64 {
        let mut old = self.0.val.borrow_mut();
        if *old >= val {
            return val;
        }
        *old = val;
        self.0.tx.unbounded_send(val).unwrap();
        val
    }

    pub fn next(&self, _timeout: Duration) -> Option<u64> {
        panic!("Not supported in wasm")
    }
    pub async fn next_async(&self) -> u64 {
        let mut rx = self.0.rx.borrow_mut();
        let mut last = 0;
        while let Ok(Some(v)) = rx.try_next() {
            last = last.max(v);
        }
        let mut old = self.0.val.borrow_mut();
        if *old >= last {
            return last;
        }
        *old = last;
        let v = rx.next().await.unwrap();
        *old = old.max(v);
        return *old;
    }
}
