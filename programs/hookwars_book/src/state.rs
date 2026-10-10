// Changed by Hookwars: new file (hook economy, docs/spec/11-hook-economy.md section 6); review 3 M-8 and L-1
// (tick and size bounds, market terms setter, skill fee floor).
use anchor_lang::prelude::*;
use hookwars_common::PARAM_FIELDS;

pub const VERSION: u8 = 1;
/// Most resting orders per side (layout constant; `BOOK_SLOTS` is at most this).
pub const SLOTS_CAP: usize = 32;

pub mod seeds {
    pub const CONFIG: &[u8] = b"book-config";
    pub const PENDING: &[u8] = b"book-pending";
    pub const BOOK: &[u8] = b"book";
    pub const ESCROW: &[u8] = b"book-escrow";
    pub const CLASS_BID: &[u8] = b"class-bid";
    pub const PENDING_TERMS: &[u8] = b"book-pending-terms";
}

pub mod side {
    pub const BID: u8 = 0;
    pub const ASK: u8 = 1;
}

/// The book's parameters (11 section 11; every one "to set").
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BookParams {
    /// `BOOK_TAKER_BPS`: paid by the taker on every fill.
    pub taker_bps: u16,
    /// `BOOK_MAKER_BPS`: paid by the maker on every fill (may be 0).
    pub maker_bps: u16,
    /// `BOOK_SLOTS`: resting orders per side (at most `SLOTS_CAP`).
    pub slots: u8,
    /// `BOOK_MATCH_MAX`: fills per `place`.
    pub match_max: u8,
    /// `BOOK_CREATE_LEVEL`: the `Trader` level `create_market` requires (0 = none).
    pub create_level: u8,
    /// Deposit each order leaves in escrow; back to the owner on fill, cancel or eviction, to the
    /// cranker when the order expires (the crank bounty, rule 5).
    pub order_bounty_lamports: u64,
    /// `ADMIN_TIMELOCK_SECS` (00).
    pub admin_timelock_secs: u32,
    /// Least and most tick a market may have (review 3 M-8; owner values, 00 section 6).
    pub tick_min_lamports: u64,
    pub tick_max_lamports: u64,
    /// Most `min_size` a market may have (review 3 M-8).
    pub min_size_max: u64,
    /// A fill counts toward the Trader skill only when its fees reach this (review 3 L-1).
    pub skill_min_fee_lamports: u64,
}

/// `BookConfig` at `["book-config"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct BookConfig {
    pub version: u8,
    pub bump: u8,
    pub admin: Pubkey,
    /// Receives every book fee (11 section 2.4).
    pub treasury: Pubkey,
    pub params: BookParams,
    /// Every lamport of fees the protocol has received.
    pub protocol_fees_total: u128,
    pub markets: u32,
    pub reserved: [u8; 32],
}

/// `PendingBook` at `["book-pending"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct PendingBook {
    pub bump: u8,
    pub treasury: Pubkey,
    pub params: BookParams,
    pub ready_at: i64,
    pub active: bool,
}

/// `PendingMarketTerms` at `["book-pending-terms", market]`: an admin change of one market's tick
/// and minimum size, applied after `admin_timelock_secs` (review 3 M-8).
#[account]
#[derive(InitSpace, Debug)]
pub struct PendingMarketTerms {
    pub bump: u8,
    pub market: Pubkey,
    pub tick_lamports: u64,
    pub min_size: u64,
    pub ready_at: i64,
    pub active: bool,
}

/// A resting order. Prices are lamports per base unit.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Order {
    pub id: u64,
    pub owner: Pubkey,
    pub price: u64,
    /// Base units left.
    pub size: u64,
    /// Bids: lamports still escrowed for this order (quote plus the maker fee reserve).
    pub quote_locked: u64,
    pub bounty: u64,
    /// 0 = never.
    pub expires_at: i64,
}

