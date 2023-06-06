use std::sync::LazyLock;

use linkspace_pkt::{consts::*, NetPkt};
use wasm_bindgen::prelude::*;

use crate::jspkt::Pkt;


#[derive(Copy,Clone)]
#[wasm_bindgen]
pub struct Constants(pub usize);
#[wasm_bindgen]
pub fn get_consts() -> Constants{ Constants(0)}

#[wasm_bindgen]
impl Constants {
    #[wasm_bindgen(getter,js_name="PUBLIC")]
    pub fn public_self(&self) -> Box<[u8]> { PUBLIC.0.into()}
}



pub static PUBLIC_GROUP_PKT: LazyLock<Pkt> = LazyLock::new(|| Pkt(linkspace_pkt::datapoint(b"Hello, Sol", linkspace_pkt::NetPktHeader::EMPTY).as_netarc()));
