// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use super::rx::Linkspace;
use crate::prelude::LocalAsync;
use futures::channel::mpsc::*;
use futures::{
    executor::LocalPool,
    task::{LocalSpawnExt, SpawnError},
    Future, StreamExt,
};
use linkspace_core::prelude::lmdb::BTreeEnv;
use std::rc::Rc;

pub fn attach(env: BTreeEnv, spawner: Rc<dyn LocalAsync>) -> Linkspace {
    let rx = Linkspace::new(env, spawner.clone());
    let rx2 = rx.clone();
    spawner
        .spawn_local(async move {
            loop {
                tracing::info!("Spawner Poll");
                rx2.poll().await;
            }
        })
        .unwrap();
    rx
}
pub fn run_until<O, Fut>(
    rx: Linkspace,
    fnc: impl FnOnce(Linkspace) -> Fut,
) -> Result<O, SpawnError>
where
    Fut: Future<Output = O>,
{
    let mut local = LocalPool::new();
    rx.spawner()
        .set(Rc::new(local.spawner()))
        .map_err(|_| ())
        .expect("Call this if you don't have a runtime setup");
    let fut = fnc(rx.clone());
    local.spawner().spawn_local(async move {
        loop {
            let v = rx.poll().await;
            tracing::info!("Polled upto {v}");
        }
    })?;
    Ok(local.run_until(fut))
}
pub fn run_until_spawn_thread<Fnc, R: Send + 'static>(
    rx: Linkspace,
    fnc: Fnc,
) -> Result<std::thread::JoinHandle<R>, SpawnError>
where
    Fnc: FnOnce(RemoteSpawn) -> R + Send + 'static,
{
    let (tx, mut recv) = unbounded::<RxFunc>();
    let handle = std::thread::spawn(move || fnc(tx));
    run_until(rx, |r| async move {
        loop {
            match recv.next().await {
                Some(func) => (func)(r.clone()),
                None => return,
            }
        }
    })?;
    Ok(handle)
}
pub fn attach_spawn(rx: Linkspace) -> RemoteSpawn {
    let (tx, mut recv) = unbounded::<RxFunc>();
    rx.clone()
        .spawn_local(async move {
            loop {
                match recv.next().await {
                    Some(func) => (func)(rx.clone()),
                    None => return,
                }
            }
        })
        .unwrap();
    tx
}
pub type RemoteSpawn = UnboundedSender<RxFunc>;
pub type RxFunc = Box<dyn FnOnce(Linkspace) + Send + Sync + 'static>;
pub fn rx_thread(env: BTreeEnv, name: String) -> (RemoteSpawn, std::thread::JoinHandle<()>) {
    let (tx, mut spawn_recv) = unbounded::<RxFunc>();
    let handle = std::thread::Builder::new()
        .name(name)
        .spawn(move || {
            let mut local = LocalPool::new();
            let rx = attach(env, Rc::new(local.spawner()));
            let fut = async move {
                loop {
                    match spawn_recv.next().await {
                        Some(func) => (func)(rx.clone()),
                        None => return,
                    }
                }
            };
            local.run_until(fut);
        })
        .unwrap();
    (tx, handle)
}
