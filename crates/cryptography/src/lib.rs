// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use std::fmt::Debug;

pub mod keygen;

pub use k256;
pub use k256::ecdsa::Error;
pub use k256::schnorr;

#[derive(Clone)]
pub struct SigningKey(pub schnorr::SigningKey);
impl SigningKey {
    pub fn try_from(bytes: [u8; 32]) -> Result<SigningKey, k256::ecdsa::Error> {
        schnorr::SigningKey::from_bytes(&bytes).map(SigningKey)
    }
    pub fn pubkey_bytes(&self) -> PublicKey {
        self.0.verifying_key().to_bytes().into()
    }
}

impl Debug for SigningKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningKey")
            .field("secret", &"...")
            .field("pubkey", &self.0.verifying_key())
            .finish()
    }
}

pub type Hash = [u8; 32];
pub type PublicKey = [u8; 32];
pub type Signature = [u8; 64];

pub use ops::*;
#[cfg(not(miri))]
pub mod ops {
    use super::*;
    pub fn sign_hash(key: &SigningKey, hash: &Hash) -> Signature {
        *key.0
            .try_sign_prehashed(hash, &Default::default())
            .unwrap()
            .as_bytes()
    }
    pub fn validate_signature(
        pubkey: &PublicKey,
        signature: &Signature,
        hash: &Hash,
    ) -> Result<(), k256::ecdsa::Error> {
        schnorr::VerifyingKey::from_bytes(pubkey)?
            .verify_prehashed(hash, &schnorr::Signature::try_from(signature.as_slice())?)
    }
    pub fn hash_segments(s: &[&[u8]]) -> Hash {
        let mut hasher = blake3::Hasher::new();
        for segm in s {
            hasher.update(segm);
        }
        let hash = hasher.finalize();
        *hash.as_bytes()
    }
}
#[cfg(miri)]
pub mod ops {
    use super::*;
    pub fn sign_hash(_key: &SigningKey, _hash: &Hash) -> Signature {
        [0; 64]
    }
    pub fn validate_signature(
        _pubkey: &PublicKey,
        _signature: &Signature,
        _hash: &Hash,
    ) -> Result<(), k256::ecdsa::Error> {
        Ok(())
    }
    pub fn hash_segments(_s: &[&[u8]]) -> Hash {
        [0; 32]
    }
}

pub use blake3::hash as blake3_hash;

pub use blake3;

pub fn public_testkey() -> SigningKey {
    SigningKey::try_from(TEST_SECRET).unwrap()
}
pub const TEST_SECRET: [u8; 32] = [
    146, 62, 166, 250, 192, 186, 32, 182, 23, 73, 248, 235, 182, 88, 57, 163, 131, 178, 223, 72,
    160, 198, 24, 228, 68, 32, 214, 155, 238, 251, 18, 124,
];
