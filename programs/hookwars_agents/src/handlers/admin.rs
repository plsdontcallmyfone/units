// Changed by Hookwars: new file (09).
//! The config and the timelock (09 2.1, D-9). Nothing the admin does applies at once.

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::*;
use crate::handlers::common::now;
use crate::state::*;
use crate::ConfigArgs;

/// The upgrade authority of `program_id` from its ProgramData account (loader v3 layout).
pub fn upgrade_authority(program_data: &AccountInfo, program_id: &Pubkey) -> Result<Option<Pubkey>> {
    let (expected, _) = Pubkey::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID);
    require_keys_eq!(*program_data.key, expected, AgentsError::NotUpgradeAuthority);
    require_keys_eq!(*program_data.owner, BPF_LOADER_UPGRADEABLE_ID, AgentsError::NotUpgradeAuthority);
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        AgentsError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&data[13..45]);
    Ok(Some(Pubkey::new_from_array(key)))
}

impl ConfigArgs {
    fn check(&self) -> Result<()> {
        require!(self.params.valid(), AgentsError::InvalidParams);
        require!(
            self.verifiers.len() <= VERIFIERS_CAP && self.targets.len() <= TARGETS_CAP,
            AgentsError::InvalidParams
        );
        require!(
            usize::from(self.params.attest_quorum) <= self.verifiers.len().max(1),
            AgentsError::InvalidParams
        );
        for t in &self.targets {
            require!(
                *t != crate::ID && *t != anchor_lang::system_program::ID && !LOADERS.contains(t),
                AgentsError::TargetNotAllowed
            );
        }
        Ok(())
    }

    fn change(&self, eta: i64) -> ConfigChange {
        ConfigChange {
            admin: self.admin,
            fee_collector: self.fee_collector,
            soulbound_item: self.soulbound_item,
            params: self.params,
            verifiers: self.verifiers.clone(),
            targets: self.targets.clone(),
            eta,
        }
    }
}

/// Accounts of `init_config`.
#[event_cpi]
#[derive(Accounts)]
pub struct InitConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + AgentsConfig::INIT_SPACE, seeds = [CONFIG_SEED], bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_init_config(ctx: Context<InitConfig>, args: ConfigArgs) -> Result<()> {
    require!(
        upgrade_authority(&ctx.accounts.program_data, &crate::ID)? == Some(ctx.accounts.authority.key()),
        AgentsError::NotUpgradeAuthority
    );
    args.check()?;
    let c = &mut ctx.accounts.config;
    c.version = VERSION;
    c.bump = ctx.bumps.config;
    c.signer_bump = pda::signer().1;
    c.admin = args.admin;
    c.fee_collector = args.fee_collector;
    c.soulbound_item = args.soulbound_item;
    c.params = args.params;
    c.verifiers = args.verifiers.clone();
    c.targets = args.targets.clone();
    c.pending = None;
    c.passports = 0;
    c.reserved = [0; 32];
    emit_cpi!(ConfigInitialized {
        admin: args.admin,
        fee_collector: args.fee_collector,
        params: args.params,
        ts: now()?
    });
    Ok(())
}

/// Accounts of `propose_config` and `cancel_config`.
#[event_cpi]
#[derive(Accounts)]
pub struct ProposeConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ AgentsError::NotAdmin)]
    pub config: Box<Account<'info, AgentsConfig>>,
}

pub fn process_propose_config(ctx: Context<ProposeConfig>, args: ConfigArgs) -> Result<()> {
    args.check()?;
    let t = now()?;
    let eta = t
        .checked_add(ctx.accounts.config.params.admin_timelock_secs)
        .ok_or(AgentsError::MathOverflow)?;
    let change = args.change(eta);
    ctx.accounts.config.pending = Some(change.clone());
    emit_cpi!(ParamsProposed { change, ts: t });
    Ok(())
}

/// Accounts of `apply_config` (anyone).
#[event_cpi]
#[derive(Accounts)]
pub struct ApplyConfig<'info> {
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
}

pub fn process_apply_config(ctx: Context<ApplyConfig>) -> Result<()> {
    let t = now()?;
    let c = &mut ctx.accounts.config;
    let p = c.pending.clone().ok_or(AgentsError::NothingPending)?;
    require!(t >= p.eta, AgentsError::TimelockActive);
    c.admin = p.admin;
    c.fee_collector = p.fee_collector;
    c.soulbound_item = p.soulbound_item;
    c.params = p.params;
    c.verifiers = p.verifiers.clone();
    c.targets = p.targets.clone();
    c.pending = None;
    emit_cpi!(ParamsApplied { change: p, ts: t });
    Ok(())
}

/// Accounts of `cancel_config`.
#[event_cpi]
#[derive(Accounts)]
pub struct CancelConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump, has_one = admin @ AgentsError::NotAdmin)]
    pub config: Box<Account<'info, AgentsConfig>>,
}

pub fn process_cancel_config(ctx: Context<CancelConfig>) -> Result<()> {
    require!(ctx.accounts.config.pending.is_some(), AgentsError::NothingPending);
    ctx.accounts.config.pending = None;
    emit_cpi!(ParamsCancelled { ts: now()? });
    Ok(())
}
