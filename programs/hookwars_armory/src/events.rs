// Changed by Hookwars: new file (M2); integration pass 3: TemplateEconomySet, ItemProtocolBpsSet, ItemCrafted, PresetRegistered.; protocol pass 4a: access, admin queue and submission events.
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
    /// Protocol pass 4a (E-1): `enforce_access` removed an item whose access lapsed.
    pub const ACCESS: u8 = 4;
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

/// Integration pass 3 (08 section 4.8): a composite preset registered.
#[event]
pub struct PresetRegistered {
    pub id: u16,
    pub template_ids: Vec<u16>,
}

/// Protocol pass 4a (11 section 1.3): an item's access mode set.
#[event]
pub struct AccessSet {
    pub item: Pubkey,
    pub holder: Pubkey,
    pub mode: u8,
    pub exclusive: bool,
    pub licence_terms: Option<crate::state::LicenceTerms>,
    pub ts: i64,
}

/// Protocol pass 4a: a Gated item approved for a token.
#[event]
pub struct Approved {
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub by: Pubkey,
    pub ts: i64,
}

/// Protocol pass 4a: an approval revoked; removal is possible from `revoke_after` (R39).
#[event]
pub struct ApprovalRevoked {
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub revoke_after: i64,
    pub ts: i64,
}

/// Protocol pass 4a: `enforce_access` removed an item whose access lapsed.
#[event]
pub struct AccessEnforced {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub to_item: Option<Pubkey>,
    pub by: Pubkey,
    pub ts: i64,
}

/// Protocol pass 4a (review 1 L-1): an admin action queued behind the timelock.
#[event]
pub struct AdminActionQueued {
    pub action_hash: [u8; 32],
    pub ready_at: i64,
}

/// Protocol pass 4a: a queued admin action cancelled before it applied.
#[event]
pub struct AdminActionCancelled {
    pub action_hash: [u8; 32],
}

/// Protocol pass 4a: a queued admin action applied.
#[event]
pub struct AdminActionApplied {
    pub action_hash: [u8; 32],
    pub ts: i64,
}

/// Protocol pass 4a (E-8, 11 section 3.3): the access and lab numbers set.
#[event]
pub struct AccessParamsSet {
    pub params: crate::state::AccessParams,
}

/// Protocol pass 4a (10 section 11.5): a template submitted with its bond.
#[event]
pub struct TemplateSubmitted {
    pub submission: Pubkey,
    pub program: Pubkey,
    pub code_hash: [u8; 32],
    pub uri_hash: [u8; 32],
    pub submitter: Pubkey,
    pub bond: u64,
    pub ts: i64,
}

/// Protocol pass 4a: a submission closed: approved (bond back), rejected (bond back) or
/// forfeited (bond to the admin).
#[event]
pub struct SubmissionSettled {
    pub submission: Pubkey,
    pub program: Pubkey,
    pub approved: bool,
    pub forfeited: bool,
    pub bond: u64,
    pub ts: i64,
}

