// Changed by Hookwars: new file (M2); M3b: settle_bounty_bps, CompositeItem.
//! Accounts of the armory (docs/spec/02-armory.md section 2).

use anchor_lang::prelude::*;
use hookwars_common::{EquipConfig, Manifest, PerformanceRule, PARAM_FIELDS};

/// Layout version.
pub const VERSION: u8 = 1;

/// Every number the armory acts on (00 section 6), set by the admin behind `admin_timelock_secs`.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArmoryParams {
    /// `MAX_ROYALTY_BPS`.
    pub max_royalty_bps: u16,
    /// `VOTE_PERIOD_SECS`.
    pub vote_period_secs: u32,
    /// `VOTE_QUORUM_BPS`.
    pub vote_quorum_bps: u16,
    /// `MIN_NOTICE_SECS`.
    pub min_notice_secs: u32,
    /// `MAX_NOTICE_SECS`.
    pub max_notice_secs: u32,
    /// `FORGE_GAIN_BPS`.
    pub forge_gain_bps: u16,
    /// `MIN_TWAP_SECS`.
    pub min_twap_secs: u32,
    /// `MAX_POOL_ITEM_CUT_BPS`: the most a Pool or Relation item may cut on one side of a swap.
    pub max_pool_item_cut_bps: u16,
    /// The most creator and holder fee discount an item may ask for.
    pub max_pool_item_discount_bps: u16,
    /// The most foreign accounts an item may read.
    pub max_item_reads: u8,
    /// `ADMIN_TIMELOCK_SECS`: delay on every admin change, this one included.
    pub admin_timelock_secs: u32,
    /// Hookwars M3b: the bounty `settle_equip` pays its sender, of what it settles
    /// (`MAX_CRANK_BOUNTY_BPS` at most; 04 section 2.5).
    pub settle_bounty_bps: u16,
}

/// `ArmoryConfig` at `["config"]` (02 section 2.1).
#[account]
#[derive(InitSpace, Debug)]
pub struct ArmoryConfig {
    pub version: u8,
    pub bump: u8,
    /// Registers and retires templates, proposes params.
    pub admin: Pubkey,
    /// Proposed admin and when it may accept.
    pub pending_admin: Option<Pubkey>,
    pub pending_admin_at: i64,
    /// Seed of the next item mint.
    pub items_minted: u64,
    /// Templates registered.
    pub templates: u16,
    /// The numbers.
    pub params: ArmoryParams,
    pub reserved: [u8; 62],
}

/// Hookwars M3b: a composite's module list at `["composite", item]` (08 section 2.2; the same
/// layout as `hookwars_common::composite::CompositeItem`).
#[account]
#[derive(Debug)]
pub struct CompositeItem {
    pub version: u8,
    pub bump: u8,
    pub item: Pubkey,
    pub modules: Vec<hookwars_common::composite::Module>,
    pub provenance: Vec<Pubkey>,
}

/// Params waiting out the timelock at `["pending-params"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct PendingParams {
    pub bump: u8,
    pub params: ArmoryParams,
    pub ready_at: i64,
}

