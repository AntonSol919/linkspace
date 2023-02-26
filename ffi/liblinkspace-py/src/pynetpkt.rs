// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::str::FromStr;

use liblinkspace::misc::FieldEnum;
use liblinkspace::prelude::*;
use pyo3::types::PyByteArray;
use pyo3::{basic::CompareOp, prelude::*, types::PyBytes};

use misc::{RecvPkt, ReroutePkt};
#[pyclass(mapping)]
#[derive(Clone)]
pub struct Pkt(pub(crate) ReroutePkt<RecvPkt<NetPktArc>>);
impl Pkt {
    pub fn from_dyn(p: &dyn NetPkt) -> Self {
        Pkt(ReroutePkt::new(RecvPkt {
            recv: p.recv().unwrap_or_else(now),
            pkt: p.as_netarc(),
        }))
    }
}
impl<'o> From<RecvPktPtr<'o>> for Pkt {
    fn from(p: RecvPktPtr) -> Self {
        let p = ReroutePkt::new(p.map(|v| v.as_netarc()));
        Pkt(p)
    }
}

#[pymethods]
impl Pkt {
    pub fn __str__(&self) -> String {
        String::from_utf8(lk_eval("{pkt}", self.0.netpktptr() as &dyn NetPkt).unwrap()).unwrap()
    }
    pub fn __getitem__<'p>(&self, py: Python<'p>, field: &str) -> anyhow::Result<&'p PyBytes> {
        let field = FieldEnum::from_str(field)?;
        let mut v = smallvec::SmallVec::<[u8; 32]>::new();
        field.bytes(self.0.netpktptr(), &mut v)?;
        Ok(PyBytes::new(py, &v))
    }
    pub fn __richcmp__(&self, other: PyRef<Pkt>, op: CompareOp) -> bool {
        use liblinkspace::misc::TreeEntry;
        let self_key = TreeEntry::from_pkt(0.into(), &self.0).ok_or(self.0.hash_ref());
        let other_key = TreeEntry::from_pkt(0.into(), &other.0).ok_or(other.0.hash_ref());
        match op {
            CompareOp::Lt => self_key < other_key,
            CompareOp::Le => self_key <= other_key,
            CompareOp::Eq => self.0.hash() == other.0.hash(),
            CompareOp::Ne => self.0.hash() != other.0.hash(),
            CompareOp::Gt => self_key > other_key,
            CompareOp::Ge => self_key >= other_key,
        }
    }
    pub fn __hash__(&self) -> isize {
        let bytes = &self.0.hash().0[8..std::mem::size_of::<isize>()];
        isize::from_ne_bytes(bytes.try_into().unwrap())
    }
    #[getter]
    pub fn pkt_type(&self) -> u8 {
        self.0.point_header().pkt_type.bits()
    }
    #[getter]
    pub fn hash<'p>(&self, py: Python<'p>) -> &'p PyBytes {
        PyBytes::new(py, &self.0.hash().0)
    }

    #[getter]
    /// data
    pub fn data<'p>(&self, py: Python<'p>) -> &'p PyBytes {
        PyBytes::new(py, self.0.netpktptr().data())
    }
    #[getter]
    /// domain
    pub fn domain<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.as_point().domain().map(|d| PyBytes::new(py, &d.0))
    }
    #[getter]
    /// create stamp
    pub fn create<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.create_stamp().map(|b| PyBytes::new(py, &b.0))
    }
    #[getter]
    pub fn group<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.group().map(|g| PyBytes::new(py, &g.0))
    }
    #[getter]
    pub fn spath<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.spath().map(|p| PyBytes::new(py, p.spath_bytes()))
    }
    #[getter]
    pub fn ipath<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.ipath().map(|p| PyBytes::new(py, p.ipath_bytes()))
    }
    #[getter]
    pub fn recv<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.recv().map(|p| PyBytes::new(py, &p.0))
    }
    pub fn path_list<'p>(&self, py: Python<'p>) -> Option<Vec<&'p PyBytes>> {
        self.0.ipath().map(|p| {
            p.comps_bytes()[0..*p.path_len() as usize]
                .into_iter()
                .map(|s| PyBytes::new(py, s))
                .collect()
        })
    }

    /*
    #[getter]
    fn links<'p>(&self) -> Option<Vec<Link>>{
        self.0.links()
            .map(|lst|
                 lst
                 .into_iter()
                 .map(|l|Link{tag:l.tag.0,ptr:l.ptr.0})
                 .collect()
            )
    }
    */
    #[getter]
    /// public key
    fn pubkey<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.pubkey().map(|b| PyBytes::new(py, &b.0))
    }
    #[getter]
    /// signature
    fn signature<'p>(&self, py: Python<'p>) -> Option<&'p PyBytes> {
        self.0.signature().map(|b| PyBytes::new(py, &b.0))
    }

    #[getter]
    /// point size
    fn point_size<'p>(&self, py: Python<'p>) -> &'p PyBytes {
        PyBytes::new(py, &self.0.point_header_ref().point_size.0)
    }
    #[getter]
    /// number of components in the path
    fn path_len<'p>(&self) -> Option<u8> {
        self.0.path_len().map(|b| *b)
    }
    #[setter]
    pub fn set_netflags(&mut self, f: u8) {
        self.0.net_header.flags =
            unsafe { liblinkspace::prelude::NetFlags::from_bits_unchecked(f) };
    }
    #[setter]
    pub fn set_hop(&mut self, b: [u8; 4]) {
        self.0.net_header.hop.0 = b
    }
    #[setter]
    pub fn set_until(&mut self, until: [u8; 8]) {
        self.0.net_header.stamp.0 = until;
    }
    #[setter]
    pub fn set_ubits0(&mut self, b: [u8; 4]) {
        self.0.net_header.ubits[0].0 = b;
    }
    #[setter]
    pub fn set_ubits1(&mut self, b: [u8; 4]) {
        self.0.net_header.ubits[1].0 = b;
    }
    #[setter]
    pub fn set_ubits2(&mut self, b: [u8; 4]) {
        self.0.net_header.ubits[2].0 = b;
    }
    #[setter]
    pub fn set_ubits3(&mut self, b: [u8; 4]) {
        self.0.net_header.ubits[3].0 = b;
    }
    /// Misc netflags
    #[getter]
    pub fn netflags(&self) -> u8 {
        self.0.net_header.flags.bits()
    }
    /// Number of hops this packet has had.
    #[getter]
    pub fn hop(&self) -> [u8; 4] {
        self.0.net_header.hop.0
    }
    /// Suggestion to others to forget this packet after this date
    #[getter]
    pub fn until(&self) -> [u8; 8] {
        self.0.net_header.stamp.0
    }
    #[getter]
    pub fn ubits0(&self) -> [u8; 4] {
        self.0.net_header.ubits[0].0
    }
    #[getter]
    pub fn ubits1(&self) -> [u8; 4] {
        self.0.net_header.ubits[1].0
    }
    #[getter]
    pub fn ubits2(&self) -> [u8; 4] {
        self.0.net_header.ubits[2].0
    }
    #[getter]
    pub fn ubits3(&self) -> [u8; 4] {
        self.0.net_header.ubits[3].0
    }

    #[getter]
    pub fn links(slf: PyRef<Self>) -> Option<Links> {
        let _ = slf.0.links()?;
        Some(Links {
            pkt: slf.into(),
            idx: 0,
        })
    }
}

