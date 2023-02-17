// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
/**
Encrypt public and private key with argon2.
Uses public key as salt, xors private key with the encoded hash.
**/


use argon2::Config;
use linkspace_crypto::*;

pub use linkspace_crypto::keygen;
use thiserror::Error;

pub const DEFAULT_MEM_COST: u32 = 16384;
pub const DEFAULT_TIME_COST: u32 = 4;

pub const DEFAULT_COST: (u32, u32) = (DEFAULT_MEM_COST, DEFAULT_TIME_COST);
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
    let argon_param = argon2::decode_string(identity).map_err(|_| KeyError::BadData)?;
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
    let argon_param = argon2::decode_string(enckey).map_err(|_| KeyError::BadData)?;
    let config = Config {
        variant: argon_param.variant,
        version: argon_param.version,
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
