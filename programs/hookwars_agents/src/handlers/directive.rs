// Changed by Hookwars: new file (hook economy, docs/spec/11-hook-economy.md section 4).
//! Memos carry words, accounts carry commitments (R41).
//!
//! - `set_directive`: an operator programs its agent. The same transaction must hold an SPL Memo
//!   instruction with a canonical version 1 `directive` message for this passport and sequence,
//!   signed by the operator. The program reads that memo through the instructions sysvar, stores
//!   its sha256 in a `Directive` account, and writes the directive's enforceable constraints into
//!   the agent's `Policy` (spend limits, targets, frozen), where `spend` already enforces them. The
//!   rest of the directive is for the agent's runtime, public, and checkable against its behaviour.
//! - `commit`: two agents bind an accepted offer by hash before settling (moves no money).
//! - `post`: optional postage on a message, paid to the fee collector, so the site can rank it.
//! - `init_memo_config` / `propose_memo_config` / `apply_memo_config`: the memo parameters
//!   (`MEMO_MAX_BYTES`, `POSTAGE_LAMPORTS`, `MEMO_MIN_PROOF`), admin, behind the timelock.

use anchor_lang::prelude::*;
use anchor_lang::system_program;
use hookwars_common::economy::MEMO_PROGRAM_ID;

use crate::constants::*;
use crate::error::AgentsError;

/// This module's errors are `AgentsError` variants (one error enum per program, so the IDL builds).
pub type DirectiveError = AgentsError;
use crate::handlers::common::*;
use crate::handlers::policy::apply_limits;
use crate::state::*;

/// Seeds of this module.
pub const DIRECTIVE_SEED: &[u8] = b"directive";
pub const COMMIT_SEED: &[u8] = b"commit";
pub const MEMO_CONFIG_SEED: &[u8] = b"memo-config";

/// Memo parameters (11 section 11; all to set, TEST values in the suites).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MemoParams {
    /// `MEMO_MAX_BYTES`: the longest memo `set_directive` reads.
    pub memo_max_bytes: u16,
    /// `POSTAGE_LAMPORTS`: paid by `post`.
    pub postage_lamports: u64,
    /// `MEMO_MIN_PROOF`: the proof level the site shows by default (read off chain).
    pub min_proof: u8,
}

/// `MemoConfig` at `["memo-config"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct MemoConfig {
    pub bump: u8,
    pub params: MemoParams,
    pub pending: Option<MemoParams>,
    pub pending_eta: i64,
}

/// What a directive makes the chain enforce (11 section 4.3).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectiveConstraints {
    /// Written into `Policy.per_action_lamports`.
    pub max_spend_per_action: u64,
    /// Written into `Policy.per_day_lamports`.
    pub max_spend_per_day: u64,
    /// Written into `Policy.targets` (each must be allowed by the config).
    #[max_len(TARGETS_CAP)]
    pub allowed_targets: Vec<Pubkey>,
    /// Access modes (bit per mode, 11 section 1.1) the agent may set on items it holds; read by
    /// the armory's `set_access` (integration request).
    pub allowed_access_modes: u8,
    /// Highest licence price the agent may set; read by the armory's `set_access`.
    pub max_licence_price: u64,
    /// Written into `Policy.frozen`.
    pub frozen: bool,
}

/// `Directive` at `["directive", passport, seq]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Directive {
    pub passport: Pubkey,
    pub seq: u32,
    /// sha256 of the memo bytes (R41).
    pub memo_hash: [u8; 32],
    pub constraints: DirectiveConstraints,
    pub posted_at: i64,
    /// The next directive's sequence, once one replaces this.
    pub superseded_by: Option<u32>,
    pub bump: u8,
}

/// `Commitment` at `["commit", passport, ref]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Commitment {
    pub passport: Pubkey,
    pub reference: [u8; 32],
    pub hash: [u8; 32],
    pub committed_at: i64,
    pub bump: u8,
}


