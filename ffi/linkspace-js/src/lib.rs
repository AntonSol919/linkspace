#![feature(try_blocks,thread_local,lazy_cell,ptr_from_ref)]
#![allow(dead_code,unused_variables)]
pub mod jspkt;
pub mod utils;
pub mod consts;

use js_sys::{Uint8Array };
use jspkt::Pkt;
use linkspace_pkt::{MIN_NETPKT_SIZE, PartialNetHeader ,*};
use utils::{JsErr,*};
use wasm_bindgen::prelude::*;
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(start)]
fn main() -> std::result::Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    Ok(())
}



fn common_args(obj:&JsValue) -> Result<(GroupID, Domain, IPathBuf, Vec<Link>, Option<Stamp>)> {
    todo!()
        /*
    let group = group
        .map(|group| GroupID::try_fit_bytes_or_b64(group))
        .transpose()?
        .unwrap_or(linkspace_pkt::PUBLIC);
    let domain = domain
        .map(|domain| Domain::try_fit_byte_slice(domain))
        .transpose()?
        .unwrap_or(AB::default());
    let path = match path {
        None => IPathBuf::new(),
        Some(p) => {
            if let Ok(p) = p.downcast::<Uint8Array>(){
                SPathBuf::try_from(p.to_vec())?.try_ipath()?
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
        */
}

#[wasm_bindgen]
pub struct SigningKey(linkspace_pkt::SigningKey);
#[wasm_bindgen]
impl SigningKey{
    #[wasm_bindgen(getter)]
    pub fn pubkey(&self) -> Box<[u8]> {
        self.0.pubkey().0.into()
    }
}


#[wasm_bindgen]
pub fn lk_keygen() -> SigningKey {
    todo!()//SigningKey(linkspace_pkt::key::lk_keygen())
}
#[wasm_bindgen]
pub fn lk_enckey(key: &SigningKey, password: &[u8]) -> String {
    todo!()//linkspace_pkt::lk_enckey(&key.0, password)
}
#[wasm_bindgen]
pub fn lk_keyopen(id: &str, password: &[u8]) -> Result<SigningKey> {
    todo!()//Ok(SigningKey(linkspace_rs::key::lk_keyopen(id, password)?))
}
#[wasm_bindgen]
pub struct Linkspace(usize);

#[wasm_bindgen]
pub fn lk_key(
    lk: &Linkspace,
    password: &[u8],
    name: Option<String>,
    create: Option<bool>,
) -> Result<SigningKey> {
    todo!()//linkspace_rs::lk_key(&lk.0, password, name, create.unwrap_or(false)).map(SigningKey)
}

#[wasm_bindgen]
pub fn lk_datapoint(data: &JsValue) -> Result<Pkt> {
    todo!()/*
    Ok(jspkt::Pkt::from_dyn(
        &linkspace_pkt::point::lk_datapoint_ref(bytelike(data)?)?,
    ))*/
}
#[wasm_bindgen]
pub fn lk_linkpoint( fields : &JsValue) -> Result<Pkt> {
    todo!()/*
    let pkt = linkspace_rs::point::lk_linkpoint_ref(domain, group, &*path, &*links, data, create)?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
        */
}
#[wasm_bindgen]
pub fn lk_keypoint(fields: &JsValue) -> Result<Pkt> {
    todo!()/*
    let pkt =
        linkspace_rs::point::lk_keypoint_ref(domain, group, &*path, &*links, data, create, &key.0)?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
        */
}

fn pptr(p: Option<&Pkt>) -> Option<&dyn NetPkt> {
    p.map(|p| &p.0 as &dyn NetPkt)
}

#[wasm_bindgen]
pub fn lk_write( pkt: &Pkt) -> Uint8Array {
    let arr = Uint8Array::new_with_length(pkt.size() as u32);
    arr.copy_from(pkt.0.as_netpkt_bytes());
    arr
}


#[wasm_bindgen]
pub fn b64(bytes:&[u8], mini:Option<bool>) -> String{
    let b = linkspace_pkt::B64(bytes);
    if mini.unwrap_or(false){b.b64_mini()} else{b.to_string()}
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
pub fn lk_read(bytes: &Uint8Array,validate: Option<bool>) ->Result<js_sys::Array,JsValue>{
    let bufsize = bytes.length() as usize ;
    if bufsize < MIN_NETPKT_SIZE {
        return Err(MIN_NETPKT_SIZE.into());
    }
    let mut partial = PartialNetHeader::EMPTY;
    use std::ptr;
    unsafe { bytes.slice(0,MIN_NETPKT_SIZE as u32 ).raw_copy_to_ptr( ptr::from_mut(&mut partial) as *mut u8)};
    partial.point_header.check().map_err(err)?;

    let pktsize = partial.point_header.net_pkt_size();
    if pktsize > bufsize { return Err(pktsize.into());};
    let pktsize = pktsize as u32;

    let pkt = unsafe{
        linkspace_pkt::NetPktArc::from_header_and_copy(
            partial,
            validate.unwrap_or(true),
            |dest|{
                bytes.slice(std::mem::size_of::<PartialNetHeader>() as u32,pktsize).raw_copy_to_ptr(dest.as_mut_ptr())
            }
        )};
    let pkt = pkt.map_err(err)?;
    
    Ok(js_sys::Array::of2(&Pkt(pkt).into(),&bytes.slice(pktsize,bufsize as u32)))
}

