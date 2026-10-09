// Changed by Hookwars: new file (expansion, 10). Price history comes from `Sold` only (10 1.3).
use anchor_lang::prelude::*;

#[event]
pub struct Listed {
    pub item: Pubkey,
    pub item_mint: Pubkey,
    pub seller: Pubkey,
    pub price_lamports: u64,
    pub expires_at: i64,
    pub ts: i64,
}

#[event]
pub struct Delisted {
    pub item_mint: Pubkey,
    pub seller: Pubkey,
    pub ts: i64,
}

#[event]
pub struct Sold {
    pub item: Pubkey,
    pub item_mint: Pubkey,
    pub seller: Pubkey,
    pub buyer: Pubkey,
    pub price: u64,
    pub fee: u64,
    pub resale: u64,
    pub ts: i64,
}

#[event]
pub struct ListingExpired {
    pub item_mint: Pubkey,
    pub seller: Pubkey,
    pub ts: i64,
}

#[event]
pub struct CollectionCreated {
    pub id: u32,
    pub name: String,
    pub curator: Pubkey,
    pub template_ids: Vec<u16>,
    pub ts: i64,
}

#[event]
pub struct LeaseOffered {
    pub item: Pubkey,
    pub lessor: Pubkey,
    pub token_mint: Pubkey,
    pub slot: u8,
    pub rent_bps: u16,
    pub fee_lamports: u64,
    pub term_secs: u32,
    pub ts: i64,
}

#[event]
pub struct LeaseStarted {
    pub item: Pubkey,
    pub payer: Pubkey,
    pub starts_at: i64,
    pub ends_at: i64,
}

#[event]
pub struct LeaseWithdrawn {
    pub item: Pubkey,
    pub ts: i64,
}

#[event]
pub struct LeaseEnded {
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub slot: u8,
    pub ts: i64,
}

#[event]
pub struct CommissionOpened {
    pub commission: Pubkey,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub slot: u8,
    pub bounty_lamports: u64,
    pub closes_at: i64,
    pub brief_uri: String,
}

#[event]
pub struct Submitted {
    pub commission: Pubkey,
    pub item: Pubkey,
    pub submitter: Pubkey,
    pub ts: i64,
}

#[event]
pub struct CommissionPaid {
    pub commission: Pubkey,
    pub item: Pubkey,
    pub submitter: Pubkey,
    pub bounty_lamports: u64,
    pub ts: i64,
}

#[event]
pub struct CommissionRefunded {
    pub commission: Pubkey,
    pub creator: Pubkey,
    pub bounty_lamports: u64,
    pub ts: i64,
}

#[event]
pub struct MarketParamsProposed {
    pub ready_at: i64,
}

#[event]
pub struct MarketParamsApplied {
    pub ts: i64,
}
