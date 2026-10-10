// Changed by Hookwars: new program (09); directives, commits and postage (11 section 4).
//! `hookwars_agents`: an identity for every AI agent that takes part in units
//! (docs/spec/09-agents.md). A passport per agent, a soulbound badge that is itself a units token,
//! three proof levels, a track record other programs credit, a policy wallet with limits the chain
//! enforces, and diplomat bonds on treaties. Agents never operate a token's rules (R25): a
//! passport grants no slot authority, no vote weight, no equip right and no access to any chest.
//!
//! Layout: [`constants`], [`state`], [`error`], [`events`], [`handlers`] (one module per area).

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod events;
pub mod handlers;
pub mod state;

pub use handlers::*;
use state::{AgentsParams, PolicyLimits};

declare_id!("GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "units agents",
    project_url: "https://github.com/plsdontcallmyfone/units",
    contacts: "link:https://github.com/plsdontcallmyfone/units/security/advisories/new",
    policy: "https://github.com/plsdontcallmyfone/units/blob/main/SECURITY.md",
    source_code: "https://github.com/plsdontcallmyfone/units"
}

#[program]
pub mod hookwars_agents {
    use super::*;

    // ---- config and timelock (2.1, 13)

    /// Creates the config, once, by this program's upgrade authority.
    pub fn init_config(ctx: Context<InitConfig>, args: ConfigArgs) -> Result<()> {
        admin::process_init_config(ctx, args)
    }

    /// The admin proposes a config change, applied after `admin_timelock_secs`.
    pub fn propose_config(ctx: Context<ProposeConfig>, args: ConfigArgs) -> Result<()> {
        admin::process_propose_config(ctx, args)
    }

    /// Anyone applies the pending change after its eta.
    pub fn apply_config(ctx: Context<ApplyConfig>) -> Result<()> {
        admin::process_apply_config(ctx)
    }

    /// The admin cancels the pending change.
    pub fn cancel_config(ctx: Context<CancelConfig>) -> Result<()> {
        admin::process_cancel_config(ctx)
    }

    // ---- passports and badges (sections 3 and 4)

    /// Registers a passport (operator and agent key sign) and creates its badge mint.
    pub fn register_passport(
        ctx: Context<RegisterPassport>,
        index: u32,
        args: ProfileArgs,
    ) -> Result<()> {
        passport::process_register(ctx, index, args)
    }

