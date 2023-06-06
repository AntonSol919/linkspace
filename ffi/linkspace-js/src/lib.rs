#![feature(try_blocks,thread_local,lazy_cell,ptr_from_ref,iterator_try_collect)]
#![allow(dead_code,unused_variables)]
pub mod jspkt;
pub mod utils;
pub mod consts;

use anyhow::Context;
use js_sys::{Uint8Array };
use jspkt::Pkt;
use linkspace_pkt::{MIN_NETPKT_SIZE, PartialNetHeader ,*};
use utils::{*};
use wasm_bindgen::prelude::*;
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen(inline_js = "
    export function set_iter(obj) {
            obj[Symbol.iterator] = function () {
debugger
return this
};
        };
")]
extern "C" {
    fn set_iter(obj: &js_sys::Object);
}
#[wasm_bindgen(start)]
fn main() -> std::result::Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    set_iter(&js_sys::Object::get_prototype_of(&jspkt::Links::default().into()));

    Ok(())
}

#[wasm_bindgen(inline_js = "export function identity(a) { return a }")] 
#[wasm_bindgen]
extern "C"{
    pub type Fields;
    #[wasm_bindgen(structural, method,getter)]
    pub fn group(this: &Fields) -> JsValue; 
    #[wasm_bindgen(structural, method,getter)]
    pub fn domain(this: &Fields) -> JsValue; 
    #[wasm_bindgen(structural, method,getter)]
    pub fn path(this: &Fields) -> JsValue; 
    #[wasm_bindgen(structural, method,getter)]
    pub fn links(this: &Fields) -> JsValue; 
    #[wasm_bindgen(structural, method,getter)]
    pub fn stamp(this: &Fields) -> JsValue; 

    pub fn identity(val:JsValue) -> Option<jspkt::Link>;

}
fn common_args(obj:&Fields) -> Result<(GroupID, Domain, IPathBuf, Vec<Link>, Stamp)> {

    let group = opt_bytelike(&obj.group())?
        .map(|group| GroupID::try_fit_bytes_or_b64(&group))
        .transpose()?
        .unwrap_or(PUBLIC);
    let domain = opt_bytelike(&obj.domain())?
        .map(|domain| Domain::try_fit_byte_slice(&domain))
        .transpose()?
        .unwrap_or(AB::default()); 
    let path =  obj.path();
    let path = if path.is_falsy() {
        IPathBuf::new()
    }else if let Some(st) = path.dyn_ref::<Uint8Array>(){
        SPathBuf::try_from_inner(st.to_vec()).map_err(err)?.ipath()
    }else {
        let it = js_sys::try_iter(&path).map_err(js_err)?.context("unknown format - expected path bytes or array")?;
        let path = it.map(|i: Result<JsValue,JsValue>| -> Result<Vec<u8>,JsErr> {
            let b = i.map_err(js_err)?;
            bytelike(&b).map_err(err)
        }).try_collect::<Vec<_>>()?;
        IPathBuf::try_from_iter(path)?
    };
    let links = obj.links();
    let links = if links.is_falsy(){ vec![]} else {
        static ERR : &str = "unknown format - expected [Link] iterator";
        let it = js_sys::try_iter(&links).map_err(js_err)?.context(ERR)?;
        it.map(|link: Result<JsValue,JsValue>| -> Result<Link,JsErr> {
            let link : jspkt::Link = identity(link.map_err(js_err)?).context(ERR)?;
            Ok(link.0)
        }).try_collect::<Vec<_>>()?
    };
    let stamp = obj.stamp();
    let stamp = if stamp.is_falsy(){ now()} else {
        Stamp::try_from(&*stamp.dyn_ref::<Uint8Array>().context("expected Uint8Array for stamp")?.to_vec())?
    };
    return Ok((group,domain,path,links,stamp))
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
pub fn lk_key_decrypt(id: &str, password: &[u8]) -> Result<SigningKey> {
    Ok(SigningKey(identity::decrypt(id,password).map_err(err)?))
}
#[wasm_bindgen]
pub struct Linkspace(usize);

#[wasm_bindgen]
pub fn lk_datapoint(data: &JsValue) -> Result<Pkt> {
    let data = bytelike(data)?;
    Ok(jspkt::Pkt::from_dyn(
        &linkspace_pkt::try_datapoint_ref(&data,NetOpts::Default)?,
    ))
}
#[wasm_bindgen]
pub fn lk_linkpoint( data:&JsValue,fields : &Fields) -> Result<Pkt> {

    let data = bytelike(data)?;
    let (domain,group,path,links,create) = common_args(fields)?;
    let pkt = linkspace_pkt::try_linkpoint_ref(domain, group, &*path, &*links, &data, create,NetOpts::Default).map_err(err)?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
}
#[wasm_bindgen]
pub fn lk_keypoint(key: &SigningKey,data:&JsValue,fields: &Fields) -> Result<Pkt> {

    let data = bytelike(data)?;
    let (domain,group,path,links,create) = common_args(fields)?;
    let pkt = linkspace_pkt::try_keypoint_ref(domain, group, &*path, &*links, &data, create,&key.0,NetOpts::Default).map_err(err)?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
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

