
use linkspace_pkt::{consts::* };
use wasm_bindgen::prelude::*;

#[derive(Copy,Clone)]
#[wasm_bindgen]
pub struct LkConsts;
#[wasm_bindgen]
pub fn get_consts() -> LkConsts{ LkConsts}

#[wasm_bindgen]
impl LkConsts {
    #[wasm_bindgen(getter,js_name="PUBLIC")]
    pub fn public_self(&self) -> Box<[u8]> { PUBLIC.0.into()}
}
