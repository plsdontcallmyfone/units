// Changed by Hookwars: new file (expansion, 10 sections 1, 4, 5).
//! Accounts of `hookwars_market`. Every number below is a parameter in [`MarketParams`], set by the
//! admin behind `admin_timelock_secs`; nothing is hard-coded but layout constants.

use anchor_lang::prelude::*;

/// Layout version of every account here.
pub const VERSION: u8 = 1;
/// Longest collection name (layout constant).
pub const NAME_MAX_LEN: usize = 32;
/// Longest commission brief URI (layout constant).
pub const URI_MAX_LEN: usize = 200;
/// Most templates one collection may group (layout bound; `collection_max_templates` must be at
/// most this).
pub const COLLECTION_TEMPLATES_CAP: usize = 16;

/// Seeds.
pub mod seeds {
    pub const CONFIG: &[u8] = b"market-config";
    pub const PENDING: &[u8] = b"market-pending";
    pub const LISTING: &[u8] = b"listing";
    pub const ESCROW: &[u8] = b"escrow";
    pub const COLLECTION: &[u8] = b"collection";
    pub const LEASE: &[u8] = b"lease";
    pub const LEASE_ESCROW: &[u8] = b"lease-escrow";
    pub const COMMISSION: &[u8] = b"commission";
    pub const COMMISSION_VAULT: &[u8] = b"commission-vault";
    pub const SUBMISSION: &[u8] = b"submission";
}

/// The market's parameters (10 section 13; every one "to set").
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MarketParams {
    /// `MARKET_FEE_BPS`: the protocol's share of a sale price.
    pub fee_bps: u16,
    /// `AUTHOR_RESALE_BPS`: the item author's share of a sale price.
    pub author_resale_bps: u16,
    /// `COLLECTION_MAX_TEMPLATES`.
    pub collection_max_templates: u8,
    /// `MAX_RENT_BPS`: the most of an item's royalty a lease may assign to the lessor (R32).
    pub max_rent_bps: u16,
    /// `LEASE_MIN_SECS`.
    pub lease_min_secs: u32,
    /// `LEASE_MAX_SECS`.
    pub lease_max_secs: u32,
    /// `COMMISSION_MIN_LAMPORTS`.
    pub commission_min_lamports: u64,
    /// `COMMISSION_VOTE_SECS`: after a commission closes, how long the slot has to equip a
    /// submission before anyone may refund the bounty.
    pub commission_vote_secs: u32,
    /// `ADMIN_TIMELOCK_SECS` (00): delay on every params change.
    pub admin_timelock_secs: u32,
}

/// `MarketConfig` at `["market-config"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct MarketConfig {
    pub version: u8,
    pub bump: u8,
    pub admin: Pubkey,
    /// Wallet that receives the protocol's share of sales.
    pub treasury: Pubkey,
    pub params: MarketParams,
    /// Collections created so far (the next id).
    pub collections: u32,
    pub reserved: [u8; 32],
}

/// `PendingMarketParams` at `["market-pending"]`: a params change waiting for its delay.
#[account]
#[derive(InitSpace, Debug)]
pub struct PendingMarketParams {
    pub bump: u8,
    pub params: MarketParams,
    pub treasury: Pubkey,
    pub ready_at: i64,
    pub active: bool,
}

/// `Listing` at `["listing", item_mint]` (10 section 1.1). One live listing per item.
#[account]
#[derive(InitSpace, Debug)]
pub struct Listing {
    pub version: u8,
    pub bump: u8,
    pub seller: Pubkey,
    pub item: Pubkey,
    pub item_mint: Pubkey,
    pub price_lamports: u64,
    pub created_at: i64,
    /// 0 = never expires.
    pub expires_at: i64,
}

