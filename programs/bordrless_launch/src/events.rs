// Changed by Hookwars: M3b slot launch events.
//! Events of the launchpad, emitted by self-CPI.

#![allow(missing_docs)]

use anchor_lang::prelude::*;

use crate::state::{LaunchRules, RuleBounds};

#[event]
pub struct ConfigSet {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub quote_mint: Pubkey,
    pub launch_fee_lamports: u64,
    pub lp_fee_bps: u16,
    pub max_creator_fee_bps: u16,
    pub sniper_window_secs: i64,
    pub sniper_start_bps: u16,
    pub curve_bps: u16,
    pub supply: u64,
    pub decimals: u8,
    pub min_virtual_quote: u64,
    pub max_virtual_quote: u64,
    pub paused: bool,
    pub rule_bounds: RuleBounds,
    pub ts: i64,
}

#[event]
pub struct LaunchCreated {
    pub launch: Pubkey,
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub pool: Pubkey,
    pub quote_mint: Pubkey,
    pub lp_mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub supply: u64,
    pub decimals: u8,
    pub creator_fee_bps: u16,
    pub lp_fee_bps: u16,
    pub sniper_window_secs: i64,
    pub sniper_start_bps: u16,
    pub virtual_quote: u64,
    pub virtual_base: u64,
    pub graduation_quote: u64,
    pub curve_tokens: u64,
    pub reserve_tokens: u64,
    pub launch_fee_lamports: u64,
    /// The token rules, fixed at launch.
    pub rules: LaunchRules,
    /// The kit modules they installed (0: no kit, no token hook).
    pub modules: u8,
    /// The token's `KitConfig` when it has a kit.
    pub kit_config: Option<Pubkey>,
    /// Where holder fees go (the kit's reward vault) when holder rewards are on.
    pub holder_vault: Option<Pubkey>,
    /// The creator wallet unlocks at this time; 0 when off.
    pub creator_unlock_at: i64,
    /// Buys from the pool before this time are locked; 0 when off.
    pub early_window_end: i64,
    /// ... until this time; 0 when off.
    pub early_unlock_at: i64,
    /// The `LaunchConfig` the launch was made from, when it was.
    pub config: Option<Pubkey>,
    /// The creator's own token hook, when the token has one.
    pub custom_hook: Option<Pubkey>,
    /// Its flags; 0 without one.
    pub custom_hook_flags: u16,
    pub slot: u64,
    pub ts: i64,
}

/// A `LaunchConfig` was made (by anyone, with the SDK).
#[event]
pub struct LaunchConfigCreated {
    pub config: Pubkey,
    pub creator: Pubkey,
    pub rules: LaunchRules,
    pub creator_fee_bps: u16,
    pub custom_hook: Option<Pubkey>,
    pub custom_hook_flags: u16,
    pub label: String,
    pub ts: i64,
}

#[event]
pub struct Graduated {
    pub launch: Pubkey,
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub cranker: Pubkey,
    pub topup: u64,
    pub burned: u64,
    pub base_reserve: u64,
    pub quote_reserve: u64,
    pub lp_minted: u64,
    pub supply: u64,
    pub slot: u64,
    pub ts: i64,
}

/// A listed config (`create_listed_config`), after its `LaunchConfigCreated`.
#[event]
pub struct ConfigListed {
    pub config: Pubkey,
    pub author: Pubkey,
    pub author_share_bps: u16,
    pub ts: i64,
}

/// A config author's part of a claim of creator fees (whoever claimed).
#[event]
pub struct AuthorFeesPaid {
    pub launch: Pubkey,
    pub mint: Pubkey,
    pub config: Pubkey,
    pub author: Pubkey,
    pub amount: u64,
    pub paid_total: u64,
    pub slot: u64,
    pub ts: i64,
}

#[event]
pub struct CreatorFeesClaimed {
    pub launch: Pubkey,
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub amount: u64,
    pub claimed_total: u64,
    pub slot: u64,
    pub ts: i64,
}

/// Hookwars M3b: a slot launch prepared (`prepare_launch`): the mint exists with its slot table
/// and no supply.
#[event]
pub struct LaunchPrepared {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub slot_count: u8,
    pub kit_slot: bool,
    pub slot: u64,
    pub ts: i64,
}

/// Hookwars M3b: one pool item's part of a callback.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct PoolItemPart {
    pub slot: u8,
    pub item: Pubkey,
    pub discount_bps: u16,
    pub cut: u64,
    pub burn: u64,
}

/// Hookwars M3b: what the pool items of a slot launch answered in one callback (side 0: input,
/// 1: output). Logged (`emit!`), not a self-CPI, so a swap's trace grows only by the items' own
/// calls. `pool_cuts_delta` is the one merged delta to the `PoolCuts` holding.
#[event]
pub struct PoolItemCuts {
    pub launch: Pubkey,
    pub pool: Pubkey,
    pub mint: Pubkey,
    pub side: u8,
    pub discount_bps: u16,
    pub parts: Vec<PoolItemPart>,
    pub pool_cuts_delta: u64,
    pub burned: u64,
    pub slot: u64,
    pub ts: i64,
}

/// Hookwars M3b: the pool registry of a slot launch rewritten from its slot table.
#[event]
pub struct PoolRegistryRefreshed {
    pub mint: Pubkey,
    pub pool: Pubkey,
    pub items: Vec<Pubkey>,
    pub accounts: u16,
    pub ts: i64,
}
