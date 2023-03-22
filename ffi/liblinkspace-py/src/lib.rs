// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
#![feature(
    control_flow_enum,
    once_cell,
    io_error_other,
    iterator_try_collect,
    thread_local,
    duration_constants
)]

use ref_cast::RefCast;
use std::{ops::ControlFlow, path::{Path, PathBuf}, time::Duration};

use liblinkspace::{prelude::*, abe::ctx::{UserData, self}};
use pyo3::{
    prelude::*,
    types::{PyBytes,  PyTuple},
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

pub type PyFunc = Py<PyAny>;

struct PyPktStreamHandler {
    on_match: PyFunc,
    on_close: Option<PyFunc>,
    on_err: Option<PyFunc>
}
impl PktHandler for PyPktStreamHandler {
    fn handle_pkt(
        &mut self,
        pkt: &dyn NetPkt,
        _lk: &liblinkspace::Linkspace,
    ) -> std::ops::ControlFlow<()> {
        let apkt = Pkt::from_dyn(pkt);
        match Python::with_gil(|py| call_cont_py(py, &self.on_match, (apkt,))) {
            Ok(true) => ControlFlow::Continue(()),
            Ok(false) => ControlFlow::Break(()),
            Err(e) => {
                if let Some(f) = &self.on_err{
                    let apkt = Pkt::from_dyn(pkt);
                    match Python::with_gil(|py| call_cont_py(py, f, (e,apkt,&self.on_match))){
                        Ok(true) => return ControlFlow::Continue(()),
                        Ok(false) => {},
                        Err(e) => tracing::warn!(?e,"Yo dog i heard i liked errors"),
                    }
                }else { tracing::warn!("default error handler " )}
                return ControlFlow::Break(());
            }
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
        if let Some(_f) = &self.on_close {
            todo!("handle stopped");
            //if let Err(e) = Python::with_gil(|py|f.call0(py));
        }
    }
}

fn common_args(
    group: Option<&[u8]>,
    domain: Option<&[u8]>,
    path: Option<&PyAny>,
    links: Option<Vec<crate::pynetpkt::Link>>,
    create_stamp: Option<&[u8]>,
) -> anyhow::Result<(GroupID, Domain, IPathBuf, Vec<Link>, Option<Stamp>)> {
    let group = group
        .map(|group| GroupID::try_fit_bytes_or_b64(group))
        .transpose()?
        .unwrap_or(consts::PUBLIC);
    let domain = domain
        .map(|domain| Domain::try_fit_byte_slice(domain))
        .transpose()?
        .unwrap_or(AB::default());
    let path = match path {
        None => IPathBuf::new(),
        Some(p) => {
            if let Ok(p) = p.downcast::<PyBytes>(){
                SPath::from_slice(p.as_bytes())?.into_spathbuf().try_ipath()?
            }else {
                let path = p
                    .iter()?
                    .map(|i| i.and_then(bytelike))
                    .try_collect::<Vec<_>>()?;
                IPathBuf::try_from_iter(path)?
            }
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
    let create_stamp = create_stamp.map(|p| Stamp::try_from(p)).transpose()?;
    Ok((group, domain, path, links, create_stamp))
}

#[pyclass]
pub struct SigningKey(pub liblinkspace::SigningKey);
#[pymethods]
impl SigningKey{
    #[getter]
    fn pubkey<'o>(&self,py:Python<'o>) -> &'o PyBytes {
        PyBytes::new(py, &*self.0.pubkey())
    }
}

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

const E:&[u8]=&[];
#[pyfunction]
#[pyo3(signature =(lk,password=E,name="me:local",create=false))]
pub fn lk_key(
    lk: &Linkspace,
    password: &[u8],
    name: &str,
    create: bool,
) -> anyhow::Result<SigningKey> {
    liblinkspace::lk_key(&lk.0, password, name, create).map(SigningKey)
}

#[pyfunction]
pub fn lk_datapoint(data: &PyAny) -> anyhow::Result<Pkt> {
    Ok(pynetpkt::Pkt::from_dyn(
        &liblinkspace::point::lk_datapoint_ref(bytelike(data)?)?,
    ))
}
#[pyfunction]
pub fn lk_linkpoint(
    group: Option<&[u8]>,
    domain: Option<&[u8]>,
    path: Option<&PyAny>,
    links: Option<Vec<crate::pynetpkt::Link>>,
    data: Option<&PyAny>,
    create: Option<&[u8]>,
) -> anyhow::Result<Pkt> {
    let data = data.map(bytelike).transpose()?.unwrap_or(&[]);
    let (group, domain, path, links, create) = common_args(group, domain, path, links, create)?;
    let pkt = liblinkspace::point::lk_linkpoint_ref(domain, group, &*path, &*links, data, create)?;
    Ok(pynetpkt::Pkt::from_dyn(&pkt))
}
#[pyfunction]
pub fn lk_keypoint(
    key: &SigningKey,
    group: Option<&[u8]>,
    domain: Option<&[u8]>,
    path: Option<&PyAny>,
    links: Option<Vec<crate::pynetpkt::Link>>,
    data: Option<&PyAny>,
    create: Option<&[u8]>,
) -> anyhow::Result<Pkt> {
    let data = data.map(bytelike).transpose()?.unwrap_or(&[]);
    let (group, domain, path, links, create) = common_args(group, domain, path, links, create)?;
    let pkt =
        liblinkspace::point::lk_keypoint_ref(domain, group, &*path, &*links, data, create, &key.0)?;
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
    let p = liblinkspace::point::lk_read(buf, validate, allow_private)?;
    let size = p.size();
    Ok((Pkt::from_dyn(&p), &buf[size..]))
}

#[pyfunction]
pub fn lk_eval<'a>(
    py: Python<'a>,
    expr: &str,
    pkt: Option<&Pkt>,
    argv: Option<&PyAny>,
) -> anyhow::Result<&'a PyBytes> {
    let argv: Vec<&[u8]> = argv
        .map(|v| v.iter()?.take(9).map(|v| bytelike(v?)).try_collect())
        .transpose()?
        .unwrap_or_default();
    let udata = UserData{argv: Some(&argv), pkt: pptr(pkt)};
    let uctx = ctx::ctx(udata)?;
    let bytes = liblinkspace::varctx::lk_eval(uctx, expr )?;
    Ok(PyBytes::new(py, &*bytes))
}
#[pyfunction]
pub fn lk_eval2str(expr: &str, pkt: Option<&Pkt>, argv: Option<&PyAny>) -> anyhow::Result<String> {
    let argv: Vec<&[u8]> = argv
        .map(|v| v.iter()?.take(9).map(|v| bytelike(v?)).try_collect())
        .transpose()?
        .unwrap_or_default();
    let udata = UserData{argv: Some(&argv), pkt: pptr(pkt)};
    let uctx = ctx::ctx(udata)?;
    let out= liblinkspace::varctx::lk_eval(uctx, expr )?;
    Ok(String::from_utf8(out)?)
}
#[pyfunction]
pub fn lk_encode(bytes: &[u8], options: Option<&str>) -> anyhow::Result<String> {
    Ok(liblinkspace::abe::lk_encode(bytes, options.unwrap_or("")))
}
#[pyclass(unsendable)]
#[derive(Clone, RefCast)]
#[repr(transparent)]
pub struct Linkspace(pub(crate) liblinkspace::Linkspace);

