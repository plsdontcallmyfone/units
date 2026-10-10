// Changed by Hookwars: new file (hook economy, 11 section 6).
use anchor_lang::prelude::*;

#[event]
pub struct BookParamsProposed {
    pub ready_at: i64,
}

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub base_mint: Pubkey,
    pub tick_lamports: u64,
    pub min_size: u64,
    pub created_by: Pubkey,
    pub ts: i64,
}

#[event]
pub struct Placed {
    pub market: Pubkey,
    pub id: u64,
    pub owner: Pubkey,
    pub side: u8,
    pub price: u64,
    /// Size asked for, and size left resting after the matches.
    pub size: u64,
    pub resting: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct Filled {
    pub market: Pubkey,
    pub maker_order: u64,
    pub maker: Pubkey,
    pub taker: Pubkey,
    /// Side of the taker.
    pub side: u8,
    pub price: u64,
    pub size: u64,
    pub taker_fee: u64,
    pub maker_fee: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct Cancelled {
    pub market: Pubkey,
    pub id: u64,
    pub owner: Pubkey,
    pub ts: i64,
}

#[event]
pub struct Evicted {
    pub market: Pubkey,
    pub id: u64,
    pub owner: Pubkey,
    pub ts: i64,
}

#[event]
pub struct Expired {
    pub market: Pubkey,
    pub id: u64,
    pub owner: Pubkey,
    pub cranker: Pubkey,
    pub bounty: u64,
    pub ts: i64,
}

#[event]
pub struct ClassBidPlaced {
    pub bid: Pubkey,
    pub bidder: Pubkey,
    pub template_id: u16,
    pub min_level: u8,
    pub price: u64,
    pub expires_at: i64,
    pub ts: i64,
}

#[event]
pub struct ClassBidCancelled {
    pub bid: Pubkey,
    pub ts: i64,
}

#[event]
pub struct ClassFilled {
    pub bid: Pubkey,
    pub item: Pubkey,
    pub seller: Pubkey,
    pub bidder: Pubkey,
    pub price: u64,
    pub taker_fee: u64,
    pub maker_fee: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

/// Every protocol collection (11 section 2.4); `mint` is the default key for native lamports.
#[event]
pub struct ProtocolFee {
    pub source: u8,
    pub mint: Pubkey,
    pub amount: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}
