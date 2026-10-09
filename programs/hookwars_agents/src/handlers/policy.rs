// Changed by Hookwars: new file (09); apply_limits visible to directives (11 section 4.3); TrackedLimit for the IDL.
//! The policy wallet (09 section 7): a vault PDA the agent key spends from through `spend`, within
//! per-action and per-day limits and a target allowlist; the operator freezes and withdraws.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::system_program;
use bordrless_token::client as token;
use hookwars_common::ids;

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::*;
use crate::handlers::common::*;
use crate::state::*;

/// Checks `limits` against the config and the caps and writes them into `policy`.
pub(crate) fn apply_limits(policy: &mut Policy, config: &AgentsConfig, limits: &PolicyLimits) -> Result<()> {
    require!(
        limits.tracked.len() <= TRACKED_CAP && limits.targets.len() <= TARGETS_CAP,
        AgentsError::InvalidParams
    );
    for t in &limits.targets {
        require!(config.targets.contains(t), AgentsError::TargetNotAllowed);
    }
    for (i, l) in limits.tracked.iter().enumerate() {
        let m = &l.mint;
        require!(
            *m != ids::BRIDGED_SOL_MINT && !limits.tracked[..i].iter().any(|o| o.mint == *m),
            AgentsError::InvalidParams
        );
    }
    policy.per_action_lamports = limits.per_action_lamports;
    policy.per_day_lamports = limits.per_day_lamports;
    let old = policy.tracked.clone();
    policy.tracked = limits
        .tracked
        .iter()
        .map(|l| TrackedMint {
            mint: l.mint,
            per_action: l.per_action,
            per_day: l.per_day,
            spent_today: old.iter().find(|o| o.mint == l.mint).map_or(0, |o| o.spent_today),
        })
        .collect();
    policy.targets = limits.targets.clone();
    Ok(())
}

// ---- init_policy -----------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct InitPolicy<'info> {
    pub operator: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(init, payer = payer, space = 8 + Policy::INIT_SPACE,
        seeds = [POLICY_SEED, passport.key().as_ref()], bump)]
    pub policy: Box<Account<'info, Policy>>,
    pub system_program: Program<'info, System>,
}

pub fn process_init_policy(ctx: Context<InitPolicy>, limits: PolicyLimits) -> Result<()> {
    let key = ctx.accounts.passport.key();
    let t = now()?;
    let p = &mut ctx.accounts.policy;
    p.passport = key;
    p.frozen = false;
    p.day_start = t;
    p.spent_today = 0;
    p.bump = ctx.bumps.policy;
    p.vault_bump = pda::vault(&key).1;
    apply_limits(p, &ctx.accounts.config, &limits)?;
    emit_cpi!(PolicySet {
        passport: key,
        per_action_lamports: limits.per_action_lamports,
        per_day_lamports: limits.per_day_lamports,
        frozen: false,
        ts: t
    });
    Ok(())
}

// ---- set_limits and freeze_policy ------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct OperatorPolicy<'info> {
    pub operator: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, seeds = [POLICY_SEED, passport.key().as_ref()], bump = policy.bump)]
    pub policy: Box<Account<'info, Policy>>,
}

pub fn process_set_limits(ctx: Context<OperatorPolicy>, limits: PolicyLimits) -> Result<()> {
    let key = ctx.accounts.passport.key();
    let p = &mut ctx.accounts.policy;
    apply_limits(p, &ctx.accounts.config, &limits)?;
    let frozen = p.frozen;
    emit_cpi!(PolicySet {
        passport: key,
        per_action_lamports: limits.per_action_lamports,
        per_day_lamports: limits.per_day_lamports,
        frozen,
        ts: now()?
    });
    Ok(())
}

pub fn process_freeze(ctx: Context<OperatorPolicy>, frozen: bool) -> Result<()> {
    let key = ctx.accounts.passport.key();
    let p = &mut ctx.accounts.policy;
    p.frozen = frozen;
    let (a, d) = (p.per_action_lamports, p.per_day_lamports);
    emit_cpi!(PolicySet {
        passport: key,
        per_action_lamports: a,
        per_day_lamports: d,
        frozen,
        ts: now()?
    });
    Ok(())
}

// ---- withdraw --------------------------------------------------------------------------------

/// Accounts of `withdraw`. For a token, the remaining accounts are `[mint, vault holding,
/// operator holding, the mint's hook accounts...]`.
#[event_cpi]
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    #[account(has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(seeds = [POLICY_SEED, passport.key().as_ref()], bump = policy.bump)]
    pub policy: Box<Account<'info, Policy>>,
    /// CHECK: the vault (`["agent-vault", passport]`, system-owned, signs by CPI).
    #[account(mut, seeds = [VAULT_SEED, passport.key().as_ref()], bump = policy.vault_bump)]
    pub vault: UncheckedAccount<'info>,
    pub token: TokenAccs<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_withdraw<'info>(
    ctx: Context<'info, Withdraw<'info>>,
    amount: u64,
    mint: Option<Pubkey>,
) -> Result<()> {
    let key = ctx.accounts.passport.key();
    let bump = [ctx.accounts.policy.vault_bump];
    let seeds: [&[u8]; 3] = [VAULT_SEED, key.as_ref(), &bump];
    match mint {
        None => {
            system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.key(),
                    system_program::Transfer {
                        from: ctx.accounts.vault.to_account_info(),
                        to: ctx.accounts.operator.to_account_info(),
                    },
                    &[&seeds],
                ),
                amount,
            )?;
        }
        Some(m) => {
            let r = ctx.remaining_accounts;
            require!(r.len() >= 3 && *r[0].key == m, AgentsError::WrongAccount);
            let vault = ctx.accounts.vault.key();
            require_keys_eq!(*r[1].key, token::holding_address(&m, &vault), AgentsError::WrongAccount);
            let extras: Vec<AccountMeta> = r[3..]
                .iter()
                .map(|a| AccountMeta {
                    pubkey: *a.key,
                    is_signer: false,
                    is_writable: a.is_writable,
                })
                .collect();
            let ix = token::transfer(vault, *r[1].key, *r[2].key, m, None, extras, amount);
            let mut infos: Vec<AccountInfo> = r.to_vec();
            infos.push(ctx.accounts.vault.to_account_info());
            infos.push(ctx.accounts.token.token_event_authority.to_account_info());
            infos.push(ctx.accounts.token.token_program.to_account_info());
            cpi(&ix, &infos, &[&seeds])?;
        }
    }
    emit_cpi!(PolicyWithdraw {
        passport: key,
        mint,
        amount,
        ts: now()?
    });
    Ok(())
}

