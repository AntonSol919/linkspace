#![feature(
    try_blocks,
    thread_local,
    lazy_cell,
    ptr_from_ref,
    iterator_try_collect
)]
pub mod consts;
pub mod jspkt;
pub mod utils;
use js_sys::Uint8Array;
use linkspace_pkt::{PartialNetHeader, MIN_NETPKT_SIZE, *};
use smallvec::SmallVec;
use utils::*;
use wasm_bindgen::prelude::*;

use crate::jspkt::Pkt;
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
fn main() -> std::result::Result<(), JsValue> {
    use js_sys::Object;
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    // Setting the Links[Symbol.iterator] = Links::as_iter
    let proto = Object::get_prototype_of(&jspkt::Links::empty().into());
    js_sys::Reflect::set(
        &proto,
        &js_sys::Symbol::iterator(),
        &js_sys::Reflect::get(&proto, &"as_iter".into())
            .unwrap()
            .into(),
    )
    .unwrap();

    Ok(())
}

#[wasm_bindgen]
pub fn lk_datapoint(data: &JsValue) -> Result<Pkt> {
    let data = bytelike(data)?;
    Ok(Pkt(try_datapoint_ref(&data, NetOpts::Default)?.as_netarc()))
}

#[wasm_bindgen]
pub struct SigningKey(linkspace_pkt::SigningKey);
#[wasm_bindgen]
impl SigningKey {
    #[wasm_bindgen(getter)]
    pub fn pubkey(&self) -> Box<[u8]> {
        self.0.pubkey().0.into()
    }
}
#[wasm_bindgen]
pub fn lk_keygen() -> SigningKey {
    SigningKey(linkspace_pkt::SigningKey::generate())
}

use linkspace_argon2_identity as identity;
#[wasm_bindgen]
pub fn lk_key_encrypt(key: &SigningKey, password: &[u8]) -> String {
    identity::encrypt(
        &key.0,
        password,
        if password.is_empty() {
            Some(identity::INSECURE_COST)
        } else {
            None
        },
    )
}

#[wasm_bindgen]
pub fn lk_key_pubkey(id: &str) -> Result<Box<[u8]>> {
    Ok(identity::pubkey(id)?.into())
}
#[wasm_bindgen]
pub fn lk_key_decrypt(id: &str, password: &[u8]) -> Result<SigningKey> {
    Ok(SigningKey(identity::decrypt(id, password)?))
}

#[wasm_bindgen]
extern "C" {

    pub type Fields;
    #[wasm_bindgen(structural, method, getter)]
    pub fn group(this: &Fields) -> JsValue;
    #[wasm_bindgen(structural, method, getter)]
    pub fn domain(this: &Fields) -> JsValue;
    #[wasm_bindgen(structural, method, getter)]
    pub fn space(this: &Fields) -> JsValue;
    #[wasm_bindgen(structural, method, getter)]
    pub fn links(this: &Fields) -> JsValue;
    #[wasm_bindgen(structural, method, getter)]
    pub fn create(this: &Fields) -> JsValue;

}
fn common_args(obj: &Fields) -> Result<(GroupID, Domain, RootedSpaceBuf, Vec<Link>, Stamp), JsErr> {
    let group = opt_bytelike(&obj.group())
        .map_err(|_| "invalid group")?
        .map(|group| {
            GroupID::try_fit_bytes_or_b64(&group)
                .ok()
                .ok_or("invalid group")
        })
        .transpose()?
        .unwrap_or(PUBLIC);
    let domain = opt_bytelike(&obj.domain())
        .map_err(|_| "invalid domain")?
        .map(|domain| {
            Domain::try_fit_byte_slice(&domain)
                .ok()
                .ok_or("invalid domain")
        })
        .transpose()?
        .unwrap_or(AB::default());
    let spacename = obj.space();
    let spacename = if spacename.is_falsy() {
        RootedSpaceBuf::new()
    } else if let Some(st) = spacename.dyn_ref::<Uint8Array>() {
        SpaceBuf::try_from_inner(st.to_vec())?.try_into_rooted()?
    } else {
        let it = js_sys::try_iter(&spacename)?
            .ok_or("unknown format - expected spacename bytes or array")?;
        let mut spacename = SmallVec::<[Bytes; 8]>::new();
        for (i, el) in it.enumerate() {
            spacename.push(
                bytelike(&el?).map_err(|_| JsErr::from(&*format!("Error reading space[{i}]")))?,
            );
        }
        RootedSpaceBuf::try_from_iter(spacename)?
    };
    let links = obj.links();

    let links = if links.is_falsy() {
        vec![]
    } else if let Some(bytes) = links.dyn_ref::<Uint8Array>() {
        use std::mem::size_of;
        let length = bytes.length() as usize;
        if length % size_of::<linkspace_pkt::Link>() != 0 {
            return Err("wrong number of bytes to turn into links".into());
        }
        let mut links = vec![Link::DEFAULT; length / size_of::<linkspace_pkt::Link>()];
        unsafe {
            let as_bytes = std::slice::from_raw_parts_mut(links.as_mut_ptr().cast::<u8>(), length);
            bytes.copy_to(as_bytes);
        }
        links
    } else {
        static ERR: &str = "expected [Link] iter";
        let it = js_sys::try_iter(&links)?.ok_or(ERR)?;
        it.map(|link| -> Result<_, JsErr> { try_as_link(&link?) })
            .try_collect::<Vec<_>>()?
    };
    let create_stamp = obj.create();
    let create_stamp = if create_stamp.is_falsy() {
        linkspace_pkt::now()
    } else {
        static ERR: &str = "expected stamp [u8;8] or falsy";
        Stamp::try_from(&*create_stamp.dyn_ref::<Uint8Array>().ok_or(ERR)?.to_vec())
            .ok()
            .ok_or(ERR)?
    };
    Ok((group, domain, spacename, links, create_stamp))
}