#[pyclass]
#[pyo3(get_all)]
pub struct Links {
    pub pkt: Py<Pkt>,
    pub idx: usize,
}

#[pymethods]
impl Links {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python<'_>) -> Option<Link> {
        let this = slf.pkt.borrow(py).0.get_links().get(slf.idx).copied();
        slf.idx += 1;
        this.map(Into::into)
    }
    fn __traverse__(&self, visit: pyo3::PyVisit<'_>) -> Result<(), pyo3::PyTraverseError> {
        visit.call(&self.pkt)
    }
}

/// Link for a linkpoint
#[pyclass]
#[derive(Clone,Copy)]
#[repr(C)]
pub struct Link {
    pub tag: [u8; 16],
    pub ptr: [u8; 32],
}
#[pymethods]
impl Link {
    #[getter]
    fn ptr<'o>(&self,py:Python<'o>) -> &'o PyBytes {
        PyBytes::new(py, &self.ptr)
    }
    #[getter]
    fn tag<'o>(&self,py:Python<'o>) -> &'o PyBytes {
        PyBytes::new(py, &self.tag)
    }
    #[new]
    fn new(py: Python, tag: &PyAny, ptr: &PyAny) -> anyhow::Result<Self> {
        let tag_b = PyByteArray::from(py, tag)?;
        let tag = unsafe { tag_b.as_bytes() };
        let ptr_b = PyByteArray::from(py, ptr)?;
        let ptr = unsafe { ptr_b.as_bytes() };
        Ok(Link {
            tag: Tag::try_fit_byte_slice(tag)?.0,
            ptr: LkHash::try_fit_bytes_or_b64(ptr)?.0,
        })
    }
    pub fn __repr__(&self, py: Python<'_>) -> String {
        let tag = PyBytes::new(py, &self.tag).repr().unwrap();
        let ptr = PyBytes::new(py, &self.ptr).repr().unwrap();
        format!("Link({tag},{ptr})")
    }
}

impl Into<liblinkspace::prelude::Link> for Link {
    fn into(self) -> liblinkspace::prelude::Link {
        prelude::Link {
            tag: self.tag.into(),
            ptr: self.ptr.into(),
        }
    }
}
impl Into<Link> for prelude::Link {
    fn into(self) -> Link {
        Link {
            tag: self.tag.0,
            ptr: self.ptr.0,
        }
    }
}
