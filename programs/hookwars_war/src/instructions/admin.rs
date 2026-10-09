//! The config and every admin power, behind the timelock (05 sections 2.1 and 11). Nothing an admin
//! does applies at once: a change is proposed with an `eta` at least `admin_timelock_secs` away and
//! applied by anyone after it, or cancelled by the admin before.

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::Foreign;
use crate::foreign::{Template, PARAM_FIELDS, TEMPLATE_ACTIVE};
use crate::state::*;

/// The upgrade authority of `program_id`, read from its ProgramData account (loader v3 layout: a
/// u32 tag of 3, the slot, then an optional authority), as upstream's configs read it.
pub fn upgrade_authority(program_data: &AccountInfo, program_id: &Pubkey) -> Result<Option<Pubkey>> {
    let (expected, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID);
    require_keys_eq!(*program_data.key, expected, WarError::NotUpgradeAuthority);
    require_keys_eq!(
        *program_data.owner,
        BPF_LOADER_UPGRADEABLE_ID,
        WarError::NotUpgradeAuthority
    );
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        WarError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    let mut key = [0u8; 32];
    key.copy_from_slice(&data[13..45]);
    Ok(Some(Pubkey::new_from_array(key)))
}

/// Closes `info` (an account of this program) into `to`.
pub fn close_into(info: &AccountInfo, to: &AccountInfo) -> Result<()> {
    let lamports = info.lamports();
    **to.try_borrow_mut_lamports()? = to
        .lamports()
        .checked_add(lamports)
        .ok_or(WarError::MathOverflow)?;
    **info.try_borrow_mut_lamports()? = 0;
    info.assign(&anchor_lang::system_program::ID);
    info.resize(0)?;
    Ok(())
}

/// An account of this program, deserialized (owner and discriminator checked).
pub fn read_own<T: AccountDeserialize + Owner>(info: &AccountInfo) -> Result<T> {
    require_keys_eq!(*info.owner, T::owner(), WarError::WrongAccount);
    let data = info.try_borrow_data()?;
    T::try_deserialize(&mut &data[..])
}

/// Arguments of `init_config` and `propose_config`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct ConfigArgs {
    pub admin: Pubkey,
    pub protocol_treasury: Pubkey,
    pub randomness_program: Pubkey,
    pub treaty_template_id: Option<u16>,
    pub params: WarParams,
}

impl ConfigArgs {
    fn change(&self) -> ConfigChange {
        ConfigChange {
            admin: self.admin,
            protocol_treasury: self.protocol_treasury,
            randomness_program: self.randomness_program,
            treaty_template_id: self.treaty_template_id,
            params: self.params,
        }
    }
}

// ---- init_config ---------------------------------------------------------------------------------

/// Accounts of `init_config`.
#[event_cpi]
#[derive(Accounts)]
pub struct InitConfig<'info> {
    /// This program's upgrade authority, paying the rent.
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = WarConfig::LEN, seeds = [WAR_CONFIG_SEED], bump)]
    pub config: Box<Account<'info, WarConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `init_config(args)`: once, by the upgrade authority. Every later change goes through the
/// timelock.
pub fn process_init_config(ctx: Context<InitConfig>, args: ConfigArgs) -> Result<()> {
    require!(
        upgrade_authority(&ctx.accounts.program_data, &crate::ID)?
            == Some(ctx.accounts.authority.key()),
        WarError::NotUpgradeAuthority
    );
    require!(args.params.valid(), WarError::InvalidParams);
    let config = &mut ctx.accounts.config;
    config.version = VERSION;
    config.bump = ctx.bumps.config;
    config.prize_vault_bump = prize_vault_address().1;
    config.admin = args.admin;
    config.protocol_treasury = args.protocol_treasury;
    config.randomness_program = args.randomness_program;
    config.treaty_template_id = args.treaty_template_id;
    config.current_season = 0;
    config.last_winner = None;
    config.last_winner_season = 0;
    config.params = args.params;
    config.pending = None;
    emit_cpi!(ConfigApplied {
        change: args.change(),
        eta: Clock::get()?.unix_timestamp
    });
    Ok(())
}

// ---- propose_config / apply_config ---------------------------------------------------------------

/// Accounts of `propose_config`.
#[event_cpi]
#[derive(Accounts)]
pub struct ProposeConfig<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [WAR_CONFIG_SEED], bump = config.bump, has_one = admin @ WarError::NotAdmin)]
    pub config: Box<Account<'info, WarConfig>>,
}

