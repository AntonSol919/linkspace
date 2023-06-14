#![feature(try_blocks,thread_local,lazy_cell,ptr_from_ref,iterator_try_collect)]
pub mod jspkt;
pub mod utils;
pub mod consts;

use js_sys::{Uint8Array, Object};
use linkspace_pkt::{MIN_NETPKT_SIZE, PartialNetHeader ,*};
use utils::{*};
use wasm_bindgen::prelude::*;

use crate::jspkt::Pkt;
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


#[wasm_bindgen(start)]
fn main() -> std::result::Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    set_iter(&js_sys::Object::get_prototype_of(&jspkt::Links::default().into()));
    Ok(())
}


#[wasm_bindgen]
pub fn lk_datapoint(data: &JsValue) -> Result<Pkt> {
    let data = bytelike(data)?;
    Ok(Pkt(try_datapoint_ref(b"", NetOpts::Default)?.as_netarc()))
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
    Ok(SigningKey(identity::decrypt(id,password)?))
}

#[wasm_bindgen(inline_js = "
export function identity(a) { return a }
export function set_iter(obj) {
     obj[Symbol.iterator] = function () { return this };
};
export function pkt_obj(pkt){
     return {
group:pkt.group,
domain:pkt.domain,
path: pkt.path,
links:pkt.links,
create:pkt.create
};
};
")]
#[wasm_bindgen]
extern "C"{
    fn set_iter(obj: &js_sys::Object);
    pub fn pkt_obj(obj:Pkt) -> Object;

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
    pub fn create(this: &Fields) -> JsValue; 

    pub fn identity(val:JsValue) -> Option<jspkt::Link>;


}
fn common_args(obj:&Fields) -> Result<(GroupID, Domain, IPathBuf, Vec<Link>, Stamp)> {

    let group = opt_bytelike(&obj.group())?
        .map(|group| GroupID::try_fit_bytes_or_b64(&group).ok().ok_or("invalid group"))
        .transpose()?
        .unwrap_or(PUBLIC);
    let domain = opt_bytelike(&obj.domain())?
        .map(|domain| Domain::try_fit_byte_slice(&domain).ok().ok_or("invalid domain"))
        .transpose()?
        .unwrap_or(AB::default()); 
    let path =  obj.path();
    let path = if path.is_falsy() {
        IPathBuf::new()
    }else if let Some(st) = path.dyn_ref::<Uint8Array>(){
        SPathBuf::try_from_inner(st.to_vec())?.ipath()
    }else {
        let it = js_sys::try_iter(&path)?.ok_or("unknown format - expected path bytes or array")?;
        let path = it.map(|b|bytelike(&b?)).try_collect::<Vec<_>>()?;
        IPathBuf::try_from_iter(path)?
    };
    let links = obj.links();

    let links = if links.is_falsy(){ vec![]} else {
        static ERR :&str= "expected [Link] iter";
        let it = js_sys::try_iter(&links)?.ok_or(ERR)?;
        it.map(|link| -> Result<_> {Ok(identity(link?).ok_or(ERR)?.0)}).try_collect::<Vec<_>>()?
    };
    let create_stamp = obj.create();
    let create_stamp = if create_stamp.is_falsy(){
        now()
    } else {
        static ERR : &str = "expected stamp [u8;8] or falsy";
        Stamp::try_from(&*create_stamp.dyn_ref::<Uint8Array>().ok_or(ERR)?.to_vec()).ok().ok_or(ERR)?
    };
    Ok((group,domain,path,links,create_stamp))
}

#[wasm_bindgen]
pub fn lk_linkpoint( data:&JsValue,fields : &Fields) -> Result<Pkt> {

    let data = bytelike(data)?;
    let (domain,group,path,links,create) = common_args(fields)?;
    let pkt = linkspace_pkt::try_linkpoint_ref(domain, group, &path, &links, &data, create,NetOpts::Default)?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
}

#[wasm_bindgen]
pub fn lk_keypoint(key: &SigningKey,data:&JsValue,fields: &Fields) -> Result<Pkt> {

    let data = bytelike(data)?;
    let (domain,group,path,links,create) = common_args(fields)?;
    let pkt = linkspace_pkt::try_keypoint_ref(domain, group, &path, &links, &data, create,&key.0,NetOpts::Default)?;
    Ok(jspkt::Pkt::from_dyn(&pkt))
}

#[wasm_bindgen]
pub fn lk_write( pkt: &Pkt,allow_private:Option<bool>) -> Result<Uint8Array,JsErr> {
    if allow_private != Some(true) {pkt.0.check_private()?;}
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
pub fn lk_read(bytes: &Uint8Array,allow_private: Option<bool>) ->Result<js_sys::Array,JsErr>{
    _read(bytes,allow_private.unwrap_or(false),false)
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
pub fn lk_read_unchecked(bytes: &Uint8Array,allow_private: Option<bool>) ->Result<js_sys::Array,JsErr>{
    _read(bytes,allow_private.unwrap_or(false),true)
}

pub fn _read(bytes: &Uint8Array,allow_private: bool, skip_hash:bool) ->Result<js_sys::Array,JsErr>{
    let bufsize = bytes.length() as usize ;
    if bufsize < MIN_NETPKT_SIZE {
        return Err(JsValue::from(MIN_NETPKT_SIZE).into());
    }
    let mut partial = PartialNetHeader::EMPTY;
    use std::ptr;
    unsafe { bytes.slice(0,MIN_NETPKT_SIZE as u32 ).raw_copy_to_ptr( ptr::from_mut(&mut partial) as *mut u8)};
    partial.point_header.check()?;

    let pktsize = partial.point_header.net_pkt_size() as usize ;
    if pktsize > bufsize { return Err(JsValue::from(pktsize).into());};
    let pktsize = pktsize as u32;

    let pkt = unsafe{
        linkspace_pkt::NetPktArc::from_header_and_copy(
            partial,
            skip_hash,
            |dest|{
                bytes.slice(std::mem::size_of::<PartialNetHeader>() as u32,pktsize).raw_copy_to_ptr(dest.as_mut_ptr())
            }
        )};
    let pkt = pkt?;
    if !allow_private{ pkt.check_private()?};
    Ok(js_sys::Array::of2(&Pkt(pkt).into(),&bytes.slice(pktsize,bufsize as u32)))
}

#[wasm_bindgen]
pub fn b64(bytes:&[u8], mini:Option<bool>) -> String{
    let b = linkspace_pkt::B64(bytes);
    if mini.unwrap_or(false){b.b64_mini()} else{b.to_string()}
}
#[wasm_bindgen]
pub fn blake3_hash(bytes:&[u8]) -> Box<[u8]>{
    blake3::hash(bytes).as_bytes().clone().into()
}
