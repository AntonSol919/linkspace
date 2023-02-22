// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(control_flow_enum, once_cell, io_error_other, iterator_try_collect)]

use std::{ops::ControlFlow, path::Path};

use liblinkspace::prelude::*;
use pyo3::{
    prelude::*,
    types::{PyBytes, PyFunction, PyTuple},
};
mod pynetpkt;
use pynetpkt::*;

#[cfg(any(PyPy, Py_3_11))]
fn call_ctx(py: Python) -> (String, i32) {
    unsafe {
        let frame_ptr = pyo3::ffi::PyEval_GetFrame();
        let frame: &pyo3::ffi::PyFrameObject = &*frame_ptr;
        let code_obj: &pyo3::ffi::PyCodeObject = &*frame.f_code;
        let filename: &pyo3::types::PyString =
            py.from_borrowed_ptr_or_err(code_obj.co_filename).unwrap();
        let line_num = pyo3::ffi::PyFrame_GetLineNumber(frame_ptr);
        (filename.to_string(), line_num)
    }
}
#[cfg(not(any(PyPy, Py_3_11)))]
fn call_ctx(_py: Python) -> (&'static str, i32) {
    unsafe {
        let frame_ptr = pyo3::ffi::PyEval_GetFrame();
        let line_num = pyo3::ffi::PyFrame_GetLineNumber(frame_ptr);
        ("<unknown>", line_num)
    }
}

struct PyPktStreamHandler {
    on_match: Py<PyFunction>,
    on_close: Option<Py<PyFunction>>,
}
impl PktHandler for PyPktStreamHandler {
    fn handle_pkt(
        &mut self,
        pkt: &dyn NetPkt,
        _rx: &liblinkspace::Linkspace,
    ) -> std::ops::ControlFlow<()> {
        let apkt = Pkt::from_dyn(pkt);
        let cont = Python::with_gil(|py| call_cont_py(py, &self.on_match, (apkt,)))
            .expect("todo - impl exceptions");
        if cont {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
    fn stopped(
        &mut self,
        _query: liblinkspace::Query,
        _lk: &liblinkspace::Linkspace,
        _reason: misc::StopReason,
        _total: u32,
        _new: u32,
    ) {
        if let Some(f) = &self.on_close {
            Python::with_gil(|py| {
                f.call0(py).expect("todo - impl exceptions");
            });
        }
    }
}

fn common_args(
    group: Option<&[u8]>,
    domain: Option<&[u8]>,
    path: Option<&PyAny>,
    links: Option<Vec<crate::pynetpkt::Link>>,
    stamp: Option<&[u8]>,
) -> anyhow::Result<(GroupID, Domain, IPathBuf, Vec<Link>, Option<Stamp>)> {
    let group = group
        .map(|group| GroupID::try_fit_bytes_or_b64(group))
        .transpose()?
        .unwrap_or(consts::PUBLIC_GROUP);
    let domain = domain
        .map(|domain| Domain::try_fit_byte_slice(domain))
        .transpose()?
        .unwrap_or(AB::default());
    let path = match path {
        None => IPathBuf::new(),
        Some(p) => {
            let path = p
                .iter()?
                .map(|i| i.and_then(PyAny::extract::<&[u8]>))
                .try_collect::<Vec<_>>()?;
            IPathBuf::try_from_iter(path)?
        }
    };
    let links = links
        .unwrap_or_default()
        .into_iter()
        .map(|l| Link {
            tag: AB(l.tag),
            ptr: B64(l.ptr),
        })
        .collect();
    let stamp = stamp.map(|p| Stamp::try_from(p)).transpose()?;
    Ok((group, domain, path, links, stamp))
}

#[pyclass]
pub struct SigningKey(pub liblinkspace::SigningKey);

use tracing::debug_span;

#[pyfunction]
pub fn lk_keygen() -> SigningKey {
    SigningKey(liblinkspace::key::lk_keygen())
}
#[pyfunction]
pub fn lk_keystr(key: &SigningKey, password: &[u8]) -> String {
    liblinkspace::key::lk_keystr(&key.0, password)
}
#[pyfunction]
pub fn lk_keyopen(_py: Python, id: &str, password: &[u8]) -> anyhow::Result<SigningKey> {
    Ok(SigningKey(liblinkspace::key::lk_keyopen(id, password)?))
}
#[pyfunction]
pub fn lk_key(
    lk: &Linkspace,
    password: &[u8],
    id: Option<&str>,
    new: Option<bool>,
) -> anyhow::Result<SigningKey> {
    liblinkspace::lk_key(&lk.0, password, id.unwrap_or(""), new.unwrap_or(false)).map(SigningKey)
}

#[pyfunction]
pub fn lk_datapoint(data: &[u8]) -> anyhow::Result<Pkt> {
    Ok(pynetpkt::Pkt::from_dyn(
        &liblinkspace::point::lk_datapoint_ref(data.as_ref())?,
    ))
}
#[pyfunction]
pub fn lk_linkpoint(
    data: Option<&[u8]>,
    group: Option<&[u8]>,
    domain: Option<&[u8]>,
    path: Option<&PyAny>,
    links: Option<Vec<crate::pynetpkt::Link>>,
    stamp: Option<&[u8]>,
) -> anyhow::Result<Pkt> {
    let data = data.unwrap_or(&[]);
    let (group, domain, path, links, stamp) = common_args(group, domain, path, links, stamp)?;
    let pkt = liblinkspace::point::lk_linkpoint_ref(domain, group, &*path, &*links, data, stamp)?;
    Ok(pynetpkt::Pkt::from_dyn(&pkt))
}
#[pyfunction]
pub fn lk_keypoint(
    key: &SigningKey,
    data: Option<&[u8]>,
    group: Option<&[u8]>,
    domain: Option<&[u8]>,
    path: Option<&PyAny>,
    links: Option<Vec<crate::pynetpkt::Link>>,
    stamp: Option<&[u8]>,
) -> anyhow::Result<Pkt> {
    let data = data.unwrap_or(&[]);
    let (group, domain, path, links, stamp) = common_args(group, domain, path, links, stamp)?;
    let pkt =
        liblinkspace::point::lk_keypoint_ref(domain, group, &*path, &*links, data, stamp, &key.0)?;
    Ok(pynetpkt::Pkt::from_dyn(&pkt))
}

fn pptr(p: Option<&Pkt>) -> Option<&dyn NetPkt> {
    p.map(|p| &p.0 as &dyn NetPkt)
}

#[pyfunction]
pub fn lk_write<'a>(py: Python<'a>, pkt: &Pkt) -> anyhow::Result<&'a PyBytes> {
    // TODO remove this copy
    let mut v = vec![];
    liblinkspace::point::lk_write(&pkt.0, &mut v)?;
    Ok(PyBytes::new(py, &v))
}
#[pyfunction]
pub fn lk_read(buf: &[u8], validate: bool, allow_private: bool) -> anyhow::Result<(Pkt, &[u8])> {
    let p = liblinkspace::point::lk_read_ref(buf, validate, allow_private)?;
    let size = p.size();
    Ok((Pkt::from_dyn(&p), &buf[size..]))
}

