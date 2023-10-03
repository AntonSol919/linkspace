// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
Encrypt public and private key with argon2.
Uses public key as salt, xors private key with the encoded hash.
**/
// This is not ideal but both the package argon2 and rust-argon2 are hiding implementation details. 
#[derive(Copy,Clone,PartialEq)]
pub struct Costs{
    pub mem:u32,
    pub time:u32,
    pub parallelism:u32
}

pub const DEFAULT_COST: Costs = Costs{
    mem:  1 << 26, // 64mb
    time: 4,
    parallelism: 4,
};
pub const EXPENSIVE_COST: Costs = Costs{
    mem:  1 << 27,
    time: 8,
    parallelism: 8,
};
pub const INSECURE_COST: Costs = Costs{
    mem:  8,
    time: 1,
    parallelism: 1,
};


impl Default for Costs{
    fn default() -> Self {
        DEFAULT_COST
    }
}
mod params {
    pub struct Decoded {
        pub costs:crate::Costs,
        pub salt: Vec<u8>,
        pub hash: [u8;32],
    }
    /// Attempts to decode the encoded string slice.
    pub fn decode_string(encoded: &str) -> Option<Decoded> {
        use base64::prelude::*;
        let items = encoded.strip_prefix("$argon2d$v=19$")?;
        let mut items_it = items.split('$');
        let options = items_it.next()?;
        let salt = BASE64_STANDARD_NO_PAD.decode(items_it.next()?).ok()?;
        let hash = BASE64_STANDARD_NO_PAD.decode(items_it.next()?).ok()?.try_into().ok()?;
        let mut opt_it = options.split(',').filter_map(|st| st.split_once('='))
            .zip(["m","t","p"])
            .filter(|((kind,_val),expect)| (kind == expect))
            .filter_map(|((_,val),_)| val.parse::<u32>().ok());
        let (mem,time,parallelism) = (opt_it.next()?,opt_it.next()?,opt_it.next()?);
        if opt_it.next().is_some() { return None}
        Some(Decoded {
            costs: crate::Costs { mem, time, parallelism},
            salt,
            hash,
        })
    }
}


use argon2::Config;
use linkspace_cryptography::{* };

pub use linkspace_cryptography::keygen;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("The data supplied is in an unknown format")]
    BadData,
    #[error("The password is incorrect")]
    WrongPassword,
}

pub fn encrypt(key: &SigningKey, password: &[u8], cost:Option<Costs>) -> String{
    _encrypt(key, password, cost.unwrap_or(DEFAULT_COST)).expect("bug - key encryption errror?")
}
fn _encrypt(key: &SigningKey, password: &[u8], cost: Costs) -> Option<String>{
    use base64::prelude::*;
    let Costs { mem, time, parallelism }=cost;
    let config = Config {
        variant: argon2::Variant::Argon2d,
        mem_cost:mem,
        time_cost:time,
        lanes:parallelism,
        ..Config::default()
    };
    let encoded = argon2::hash_encoded(password, &key.pubkey_bytes(), &config).ok()?;
    // This is some dirty work to decode the internal argon representation so we can xor our secret
    let (conf,digest) = encoded.rsplit_once('$')?;
    let digest = BASE64_STANDARD_NO_PAD.decode(digest).ok()?;
    
    let xored = digest
        .into_iter()
        .zip(key.0.to_bytes().iter())
        .map(|(a, b)| a ^ b)
        .collect::<Vec<_>>();

    let encrypted_secret = BASE64_STANDARD_NO_PAD.encode(xored);
    let result = String::from_iter([conf, "$", &*encrypted_secret]);
    if cfg!(debug_assertions) {
        decrypt(&result, password).expect("encrypt/decrypt bug!");
    }
    Some(result)
}
pub fn pubkey(identity: &str) -> Result<PublicKey, KeyError> {
    let argon_param = params::decode_string(identity).ok_or(KeyError::BadData)?;
    if argon_param.salt.len() != std::mem::size_of::<PublicKey>() {
        return Err(KeyError::BadData);
    }

    if argon_param.hash.len() != std::mem::size_of::<k256::FieldBytes>() {
        return Err(KeyError::BadData);
    }
    let mut pubkey = [0; 32];
    pubkey.copy_from_slice(&argon_param.salt);
    Ok(pubkey)
}

pub fn decrypt(enckey: &str, password: &[u8]) -> Result<SigningKey, KeyError> {
    let argon_param = params::decode_string(enckey).ok_or(KeyError::BadData)?;
    let Costs { mem, time, parallelism } = argon_param.costs;
    let config = Config {
        variant: argon2::Variant::Argon2d,
        version: argon2::Version::Version13,
        mem_cost: mem,
        time_cost: time,
        lanes: parallelism,
        secret: &[],
        ad: &[],
        hash_length: 32,
    };

    let digest = argon2::hash_raw(password, &argon_param.salt, &config).map_err(|_| KeyError::BadData)?;
    let digest : [u8;32] = digest.try_into().map_err(|_| KeyError::BadData)?;
    let secret = std::array::from_fn(|i| digest[i] ^ argon_param.hash[i]);
    let secret = SigningKey::try_from(secret).map_err(|_| KeyError::WrongPassword)?;

    let pubkey = secret.pubkey_bytes();
    if pubkey.as_ref() != argon_param.salt {
        return Err(KeyError::WrongPassword);
    }
    Ok(secret)
}


#[test]
pub fn test_encoding_decoding(){
    fn check_key(i:&str,pass:&[u8]){
        println!("Checking {i}");
        let key = decrypt(i, pass).unwrap();
        let pubkey = pubkey(i).unwrap();
        let from_key = key.pubkey_bytes();
        assert_eq!(pubkey,from_key);
        let st = encrypt(&key, pass, None);
        assert_eq!(i,st)
    }
    let key = SigningKey::generate();
    let e = encrypt(&key, b"hello", None);
    check_key(&e, b"hello");

    assert!(decrypt(&e,b"not hello").is_err());
}

pub static TEST_KEY_ID : &str = "$argon2d$v=19$m=8,t=1,p=1$tb0anwpH0rSbYe6JLd1Bgtf00QQUAYuhOcBqeSjAgW4$kYAtGyF78cfPjRqcm4Y/s1hgQTRysELK/L910P2u27c";

#[test]
pub fn test_key_encoding(){
    let key = public_testkey();
    let e = encrypt(&key,b"",Some(INSECURE_COST));
    assert_eq!(e,TEST_KEY_ID);
}
