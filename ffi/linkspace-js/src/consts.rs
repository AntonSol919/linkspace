use linkspace_pkt::consts::*;
use wasm_bindgen::prelude::*;


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