/// `Collection` at `["collection", id]` (10 section 1.5). Discovery only, no economic effect.
#[account]
#[derive(InitSpace, Debug)]
pub struct Collection {
    pub version: u8,
    pub bump: u8,
    pub id: u32,
    #[max_len(NAME_MAX_LEN)]
    pub name: String,
    pub curator: Pubkey,
    #[max_len(COLLECTION_TEMPLATES_CAP)]
    pub template_ids: Vec<u16>,
    pub created_at: i64,
}

/// Lease states.
pub mod lease_state {
    pub const OFFERED: u8 = 0;
    pub const ACTIVE: u8 = 1;
}

/// `Lease` at `["lease", item]` (10 section 4.2). Closed when it ends or is withdrawn.
#[account]
#[derive(InitSpace, Debug)]
pub struct Lease {
    pub version: u8,
    pub bump: u8,
    pub lessor: Pubkey,
    pub item: Pubkey,
    pub item_mint: Pubkey,
    /// The token that may equip the item during the term.
    pub token_mint: Pubkey,
    pub slot: u8,
    /// Share of the item's royalty paid to the lessor during the term (R32: never on top).
    pub rent_bps: u16,
    /// Paid once to the lessor by whoever accepts.
    pub fee_lamports: u64,
    pub term_secs: u32,
    pub starts_at: i64,
    pub ends_at: i64,
    pub state: u8,
}

/// Commission states.
pub mod commission_state {
    pub const OPEN: u8 = 0;
    pub const PAID: u8 = 2;
    pub const REFUNDED: u8 = 3;
}

/// `Commission` at `["commission", token_mint, nonce]` (10 section 5.2).
#[account]
#[derive(InitSpace, Debug)]
pub struct Commission {
    pub version: u8,
    pub bump: u8,
    pub vault_bump: u8,
    pub creator: Pubkey,
    pub token_mint: Pubkey,
    pub slot: u8,
    pub nonce: u64,
    #[max_len(URI_MAX_LEN)]
    pub brief_uri: String,
    pub bounty_lamports: u64,
    /// The item in the slot when the commission opened (a payment needs a different one).
    pub incumbent: Pubkey,
    pub opens_at: i64,
    pub closes_at: i64,
    pub state: u8,
    pub winner: Option<Pubkey>,
    pub submissions: u32,
}

/// `Submission` at `["submission", commission, item]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Submission {
    pub version: u8,
    pub bump: u8,
    pub commission: Pubkey,
    pub item: Pubkey,
    pub submitter: Pubkey,
    pub submitted_at: i64,
}

impl Listing {
    pub fn address(item_mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::LISTING, item_mint.as_ref()], &crate::ID)
    }
}

/// `["escrow", item_mint]`: owner of a listed item's holding.
pub fn escrow_address(item_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::ESCROW, item_mint.as_ref()], &crate::ID)
}

/// `["lease-escrow", item_mint]`: owner of a leased item's holding.
pub fn lease_escrow_address(item_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::LEASE_ESCROW, item_mint.as_ref()], &crate::ID)
}

/// `["lease", item]`.
pub fn lease_address(item: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::LEASE, item.as_ref()], &crate::ID)
}

/// `["commission", token_mint, nonce]`.
pub fn commission_address(token_mint: &Pubkey, nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::COMMISSION, token_mint.as_ref(), &nonce.to_le_bytes()],
        &crate::ID,
    )
}

/// `["commission-vault", commission]`.
pub fn commission_vault_address(commission: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::COMMISSION_VAULT, commission.as_ref()], &crate::ID)
}

/// `["submission", commission, item]`.
pub fn submission_address(commission: &Pubkey, item: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[seeds::SUBMISSION, commission.as_ref(), item.as_ref()],
        &crate::ID,
    )
}

/// `["collection", id]`.
pub fn collection_address(id: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::COLLECTION, &id.to_le_bytes()], &crate::ID)
}

/// `["market-config"]`.
pub fn config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::CONFIG], &crate::ID)
}

/// `["market-pending"]`.
pub fn pending_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::PENDING], &crate::ID)
}
