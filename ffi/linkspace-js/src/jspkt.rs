// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::{*};
use js_sys::{Object };
use linkspace_pkt::{Tag, LkHash, NetPkt, Point, PointExt,  NetPktExt, NetPktArc };
use wasm_bindgen::prelude::*;
use web_sys::TextDecoder;

use crate::bytelike;


// Ideally this is an ArrayBuffer and we give out readonly views
#[derive(Clone)]
#[wasm_bindgen]
pub struct Pkt(pub(crate) NetPktArc);
impl Pkt {
    pub fn from_dyn(p: &dyn NetPkt) -> Self {
        Pkt(p.as_netarc())
    }
    pub fn empty() -> Self {
        Pkt(unsafe {try_datapoint_ref(b"", NetOpts::Default).unwrap_unchecked()}.as_netarc())
    }
}


#[wasm_bindgen]
impl Pkt {
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> String {
        #[cfg(feature = "abe")]
        {linkspace_pkt::pkt_fmt(&self.0.netpktptr() as &dyn NetPkt)}

        #[cfg(not(feature = "abe"))]
        PktFmt(&self.0.netpktptr()).to_string()
        //{format!("<not compiled>")}
    }
    #[wasm_bindgen(js_name = toHTML)]
    pub fn to_html(&self) -> Result<String,JsValue>{
        #[cfg(feature = "abe")]
        { todo!()}

        #[cfg(not(feature = "abe"))]
        {
            let mut buf = String::new();
            PktFmt(&self.0.netpktptr()).to_html(&mut buf, |_,_| Ok(()))
                .map_err(|e|e.to_string())?;
            Ok(buf)
        }
    }
    #[wasm_bindgen(getter)]
    pub fn obj(&self) -> Object {
        crate::pkt_obj(self.clone())
    }
    /*
    pub fn __richcmp__(&self, other: PyRef<Pkt>, op: CompareOp) -> bool {
        use linkspace::misc::TreeEntry;
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
    */
    #[wasm_bindgen(getter)]
    pub fn pkt_type(&self) -> u8 {
        self.0.point_header().point_type.bits()
    }
    #[wasm_bindgen(getter)]
    pub fn hash(&self) -> Box<[u8]> {
        self.0.hash_ref().0.into()
    }

