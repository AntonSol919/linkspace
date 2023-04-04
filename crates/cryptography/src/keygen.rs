// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
use crate::*;
pub use k256::schnorr::SigningKey as SigningKey_;
use rand_core::{CryptoRng, OsRng, RngCore};

impl SigningKey {
    pub fn generate() -> SigningKey {
        Self::generate_with(&mut OsRng)
    }
    pub fn generate_with<R: RngCore + CryptoRng>(rng: &mut R) -> SigningKey {
        SigningKey(SigningKey_::random(rng))
    }
}
#[test]
pub fn gen_testkey() {
    use rand::prelude::*;
    let mut gen = rand_chacha::ChaCha8Rng::seed_from_u64(5010);
    let bytes = k256::NonZeroScalar::random(&mut gen);
    let pub_test = SigningKey::try_from(bytes.to_bytes().as_slice().try_into().unwrap()).unwrap();
    assert_eq!(
        pub_test.0.to_bytes().as_slice(),
        crate::TEST_SECRET.as_slice(),
        "public"
    );
}
