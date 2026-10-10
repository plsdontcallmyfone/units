// Changed by Hookwars: new file (M2); integration pass 3: TemplateEconomySet, ItemProtocolBpsSet.
//! Events of the armory (docs/spec/02-armory.md section 13), by self-CPI.

use anchor_lang::prelude::*;
use hookwars_common::{Manifest, PARAM_FIELDS};

#[event]
pub struct TemplateRegistered {
    pub template_id: u16,
    pub program: Pubkey,
    pub code_hash: [u8; 32],
    pub deploy_slot: Option<u64>,
    pub kind: u8,
    pub field_count: u8,
    pub field_min: [u32; PARAM_FIELDS],
    pub field_max: [u32; PARAM_FIELDS],
    pub name: String,
    pub ts: i64,
}

/// Integration pass 3 (E-7).
#[event]
pub struct TemplateEconomySet {
    pub template_id: u16,
    pub author_bps: u16,
    pub default_access: u8,
    pub allowed_access: u8,
    pub charges_on_create: u32,
}

/// Integration pass 3 (E-2).
#[event]
pub struct ItemProtocolBpsSet {
    pub item_protocol_bps: u16,
}

#[event]
pub struct TemplateRetired {
    pub template_id: u16,
    pub ts: i64,
}

#[event]
pub struct ItemCreated {
    pub item: Pubkey,
    pub item_mint: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub manifest: Manifest,
    pub author: Pubkey,
    pub royalty_bps: u16,
    pub level: u8,
    pub source: u8,
    pub ts: i64,
}

#[event]
pub struct LootMinted {
    pub item: Pubkey,
    pub owner: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub ts: i64,
}

#[event]
pub struct Forged {
    pub burned: [Pubkey; 2],
    pub item: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub level: u8,
    pub forger: Pubkey,
    pub ts: i64,
}

#[event]
pub struct RoyaltyClaimed {
    pub item: Pubkey,
    pub cut_mint: Pubkey,
    pub claimant: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[event]
pub struct ProposalCreated {
    pub mint: Pubkey,
    pub slot: u8,
    pub nonce: u64,
    pub proposer: Pubkey,
    pub item: Option<Pubkey>,
    pub vote_end: i64,
    pub executable_at: i64,
}

#[event]
pub struct VoteLocked {
    pub proposal: Pubkey,
    pub voter: Pubkey,
    pub support: bool,
    pub amount: u64,
    pub until: i64,
}

#[event]
pub struct VoteUnlocked {
    pub proposal: Pubkey,
    pub voter: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ProposalResolved {
    pub proposal: Pubkey,
    pub status: u8,
    pub votes_for: u64,
    pub votes_against: u64,
    pub eligible: u64,
    pub ts: i64,
}

/// How an equip happened.
pub mod equip_by {
    pub const LAUNCH: u8 = 0;
    pub const VOTE: u8 = 1;
    pub const PERFORMANCE: u8 = 2;
    /// Integration pass 2: the market ended a lease (10 section 17 I-3).
    pub const LEASE_END: u8 = 3;
}

#[event]
pub struct EquipApplied {
    pub mint: Pubkey,
    pub slot: u8,
    pub old_item: Option<Pubkey>,
    pub new_item: Option<Pubkey>,
    pub by: u8,
    pub ts: i64,
}

#[event]
pub struct PerformanceCondition {
    pub mint: Pubkey,
    pub slot: u8,
    pub true_since: Option<i64>,
}

#[event]
pub struct PerformanceReverted {
    pub mint: Pubkey,
    pub slot: u8,
    pub from_item: Option<Pubkey>,
    pub to_item: Option<Pubkey>,
    pub ts: i64,
}

#[event]
pub struct ParamsProposed {
    pub params: crate::state::ArmoryParams,
    pub ready_at: i64,
}

#[event]
pub struct ParamsApplied {
    pub params: crate::state::ArmoryParams,
    pub ts: i64,
}

#[event]
pub struct AdminProposed {
    pub admin: Pubkey,
    pub ready_at: i64,
}

#[event]
pub struct AdminAccepted {
    pub admin: Pubkey,
    pub ts: i64,
}

/// Integration pass 3 (E-5): an item made by craft's `mint_crafted`.
#[event]
pub struct ItemCrafted {
    pub item: Pubkey,
    pub owner: Pubkey,
    pub template_id: u16,
    pub recipe_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub ts: i64,
}