#[pyfunction]
pub fn lk_eval<'a>(py: Python<'a>, expr: &str, pkt: Option<&Pkt>) -> anyhow::Result<&'a PyBytes> {
    let bytes = liblinkspace::lk_eval(expr, pptr(pkt))?;
    Ok(PyBytes::new(py, &*bytes))
}
#[pyfunction]
pub fn lk_eval2str(expr: &str, pkt: Option<&Pkt>) -> anyhow::Result<String> {
    let out = liblinkspace::lk_eval(expr, pptr(pkt))?;
    Ok(String::from_utf8(out)?)
}
#[pyfunction]
pub fn lk_encode(bytes: &[u8], options: &str) -> anyhow::Result<String> {
    Ok(liblinkspace::abe::lk_encode(bytes, options))
}

#[pyclass(unsendable)]
#[derive(Clone)]
pub struct Linkspace(pub(crate) liblinkspace::Linkspace);

#[pyfunction]
pub fn lk_open(path: Option<&str>, create: Option<bool>) -> anyhow::Result<Linkspace> {
    Ok(Linkspace(liblinkspace::lk_open(
        path.map(Path::new),
        create.unwrap_or(false),
    )?))
}
#[pyfunction]
pub fn lk_save(runtime: &Linkspace, pkt: &Pkt) -> anyhow::Result<bool> {
    Ok(liblinkspace::lk_save(&runtime.0, pkt.0.netpktptr())?)
}

#[pyclass]
#[derive(Clone)]
pub struct Query(pub(crate) liblinkspace::Query);

#[pyfunction]
#[pyo3(signature =(*exprs, pkt=None))]
pub fn lk_query(exprs: &PyTuple, pkt: Option<&Pkt>) -> anyhow::Result<Query> {
    let mut q = Query(liblinkspace::lk_query());
    lk_query_parse(&mut q, exprs, pkt)?;
    Ok(q)
}
use liblinkspace::abe::ctx::ctx;
use liblinkspace::varctx;

