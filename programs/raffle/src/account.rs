use anchor_lang::prelude::*;
use std::clone::Clone;
use std::result::Result;

use crate::constants::*;
use crate::error::*;

#[account]
#[derive(Default)]
pub struct GlobalPool {
    pub super_admin: Pubkey, // 32
}

pub const DISCRIMINATOR_LENGTH: usize = 8;
pub const PUBLIC_KEY_LENGTH: usize = 32;

impl GlobalPool {
    pub const LEN: usize = DISCRIMINATOR_LENGTH + PUBLIC_KEY_LENGTH;
}

#[account(zero_copy)]
pub struct CollectionPool {
    // 32*400+8+8 = 12816
    pub count: u64,                            // 8
    pub collections: [Pubkey; MAX_COLLECTION], //32*400
}

#[account(zero_copy)]
pub struct RafflePool {
    // 32*2000+8*9 +96 = 64168
    pub creator: Pubkey,                  //32
    pub nft_mint: Pubkey,                 //32
    pub count: u64,                       //8
    pub no_repeat: u64,                   //8
    pub max_entrants: u64,                //8
    pub start_timestamp: i64,             //8
    pub end_timestamp: i64,               //8
    pub ticket_price_sol: u64,            //8
    pub claimed: u64,                     //8
    pub winner_index: u64,                //8
    pub winner: Pubkey,                   //32
    pub entrants: [Pubkey; MAX_ENTRANTS], //32*2000
}

impl Default for RafflePool {
    #[inline]
    fn default() -> RafflePool {
        RafflePool {
            creator: Pubkey::default(),
            nft_mint: Pubkey::default(),
            count: 0,
            no_repeat: 0,
            max_entrants: 0,
            start_timestamp: 0,
            end_timestamp: 0,
            ticket_price_sol: 0,
            claimed: 0,
            winner_index: 0,
            winner: Pubkey::default(),
            entrants: [Pubkey::default(); MAX_ENTRANTS],
        }
    }
}

impl Default for CollectionPool {
    #[inline]
    fn default() -> CollectionPool {
        CollectionPool {
            count: 0,
            collections: [Pubkey::default(); MAX_COLLECTION],
        }
    }
}
impl RafflePool {
    pub fn append(&mut self, buyer: Pubkey) {
        self.entrants[self.count as usize] = buyer;
        self.count += 1;
    }
}
impl CollectionPool {
    pub fn append(&mut self, collection: Pubkey) {
        let mut valid: u8 = 0;
        for i in 0..self.count {
            if self.collections[i as usize] == collection {
                valid = 1;
                break;
            }
        }
        if valid == 0 {
            self.collections[self.count as usize] = collection;
            self.count += 1;
        }
    }
}
