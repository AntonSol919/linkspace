use js_sys::{JsString, JSON};
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct JsErr(Variant);
enum Variant {
    JsVal(JsValue),
    PktErr(linkspace_pkt::Error),
    KeyErr(linkspace_argon2_identity::KeyError),
}

pub fn ferr<E: Into<JsErr>>(error: E) -> JsErr {
    error.into()
}

impl From<linkspace_pkt::Error> for JsErr {
    fn from(value: linkspace_pkt::Error) -> Self {
        JsErr(Variant::PktErr(value))
    }
}

impl From<linkspace_pkt::space::SpaceError> for JsErr {
    fn from(value: linkspace_pkt::space::SpaceError) -> Self {
        JsErr(Variant::PktErr(value.into()))
    }
}
impl From<linkspace_argon2_identity::KeyError> for JsErr {
    fn from(value: linkspace_argon2_identity::KeyError) -> Self {
        JsErr(Variant::KeyErr(value))
    }
}

impl From<&str> for JsErr {
    fn from(value: &str) -> Self {
        JsErr(Variant::JsVal(JsValue::from_str(value)))
    }
}
impl From<JsValue> for JsErr {
    fn from(value: JsValue) -> Self {
        JsErr(Variant::JsVal(value))
    }
}

#[wasm_bindgen]
impl JsErr {
    #[wasm_bindgen(js_name = toJSON)]
    pub fn to_json(&self) -> JsValue {
        match &self.0 {
            Variant::JsVal(v) => v.into(),
            Variant::PktErr(e) => e.to_string().into(),
            Variant::KeyErr(e) => e.to_string().into(),
        }
    }
    #[wasm_bindgen(js_name = toString)]
    pub fn to_string(&self) -> Result<JsString, JsValue> {
        JSON::stringify(&self.to_json())
    }
}

pub type Result<T, E = JsErr> = std::result::Result<T, E>;
pub fn bytelike(obj: &JsValue) -> Result<Vec<u8>, JsValue> {
    if let Some(st) = obj.as_string() {
        return Ok(st.into_bytes());
    };
    let bytes = obj
        .dyn_ref::<js_sys::Uint8Array>()
        .ok_or("expected string or Uint8Array")?;
    Ok(bytes.to_vec()) // todo use copy_into
}
pub fn opt_bytelike(obj: &JsValue) -> Result<Option<Vec<u8>>, JsValue> {
    if obj.is_falsy() {
        return Ok(None);
    }
    bytelike(obj).map(Some)
}