/// `BookMarket` at `["book", base_mint]`: a material book (11 section 6.2). Quote is native SOL
/// (lamports), held by `["book-escrow", market]` with the base of the asks.
#[account]
#[derive(InitSpace, Debug)]
pub struct BookMarket {
    pub version: u8,
    pub bump: u8,
    pub escrow_bump: u8,
    pub base_mint: Pubkey,
    pub material_id: u16,
    pub tick_lamports: u64,
    /// `min_size`: the least base units an order may rest with.
    pub min_size: u64,
    pub seq: u64,
    /// Best first: highest price, then oldest.
    #[max_len(SLOTS_CAP)]
    pub bids: Vec<Order>,
    /// Best first: lowest price, then oldest.
    #[max_len(SLOTS_CAP)]
    pub asks: Vec<Order>,
    pub created_by: Pubkey,
    pub fills: u64,
}

impl BookMarket {
    /// Where an order of `side` at `price` goes (after every order at least as good).
    pub fn insert_at(&self, side_: u8, price: u64) -> usize {
        if side_ == side::BID {
            self.bids.iter().position(|o| o.price < price).unwrap_or(self.bids.len())
        } else {
            self.asks.iter().position(|o| o.price > price).unwrap_or(self.asks.len())
        }
    }

    /// Lamports the escrow must hold above its rent for this book: every bid's locked quote plus
    /// every order's bounty (the escrow conservation invariant, 11 section 6.6).
    pub fn lamports_owed(&self) -> u128 {
        let b: u128 = self.bids.iter().map(|o| u128::from(o.quote_locked) + u128::from(o.bounty)).sum();
        let a: u128 = self.asks.iter().map(|o| u128::from(o.bounty)).sum();
        a + b
    }

    /// Base units the escrow must hold: every ask's size.
    pub fn base_owed(&self) -> u128 {
        self.asks.iter().map(|o| u128::from(o.size)).sum()
    }
}

/// `BookEscrow` at `["book-escrow", market]`: holds the bids' lamports (above its rent) and owns
/// the holding of the asks' base.
#[account]
#[derive(InitSpace, Debug)]
pub struct BookEscrow {
    pub bump: u8,
    pub market: Pubkey,
}

/// What a class bid asks for (11 section 6.3 `ClassKey`).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClassKey {
    pub template_id: u16,
    pub min_level: u8,
    pub param_min: [u32; PARAM_FIELDS],
    pub param_max: [u32; PARAM_FIELDS],
}

impl ClassKey {
    pub fn valid(&self) -> bool {
        self.param_min.iter().zip(self.param_max.iter()).all(|(a, b)| a <= b)
    }

    pub fn fits(&self, template_id: u16, level: u8, params: &[u32; PARAM_FIELDS]) -> bool {
        template_id == self.template_id
            && level >= self.min_level
            && params
                .iter()
                .zip(self.param_min.iter().zip(self.param_max.iter()))
                .all(|(p, (lo, hi))| p >= lo && p <= hi)
    }
}

/// `ClassBid` at `["class-bid", bidder, nonce]`: a standing bid for any item of a class (R44). The
/// account itself holds the price and the maker fee above its rent.
#[account]
#[derive(InitSpace, Debug)]
pub struct ClassBid {
    pub version: u8,
    pub bump: u8,
    pub bidder: Pubkey,
    pub nonce: u64,
    pub class: ClassKey,
    pub price: u64,
    pub maker_fee: u64,
    pub expires_at: i64,
    pub created_at: i64,
}

pub fn config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::CONFIG], &crate::ID)
}
pub fn pending_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::PENDING], &crate::ID)
}
pub fn market_address(base_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::BOOK, base_mint.as_ref()], &crate::ID)
}
pub fn pending_terms_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::PENDING_TERMS, market.as_ref()], &crate::ID)
}
pub fn escrow_address(market: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::ESCROW, market.as_ref()], &crate::ID)
}
pub fn class_bid_address(bidder: &Pubkey, nonce: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::CLASS_BID, bidder.as_ref(), &nonce.to_le_bytes()], &crate::ID)
}