#[pyfunction]
#[pyo3(signature =(query,*exprs, pkt=None))]
pub fn lk_query_parse(
    query: &mut Query,
    exprs: &PyTuple,
    pkt: Option<&Pkt>,
) -> anyhow::Result<bool> {
    let pstr = exprs
        .iter()
        .map(|p| p.extract::<String>())
        .try_collect::<Vec<_>>()?
        .join("\n");
    let changed = varctx::lk_query_parse(ctx(pptr(pkt)), &mut query.0, &pstr)?;
    Ok(changed)
}
#[pyfunction]
#[pyo3(signature =(query,as_expr=false))]
pub fn lk_query_print(query: &Query, as_expr: bool) -> String {
    liblinkspace::lk_query_print(&query.0, as_expr)
}
#[pyfunction]
pub fn lk_query_clear(query: &mut Query) {
    liblinkspace::query::lk_query_clear(&mut query.0)
}

fn call_cont_py(
    py: Python,
    func: &Py<PyFunction>,
    args: impl IntoPy<Py<PyTuple>>,
) -> anyhow::Result<bool> {
    match func.call1(py, args)?.extract::<bool>(py) {
        Ok(b) => Ok(b) as anyhow::Result<bool>,
        Err(_) => Ok(true),
    }
}

#[pyfunction]
pub fn lk_get(linkspace: &Linkspace, query: &Query) -> anyhow::Result<Option<Pkt>> {
    liblinkspace::linkspace::lk_get_ref(&linkspace.0, &query.0, &mut |pkt| Pkt::from_dyn(&pkt))
}
#[pyfunction]
pub fn lk_get_all(
    py: Python,
    linkspace: &Linkspace,
    query: &Query,
    cb: Py<PyFunction>,
) -> anyhow::Result<u32> {
    let mut cb_err = Ok(());
    let count = liblinkspace::linkspace::lk_get_all(&linkspace.0, &query.0, &mut |pkt| {
        let pkt = Pkt::from_dyn(pkt);
        let mut cont = false;
        cb_err = call_cont_py(py, &cb, (pkt,)).map(|c| cont = c);
        cont
    })?;
    cb_err?;
    Ok(count)
}
#[pyfunction]
pub fn lk_watch(
    py: Python,
    linkspace: &Linkspace,
    query: &Query,
    on_match: Py<PyFunction>,
    on_close: Option<Py<PyFunction>>,
) -> anyhow::Result<u32> {
    let watch_handler = PyPktStreamHandler { on_match, on_close };
    let (file, line) = call_ctx(py);
    let span = debug_span!("lk_watch",%file,%line);
    liblinkspace::linkspace::lk_watch2(&linkspace.0, &query.0, watch_handler, span)
}

#[pyfunction]
pub fn lk_process(linkspace: &Linkspace) -> [u8; 8] {
    liblinkspace::lk_process(&linkspace.0).0
}

/**
continiously trigger watch callbacks unless
- max_wait has elapsed between new packets - return false
e.g. lk_eval("{s:+1M}") or 0u64 to ignore
- untill time has been reached - returns false
e.g. lk_eval("{now:+1M}") or 0u64 to ignore
- no more watch callbacks exists - returns true
 **/
#[pyfunction]
pub fn lk_process_while(
    linkspace: &Linkspace,
    max_wait: Option<&[u8]>,
    untill: Option<&[u8]>,
) -> anyhow::Result<bool> {
    let try_stamp = |s: Option<&[u8]>| -> anyhow::Result<Stamp> {
        Ok(s.map(|s| Stamp::try_from(s))
            .transpose()?
            .unwrap_or(Stamp::ZERO))
    };
    liblinkspace::lk_process_while(&linkspace.0, try_stamp(max_wait)?, try_stamp(untill)?)
}

/** linkspace python bindings.
**/
#[pymodule]
fn lkpy(_py: Python, m: &PyModule) -> PyResult<()> {
    let filter = tracing_subscriber::EnvFilter::from_default_env();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .pretty()
        .with_thread_ids(true)
        .with_thread_names(true)
        .init();

    m.add_class::<pynetpkt::Pkt>()?;
    m.add_class::<pynetpkt::Links>()?;
    m.add_class::<pynetpkt::Link>()?;
    m.add_class::<crate::Linkspace>()?;
    m.add_class::<crate::Query>()?;

    m.add_function(wrap_pyfunction!(crate::lk_datapoint, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_linkpoint, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keypoint, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_read, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_write, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_key, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keygen, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keystr, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keyopen, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_eval, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_eval2str, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_encode, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_query, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_parse, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_print, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_clear, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_open, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_save, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_get, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_get_all, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_watch, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_process, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_process_while, m)?)?;

    m.add("LOCAL_ONLY", consts::LOCAL_ONLY_GROUP.0)?;
    m.add("PUBLIC", consts::PUBLIC_GROUP.0)?;
    m.add("DEFAULT_PKT", abe::DEFAULT_PKT)?;

    Ok(())
}