    /// Equips the shared Soulbound item into the badge's slot through the armory, signing as
    /// `["armory-caller", badge_mint]` (R28: needs the armory to accept this caller).
    pub fn equip_badge<'info>(ctx: Context<'info, EquipBadge<'info>>) -> Result<()> {
        passport::process_equip_badge(ctx)
    }

    /// Mints the badge to the agent key once its slot holds the Soulbound item, then revokes the
    /// mint authority.
    pub fn issue_badge(ctx: Context<IssueBadge>) -> Result<()> {
        passport::process_issue_badge(ctx)
    }

    /// The operator changes the profile.
    pub fn update_profile(ctx: Context<UpdateProfile>, args: ProfileArgs) -> Result<()> {
        passport::process_update_profile(ctx, args)
    }

    /// The operator pauses, resumes or retires the passport.
    pub fn set_status(ctx: Context<SetStatus>, status: u8) -> Result<()> {
        passport::process_set_status(ctx, status)
    }

    /// The operator and a new key move the passport to that key; a new badge generation.
    pub fn rotate_agent_key(ctx: Context<RotateAgentKey>) -> Result<()> {
        passport::process_rotate(ctx)
    }

    // ---- proof levels (section 5)

    /// Links a social account: an ed25519 instruction in the same transaction signs the
    /// statement with the agent key.
    pub fn link_social(
        ctx: Context<LinkSocial>,
        platform: u8,
        handle: String,
        post_uri: String,
    ) -> Result<()> {
        proof::process_link_social(ctx, platform, handle, post_uri)
    }

    /// The operator or the agent key removes a link.
    pub fn unlink(ctx: Context<Unlink>, platform: u8) -> Result<()> {
        proof::process_unlink(ctx, platform)
    }

    /// The agent key submits (or replaces) its attestation.
    pub fn submit_attestation(ctx: Context<SubmitAttestation>, args: AttestArgs) -> Result<()> {
        proof::process_submit_attestation(ctx, args)
    }

    /// A registered verifier endorses the current attestation.
    pub fn endorse_attestation(ctx: Context<EndorseAttestation>) -> Result<()> {
        proof::process_endorse(ctx)
    }

    /// The verifier revokes its endorsement.
    pub fn revoke_endorsement(ctx: Context<RevokeEndorsement>) -> Result<()> {
        proof::process_revoke(ctx)
    }

    /// Anyone recomputes the proof level (expiry).
    pub fn refresh_proof(ctx: Context<RefreshProof>) -> Result<()> {
        proof::process_refresh(ctx)
    }

    // ---- track record (section 6)

    /// Credits the agent: CPI only, from `["agents-caller"]` under the armory, war or items.
    pub fn record(ctx: Context<Record>, kind: u8, value: u64) -> Result<()> {
        record::process_record(ctx, kind, value)
    }

    // ---- policy wallet (section 7)

    /// The operator creates the policy.
    pub fn init_policy(ctx: Context<InitPolicy>, limits: PolicyLimits) -> Result<()> {
        policy::process_init_policy(ctx, limits)
    }

    /// The operator changes the limits.
    pub fn set_limits(ctx: Context<OperatorPolicy>, limits: PolicyLimits) -> Result<()> {
        policy::process_set_limits(ctx, limits)
    }

    /// The operator freezes or unfreezes the vault.
    pub fn freeze_policy(ctx: Context<OperatorPolicy>, frozen: bool) -> Result<()> {
        policy::process_freeze(ctx, frozen)
    }

    /// The operator withdraws SOL (`mint` None) or a token from the vault, any time.
    pub fn withdraw<'info>(
        ctx: Context<'info, Withdraw<'info>>,
        amount: u64,
        mint: Option<Pubkey>,
    ) -> Result<()> {
        policy::process_withdraw(ctx, amount, mint)
    }

    /// The agent key calls an allowed program with the vault signing, within the limits.
    pub fn spend<'info>(ctx: Context<'info, Spend<'info>>, data: Vec<u8>) -> Result<()> {
        policy::process_spend(ctx, data)
    }

    /// The operator clears a delegate on one of the vault's holdings (review 3 H-1).
    pub fn revoke_vault(ctx: Context<RevokeVault>) -> Result<()> {
        policy::process_revoke_vault(ctx)
    }

    // ---- diplomat bonds (section 8)

    /// The agent posts a bond tying two treaty proposals.
    pub fn post_bond(ctx: Context<PostBond>) -> Result<()> {
        bonds::process_post_bond(ctx)
    }

    /// Anyone resolves a bond once its proposals are final.
    pub fn resolve_bond(ctx: Context<ResolveBond>) -> Result<()> {
        bonds::process_resolve_bond(ctx)
    }

    /// Anyone marks a ratified treaty held or broken.
    pub fn mark_treaty_outcome(ctx: Context<MarkTreatyOutcome>) -> Result<()> {
        bonds::process_mark_outcome(ctx)
    }

    // ---- directives, commits and postage (11 section 4, R41)

    /// Creates the memo parameters, once, by the config admin.
    pub fn init_memo_config(ctx: Context<InitMemoConfig>, params: MemoParams) -> Result<()> {
        directive::process_init_memo_config(ctx, params)
    }

    /// Proposes new memo parameters (applied after the admin timelock).
    pub fn propose_memo_config(ctx: Context<ProposeMemoConfig>, params: MemoParams) -> Result<()> {
        directive::process_propose_memo_config(ctx, params)
    }

    /// Applies proposed memo parameters after the timelock (anyone).
    pub fn apply_memo_config(ctx: Context<ApplyMemoConfig>) -> Result<()> {
        directive::process_apply_memo_config(ctx)
    }

    /// The operator programs its agent: a `Directive` bound by hash to a memo in this transaction,
    /// its constraints written into the policy wallet.
    pub fn set_directive(ctx: Context<SetDirective>, seq: u32, constraints: DirectiveConstraints) -> Result<()> {
        directive::process_set_directive(ctx, seq, constraints)
    }

    /// The agent binds an accepted offer by hash.
    pub fn commit(ctx: Context<Commit>, reference: [u8; 32], hash: [u8; 32]) -> Result<()> {
        directive::process_commit(ctx, reference, hash)
    }

    /// The agent pays postage on a message.
    pub fn post(ctx: Context<Post>, reference: [u8; 32]) -> Result<()> {
        directive::process_post(ctx, reference)
    }
}

/// Arguments of `init_config` and `propose_config`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct ConfigArgs {
    pub admin: Pubkey,
    pub fee_collector: Pubkey,
    pub soulbound_item: Pubkey,
    pub params: AgentsParams,
    pub verifiers: Vec<Pubkey>,
    pub targets: Vec<Pubkey>,
}

/// Arguments of `register_passport` and `update_profile`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default)]
pub struct ProfileArgs {
    pub name: String,
    pub avatar_uri: String,
    pub bio_uri: String,
    pub hire_uri: String,
    pub kinds: u8,
    pub credit_agent_id: Option<[u8; 32]>,
}

/// Arguments of `submit_attestation`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct AttestArgs {
    pub tee_kind: u8,
    pub measurement: [u8; 48],
    pub report_data: [u8; 64],
    pub nonce: [u8; 32],
    pub quote_hash: [u8; 32],
    pub quote_uri: String,
    pub source_uri: String,
    pub expires_at: i64,
}