// ---- spend -----------------------------------------------------------------------------------

/// Accounts of `spend`; the target instruction's accounts follow as remaining accounts, in its
/// order (the vault among them, marked a signer by this program).
#[event_cpi]
#[derive(Accounts)]
pub struct Spend<'info> {
    pub agent_key: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(constraint = passport.agent_key == agent_key.key() @ AgentsError::NotThisAgent)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, seeds = [POLICY_SEED, passport.key().as_ref()], bump = policy.bump)]
    pub policy: Box<Account<'info, Policy>>,
    /// CHECK: the vault.
    #[account(mut, seeds = [VAULT_SEED, passport.key().as_ref()], bump = policy.vault_bump)]
    pub vault: UncheckedAccount<'info>,
    /// CHECK: the program called (checked against both allowlists).
    pub target_program: UncheckedAccount<'info>,
}

/// A vault holding seen before the call.
struct Seen {
    index: usize,
    mint: Pubkey,
    amount: u64,
}

/// The vault holdings among `accounts` that are writable, with their amounts.
fn vault_holdings(accounts: &[AccountInfo], vault: &Pubkey) -> Vec<Seen> {
    let mut out = Vec::new();
    for (index, a) in accounts.iter().enumerate() {
        if !a.is_writable || *a.owner != bordrless_token::ID {
            continue;
        }
        if let Ok(h) = token::read_holding(a) {
            if h.owner == *vault && !out.iter().any(|s: &Seen| accounts[s.index].key == a.key) {
                out.push(Seen {
                    index,
                    mint: h.mint,
                    amount: h.amount,
                });
            }
        }
    }
    out
}

pub fn process_spend<'info>(ctx: Context<'info, Spend<'info>>, data: Vec<u8>) -> Result<()> {
    let key = ctx.accounts.passport.key();
    require!(
        ctx.accounts.passport.status == status::ACTIVE,
        AgentsError::AgentPaused
    );
    require!(!ctx.accounts.policy.frozen, AgentsError::PolicyFrozen);
    let target = ctx.accounts.target_program.key();
    require!(
        target != crate::ID
            && target != anchor_lang::system_program::ID
            && !LOADERS.contains(&target)
            && ctx.accounts.policy.targets.contains(&target)
            && ctx.accounts.config.targets.contains(&target),
        AgentsError::TargetNotAllowed
    );
    let vault = ctx.accounts.vault.key();
    let r = ctx.remaining_accounts;
    let before = vault_holdings(r, &vault);
    for s in &before {
        require!(
            s.mint == ids::BRIDGED_SOL_MINT
                || ctx.accounts.policy.tracked.iter().any(|t| t.mint == s.mint),
            AgentsError::UntrackedHolding
        );
    }
    let lamports_before = ctx.accounts.vault.lamports();
    let accounts = r
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: a.is_signer || *a.key == vault,
            is_writable: a.is_writable,
        })
        .collect();
    let ix = Instruction {
        program_id: target,
        accounts,
        data,
    };
    let mut infos: Vec<AccountInfo> = r.to_vec();
    infos.push(ctx.accounts.vault.to_account_info());
    infos.push(ctx.accounts.target_program.to_account_info());
    let bump = [ctx.accounts.policy.vault_bump];
    cpi(&ix, &infos, &[&[VAULT_SEED, key.as_ref(), &bump]])?;

    // Measure what left the vault; increases are not counted.
    let mut sol_out = lamports_before.saturating_sub(ctx.accounts.vault.lamports());
    let t = now()?;
    let p = &mut ctx.accounts.policy;
    let day = ctx.accounts.config.params.policy_day_secs;
    if t >= p.day_start.saturating_add(day) {
        p.day_start = t;
        p.spent_today = 0;
        for m in p.tracked.iter_mut() {
            m.spent_today = 0;
        }
    }
    for s in &before {
        let after = token::read_holding(&r[s.index]).map(|h| h.amount).unwrap_or(0);
        let out = s.amount.saturating_sub(after);
        if s.mint == ids::BRIDGED_SOL_MINT {
            sol_out = sol_out.saturating_add(out);
        } else if out > 0 {
            let m = p
                .tracked
                .iter_mut()
                .find(|m| m.mint == s.mint)
                .ok_or(AgentsError::UntrackedHolding)?;
            m.spent_today = m.spent_today.saturating_add(out);
            require!(
                out <= m.per_action && m.spent_today <= m.per_day,
                AgentsError::LimitExceeded
            );
        }
    }
    p.spent_today = p.spent_today.saturating_add(sol_out);
    require!(
        sol_out <= p.per_action_lamports && p.spent_today <= p.per_day_lamports,
        AgentsError::LimitExceeded
    );
    emit_cpi!(PolicySpend {
        passport: key,
        target_program: target,
        sol_out,
        ts: t
    });
    Ok(())
}