#[event]
pub struct DirectiveSet {
    pub passport: Pubkey,
    pub seq: u32,
    pub memo_hash: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct Committed {
    pub passport: Pubkey,
    pub reference: [u8; 32],
    pub hash: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct MessagePosted {
    pub passport: Pubkey,
    pub reference: [u8; 32],
    pub postage: u64,
    pub ts: i64,
}

#[event]
pub struct MemoParamsProposed {
    pub eta: i64,
}

fn check_memo_params(p: &MemoParams) -> Result<()> {
    // The memo must leave room for signatures and accounts in a 1,232-byte transaction.
    require!(
        p.memo_max_bytes > 0 && p.memo_max_bytes <= 1_000 && p.min_proof <= proof::ATTESTED,
        DirectiveError::BadMemoParams
    );
    Ok(())
}

/// Pass 5 (review 3 L-6): lowercase hex of `sha256(borsh(constraints))`, the memo's `c`.
pub fn constraints_hash_hex(constraints: &DirectiveConstraints) -> Result<String> {
    let bytes = constraints.try_to_vec()?;
    let h = sha256(&bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in h {
        out.push(char::from(HEX[usize::from(b >> 4)]));
        out.push(char::from(HEX[usize::from(b & 15)]));
    }
    Ok(out)
}

/// Finds, in this transaction, a memo instruction that is the canonical directive for
/// `passport`/`seq` and lists `operator` as a signer; returns its sha256. Pass 5 (L-6): the
/// memo's `c` must be `constraints_c`, the hash of the constraints this call writes.
pub fn find_directive_memo(
    sysvar: &AccountInfo,
    passport: &Pubkey,
    operator: &Pubkey,
    seq: u32,
    max_bytes: usize,
    constraints_c: &str,
) -> Result<[u8; 32]> {
    let n = {
        let data = sysvar.try_borrow_data()?;
        require!(data.len() >= 2, AgentsError::WrongAccount);
        usize::from(u16::from_le_bytes([data[0], data[1]]))
    };
    let pp = passport.to_string();
    let mut saw_memo = false;
    for i in 0..n {
        let ix = solana_instructions_sysvar::load_instruction_at_checked(i, sysvar)?;
        if ix.program_id != MEMO_PROGRAM_ID {
            continue;
        }
        let Ok(m) = units_memo::Message::parse(&ix.data, max_bytes) else {
            continue;
        };
        if m.kind != units_memo::kind::DIRECTIVE {
            continue;
        }
        saw_memo = true;
        let Ok(body) = units_memo::DirectiveBody::from_value(&m.body) else {
            continue;
        };
        if body.passport != pp || m.from != pp || body.seq != u64::from(seq) {
            continue;
        }
        require!(body.c == constraints_c, DirectiveError::MemoMismatch);
        require!(
            ix.accounts.iter().any(|a| a.pubkey == *operator && a.is_signer),
            DirectiveError::MemoNotSigned
        );
        return Ok(sha256(&ix.data));
    }
    if saw_memo {
        return err!(DirectiveError::MemoMismatch);
    }
    err!(DirectiveError::DirectiveMemoMissing)
}

pub fn process_set_directive(
    ctx: Context<SetDirective>,
    seq: u32,
    constraints: DirectiveConstraints,
) -> Result<()> {
    let passport_key = ctx.accounts.passport.key();
    // The sequence: 0 starts, every next one names and supersedes the previous.
    match &mut ctx.accounts.previous {
        None => require!(seq == 0, DirectiveError::BadSeq),
        Some(prev) => {
            require!(
                prev.passport == passport_key
                    && seq > 0
                    && prev.seq == seq - 1
                    && prev.superseded_by.is_none(),
                DirectiveError::BadSeq
            );
            prev.superseded_by = Some(seq);
        }
    }
    let memo_hash = find_directive_memo(
        &ctx.accounts.instructions.to_account_info(),
        &passport_key,
        &ctx.accounts.operator.key(),
        seq,
        usize::from(ctx.accounts.memo_config.params.memo_max_bytes),
        &constraints_hash_hex(&constraints)?,
    )?;
    // The enforceable part goes into the policy, where `spend` already checks it.
    let limits = PolicyLimits {
        per_action_lamports: constraints.max_spend_per_action,
        per_day_lamports: constraints.max_spend_per_day,
        tracked: ctx
            .accounts
            .policy
            .tracked
            .iter()
            .map(|t| TrackedLimit {
                mint: t.mint,
                per_action: t.per_action,
                per_day: t.per_day,
            })
            .collect(),
        targets: constraints.allowed_targets.clone(),
    };
    apply_limits(&mut ctx.accounts.policy, &ctx.accounts.config, &limits)?;
    ctx.accounts.policy.frozen = constraints.frozen;
    let t = now()?;
    let d = &mut ctx.accounts.directive;
    d.passport = passport_key;
    d.seq = seq;
    d.memo_hash = memo_hash;
    d.constraints = constraints;
    d.posted_at = t;
    d.superseded_by = None;
    d.bump = ctx.bumps.directive;
    emit_cpi!(DirectiveSet {
        passport: passport_key,
        seq,
        memo_hash,
        ts: t
    });
    Ok(())
}

pub fn process_commit(ctx: Context<Commit>, reference: [u8; 32], hash: [u8; 32]) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.passport.agent_key,
        ctx.accounts.agent.key(),
        DirectiveError::NotAgent
    );
    let t = now()?;
    let c = &mut ctx.accounts.commitment;
    c.passport = ctx.accounts.passport.key();
    c.reference = reference;
    c.hash = hash;
    c.committed_at = t;
    c.bump = ctx.bumps.commitment;
    let passport = c.passport;
    emit_cpi!(Committed {
        passport,
        reference,
        hash,
        ts: t
    });
    Ok(())
}

pub fn process_post(ctx: Context<Post>, reference: [u8; 32]) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.passport.agent_key,
        ctx.accounts.agent.key(),
        DirectiveError::NotAgent
    );
    let postage = ctx.accounts.memo_config.params.postage_lamports;
    if postage > 0 {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                system_program::Transfer {
                    from: ctx.accounts.agent.to_account_info(),
                    to: ctx.accounts.fee_collector.to_account_info(),
                },
            ),
            postage,
        )?;
    }
    emit_cpi!(MessagePosted {
        passport: ctx.accounts.passport.key(),
        reference,
        postage,
        ts: now()?
    });
    Ok(())
}