/// `propose_config(args)`: writes the change with `eta = now + admin_timelock_secs`.
pub fn process_propose_config(ctx: Context<ProposeConfig>, args: ConfigArgs) -> Result<()> {
    require!(args.params.valid(), WarError::InvalidParams);
    let now = Clock::get()?.unix_timestamp;
    let config = &mut ctx.accounts.config;
    let eta = now
        .checked_add(config.params.admin_timelock_secs)
        .ok_or(WarError::MathOverflow)?;
    config.pending = Some(PendingConfig {
        admin: args.admin,
        protocol_treasury: args.protocol_treasury,
        randomness_program: args.randomness_program,
        treaty_template_id: args.treaty_template_id,
        params: args.params,
        eta,
    });
    emit_cpi!(ConfigProposed {
        change: args.change(),
        eta
    });
    Ok(())
}

/// Accounts of `apply_config`.
#[event_cpi]
#[derive(Accounts)]
pub struct ApplyConfig<'info> {
    #[account(mut, seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
}

/// `apply_config`: anyone, after the pending change's `eta`.
pub fn process_apply_config(ctx: Context<ApplyConfig>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let config = &mut ctx.accounts.config;
    let p = config.pending.ok_or(WarError::NothingPending)?;
    require!(now >= p.eta, WarError::TimelockNotPassed);
    config.admin = p.admin;
    config.protocol_treasury = p.protocol_treasury;
    config.randomness_program = p.randomness_program;
    config.treaty_template_id = p.treaty_template_id;
    config.params = p.params;
    config.pending = None;
    emit_cpi!(ConfigApplied {
        change: ConfigChange {
            admin: p.admin,
            protocol_treasury: p.protocol_treasury,
            randomness_program: p.randomness_program,
            treaty_template_id: p.treaty_template_id,
            params: p.params,
        },
        eta: p.eta
    });
    Ok(())
}

// ---- cancel_pending ------------------------------------------------------------------------------

/// What `cancel_pending` cancels.
pub mod pending_kind {
    pub const CONFIG: u8 = 0;
    pub const SEASON: u8 = 1;
    pub const LOOT_TABLE: u8 = 2;
}

/// Accounts of `cancel_pending`.
#[event_cpi]
#[derive(Accounts)]
pub struct CancelPending<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [WAR_CONFIG_SEED], bump = config.bump, has_one = admin @ WarError::NotAdmin)]
    pub config: Box<Account<'info, WarConfig>>,
    /// CHECK: the `Season` or `LootTable` cancelled (seeds checked in the handler); the config for
    /// a config change.
    #[account(mut)]
    pub target: UncheckedAccount<'info>,
}

/// `cancel_pending(what, season)`: the admin, before it applies. A season or loot table cannot be
/// cancelled once its season opened (then it is in force, and frozen).
pub fn process_cancel_pending(ctx: Context<CancelPending>, what: u8, season: u32) -> Result<()> {
    let config = &mut ctx.accounts.config;
    let eta;
    match what {
        pending_kind::CONFIG => {
            let p = config.pending.ok_or(WarError::NothingPending)?;
            eta = p.eta;
            config.pending = None;
        }
        pending_kind::SEASON | pending_kind::LOOT_TABLE => {
            require!(season > config.current_season, WarError::SeasonClosed);
            let target = &ctx.accounts.target;
            if what == pending_kind::SEASON {
                require_keys_eq!(*target.key, Season::address(season).0, WarError::WrongAccount);
                eta = read_own::<Season>(target)?.eta;
            } else {
                require_keys_eq!(*target.key, LootTable::address(season).0, WarError::WrongAccount);
                eta = read_own::<LootTable>(target)?.eta;
            }
            close_into(&target.to_account_info(), &ctx.accounts.admin.to_account_info())?;
        }
        _ => return err!(WarError::InvalidParams),
    }
    emit_cpi!(PendingCancelled { what, season, eta });
    Ok(())
}

// ---- propose_season ------------------------------------------------------------------------------

/// Arguments of `propose_season`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct SeasonArgs {
    pub number: u32,
    pub starts_at: i64,
    pub weights: ScoreWeights,
    pub penalize_besieged: bool,
}

