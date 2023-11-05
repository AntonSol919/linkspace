use linkspace_pkt::consts::*;
use wasm_bindgen::prelude::*;

#[derive(Copy, Clone)]
#[wasm_bindgen(js_name="CONSTS")]
pub struct LkConsts;

#[wasm_bindgen(js_class="CONSTS")]
impl LkConsts {

    #[wasm_bindgen(getter, js_name = "PUBLIC")]
    pub fn public_static() -> Box<[u8]> {
        PUBLIC.0.into()
    }
    #[wasm_bindgen(getter, js_name = "PRIVATE")]
    pub fn private_static() -> Box<[u8]> {
        PRIVATE.0.into()
    }
}
