use wasm_bindgen::prelude::*;
use anyhow::Context;
use js_sys::{Array, JsString, JSON};
#[wasm_bindgen]
pub struct JsErr(pub (crate) anyhow::Error);
pub fn err<E:Into<anyhow::Error>>(error:E) -> JsErr{JsErr(error.into())}
impl<E> From<E> for JsErr
where
    E: Into<anyhow::Error>,
{
    fn from(error: E) -> Self {
        JsErr(error.into())
    }
}

#[wasm_bindgen]
impl JsErr {
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue{
        self.0.chain().map(|e| JsValue::from(format!("{:?}",e))).collect::<Array>().into()
    }
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> JsString {
        JSON::stringify(&self.to_json()).unwrap()
    }
}

pub type Result<T,E=JsErr> = std::result::Result<T,E>;
pub fn bytelike(obj:&JsValue) -> anyhow::Result<Vec<u8>>{
    if let Some(st) = obj.as_string(){ return Ok(st.into_bytes())};
    let bytes  = obj.dyn_ref::<js_sys::Uint8Array>().context("expected String or Uint8Array")?;
    Ok(bytes.to_vec()) // todo use copy_into 
}