    #[wasm_bindgen(getter)]
    /// data
    pub fn data(&self) -> Box<[u8]> {
        self.0.data().into()
    }
    #[wasm_bindgen(getter)]
    pub fn domain(&self) -> Option<Box<[u8]>> {
        self.0.as_point().domain().map(|d| d.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn create(&self) -> Option<Box<[u8]>> {
        self.0.create_stamp().map(|b|b.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn group(&self) -> Option<Box<[u8]>> {
        self.0.group().map(|g| g.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> Option<Box<[u8]>> {
        self.0.path().map(|p| p.spath_bytes().into())
    }
    #[wasm_bindgen(getter)]
    pub fn ipath(&self) -> Option<Box<[u8]>> {
        self.0.ipath().map(|p| p.ipath_bytes().into())
    }
    #[wasm_bindgen(getter)]
    pub fn recv(&self) -> Option<Box<[u8]>> {
        self.0.recv().map(|p| p.0.into())
    }
    #[wasm_bindgen(getter)] pub fn path0(&self) -> Box<[u8]> {self.0.get_ipath().path0().into()}
    #[wasm_bindgen(getter)] pub fn path1(&self) -> Box<[u8]> {self.0.get_ipath().path1().into()}
    #[wasm_bindgen(getter)] pub fn path2(&self) -> Box<[u8]> {self.0.get_ipath().path2().into()}
    #[wasm_bindgen(getter)] pub fn path3(&self) -> Box<[u8]> {self.0.get_ipath().path3().into()}
    #[wasm_bindgen(getter)] pub fn path4(&self) -> Box<[u8]> {self.0.get_ipath().path4().into()}
    #[wasm_bindgen(getter)] pub fn path5(&self) -> Box<[u8]> {self.0.get_ipath().path5().into()}
    #[wasm_bindgen(getter)] pub fn path6(&self) -> Box<[u8]> {self.0.get_ipath().path6().into()}
    #[wasm_bindgen(getter)] pub fn path7(&self) -> Box<[u8]> {self.0.get_ipath().path7().into()}
    pub fn path_list(&self) -> Option<js_sys::Array> {
        self.0.ipath().map(|p| {
            p.comps_bytes()[0..*p.path_len() as usize]
                .iter()
                .map(|s| -> js_sys::Uint8Array { (*s).into()})
                .collect()
        })
    }

    #[wasm_bindgen(getter)]
    pub fn pubkey(&self) -> Option<Box<[u8]>> {
        self.0.pubkey().map(|b|  b.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Option<Box<[u8]>> {
        self.0.signature().map(|b|  b.0.into())
    }

    #[wasm_bindgen(getter)]
    pub fn point_size(&self) -> u16{
        self.0.point_header_ref().point_size.get()
    }
    #[wasm_bindgen(getter)]
    pub fn path_len(&self) -> Option<u8> {
        self.0.path_len().copied()
    }
    pub fn size(&self) -> u16 {
        self.0.size()
    }
    #[wasm_bindgen(getter)]
    pub fn links(&self) -> Links {
        Links {idx : 0 , pkt: self.clone()}
    }
    pub fn links_array(&self) -> js_sys::Array{
        self.0.get_links().iter().copied().map(Link).map(|v| -> JsValue{v.into()} ).collect()
    }
    pub fn links_bytes(&self) -> Option<js_sys::Uint8Array>{
        self.0.tail().map(|t| t.links_as_bytes().into())
    }
}



#[wasm_bindgen]
#[derive(Clone)]
pub struct Links{
    idx: usize,
    pkt: Pkt
}

#[wasm_bindgen]
pub struct LinkRes {
    pub done: bool,
    pub value:Option<Link>
}
#[wasm_bindgen]
impl Links{
    pub fn default()-> Links { Links{ idx:0, pkt:Pkt::empty()}}
    #[wasm_bindgen]
    pub fn next(&mut self) -> LinkRes{
        let val = self.pkt.0.get_links().get(self.idx).copied().map(Link);
        self.idx +=1;
        LinkRes { done: val.is_none(), value: val }
    }
}


/// Link for a linkpoint
#[derive(Clone,Copy,Eq,PartialEq,Ord,PartialOrd,Hash)]
#[wasm_bindgen]
#[repr(transparent)]
pub struct Link(pub(crate)linkspace_pkt::Link);

#[wasm_bindgen]
impl Link {

    #[wasm_bindgen(constructor)]
    pub fn new(tag: &JsValue, ptr: &JsValue) -> Result<Link,JsValue> {
        let tag = bytelike(tag)?;
        let ptr = bytelike(ptr)?;
        Ok(Link(linkspace_pkt::Link{
            tag: Tag::try_fit_byte_slice(&tag).ok().ok_or("invalid tag")?,
            ptr: LkHash::try_fit_bytes_or_b64(&ptr).ok().ok_or("invalid hash")?,
        }))
    }
    #[wasm_bindgen(js_name = toJSON)]
    pub fn as_json(&self) -> Result<JsValue,JsValue>{
        let string = format!("{{\"tag\":{:?},\"ptr\":\"{}\"}}",self.0.tag.0,self.0.ptr);
        js_sys::JSON::parse(&string)
    }
    #[wasm_bindgen(js_name = toAbeJSON)]
    pub fn as_abe_json(&self) -> Result<JsValue,JsValue>{
        // we have to debug output the abe string representation
        let string = format!("{{\"abe_tag\":{:?},\"ptr\":\"{}\"}}",self.0.tag.to_string(),self.0.ptr);
        js_sys::JSON::parse(&string)
    }
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> Result<String,JsValue>{
        let mut tag = self.0.tag.0;
        let tag = TextDecoder::new()?.decode_with_u8_array(&mut tag)?;
        Ok(format!("{{\"utf16_tag\":\"{}\",\"ptr\":\"{}\"}}",tag,self.0.ptr))
    }
    #[wasm_bindgen(js_name = toHTML)]
    pub fn to_html(&self) -> Result<String,JsValue>{
        let mut tag = self.0.tag.0;
        let tag = TextDecoder::new()?.decode_with_u8_array(&mut tag)?;
        Ok(format!("{{\"utf16_tag\":\"{}\",\"ptr\":\"{}\"}}",tag,self.0.ptr))
    }
    #[wasm_bindgen(getter)]
    pub fn ptr(&self) -> Box<[u8]> {
        self.0.ptr.0.into()
    }
    #[wasm_bindgen(getter)]
    pub fn tag(&self) -> Box<[u8]>{
        self.0.tag.0.into()
    }
}

