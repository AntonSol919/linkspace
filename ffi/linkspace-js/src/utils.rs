use js_sys::{JsString, JSON};
use wasm_bindgen::prelude::*;
pub type Result<T, E = JsError> = std::result::Result<T, E>;
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

impl From<String> for JsErr {
    fn from(value: String) -> Self {
        JsErr(Variant::JsVal(JsValue::from_str(&value)))
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
impl std::error::Error for JsErr {}
impl std::fmt::Debug for JsErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}
impl std::fmt::Display for JsErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Variant::JsVal(v) => match v.as_string() {
                Some(s) => write!(f, "{s}"),
                None => write!(f, "{v:?}"),
            },
            Variant::PktErr(e) => write!(f, "{e}"),
            Variant::KeyErr(e) => write!(f, "{e}"),
        }
    }
}

use smallvec::SmallVec;
pub type Bytes = SmallVec<[u8; 16]>;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

pub fn bytelike(obj: &JsValue) -> Result<Bytes, JsErr> {
    if let Some(st) = obj.as_string() {
        return Ok(st.into_bytes().into());
    };
    let bytes = obj
        .dyn_ref::<js_sys::Uint8Array>()
        .ok_or("expected string or Uint8Array")?;
    let mut result = SmallVec::from_elem(0, bytes.length() as usize);
    bytes.copy_to(&mut result);
    Ok(result)
}
pub fn opt_bytelike(obj: &JsValue) -> Result<Option<Bytes>, JsErr> {
    if obj.is_falsy() {
        return Ok(None);
    }
    bytelike(obj).map(Some)
}
