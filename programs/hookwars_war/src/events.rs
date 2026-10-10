// Changed by Hookwars: security review 1: RazeWaited (M-3); pass 4b: boss, coalition, rivalry events
//! Events, all emitted by self-CPI (`emit_cpi!`), with the names 06 uses (05 section 12).

use anchor_lang::prelude::*;

use crate::foreign::PARAM_FIELDS;
use crate::state::{ScoreWeights, WarParams};

#[event]
pub struct WarChestCreated {
    pub mint: Pubkey,
    pub chest: Pubkey,
    pub treaty_inbox: Pubkey,
    pub war_state: Pubkey,
}

#[event]
pub struct WarFunded {
    pub mint: Pubkey,
    pub amount: u64,
    pub balance: u64,
    pub funded_total: u64,
}

#[event]
pub struct SiegeExecuted {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub spent: u64,
    pub bought: u64,
    pub bounty: u64,
    pub cranker: Pubkey,
    pub captured_total: u64,
}

#[event]
pub struct SiegeWaited {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub rival_price: u128,
    pub rival_twap: u128,
}

/// Security review 1, M-3: a raze waited (the rival trades too far below its TWAP).
#[event]
pub struct RazeWaited {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub rival_price: u128,
    pub rival_twap: u128,
}

#[event]
pub struct CounterStrikeExecuted {
    pub mint: Pubkey,
    pub spent: u64,
    pub burned: u64,
    pub bounty: u64,
    pub cranker: Pubkey,
}

#[event]
pub struct Razed {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub sold: u64,
    pub got: u64,
    pub bounty: u64,
    pub cranker: Pubkey,
    pub captured_left: u64,
}

#[event]
pub struct CapturedReturned {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub amount: u64,
    pub treaty_item: Pubkey,
}

#[event]
pub struct TreatyInflowShared {
    pub mint: Pubkey,
    pub amount: u64,
    pub bounty: u64,
    pub cranker: Pubkey,
}

#[event]
pub struct TreatyTimeAccrued {
    pub mint: Pubkey,
    pub treaty_item: Pubkey,
    pub secs: u64,
    pub season: u32,
}

#[event]
pub struct BountyClaimed {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub points: u32,
    pub paid: u64,
}

#[event]
pub struct RollRequested {
    pub roll: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub holding: Pubkey,
    pub season: u32,
    pub requested_slot: u64,
    pub oracle_program: Pubkey,
    pub oracle_account: Pubkey,
}

#[event]
pub struct RollRevealed {
    pub roll: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
    /// The new item, as the armory derives it; default when the armory names it in its own event
    /// (`LootMinted`).
    pub item: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub season: u32,
}

#[event]
pub struct RollCancelled {
    pub roll: Pubkey,
    pub mint: Pubkey,
    pub owner: Pubkey,
}

#[event]
pub struct QuestClaimed {
    pub quest_id: u8,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub season: u32,
    pub period: u32,
}

#[event]
pub struct SeasonProposed {
    pub season: u32,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weights: ScoreWeights,
    pub penalize_besieged: bool,
    pub eta: i64,
}

#[event]
pub struct LootTableProposed {
    pub season: u32,
    pub eta: i64,
}

#[event]
pub struct SeasonOpened {
    pub season: u32,
    pub starts_at: i64,
    pub ends_at: i64,
}

#[event]
pub struct CandidateSubmitted {
    pub season: u32,
    pub mint: Pubkey,
    pub score: i128,
    pub submitted_by: Pubkey,
}

#[event]
pub struct CandidateChallenged {
    pub season: u32,
    pub mint: Pubkey,
    pub score: i128,
    pub submitted_by: Pubkey,
    pub beaten: Pubkey,
}

#[event]
pub struct SeasonFinalized {
    pub season: u32,
    pub winner: Option<Pubkey>,
    pub score: i128,
}

#[event]
pub struct PrizePaid {
    pub season: u32,
    pub winner: Option<Pubkey>,
    pub to_winner: u64,
    pub to_treasury: u64,
    pub bounty: u64,
    pub cranker: Pubkey,
}

/// What a config change proposes (05 section 11).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigChange {
    pub admin: Pubkey,
    pub protocol_treasury: Pubkey,
    pub randomness_program: Pubkey,
    pub treaty_template_id: Option<u16>,
    pub params: WarParams,
}

#[event]
pub struct ConfigProposed {
    pub change: ConfigChange,
    pub eta: i64,
}

#[event]
pub struct ConfigApplied {
    pub change: ConfigChange,
    pub eta: i64,
}

/// What `cancel_pending` cancelled: 0 the config change, 1 a season, 2 a loot table.
#[event]
pub struct PendingCancelled {
    pub what: u8,
    pub season: u32,
    pub eta: i64,
}

// ---- pass 4b (10 sections 8, 11.1, 11.3) -----------------------------------------------------------

#[event]
pub struct BossPoolOpened {
    pub season: u32,
    pub boss_mint: Pubkey,
}

#[event]
pub struct BossPoolFunded {
    pub season: u32,
    pub amount: u64,
    pub funded: u64,
}

#[event]
pub struct BossPoolSealed {
    pub season: u32,
    pub to_share: u64,
    pub total_volume: u64,
    pub sources: u8,
}

#[event]
pub struct BossShareClaimed {
    pub season: u32,
    pub source_mint: Pubkey,
    pub volume: u64,
    pub amount: u64,
}

#[event]
pub struct CoalitionFormed {
    pub id: u32,
    pub members: Vec<Pubkey>,
    pub ends_at: i64,
}

#[event]
pub struct CoalitionContributed {
    pub id: u32,
    pub mint: Pubkey,
    pub amount: u64,
    pub contributed: u64,
}

#[event]
pub struct CoalitionSiegeExecuted {
    pub id: u32,
    pub rival_mint: Pubkey,
    pub spent: u64,
    pub bought: u64,
    pub bounty: u64,
    pub cranker: Pubkey,
}

#[event]
pub struct CoalitionRazed {
    pub id: u32,
    pub rival_mint: Pubkey,
    pub sold: u64,
    pub got: u64,
    pub bounty: u64,
}

#[event]
pub struct CoalitionDissolved {
    pub id: u32,
    pub returned: Vec<u64>,
}

#[event]
pub struct RivalryOpened {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub budget: u64,
    pub ends_at: i64,
}

/// `won`: this token's raid volume from the rival beat the rival's from it (score and badge only;
/// nothing moves between the chests, R36).
#[event]
pub struct RivalrySettled {
    pub mint: Pubkey,
    pub rival_mint: Pubkey,
    pub ours: u64,
    pub theirs: u64,
    pub won: bool,
    pub spent: u64,
    pub early: bool,
}
