#![feature(try_blocks,thread_local,lazy_cell,ptr_from_ref)]
pub mod jspkt;
pub mod utils;
use std::cell::Cell;

use js_sys::Uint8Array;
use jspkt::Pkt;
use linkspace_pkt::{PUBLIC, MIN_NETPKT_SIZE, PartialNetHeader, NetPktBox };
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

#[wasm_bindgen]
pub fn test_input(obj:&JsValue) ->Result<Vec<u8>>{
    web_sys::console::log_1(&obj);
    bytelike(obj).map_err(JsErr)
}
#[wasm_bindgen]
pub fn test_round(obj:&JsValue) ->Result<String>{
    let obj = bytelike(obj).map_err(JsErr)?;
    let st = String::from_utf8(obj).map_err(err)?;
    Ok(st)
}


#[wasm_bindgen(js_name = "PUBLIC")]
pub fn public_hash() -> Box<[u8]>{
    PUBLIC.0.into()
}


#[thread_local]
pub static SCRATCH : Cell<[u8;MIN_NETPKT_SIZE]>= Cell::new([0;MIN_NETPKT_SIZE]);

#[wasm_bindgen]
pub fn b64(bytes:&[u8], mini:Option<bool>) -> String{
    let b = linkspace_pkt::B64(bytes);
    if mini.unwrap_or(false){b.b64_mini()} else{b.to_string()}
}
#[wasm_bindgen]
pub fn lk_read(bytes: &Uint8Array,validate: Option<bool>) ->Result<Pkt,JsValue>{
    use linkspace_pkt::NetPktFatPtr;
    let bufsize = bytes.length() as usize;
    if bufsize < MIN_NETPKT_SIZE {
        return Err(MIN_NETPKT_SIZE.into());
    }
    let mut partial = PartialNetHeader::EMPTY;
    use std::ptr;
    unsafe { bytes.slice(0,MIN_NETPKT_SIZE as u32 ).raw_copy_to_ptr( ptr::from_mut(&mut partial) as *mut u8)};
    partial.point_header.check().map_err(err)?;

    let pktsize = partial.point_header.net_pkt_size();
    if pktsize > bufsize { return Err(pktsize.into());};

    let mut pkt : NetPktBox = unsafe { partial.alloc() };
    {
        let s: &mut [u8] = unsafe {
            std::slice::from_raw_parts_mut((&mut *pkt) as *mut NetPktFatPtr as *mut u8, pktsize)
        };
        unsafe {bytes.slice(0,pktsize as u32).raw_copy_to_ptr(s.as_mut_ptr())}
    };
    if validate.unwrap_or(true) {
        pkt.check::<true>().map_err(err)?
    } else {
        pkt.check::<false>().map_err(err)?
    };
    Ok(Pkt(pkt))
}
