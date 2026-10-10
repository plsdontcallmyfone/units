// Changed by Hookwars: new file (09). TrackedLimit in place of the tracked tuple (app request).
//! Accounts of `hookwars_agents` (09 section 2).

use anchor_lang::prelude::*;

use crate::constants::*;

/// Every parameter of 09 section 13 the program reads (all to set; TEST values in the suites).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AgentsParams {
    /// `NAME_MAX_LEN` (at most [`NAME_CAP`]).
    pub name_max_len: u8,
    /// `URI_MAX_LEN` (at most [`URI_CAP`]).
    pub uri_max_len: u16,
    /// `HANDLE_MAX_LEN` (at most [`HANDLE_CAP`]).
    pub handle_max_len: u8,
    /// `PASSPORT_FEE_LAMPORTS`, to the fee collector; may be 0.
    pub passport_fee_lamports: u64,
    /// `MAX_PASSPORTS_PER_OPERATOR`; 0 means unlimited.
    pub max_passports_per_operator: u32,
    /// `ATTEST_QUORUM`: endorsements needed for `Attested` (at least 1).
    pub attest_quorum: u8,
    /// `ATTEST_MAX_TTL_SECS`.
    pub attest_max_ttl_secs: i64,
    /// `POLICY_DAY_SECS`.
    pub policy_day_secs: i64,
    /// `BOND_LAMPORTS`.
    pub bond_lamports: u64,
    /// `BOND_MIN_PASSPORT_AGE_SECS`.
    pub bond_min_passport_age_secs: i64,
    /// `BOND_CANCEL_GRACE_SECS`.
    pub bond_cancel_grace_secs: i64,
    /// `TREATY_HOLD_SECS`.
    pub treaty_hold_secs: i64,
    /// `ADMIN_TIMELOCK_SECS` for this program's setters (D-9).
    pub admin_timelock_secs: i64,
}

impl AgentsParams {
    /// Within the structural caps and positive where a zero would break a rule.
    pub fn valid(&self) -> bool {
        usize::from(self.name_max_len) <= NAME_CAP
            && self.name_max_len > 0
            && usize::from(self.uri_max_len) <= URI_CAP
            && usize::from(self.handle_max_len) <= HANDLE_CAP
            && self.handle_max_len > 0
            && self.attest_quorum > 0
            && self.attest_max_ttl_secs > 0
            && self.policy_day_secs > 0
            && self.bond_min_passport_age_secs >= 0
            && self.bond_cancel_grace_secs >= 0
            && self.treaty_hold_secs >= 0
            && self.admin_timelock_secs >= 0
    }
}

/// A pending config change (applied after the timelock).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, PartialEq, Eq)]
pub struct ConfigChange {
    pub admin: Pubkey,
    pub fee_collector: Pubkey,
    pub soulbound_item: Pubkey,
    pub params: AgentsParams,
    #[max_len(VERIFIERS_CAP)]
    pub verifiers: Vec<Pubkey>,
    #[max_len(TARGETS_CAP)]
    pub targets: Vec<Pubkey>,
    pub eta: i64,
}

/// `AgentsConfig` at `["agents-config"]` (09 2.1).
#[account]
#[derive(InitSpace, Debug)]
pub struct AgentsConfig {
    pub version: u8,
    pub bump: u8,
    pub signer_bump: u8,
    /// Changes only behind the timelock.
    pub admin: Pubkey,
    /// Receives `PASSPORT_FEE_LAMPORTS`.
    pub fee_collector: Pubkey,
    /// The one shared Soulbound item (template 42) every badge equips; default until set.
    pub soulbound_item: Pubkey,
    pub params: AgentsParams,
    /// Attestation verifiers.
    #[max_len(VERIFIERS_CAP)]
    pub verifiers: Vec<Pubkey>,
    /// Programs a policy vault may call.
    #[max_len(TARGETS_CAP)]
    pub targets: Vec<Pubkey>,
    pub pending: Option<ConfigChange>,
    pub passports: u64,
    pub reserved: [u8; 32],
}

/// On-chain track record (09 6.1). Raids are counted by the indexer only.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackRecord {
    pub items_authored: u32,
    pub items_equipped: u32,
    pub items_forged: u32,
    pub royalty_claims: u32,
    pub royalties_claimed_sol: u64,
    pub treaties_proposed: u32,
    pub treaties_ratified: u32,
    pub treaties_held: u32,
    pub treaties_broken: u32,
    pub bonds_forfeited: u32,
    pub cranks: u32,
    pub crank_value_lamports: u64,
    pub bounties_claimed_lamports: u64,
    pub loot_reveals: u32,
}

