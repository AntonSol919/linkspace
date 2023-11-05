// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::bytelike;
use crate::*;
use linkspace_pkt::{LkHash, NetPkt, NetPktArc, NetPktExt, Point, PointExt, Tag};
use wasm_bindgen::prelude::*;
use web_sys::TextDecoder;

// Ideally this is an ArrayBuffer and we give out readonly views
#[derive(Clone)]
#[wasm_bindgen]
pub struct Pkt(pub(crate) NetPktArc);
impl Pkt {
    pub fn from_dyn(p: &dyn NetPkt) -> Self {
        Pkt(p.as_netarc())
    }
    pub fn empty() -> Self {
        Pkt(unsafe { try_datapoint_ref(b"", NetOpts::Default).unwrap_unchecked() }.as_netarc())
    }
}

#[wasm_bindgen]
impl Pkt {
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> String {
        #[cfg(feature = "abe")]
        {
            linkspace_pkt::pkt_fmt(&self.0.netpktptr() as &dyn NetPkt)
        }

        #[cfg(not(feature = "abe"))]
        PktFmt(&self.0.netpktptr()).to_string()
        //{format!("<not compiled>")}
    }
    // TODO pass js function to format data field
    #[wasm_bindgen(js_name = toHTML)]
    pub fn to_html(&self, include_lossy_escaped_data: Option<bool>) -> Result<String, JsValue> {
        #[cfg(feature = "abe")]
        {
            todo!("abe not yet impl {include_lossy_escaped_data:?}")
        }

        #[cfg(not(feature = "abe"))]
        {
            let mut buf = String::new();

            PktFmt(&self.0.netpktptr())
                .to_html(&mut buf, include_lossy_escaped_data.unwrap_or(true), None)
                .map_err(|e| e.to_string())?;
            Ok(buf)
        }
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

    pub fn get_data_str(&self) -> Result<String, JsErr> {
        self.0
            .get_data_str()
            .map_err(|e| e.to_string().into())
            .map(String::from)
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
        self.0.create_stamp().map(|b| b.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn group(&self) -> Option<Box<[u8]>> {
        self.0.group().map(|g| g.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn spacename(&self) -> Option<Box<[u8]>> {
        self.0.spacename().map(|p| p.space_bytes().into())
    }
    #[wasm_bindgen(getter)]
    pub fn rooted_spacename(&self) -> Option<Box<[u8]>> {
        self.0.rooted_spacename().map(|p| p.rooted_bytes().into())
    }
    #[wasm_bindgen(getter)]
    pub fn recv(&self) -> Option<Box<[u8]>> {
        self.0.recv().map(|p| p.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn comp0(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp0().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp1(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp1().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp2(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp2().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp3(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp3().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp4(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp4().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp5(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp5().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp6(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp6().into()
    }
    #[wasm_bindgen(getter)]
    pub fn comp7(&self) -> Box<[u8]> {
        self.0.get_rooted_spacename().comp7().into()
    }
    pub fn comp_list(&self) -> Option<js_sys::Array> {
        self.0.rooted_spacename().map(|p| {
            p.comps_bytes()[0..p.depth()]
                .iter()
                .map(|s| -> js_sys::Uint8Array { (*s).into() })
                .collect()
        })
    }

    #[wasm_bindgen(getter)]
    pub fn pubkey(&self) -> Option<Box<[u8]>> {
        self.0.pubkey().map(|b| b.0.into())
    }
    #[wasm_bindgen(getter)]
    pub fn signature(&self) -> Option<Box<[u8]>> {
        self.0.signature().map(|b| b.0.into())
    }

    #[wasm_bindgen(getter)]
    pub fn depth(&self) -> Option<u8> {
        self.0.depth().copied()
    }
    #[wasm_bindgen(getter)]
    pub fn size(&self) -> u16 {
        self.0.size()
    }
    #[wasm_bindgen(getter)]
    pub fn links(&self) -> Links {
        Links {
            idx: 0,
            pkt: self.clone(),
        }
    }
    pub fn links_array(&self) -> js_sys::Array {
        use std::mem::size_of;
        if let Some(b) = self.links_bytes() {
            (0..self.0.get_links().len() as u32)
                .map(|i| {
                    let start = i * size_of::<Link>() as u32;
                    let start_hash = start + size_of::<Tag>() as u32;
                    let end = start_hash + size_of::<LkHash>() as u32;
                    js_sys::Array::of2(&b.subarray(start, start_hash), &b.subarray(start_hash, end))
                })
                .collect()
        } else {
            js_sys::Array::new()
        }
    }
    pub fn links_bytes(&self) -> Option<js_sys::Uint8Array> {
        self.0.tail().map(|t| t.links_as_bytes().into())
    }
}

#[wasm_bindgen]
#[derive(Clone)]
pub struct Links {
    idx: usize,
    pkt: Pkt,
}

#[wasm_bindgen]
pub struct LinkRes {
    pub done: bool,
    pub value: Option<Link>,
}
#[wasm_bindgen]
impl Links {
    #[wasm_bindgen]
    pub fn as_iter(self) -> Links {
        self
    }
    pub fn empty() -> Links {
        Links {
            idx: 0,
            pkt: Pkt::empty(),
        }
    }
    // https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols
    #[wasm_bindgen(js_name = next)]
    pub fn next_js(&mut self) -> LinkRes {
        let val = self.pkt.0.get_links().get(self.idx).copied().map(Link);
        self.idx += 1;
        LinkRes {
            done: val.is_none(),
            value: val,
        }
    }
}

/// Link for a linkpoint
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[wasm_bindgen]
#[repr(transparent)]
pub struct Link(pub(crate) linkspace_pkt::Link);

#[wasm_bindgen]
impl Link {
    #[wasm_bindgen(constructor)]
    pub fn new(tag: &JsValue, ptr: &JsValue) -> Result<Link, JsValue> {
        let tag = bytelike(tag)?;
        let ptr = bytelike(ptr)?;
        Ok(Link(linkspace_pkt::Link {
            tag: Tag::try_fit_byte_slice(&tag).ok().ok_or("invalid tag")?,
            ptr: LkHash::try_fit_bytes_or_b64(&ptr)
                .ok()
                .ok_or("invalid hash")?,
        }))
    }
    #[wasm_bindgen(js_name = toJSON)]
    pub fn as_json(&self) -> Result<JsValue, JsValue> {
        let string = format!("{{\"tag\":{:?},\"ptr\":\"{}\"}}", self.0.tag.0, self.0.ptr);
        js_sys::JSON::parse(&string)
    }
    #[wasm_bindgen(js_name = toAbeJSON)]
    pub fn as_abe_json(&self) -> Result<JsValue, JsValue> {
        // we have to debug output the abe string representation
        let string = format!(
            "{{\"abe_tag\":{:?},\"ptr\":\"{}\"}}",
            self.0.tag.to_string(),
            self.0.ptr
        );
        js_sys::JSON::parse(&string)
    }
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string_js(&self) -> Result<String, JsValue> {
        let mut tag = self.0.tag.0;
        let tag = TextDecoder::new()?.decode_with_u8_array(&mut tag)?;
        Ok(format!(
            "{{\"utf16_tag\":\"{}\",\"ptr\":\"{}\"}}",
            tag, self.0.ptr
        ))
    }
    #[wasm_bindgen(js_name = toHTML)]
    pub fn to_html(&self) -> Result<String, JsValue> {
        let mut tag = self.0.tag.0;
        let tag = TextDecoder::new()?.decode_with_u8_array(&mut tag)?;
        Ok(format!(
            "{{\"utf16_tag\":\"{}\",\"ptr\":\"{}\"}}",
            tag, self.0.ptr
        ))
    }
    #[wasm_bindgen(getter)]
    pub fn ptr(&self) -> Box<[u8]> {
        self.0.ptr.0.into()
    }
    #[wasm_bindgen(getter)]
    pub fn tag(&self) -> Box<[u8]> {
        self.0.tag.0.into()
    }
}
