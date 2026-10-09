// Changed by Hookwars: new file (09).
//! Events of `hookwars_agents` (09 section 16), emitted by self-CPI.

use anchor_lang::prelude::*;

use crate::state::{AgentsParams, ConfigChange};

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
    pub fee_collector: Pubkey,
    pub params: AgentsParams,
    pub ts: i64,
}

#[event]
pub struct ParamsProposed {
    pub change: ConfigChange,
    pub ts: i64,
}

#[event]
pub struct ParamsApplied {
    pub change: ConfigChange,
    pub ts: i64,
}

#[event]
pub struct ParamsCancelled {
    pub ts: i64,
}

#[event]
pub struct PassportRegistered {
    pub passport: Pubkey,
    pub operator: Pubkey,
    pub agent_key: Pubkey,
    pub name: String,
    pub kinds: u8,
    pub badge_mint: Pubkey,
    pub ts: i64,
}

#[event]
pub struct BadgeIssued {
    pub passport: Pubkey,
    pub badge_mint: Pubkey,
    pub agent_key: Pubkey,
    pub generation: u8,
    pub ts: i64,
}

#[event]
pub struct ProfileUpdated {
    pub passport: Pubkey,
    pub name: String,
    pub kinds: u8,
    pub ts: i64,
}

#[event]
pub struct PassportStatus {
    pub passport: Pubkey,
    pub status: u8,
    pub ts: i64,
}

#[event]
pub struct AgentKeyRotated {
    pub passport: Pubkey,
    pub old_key: Pubkey,
    pub new_key: Pubkey,
    pub badge_mint: Pubkey,
    pub generation: u8,
    pub ts: i64,
}

#[event]
pub struct LinkAdded {
    pub passport: Pubkey,
    pub platform: u8,
    pub handle: String,
    pub post_uri: String,
    pub statement_hash: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct LinkRemoved {
    pub passport: Pubkey,
    pub platform: u8,
    pub ts: i64,
}

#[event]
pub struct AttestationSubmitted {
    pub passport: Pubkey,
    pub tee_kind: u8,
    pub measurement: [u8; 48],
    pub quote_hash: [u8; 32],
    pub expires_at: i64,
    pub round: u32,
    pub ts: i64,
}

#[event]
pub struct AttestationEndorsed {
    pub passport: Pubkey,
    pub verifier: Pubkey,
    pub quote_hash: [u8; 32],
    pub endorsements: u8,
    pub ts: i64,
}

#[event]
pub struct EndorsementRevoked {
    pub passport: Pubkey,
    pub verifier: Pubkey,
    pub endorsements: u8,
    pub ts: i64,
}

#[event]
pub struct ProofChanged {
    pub passport: Pubkey,
    pub old: u8,
    pub new: u8,
    pub ts: i64,
}

#[event]
pub struct AgentCredited {
    pub passport: Pubkey,
    pub kind: u8,
    pub value: u64,
    pub ts: i64,
}

#[event]
pub struct PolicySet {
    pub passport: Pubkey,
    pub per_action_lamports: u64,
    pub per_day_lamports: u64,
    pub frozen: bool,
    pub ts: i64,
}

#[event]
pub struct PolicySpend {
    pub passport: Pubkey,
    pub target_program: Pubkey,
    pub sol_out: u64,
    pub ts: i64,
}

#[event]
pub struct PolicyWithdraw {
    pub passport: Pubkey,
    pub mint: Option<Pubkey>,
    pub amount: u64,
    pub ts: i64,
}

#[event]
pub struct BondPosted {
    pub bond: Pubkey,
    pub passport: Pubkey,
    pub proposal_a: Pubkey,
    pub proposal_b: Pubkey,
    pub treaty_item: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[event]
pub struct BondReturned {
    pub bond: Pubkey,
    pub passport: Pubkey,
    pub ratified: bool,
    pub amount: u64,
    pub ts: i64,
}

#[event]
pub struct BondForfeited {
    pub bond: Pubkey,
    pub passport: Pubkey,
    pub inbox_a: Pubkey,
    pub inbox_b: Pubkey,
    pub amount: u64,
    pub ts: i64,
}

#[event]
pub struct TreatyHeld {
    pub bond: Pubkey,
    pub passport: Pubkey,
    pub ts: i64,
}

#[event]
pub struct TreatyBroken {
    pub bond: Pubkey,
    pub passport: Pubkey,
    pub ts: i64,
}