/// `Passport` at `["passport", operator, index]` (09 2.2).
#[account]
#[derive(InitSpace, Debug)]
pub struct Passport {
    pub version: u8,
    pub bump: u8,
    pub operator: Pubkey,
    pub index: u32,
    pub agent_key: Pubkey,
    #[max_len(NAME_CAP)]
    pub name: String,
    #[max_len(URI_CAP)]
    pub avatar_uri: String,
    #[max_len(URI_CAP)]
    pub bio_uri: String,
    #[max_len(URI_CAP)]
    pub hire_uri: String,
    pub kinds: u8,
    pub status: u8,
    /// Highest current level (recomputed from `links` and `attested_until`).
    pub proof: u8,
    /// Live `Link` accounts.
    pub links: u8,
    /// While above `now`, the proof is `Attested` (an endorsed, unexpired attestation of this key).
    pub attested_until: i64,
    pub badge_mint: Pubkey,
    pub badge_generation: u8,
    /// Whether the current badge has been minted to the agent key.
    pub badge_issued: bool,
    pub credit_agent_id: Option<[u8; 32]>,
    pub created_at: i64,
    pub last_active_at: i64,
    pub record: TrackRecord,
    pub reserved: [u8; 32],
}

impl Passport {
    /// The proof level at `now`.
    pub fn proof_at(&self, now: i64) -> u8 {
        if self.attested_until > now {
            proof::ATTESTED
        } else if self.links > 0 {
            proof::LINKED
        } else {
            proof::DECLARED
        }
    }
}

/// `AgentKey` at `["agent-key", agent_key]` (09 2.3).
#[account]
#[derive(InitSpace, Debug)]
pub struct AgentKey {
    pub passport: Pubkey,
    pub bump: u8,
}

/// `OperatorIndex` at `["operator", operator]` (09 2.4).
#[account]
#[derive(InitSpace, Debug)]
pub struct OperatorIndex {
    pub operator: Pubkey,
    pub next: u32,
    pub active: u32,
    pub bump: u8,
}

/// `Link` at `["link", passport, platform]` (09 2.5).
#[account]
#[derive(InitSpace, Debug)]
pub struct Link {
    pub passport: Pubkey,
    pub platform: u8,
    #[max_len(HANDLE_CAP)]
    pub handle: String,
    #[max_len(URI_CAP)]
    pub post_uri: String,
    pub statement_hash: [u8; 32],
    pub linked_at: i64,
    pub bump: u8,
}

/// `Attestation` at `["attest", passport]` (09 2.6).
#[account]
#[derive(InitSpace, Debug)]
pub struct Attestation {
    pub passport: Pubkey,
    /// The agent key the report data binds (a rotation leaves this stale).
    pub agent_key: Pubkey,
    pub tee_kind: u8,
    pub measurement: [u8; 48],
    pub report_data: [u8; 64],
    pub nonce: [u8; 32],
    pub quote_hash: [u8; 32],
    #[max_len(URI_CAP)]
    pub quote_uri: String,
    #[max_len(URI_CAP)]
    pub source_uri: String,
    pub submitted_at: i64,
    pub expires_at: i64,
    /// Endorsements of this round.
    pub endorsements: u8,
    /// Incremented on each submission; endorsements of older rounds no longer count.
    pub round: u32,
    pub bump: u8,
}

/// `Endorsement` at `["endorse", attestation, verifier]` (09 2.7).
#[account]
#[derive(InitSpace, Debug)]
pub struct Endorsement {
    pub attestation: Pubkey,
    pub verifier: Pubkey,
    pub quote_hash: [u8; 32],
    /// The attestation round it endorses.
    pub round: u32,
    pub endorsed_at: i64,
    pub bump: u8,
}

/// A tracked mint of a policy (09 2.8), in raw units.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackedMint {
    pub mint: Pubkey,
    pub per_action: u64,
    pub per_day: u64,
    pub spent_today: u64,
}

/// One tracked mint's limits in `PolicyLimits` (a struct, not a tuple, so the IDL builds; same
/// bytes as the former `(mint, per_action, per_day)`).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackedLimit {
    pub mint: Pubkey,
    pub per_action: u64,
    pub per_day: u64,
}

/// Limits an operator sets.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct PolicyLimits {
    pub per_action_lamports: u64,
    pub per_day_lamports: u64,
    pub tracked: Vec<TrackedLimit>,
    pub targets: Vec<Pubkey>,
}

/// `Policy` at `["policy", passport]` (09 2.8).
#[account]
#[derive(InitSpace, Debug)]
pub struct Policy {
    pub passport: Pubkey,
    pub frozen: bool,
    pub per_action_lamports: u64,
    pub per_day_lamports: u64,
    pub day_start: i64,
    pub spent_today: u64,
    #[max_len(TRACKED_CAP)]
    pub tracked: Vec<TrackedMint>,
    #[max_len(TARGETS_CAP)]
    pub targets: Vec<Pubkey>,
    pub bump: u8,
    pub vault_bump: u8,
}

/// `Bond` at `["bond", passport, proposal_a]` (09 2.9); holds its lamports above rent.
#[account]
#[derive(InitSpace, Debug)]
pub struct Bond {
    pub passport: Pubkey,
    pub proposal_a: Pubkey,
    pub proposal_b: Pubkey,
    pub mint_a: Pubkey,
    pub mint_b: Pubkey,
    pub treaty_item: Pubkey,
    pub amount: u64,
    pub posted_at: i64,
    pub ratified_at: i64,
    pub status: u8,
    pub bump: u8,
}

/// `["bond-mark", proposal]`: a proposal is bonded at most once.
#[account]
#[derive(InitSpace, Debug)]
pub struct BondMark {
    pub bond: Pubkey,
    pub bump: u8,
}
