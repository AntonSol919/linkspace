// Copyright Anton Sol
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use anyhow::{ ensure };
use bitvec::prelude::*;
use linkspace_pkt::abe::ast::{ is_colon, take_expr_ctr2, single, as_expr, Ctr};
use linkspace_pkt::abe::eval::ABList;
use linkspace_pkt::{ B64, U16, U32};

pub type Bits = bitvec::BitArr!(for LkBloom::BIT_SIZE, in u64, Msb0);
/**
We're abusing the fact that:
- The hash is random
- We'll probably set a common implied limit to the max guaranteed results.

This isn't perfect for every case.
**/
pub struct LkBloom{
    pub item_count:u16,
    pub seed:u32,
    pub bit: Bits
}

impl LkBloom {

    pub const HASHES:u16 = 23;
    pub const BIT_SIZE:usize = 4096*8;
    pub fn new(seed: u32) -> Self {
        Self {
            bit: BitArray::ZERO,
            item_count:0,
            seed
        }
    }

    pub fn from(
        item_count:u16,
        seed:u32,
        bit: Bits,
    ) -> Self {
        Self {
            item_count,
            bit,
            seed
        }
    }

    pub fn set(&mut self, item: &[u8;32])
    {
        for i in 0u16..Self::HASHES{
            let bhash = self.bloom_hash(self.seed.wrapping_add(i as u32),item);
            let offset = bhash % Self::BIT_SIZE as u64;
            self.bit.set(offset as usize, true);
        }
        self.item_count +=1;
    }

    pub fn check(&self, item: &[u8;32]) -> bool{
        for i in 0u16..Self::HASHES{
            let bhash = self.bloom_hash(self.seed.wrapping_add(i as u32),item);
            let offset = bhash % Self::BIT_SIZE as u64;
            if self.bit.get(offset as usize).unwrap() == false{ return false }
        }
        true
    }


    pub fn bloom_hash(&self, hash_id: u32, item: &[u8;32]) -> u64{
        let idx = hash_id % 4;
        let xor = (hash_id as u64).to_le_bytes();
        let val : [u8;8] = item[idx as usize *8..][..8].try_into().unwrap();
        u64::from_be_bytes(val) ^ u64::from_be_bytes(xor)
    }
    
}
use linkspace_pkt::abe::{ToABE, abev, ABEValidator};
impl ToABE for LkBloom {
    fn to_abe(&self) -> Vec<linkspace_pkt::abe::ABE> {
        let v : [u64;512]= self.bit.into_inner();
        let v : [u8;4096]= unsafe {*( v.as_ptr() as *const [u8;4096])};
        abev!( +(U16::new(self.item_count).to_abe()) :+(U32::new(self.seed).to_abe()) : +(B64(v).to_abe()))
    }
}
impl ABEValidator  for LkBloom{
    fn check(b: &[linkspace_pkt::abe::ABE]) -> Result<(), linkspace_pkt::abe::ast::MatchError> {
        let (_items,b) = take_expr_ctr2(b, is_colon)?;
        let (_seed,b) = take_expr_ctr2(b, is_colon)?;
        as_expr(single(b)?)?;
        Ok(())
    }
}
impl TryFrom<ABList> for LkBloom{
    type Error = anyhow::Error;
    fn try_from(value: ABList) -> Result<Self, Self::Error> {
        let lst = value.inner();
        if lst.len() != 3 { anyhow::bail!("expected 3 items")}
        ensure!(matches!(lst[0].1,Some(Ctr::Colon)), "expected :");
        ensure!(matches!(lst[1].1,Some(Ctr::Colon)), "expected :");

        let item_count = U16::try_from(&*lst[0].0)?.get();
        let seed = U32::try_from(&*lst[1].0)?.get();
        ensure!(lst[2].0.len() == 4096, "wrong number of bloom bits");
        let mut bits = [0u8;4096];
        bits.copy_from_slice(&lst[2].0);
        let bits : [u64;512]= unsafe {*( bits.as_ptr() as *const [u64;512])};
        let bit = Bits::from(bits);
        Ok(LkBloom { item_count, seed, bit })
    }
}