/// Accounts of `propose_season`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(args: SeasonArgs)]
pub struct ProposeSeason<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump, has_one = admin @ WarError::NotAdmin)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(init, payer = admin, space = Season::LEN, seeds = [SEASON_SEED, &args.number.to_le_bytes()], bump)]
    pub season: Box<Account<'info, Season>>,
    pub system_program: Program<'info, System>,
}

/// `propose_season(number, starts_at, weights, penalize_besieged)`: a future season, starting no
/// earlier than its `eta`; opened by `open_season`.
pub fn process_propose_season(ctx: Context<ProposeSeason>, args: SeasonArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let config = &ctx.accounts.config;
    require!(args.number > config.current_season, WarError::SeasonClosed);
    let eta = now
        .checked_add(config.params.admin_timelock_secs)
        .ok_or(WarError::MathOverflow)?;
    require!(args.starts_at >= eta, WarError::TimelockNotPassed);
    let ends_at = args
        .starts_at
        .checked_add(config.params.season_secs)
        .ok_or(WarError::MathOverflow)?;
    let s = &mut ctx.accounts.season;
    s.version = VERSION;
    s.bump = ctx.bumps.season;
    s.number = args.number;
    s.starts_at = args.starts_at;
    s.ends_at = ends_at;
    s.weights = args.weights;
    s.penalize_besieged = args.penalize_besieged;
    s.eta = eta;
    s.opened = false;
    s.leader = None;
    s.leader_score = 0;
    s.finalized = false;
    s.prize_paid = 0;
    emit_cpi!(SeasonProposed {
        season: args.number,
        starts_at: args.starts_at,
        ends_at,
        weights: args.weights,
        penalize_besieged: args.penalize_besieged,
        eta
    });
    Ok(())
}

// ---- propose_loot_table --------------------------------------------------------------------------

/// Accounts of `propose_loot_table`. Remaining: the `Template` of every entry, in entry order.
#[event_cpi]
#[derive(Accounts)]
#[instruction(season: u32)]
pub struct ProposeLootTable<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump, has_one = admin @ WarError::NotAdmin)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(init, payer = admin, space = LootTable::LEN, seeds = [LOOT_SEED, &season.to_le_bytes()], bump)]
    pub loot_table: Box<Account<'info, LootTable>>,
    pub system_program: Program<'info, System>,
}

/// `propose_loot_table(season, entries)`: rarity, never code (00 rule 4). Each entry's ranges lie
/// inside its template's floor and ceiling, its template is active and loot-enabled.
pub fn process_propose_loot_table<'info>(
    ctx: Context<'info, ProposeLootTable<'info>>,
    season: u32,
    entries: Vec<LootEntry>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let config = &ctx.accounts.config;
    require!(season > config.current_season, WarError::SeasonClosed);
    require!(
        !entries.is_empty() && entries.len() <= LOOT_TABLE_LEN,
        WarError::InvalidLootEntry
    );
    require!(
        ctx.remaining_accounts.len() == entries.len(),
        WarError::MissingAccount
    );
    let mut total: u64 = 0;
    for (e, info) in entries.iter().zip(ctx.remaining_accounts.iter()) {
        require!(e.weight > 0, WarError::InvalidLootEntry);
        total = total
            .checked_add(u64::from(e.weight))
            .ok_or(WarError::MathOverflow)?;
        require_keys_eq!(
            *info.key,
            crate::foreign::template_address(e.template_id),
            WarError::InvalidLootEntry
        );
        let t = Template::read(info)?;
        require!(
            t.id == e.template_id && t.status == TEMPLATE_ACTIVE && t.loot_enabled,
            WarError::InvalidLootEntry
        );
        for i in 0..PARAM_FIELDS {
            let r = e.ranges[i];
            if i < usize::from(t.field_count) {
                require!(
                    t.field_min[i] <= r.min && r.min <= r.max && r.max <= t.field_max[i],
                    WarError::InvalidLootEntry
                );
            } else {
                require!(r.min == 0 && r.max == 0, WarError::InvalidLootEntry);
            }
        }
    }
    let eta = now
        .checked_add(config.params.admin_timelock_secs)
        .ok_or(WarError::MathOverflow)?;
    let table = &mut ctx.accounts.loot_table;
    table.version = VERSION;
    table.bump = ctx.bumps.loot_table;
    table.season = season;
    table.count = entries.len() as u8;
    for (i, e) in entries.iter().enumerate() {
        table.entries[i] = *e;
    }
    table.eta = eta;
    emit_cpi!(LootTableProposed { season, eta });
    Ok(())
}