/// `Template` at `["template", id]` (02 section 2.3).
#[account]
#[derive(InitSpace, Debug)]
pub struct Template {
    pub version: u8,
    pub bump: u8,
    pub id: u16,
    pub program: Pubkey,
    pub code_hash: [u8; 32],
    pub deploy_slot: Option<u64>,
    pub kind: u8,
    pub field_count: u8,
    pub field_min: [u32; PARAM_FIELDS],
    pub field_max: [u32; PARAM_FIELDS],
    pub open_authoring: bool,
    pub loot_enabled: bool,
    pub forge_enabled: bool,
    pub max_level: u8,
    pub loot_royalty_bps: u16,
    /// Most targets an equip of its items may name (M2 notes: the manifest promises the most).
    pub max_targets: u8,
    /// `Active` 0, `Retired` 1.
    pub status: u8,
    #[max_len(32)]
    pub name: String,
    pub registered_by: Pubkey,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

/// Template status.
pub mod template_status {
    pub const ACTIVE: u8 = 0;
    pub const RETIRED: u8 = 1;
}

/// `Item` at `["item", item_mint]` (02 section 2.4).
#[account]
#[derive(InitSpace, Debug)]
pub struct Item {
    pub version: u8,
    pub bump: u8,
    pub item_mint: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub manifest: Manifest,
    pub author: Pubkey,
    pub royalty_bps: u16,
    pub level: u8,
    /// `Authored` 0, `Loot` 1, `Forged` 2.
    pub source: u8,
    pub equipped_count: u32,
    pub royalty_owner_bump: u8,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

/// Item sources.
pub mod source {
    pub const AUTHORED: u8 = 0;
    pub const LOOT: u8 = 1;
    pub const FORGED: u8 = 2;
}

/// `SlotState` at `["slot-state", mint, slot]` (02 section 2.7).
#[account]
#[derive(Debug)]
pub struct SlotState {
    pub version: u8,
    pub bump: u8,
    pub mint: Pubkey,
    pub slot: u8,
    /// The slot's equip rule (copied from the mint at launch).
    pub equip_rule: u8,
    /// Delay between a passed vote and its equip, fixed at launch.
    pub notice_secs: u32,
    pub launch_item: Option<Pubkey>,
    /// The launch item's targets and role: a performance revert re-aims it the same way.
    pub launch_config: EquipConfig,
    pub rule: Option<PerformanceRule>,
    pub open_proposal: Option<u64>,
    pub next_nonce: u64,
    pub condition_since: Option<i64>,
    pub last_check: i64,
    pub reserved: [u8; 32],
}

impl SlotState {
    /// Space with a launch config of `targets` targets.
    pub fn space(targets: usize) -> usize {
        8 + 1 + 1 + 32 + 1 + 1 + 4 + 33 + (4 + 32 * targets + 1) + (1 + 16) + 9 + 8 + 9 + 8 + 32
    }
}

/// `Proposal` at `["proposal", mint, slot, nonce]` (02 section 2.8).
#[account]
#[derive(Debug)]
pub struct Proposal {
    pub version: u8,
    pub bump: u8,
    pub mint: Pubkey,
    pub slot: u8,
    pub nonce: u64,
    pub proposer: Pubkey,
    /// `None` empties the slot.
    pub item: Option<Pubkey>,
    /// Targets and role of the equip (04 section 2.3).
    pub config: EquipConfig,
    pub created_at: i64,
    pub vote_end: i64,
    pub executable_at: i64,
    pub votes_for: u64,
    pub votes_against: u64,
    /// `VoteLock` accounts not yet closed (the proposal closes after them).
    pub voters_open: u32,
    pub status: u8,
    pub reserved: [u8; 32],
}

impl Proposal {
    /// Space with `targets` targets.
    pub fn space(targets: usize) -> usize {
        8 + 1 + 1 + 32 + 1 + 8 + 32 + 33 + (4 + 32 * targets + 1) + 8 + 8 + 8 + 8 + 8 + 4 + 1 + 32
    }
}

/// Proposal status.
pub mod proposal_status {
    pub const OPEN: u8 = 0;
    pub const PASSED: u8 = 1;
    pub const FAILED: u8 = 2;
    pub const EXECUTED: u8 = 3;
    pub const CANCELLED: u8 = 4;
}

/// `VoteLock` at `["vote", proposal, voter]` (02 section 2.9).
#[account]
#[derive(InitSpace, Debug)]
pub struct VoteLock {
    pub proposal: Pubkey,
    pub voter: Pubkey,
    pub amount: u64,
    pub support: bool,
    pub bump: u8,
}

/// `ForgeCounter` at `["forges", wallet]` (02 section 2.10).
#[account]
#[derive(InitSpace, Debug)]
pub struct ForgeCounter {
    pub wallet: Pubkey,
    pub count: u64,
    pub bump: u8,
}

const _: () = assert!(PARAM_FIELDS == 11);
