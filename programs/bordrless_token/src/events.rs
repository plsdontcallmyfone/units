// Changed by Hookwars: slot events; Transferred gains slot_cuts; R21 SlotInfo.may_burn.
//! Events of the token standard, emitted by self-CPI. Transfers, mints and burns carry the
//! post-balances of the holdings they touched, so an indexer keeps exact balances.

#![allow(missing_docs)]

use anchor_lang::prelude::*;

#[event]
pub struct MintCreated {
    pub mint: Pubkey,
    pub creator: Pubkey,
    pub decimals: u8,
    pub max_supply: u64,
    pub mint_authority: Option<Pubkey>,
    pub freeze_authority: Option<Pubkey>,
    pub hook_authority: Option<Pubkey>,
    pub metadata_authority: Option<Pubkey>,
    pub hook_program: Option<Pubkey>,
    pub hook_flags: u16,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub ts: i64,
}

#[event]
pub struct HoldingCreated {
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub owner: Pubkey,
    pub payer: Pubkey,
    pub ts: i64,
}

/// One delta a `before_transfer` answer took: the holding it credited, that holding's owner, the
/// amount and the holding's balance after.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeltaApplied {
    pub holding: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub post: u64,
}

/// A transfer. The source lost `amount`; the destination gained `amount` less the deltas; each
/// delta holding gained its amount. Hook data is not in events.
#[event]
pub struct Transferred {
    pub mint: Pubkey,
    pub source: Pubkey,
    pub destination: Pubkey,
    pub source_owner: Pubkey,
    pub destination_owner: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
    pub deltas: Vec<DeltaApplied>,
    pub source_post: u64,
    pub destination_post: u64,
    pub slot: u64,
    pub ts: i64,
    /// Hookwars: one entry per slot that cut (empty for a mint without slots).
    pub slot_cuts: Vec<SlotCut>,
}

/// One slot's cut of a transfer.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SlotCut {
    pub slot: u8,
    /// The item (default for the Locked slot).
    pub item: Pubkey,
    pub cut: u64,
}

/// What `SlotsInitialized` reports per slot.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct SlotInfo {
    pub kind: u8,
    pub equip_rule: u8,
    pub max_cut_bps: u16,
    pub may_refuse: bool,
    pub may_write_data: bool,
    pub may_answer_touch: bool,
    /// Hookwars R21.
    pub may_burn: bool,
    pub data_offset: u8,
    pub data_len: u8,
    pub equip_vault: Pubkey,
    /// The Locked slot's program (default otherwise).
    pub locked_program: Pubkey,
}

/// A mint was created with a slot table (after `MintCreated`).
#[event]
pub struct SlotsInitialized {
    pub mint: Pubkey,
    pub slot_authority: Option<Pubkey>,
    pub slots: Vec<SlotInfo>,
}

/// A slot's item changed (an empty included).
#[event]
pub struct SlotEquipped {
    pub mint: Pubkey,
    pub slot: u8,
    pub old_item: Pubkey,
    pub new_item: Pubkey,
    pub program: Pubkey,
    pub flags: u16,
    pub pool_flags: u16,
    pub data_epoch: u8,
    pub ts: i64,
}

/// The armory set a holding's vote lock.
#[event]
pub struct VoteLockSet {
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub owner: Pubkey,
    pub amount: u64,
    pub until: i64,
    pub ts: i64,
}

/// The mint's hook program wrote a holding's hook data through `write_hook_data`.
#[event]
pub struct HookDataWritten {
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub owner: Pubkey,
    pub data: [u8; 64],
}

#[event]
pub struct Minted {
    pub mint: Pubkey,
    pub destination: Pubkey,
    pub destination_owner: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
    pub destination_post: u64,
    pub supply_post: u64,
    pub slot: u64,
    pub ts: i64,
}

#[event]
pub struct Burned {
    pub mint: Pubkey,
    pub source: Pubkey,
    pub source_owner: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
    pub source_post: u64,
    pub supply_post: u64,
    pub slot: u64,
    pub ts: i64,
}

#[event]
pub struct HookSet {
    pub mint: Pubkey,
    pub hook_program: Option<Pubkey>,
    pub hook_flags: u16,
    pub ts: i64,
}

#[event]
pub struct AuthoritySet {
    pub mint: Pubkey,
    pub kind: u8,
    pub new_authority: Option<Pubkey>,
    pub ts: i64,
}

#[event]
pub struct MetadataUpdated {
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub ts: i64,
}

#[event]
pub struct FrozenSet {
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub frozen: bool,
    pub ts: i64,
}

#[event]
pub struct HoldingClosed {
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub owner: Pubkey,
    pub ts: i64,
}

#[event]
pub struct DelegateSet {
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub delegate: Option<Pubkey>,
    pub amount: u64,
    pub ts: i64,
}