pub fn try_as_link(obj: &JsValue) -> Result<Link, JsErr> {
    use js_sys::Reflect;
    match Reflect::get(obj, &JsValue::from_str("ptr")) {
        Ok(ptr) => {
            let ptr: LkHash = B64(as_byte_array(&ptr)?);
            match Reflect::get(obj, &JsValue::from_str("tag")) {
                Ok(tag) => {
                    let tag: Tag = AB(as_byte_array(&tag)?);
                    Ok(Link { tag, ptr })
                }
                Err(_e) => Err("Expected obj with [ptr] and [tag]".into()),
            }
        }
        Err(_e) => {
            let bytes = as_byte_array::<48>(obj)?;
            Ok(unsafe { std::mem::transmute(bytes) })
        }
    }
}
pub fn as_byte_array<const N: usize>(obj: &JsValue) -> Result<[u8; N], JsErr> {
    let bytes = obj
        .dyn_ref::<js_sys::Uint8Array>()
        .ok_or("expected string or Uint8Array")?;
    if bytes.length() != N as u32 {
        Err("expected N bytes")?
    }
    let mut arr = [0; N];
    bytes.copy_to(&mut arr);
    Ok(arr)
}
#[wasm_bindgen]
pub fn lk_linkpoint(data: &JsValue, fields: &Fields) -> Result<Pkt> {
    let data = bytelike(data)?;
    let (domain, group, spacename, links, create) = common_args(fields)?;
    let pkt = linkspace_pkt::try_linkpoint_ref(
        domain,
        group,
        &spacename,
        &links,
        &data,
        create,
        NetOpts::Default,
    )?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
}

#[wasm_bindgen]
pub fn lk_keypoint(key: &SigningKey, data: &JsValue, fields: &Fields) -> Result<Pkt> {
    let data = bytelike(data)?;
    let (domain, group, spacename, links, create) = common_args(fields)?;
    let pkt = linkspace_pkt::try_keypoint_ref(
        domain,
        group,
        &spacename,
        &links,
        &data,
        create,
        &key.0,
        NetOpts::Default,
    )?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
}

#[wasm_bindgen]
pub fn lk_write(pkt: &Pkt, allow_private: Option<bool>) -> Result<Uint8Array, JsErr> {
    if allow_private != Some(true) {
        pkt.0.check_private()?;
    }
    let arr = Uint8Array::new_with_length(pkt.size() as u32);
    arr.copy_from(pkt.0.as_netpkt_bytes());
    Ok(arr)
}

#[wasm_bindgen(typescript_custom_section)]
const ITEXT_STYLE: &'static str = r#"
/**
* @param {Uint8Array} bytes
* @param {boolean | undefined} validate
* @returns {[Pkt,Uint8Array]}
*/
export function lk_read(bytes: Uint8Array, validate?: boolean): [Pkt,Uint8Array];
"#;
#[wasm_bindgen(skip_typescript)]
pub fn lk_read(bytes: &Uint8Array, allow_private: Option<bool>) -> Result<js_sys::Array, JsErr> {
    _read(bytes, allow_private.unwrap_or(false), false)
}
#[wasm_bindgen(typescript_custom_section)]
const ITEXT_STYLE: &'static str = r#"
/**
* @param {Uint8Array} bytes
* @param {boolean | undefined} validate
* @returns {[Pkt,Uint8Array]}
*/
export function lk_read_unchecked(bytes: Uint8Array, validate?: boolean): [Pkt,Uint8Array];
"#;
#[wasm_bindgen(skip_typescript)]
pub fn lk_read_unchecked(
    bytes: &Uint8Array,
    allow_private: Option<bool>,
) -> Result<js_sys::Array, JsErr> {
    _read(bytes, allow_private.unwrap_or(false), true)
}