#[pyfunction]
#[pyo3(signature =(path="",create=false))]
/// open a linkspace runtime.
///
/// will look at `path` or $LINKSPACE or '$HOME'
/// and open 'PATH/linkspace' unless the basename of PATH is linkspace 'linkspace'
pub fn lk_open(path: &str, create: bool) -> anyhow::Result<Linkspace> {
    Ok(Linkspace(liblinkspace::lk_open(
        if path.is_empty(){None} else {Some(Path::new(path))},
        create,
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
pub fn lk_query(copy_from: Option<&Query>) -> Query {
    Query(liblinkspace::lk_query(copy_from.map(|v| &v.0)))
}
#[pyfunction]
/// A short hand for [:mode:hash-asc,i:=:{u32:0},hash:=:HASH]
pub fn lk_hash_query(hash: &PyAny) -> anyhow::Result<Query> {
    let hash = LkHash::try_fit_bytes_or_b64(bytelike(hash)?)?;
    Ok(Query(liblinkspace::query::lk_hash_query(hash)))
}
fn bytelike(p: &PyAny) -> PyResult<&[u8]> {
    p.extract::<&[u8]>()
        .or_else(|_| Ok(p.extract::<&str>()?.as_bytes()))
}

#[pyfunction]
pub fn lk_query_parse(
    query: &mut Query,
    statements: &str,
    pkt: Option<&Pkt>,
    argv: Option<&PyAny>,
) -> anyhow::Result<bool> {
    let argv: Vec<&[u8]> = argv
        .map(|v| v.iter()?.take(9).map(|v| bytelike(v?)).try_collect())
        .transpose()?
        .unwrap_or_default();
    let udata = UserData{argv: Some(&argv), pkt: pptr(pkt)};
    let uctx = ctx::ctx(udata)?;
    let changed = liblinkspace::varctx::lk_query_parse(uctx,&mut query.0, &statements)?;
    Ok(changed)
}
#[pyfunction]
pub fn lk_query_push(query: &mut Query, field: &str, op: &str, bytes: &PyAny) -> LkResult<bool> {
    liblinkspace::lk_query_push(&mut query.0, field, op, bytelike(bytes)?)
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
    func: &PyFunc,
    args: impl IntoPy<Py<PyTuple>>,
) -> PyResult<bool> {
    let result = func.call1(py, args)?;
    let as_bool = result.extract::<bool>(py);
    match as_bool {
        Ok(b) => Ok(b) as PyResult<bool>,
        Err(_) => Ok(true),
    }
}

#[pyfunction]
pub fn lk_get(lk: &Linkspace, query: &Query) -> anyhow::Result<Option<Pkt>> {
    liblinkspace::linkspace::lk_get_ref(&lk.0, &query.0, &mut |pkt| Pkt::from_dyn(&pkt))
}
#[pyfunction]
pub fn lk_get_all(
    py: Python,
    lk: &Linkspace,
    query: &Query,
    cb: PyFunc,
) -> anyhow::Result<u32> {
    let mut cb_err = Ok(());
    let count = liblinkspace::linkspace::lk_get_all(&lk.0, &query.0, &mut |pkt| {
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
    lk: &Linkspace,
    query: &Query,
    on_match: PyFunc,
    on_close: Option<PyFunc>,
    on_err: Option<PyFunc>,
) -> anyhow::Result<u32> {
    let watch_handler = PyPktStreamHandler { on_match, on_close,on_err };
    let (file, line) = call_ctx(py);
    let span = debug_span!("lk_watch",%file,%line);
    liblinkspace::linkspace::lk_watch2(&lk.0, &query.0, watch_handler, span)
}
#[pyfunction]
pub fn lk_list_watches(py: Python, lk: &Linkspace, cb: PyFunc) -> PyResult<PyObject> {
    let mut r = Ok(py.None());
    liblinkspace::linkspace::lk_list_watches(&lk.0, &mut |id, query| {
        if r.is_err() {
            return;
        };
        r = cb.call1(py, (PyBytes::new(py, id), Query(query.clone())));
    });
    r
}

#[pyfunction]
pub fn lk_process(lk: &Linkspace) -> [u8; 8] {
    liblinkspace::lk_process(&lk.0).0
}

/**
continiously trigger watch callbacks unless
- max_wait has elapsed between new packets - return false
e.g. lk_eval("{s:+1M}") or 0u64 to ignore
- until time has been reached - returns false
e.g. lk_eval("{now:+1M}") or 0u64 to ignore
- if !watch_finish && watch_id was triggered - returns true
- if watch_finish && watch_id was finished - returns true
- no more watch callbacks exists - returns true

 **/
#[pyfunction]
pub fn lk_process_while(
    lk: &Linkspace,
    watch: Option<&[u8]>,
    watch_finish:Option<bool>,
    max_wait: Option<&[u8]>,
    until: Option<&[u8]>,
) -> anyhow::Result<bool> {
    let as_stamp = |opt| match opt{
        None => Ok(Stamp::MAX),
        Some(v) if v == &[0;8] => Ok(Stamp::MAX),
        Some(v) => Stamp::try_from(v)
    };
    let max_wait = as_stamp(max_wait)?;
    let until = as_stamp(until)?;

    // we do a little dance to check signals ( Ctr+C )  every 1 second
    const CHECK_SIGNALS_INTERVAL: Stamp = Stamp::new(Duration::SECOND.as_micros() as u64);
    let mut check_at = now().saturating_add(CHECK_SIGNALS_INTERVAL);
    let mut empty_watches = false;
    let watch = watch.map(|id| (id,watch_finish.unwrap_or(false)));
    while !empty_watches && until > check_at {
        empty_watches = liblinkspace::lk_process_while(&lk.0,watch, max_wait, check_at)?;
        tracing::trace!("Checking signals");
        Python::with_gil(|py| py.check_signals())?;
        check_at = check_at.saturating_add(CHECK_SIGNALS_INTERVAL);
        check_at = check_at.max(now());
    }
    Ok(empty_watches)
}
#[pyfunction]
pub fn lk_status_set(
    lk: &Linkspace,
    callback: PyFunc,
    group: &[u8],
    domain: &[u8],
    objtype: &[u8],
    instance: Option<&[u8]>,
) -> anyhow::Result<()> {
    use liblinkspace::conventions::status::*;
    let group = GroupID::try_fit_bytes_or_b64(group)?;
    let domain = Domain::try_fit_byte_slice(domain)?;
    let status_ctx = LkStatus {
        domain,
        group,
        objtype,
        instance,
    };
    tracing::info!("setup status {:?}",status_ctx);
    lk_status_set(&lk.0, status_ctx, move |_lk, domain, group, path, link| {
        tracing::info!("get gil status");
        Python::with_gil(|py| {
            let val = callback.call0(py)?;
            let bytes = bytelike(val.as_ref(py))?;

            let pkt = liblinkspace::lk_linkpoint(domain, group, path, &[link], bytes, None);
            tracing::info!("Status result {:?}",pkt);
            pkt
        })
    })
}
#[pyfunction]
pub fn lk_status_poll(
    lk: &Linkspace,
    callback: PyFunc,
    timeout: &[u8],
    group: &[u8],
    domain: &[u8],
    objtype: &[u8],
    instance: Option<&[u8]>,
    watch_id: Option<&[u8]>
) -> anyhow::Result<()> {
    use liblinkspace::conventions::status::*;
    let timeout = Stamp::try_from(timeout)?;
    let group = GroupID::try_fit_bytes_or_b64(group)?;
    let domain = Domain::try_fit_byte_slice(domain)?;
    let status_ctx = LkStatus {
        domain,
        group,
        objtype,
        instance,
    };
    let handler = PyPktStreamHandler {
        on_match: callback,
        on_close: None,
        on_err:None
    };
    lk_status_poll(&lk.0, status_ctx, timeout, handler,watch_id)
}

#[pyfunction]
pub fn lk_pull<'o>(py: Python<'o>, lk: &Linkspace, query: &Query) -> anyhow::Result<&'o PyBytes> {
    let hash = liblinkspace::conventions::lk_pull(&lk.0, &query.0)?;
    Ok(PyBytes::new(py, &hash.0))
}


#[pyclass]
#[pyo3(get_all)]
pub struct LkInfo {
    pub path:PathBuf
}
#[pyfunction]
pub fn lk_info<'o>(lk: &Linkspace) -> anyhow::Result<LkInfo> {
    let liblinkspace::linkspace::LkInfo{
        path
    }= liblinkspace::linkspace::lk_info(&lk.0);
    Ok(LkInfo{path:path.into()})
}




/** linkspace python bindings.
**/
#[pymodule]
fn lkpy(py: Python, m: &PyModule) -> PyResult<()> {
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

    m.add_function(wrap_pyfunction!(crate::spath, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_datapoint, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_linkpoint, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keypoint, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_read, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_write, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_keygen, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keystr, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_keyopen, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_eval, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_eval2str, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_encode, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_query, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_hash_query, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_parse, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_push, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_print, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_query_clear, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_open, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_save, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_get, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_get_all, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_watch, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_process, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_process_while, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_list_watches, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_info, m)?)?;

    m.add_function(wrap_pyfunction!(crate::lk_key, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_pull, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_status_poll, m)?)?;
    m.add_function(wrap_pyfunction!(crate::lk_status_set, m)?)?;

    m.add("PRIVATE", PyBytes::new(py, &consts::PRIVATE.0))?;
    m.add("PUBLIC", PyBytes::new(py, &consts::PUBLIC.0))?;
    m.add("DEFAULT_PKT", abe::DEFAULT_PKT)?;

    Ok(())
}

#[pyfunction]
/// Compatible with lk_query_push
pub fn spath<'o>(py: Python<'o>, components: &PyAny) -> anyhow::Result<&'o PyBytes> {
    let path = components
        .iter()?
        .map(|i| i.and_then(PyAny::extract::<&[u8]>))
        .try_collect::<Vec<_>>()?;
    let sp = liblinkspace::prelude::spath_buf(&path);
    Ok(PyBytes::new(py, &sp.spath_bytes()))
}

