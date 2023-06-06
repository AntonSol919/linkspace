#![feature(try_blocks,thread_local,lazy_cell,ptr_from_ref)]
pub mod jspkt;
pub mod utils;
pub mod consts;
use std::cell::Cell;

use js_sys::{Uint8Array };
use jspkt::Pkt;
use linkspace_pkt::{MIN_NETPKT_SIZE, PartialNetHeader };
use utils::{JsErr,*};
use wasm_bindgen::prelude::*;
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
fn main() -> std::result::Result<(), JsValue> {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
    Ok(())
}





#[thread_local]
pub static SCRATCH : Cell<[u8;MIN_NETPKT_SIZE]>= Cell::new([0;MIN_NETPKT_SIZE]);

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