pub fn _read(
    bytes: &Uint8Array,
    allow_private: bool,
    skip_hash: bool,
) -> Result<js_sys::Array, JsErr> {
    let bufsize = bytes.length() as usize;
    if bufsize < MIN_NETPKT_SIZE {
        return Err(JsValue::from(MIN_NETPKT_SIZE).into());
    }
    let mut partial = PartialNetHeader::EMPTY;
    use std::ptr;
    unsafe {
        bytes
            .slice(0, MIN_NETPKT_SIZE as u32)
            .raw_copy_to_ptr(ptr::from_mut(&mut partial) as *mut u8)
    };
    partial.point_header.check()?;

    let pktsize = partial.point_header.size() as usize;
    if pktsize > bufsize {
        return Err(JsValue::from(pktsize).into());
    };
    let pktsize = pktsize as u32;

    let pkt = unsafe {
        linkspace_pkt::NetPktArc::from_header_and_copy(partial, skip_hash, |dest| {
            bytes
                .slice(std::mem::size_of::<PartialNetHeader>() as u32, pktsize)
                .raw_copy_to_ptr(dest.as_mut_ptr())
        })
    };
    let pkt = pkt?;
    if !allow_private {
        pkt.check_private()?
    };
    Ok(js_sys::Array::of2(
        &Pkt(pkt).into(),
        &bytes.slice(pktsize, bufsize as u32),
    ))
}

fn pptr(p: Option<&Pkt>) -> Option<&dyn NetPkt> {
    p.map(|p| &p.0 as &dyn NetPkt)
}
#[wasm_bindgen]
pub fn lk_eval(
    expr: &str,
    pkt: Option<Pkt>,
    argv: Option<js_sys::Iterator>,
    loose: Option<bool>,
) -> Result<Box<[u8]>, JsError> {
    let v: SmallVec<[Bytes; 4]> = match argv {
        Some(argv) => argv.into_iter().map(|v| bytelike(&v?)).try_collect()?,
        None => SmallVec::new(),
    };
    let argv: SmallVec<[&[u8]; 4]> = v.iter().map(|s| s.as_slice()).collect();
    use linkspace::abe::scope::*;
    let udata = UserData {
        argv: Some(&argv),
        pkt: pptr(pkt.as_ref()),
    };
    let uscope = scope(udata).map_err(|e| JsError::new(&format!("{e:#?}")))?;
    linkspace::varscope::lk_eval(uscope, expr, loose.unwrap_or(false))
        .map_err(|e| JsError::new(&format!("{e:#?}")))
        .map(|v| v.into_boxed_slice())
}
#[wasm_bindgen]
pub fn lk_eval2str(
    expr: &str,
    pkt: Option<Pkt>,
    argv: Option<js_sys::Iterator>,
    loose: Option<bool>,
) -> Result<String, JsError> {
    String::from_utf8(lk_eval(expr, pkt, argv, loose)?.into())
        .map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn b64(bytes: &[u8], mini: Option<bool>) -> String {
    let b = linkspace_pkt::B64(bytes);
    if mini.unwrap_or(false) {
        b.b64_mini()
    } else {
        b.to_string()
    }
}

#[wasm_bindgen]
pub fn lk_encode(bytes: &[u8], options: Option<String>) -> String {
    linkspace::lk_encode(bytes, options.as_deref().unwrap_or(""))
}
#[wasm_bindgen]
pub fn blake3_hash(bytes: &[u8]) -> Box<[u8]> {
    Box::new(*blake3::hash(bytes).as_bytes())
}
#[wasm_bindgen]
pub fn now() -> Box<[u8]> {
    Box::new(linkspace_pkt::now().0)
}
#[wasm_bindgen]
pub fn build_info() -> String {
    static BUILD_INFO: &str = concat!(
        env!("CARGO_PKG_NAME"),
        " - ",
        env!("CARGO_PKG_VERSION"),
        " - ",
        env!("VERGEN_GIT_BRANCH"),
        " - ",
        env!("VERGEN_GIT_DESCRIBE"),
        " - ",
        env!("VERGEN_RUSTC_SEMVER")
    );

    BUILD_INFO.to_string()
}
