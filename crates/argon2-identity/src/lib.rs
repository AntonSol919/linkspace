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
mod params {
    pub struct Decoded {
        pub mem_cost: u32,
        pub time_cost: u32,
        pub parallelism: u32,
        pub salt: Vec<u8>,
        pub hash: Vec<u8>,
    }
    /// Attempts to decode the encoded string slice.
    pub fn decode_string(encoded: &str) -> Option<Decoded> {
        let items = encoded.strip_prefix("$argon2d$v=19$")?;
        let mut items_it = items.split("$");
        let options = items_it.next()?;
        let salt = base64::decode(items_it.next()?).ok()?;
        let hash = base64::decode(items_it.next()?).ok()?;
        let mut opt_it = options.split(",").filter_map(|st| st.split_once("="))
            .zip(["m","t","p"])
            .filter(|((kind,_val),expect)| (kind == expect))
            .filter_map(|((_,val),_)| val.parse::<u32>().ok());
        let (mem_cost,time_cost,parallelism) = (opt_it.next()?,opt_it.next()?,opt_it.next()?);
        if opt_it.next().is_some() { return None}
        Some(Decoded {
            mem_cost,
            time_cost,
            parallelism,
            salt,
            hash,
        })
    }
}


use argon2::Config;
use linkspace_cryptography::*;

pub use linkspace_cryptography::keygen;
use thiserror::Error;

pub const DEFAULT_MEM_COST: u32 = 16384;
pub const DEFAULT_TIME_COST: u32 = 4;

pub const DEFAULT_COST: (u32, u32) = (DEFAULT_MEM_COST, DEFAULT_TIME_COST);
pub const EXPENSIVE_COST: (u32, u32) = (DEFAULT_MEM_COST*2, DEFAULT_TIME_COST*2);
pub const INSECURE_COST: (u32, u32) = (8, 1);

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("The data supplied is in an unknown format")]
    BadData,
    #[error("The password is incorrect")]
    WrongPassword,
}

pub fn encrypt(key: &SigningKey, password: &[u8], cost: Option<(u32, u32)>) -> String {
    let (mem_cost, time_cost) = cost.unwrap_or(DEFAULT_COST);
    let config = Config {
        variant: argon2::Variant::Argon2d,
        mem_cost,
        time_cost,
        ..Config::default()
    };
    let encoded = argon2::hash_encoded(password, &key.pubkey_bytes(), &config).unwrap();
    // This is some dirty work to decode the internal argon representation so we can xor our secret
    let (conf,digest) = encoded.rsplit_once('$').unwrap();
    let digest = base64::decode(digest).unwrap();

    let xored = digest
        .into_iter()
        .zip(key.0.to_bytes().iter())
        .map(|(a, b)| a ^ b)
        .collect::<Vec<_>>();

    let encrypted_secret = base64::encode(xored);
    let result = String::from_iter([conf, "$", &*encrypted_secret]);
    if cfg!(debug_assertions) {
        decrypt(&result, password).unwrap();
    }
    result
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
    let config = Config {
        variant: argon2::Variant::Argon2d,
        version: argon2::Version::Version13,
        mem_cost: argon_param.mem_cost,
        time_cost: argon_param.time_cost,
        lanes: argon_param.parallelism,
        thread_mode: argon2::ThreadMode::Sequential,
        secret: &[],
        ad: &[],
        hash_length: 32,
    };

    let mut digest =
        argon2::hash_raw(password, &argon_param.salt, &config).map_err(|_| KeyError::BadData)?;

    digest
        .iter_mut()
        .zip(argon_param.hash)
        .for_each(|(a, b)| *a ^= b);
    let secret =
        SigningKey::try_from(digest.try_into().unwrap()).map_err(|_| KeyError::WrongPassword)?;

    let pubkey = secret.pubkey_bytes();
    if pubkey.as_ref() != argon_param.salt {
        return Err(KeyError::WrongPassword);
    }
    Ok(secret)
}


#[test]
pub fn test_encoding_decoding(){
    fn check_key(i:&str,pass:&[u8]){
        let key = decrypt(i, pass).unwrap();
        let pubkey = pubkey(i).unwrap();
        let from_key = key.pubkey_bytes();
        assert_eq!(pubkey,from_key);
        let st = encrypt(&key, pass, None);
        assert_eq!(i,st)
    }
    let empty = "$argon2d$v=19$m=16384,t=4,p=1$WXRa3NqtPwBpbyQPhEx1rCaMBKqiUDyZaecjdgIPLbY$EoCRQphpbp05CiLWEVncNE5zEqs4/K6KU2rrCtiSf0Y=";
    check_key(empty,b"");
    let hello = "$argon2d$v=19$m=16384,t=4,p=1$Au5RqvgqltiHj8ajyxwHrYBmaOudIx1XExjeM4zxS5I$mt8Q/Gq+jTM/4Ci9bW0P/4AJFWNuY5PWxzzsDyeBCb0=";
    check_key(hello,b"hello");
    let hello = "$argon2d$v=19$m=16384,t=4,p=1$Au5RqvgqltiHj8ajyxwHrYBmaOudIx1XExjeM4zxS5I$mt8Q/Gq+jTM/4Ci9bW0P/4AJFWNuY5PWxzzsDyeBCb0=";
    assert!(decrypt(hello,b"not hello").is_err());
}