pub fn process_init_memo_config(ctx: Context<InitMemoConfig>, params: MemoParams) -> Result<()> {
    check_memo_params(&params)?;
    let c = &mut ctx.accounts.memo_config;
    c.bump = ctx.bumps.memo_config;
    c.params = params;
    c.pending = None;
    c.pending_eta = 0;
    Ok(())
}

pub fn process_propose_memo_config(ctx: Context<ProposeMemoConfig>, params: MemoParams) -> Result<()> {
    check_memo_params(&params)?;
    let eta = now()?
        .checked_add(ctx.accounts.config.params.admin_timelock_secs)
        .ok_or(AgentsError::MathOverflow)?;
    let c = &mut ctx.accounts.memo_config;
    c.pending = Some(params);
    c.pending_eta = eta;
    emit_cpi!(MemoParamsProposed { eta });
    Ok(())
}

pub fn process_apply_memo_config(ctx: Context<ApplyMemoConfig>) -> Result<()> {
    let c = &mut ctx.accounts.memo_config;
    let p = c.pending.ok_or(AgentsError::NothingPending)?;
    require!(now()? >= c.pending_eta, AgentsError::TimelockActive);
    c.params = p;
    c.pending = None;
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(seq: u32)]
pub struct SetDirective<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(seeds = [MEMO_CONFIG_SEED], bump = memo_config.bump)]
    pub memo_config: Box<Account<'info, MemoConfig>>,
    #[account(has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, seeds = [POLICY_SEED, passport.key().as_ref()], bump = policy.bump)]
    pub policy: Box<Account<'info, Policy>>,
    #[account(init, payer = operator, space = 8 + Directive::INIT_SPACE,
        seeds = [DIRECTIVE_SEED, passport.key().as_ref(), &seq.to_le_bytes()], bump)]
    pub directive: Box<Account<'info, Directive>>,
    /// The directive this one replaces (absent for sequence 0).
    #[account(mut, seeds = [DIRECTIVE_SEED, passport.key().as_ref(), &seq.wrapping_sub(1).to_le_bytes()], bump = previous.bump)]
    pub previous: Option<Box<Account<'info, Directive>>>,
    /// CHECK: the instructions sysvar.
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(reference: [u8; 32])]
pub struct Commit<'info> {
    #[account(mut)]
    pub agent: Signer<'info>,
    pub passport: Box<Account<'info, Passport>>,
    #[account(init, payer = agent, space = 8 + Commitment::INIT_SPACE,
        seeds = [COMMIT_SEED, passport.key().as_ref(), &reference], bump)]
    pub commitment: Box<Account<'info, Commitment>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Post<'info> {
    #[account(mut)]
    pub agent: Signer<'info>,
    pub passport: Box<Account<'info, Passport>>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(seeds = [MEMO_CONFIG_SEED], bump = memo_config.bump)]
    pub memo_config: Box<Account<'info, MemoConfig>>,
    /// CHECK: the config's fee collector.
    #[account(mut, address = config.fee_collector @ AgentsError::WrongAccount)]
    pub fee_collector: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitMemoConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ AgentsError::NotAdmin)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(init, payer = admin, space = 8 + MemoConfig::INIT_SPACE, seeds = [MEMO_CONFIG_SEED], bump)]
    pub memo_config: Box<Account<'info, MemoConfig>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeMemoConfig<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ AgentsError::NotAdmin)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, seeds = [MEMO_CONFIG_SEED], bump = memo_config.bump)]
    pub memo_config: Box<Account<'info, MemoConfig>>,
}

#[derive(Accounts)]
pub struct ApplyMemoConfig<'info> {
    #[account(mut, seeds = [MEMO_CONFIG_SEED], bump = memo_config.bump)]
    pub memo_config: Box<Account<'info, MemoConfig>>,
}

/// `["directive", passport, seq]`.
pub fn directive_address(passport: &Pubkey, seq: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[DIRECTIVE_SEED, passport.as_ref(), &seq.to_le_bytes()], &crate::ID)
}

/// `["commit", passport, ref]`.
pub fn commitment_address(passport: &Pubkey, reference: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[COMMIT_SEED, passport.as_ref(), reference], &crate::ID)
}

/// `["memo-config"]`.
pub fn memo_config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MEMO_CONFIG_SEED], &crate::ID)
}

/// The live directive's constraints when `info` is a current (not superseded) `Directive` of
/// `passport` owned by this program; for the armory's `set_access` (integration request).
pub fn live_constraints(info: &AccountInfo, passport: &Pubkey) -> Result<Option<DirectiveConstraints>> {
    if *info.owner != crate::ID || info.data_is_empty() {
        return Ok(None);
    }
    let data = info.try_borrow_data()?;
    let d = Directive::try_deserialize(&mut &data[..])?;
    if d.passport != *passport || d.superseded_by.is_some() {
        return Ok(None);
    }
    Ok(Some(d.constraints))
}
