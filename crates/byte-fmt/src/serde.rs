// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

impl<N: AsRef<[u8]> + Serialize> Serialize for AB<N>
where
    Self: fmt::Display,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            self.to_string().serialize(serializer)
        } else {
            self.0.as_ref().serialize(serializer)
        }
    }
}

impl<'de, const N: usize> Deserialize<'de> for AB<[u8; N]> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ABTXTVisitor<const N: usize>(PhantomData<[u8; N]>);
        impl<'de, const N: usize> Visitor<'de> for ABTXTVisitor<N> {
            type Value = AB<[u8; N]>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Expecting str or bytes")
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse()
                    .map_err(|e: ABTxtError| Error::custom(e.to_string()))
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut r = [0; N];
                let mut i = 0usize;
                while let Some(v) = seq.next_element()? {
                    if i >= N {
                        return Err(Error::invalid_length(N + 1, &"ToMuchData"));
                    }
                    r[i] = v;
                    i += 1;
                }
                if seq.next_element::<u8>()?.is_some() {
                    return Err(Error::invalid_length(N + 1, &"ToMuchData"));
                }
                Ok(AB::try_fit_byte_slice(&r[0..i.saturating_sub(1)]).unwrap())
            }
            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if value.len() > N {
                    return Err(Error::invalid_length(value.len(), &"wrong n of bytes"));
                }

                Ok(AB::try_fit_byte_slice(value).unwrap())
            }
        }
        deserializer.deserialize_any(ABTXTVisitor(PhantomData))
    }
}

impl<'de> Deserialize<'de> for AB<Vec<u8>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ABTXTVisitor;
        impl<'de> Visitor<'de> for ABTXTVisitor {
            type Value = AB<Vec<u8>>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Expecting str or bytes")
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                parse_abtxt_upto_max(value, u16::MAX as usize)
                    .map(AB)
                    .map_err(|e: ABTxtError| Error::custom(e.to_string()))
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut r: Vec<u8> = vec![];
                while let Some(next) = seq.next_element()? {
                    r.push(next);
                }
                Ok(AB(r))
            }
            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(AB(value.to_vec()))
            }
        }
        deserializer.deserialize_any(ABTXTVisitor)
    }
}

impl<N: AsRef<[u8]> + Serialize> Serialize for B64<N> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            base64(self.0.as_ref()).serialize(serializer)
        } else {
            self.0.serialize(serializer)
        }
    }
}
impl<'de, const N: usize> Deserialize<'de> for B64<[u8; N]> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct B64Visitor<const N: usize>(PhantomData<[u8; N]>);
        impl<'de, const N: usize> Visitor<'de> for B64Visitor<N> {
            type Value = B64<[u8; N]>;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Expecting str or bytes")
            }
            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse()
                    .map_err(|e: base64::DecodeError| Error::custom(e.to_string()))
            }
            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut r = [0; N];
                for d in r.iter_mut() {
                    *d = seq
                        .next_element()?
                        .ok_or_else(|| Error::invalid_length(0, &"To little data"))?;
                }
                if seq.next_element::<u8>()?.is_some() {
                    return Err(Error::invalid_length(N + 1, &"ToMuchData"));
                }
                Ok(B64(r))
            }
            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .try_into()
                    .map_err(|_e| Error::invalid_length(value.len(), &"wrong n of bytes"))
                    .map(B64)
            }
        }
        deserializer.deserialize_any(B64Visitor(PhantomData))

        /*
        if deserializer.is_human_readable() {
            let st = String::deserialize(deserializer)?;
            Ok(st
                .parse()
                .map_err(|e: base64::DecodeError| serde::de::Error::custom(e.to_string()))?)
        } else {
            let r = <&[u8]>::deserialize(deserializer)?;
            Ok(B64(r.try_into().map_err(|_e| {
                serde::de::Error::invalid_length(r.len(), &"wrong n of bytes")
            })?))
        }
            */
    }
}

/*
pub mod with_abtxt{
    use super::*;
    pub fn serialize<S:serde::Serializer,const N:usize>(v:&[u8;N], serializer:S) -> Result<S::Ok,S::Error>{
        serialize(&ABTXT(v as &[u8]),serializer)
    }
    pub fn deserialize<'de,D:serde::Deserializer<'de>,const N:usize>(deserializer: D) -> Result<[u8;N], D::Error>{
        <[u8;N]>::deserialize(deserializer).map(|ABTXT(v)| v)
    }
}
*/
pub mod with_b64 {
    use super::*;
    pub fn serialize<S: serde::Serializer, const N: usize>(
        v: &[u8; N],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        B64::serialize(&B64(v as &[u8]), serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>, const N: usize>(
        deserializer: D,
    ) -> Result<[u8; N], D::Error> {
        B64::<[u8; N]>::deserialize(deserializer).map(|B64(v)| v)
    }
    pub mod boxed {
        #[allow(clippy::borrowed_box)]
        pub fn serialize<S: serde::Serializer, const N: usize>(
            v: &Box<[u8; N]>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            super::serialize(&*v, serializer)
        }
        pub fn deserialize<'de, D: serde::Deserializer<'de>, const N: usize>(
            deserializer: D,
        ) -> Result<Box<[u8; N]>, D::Error> {
            super::deserialize(deserializer).map(Box::new)
        }
    }
}

/*
#[test]
fn _fallback(){
    let x = B64([0;32]);
    let st = serde_json::to_string(&x).unwrap();
    let v1 : B64<[u8;32]> = serde_json::from_str(&st).unwrap();
    let alt = [0u8;32];
    let st = serde_json::to_string(&alt).unwrap();
    let v2 : B64<[u8;32]> = serde_json::from_str(&st).unwrap();
    assert_eq!(v1,v2);


    let x = ABTXT([0;32]);
    let st = serde_json::to_string(&x).unwrap();
    let v1 : ABTXT<[u8;32]> = serde_json::from_str(&st).unwrap();
    let alt = [0u8;32];
    let st = serde_json::to_string(&alt).unwrap();
    let v2 : ABTXT<[u8;32]> = serde_json::from_str(&st).unwrap();
    assert_eq!(v1,v2)
}
*/

