// Changed by Hookwars: new file (M2), the template program's armory-facing entry points.
//! `hookwars_items`: one program implementing every template (docs/spec/04-templates.md, 00 D-2).
//!
//! M2 builds what the armory (docs/spec/02-armory.md) calls, all signed by the armory's
//! `["armory"]` PDA:
//!
//! - `validate_params`, `manifest`, `combine_params`: pure, answering through return data;
//! - `init_equip`: creates or resets the slot's `EquipState` at `["equip", mint, slot]`, writes the
//!   item's registry at `["bordrless-hook-accounts", mint, item]` and creates the equip vault (the
//!   `EquipState`'s holding of the mint) for an item that may cut; returns the registry length;
//! - `close_equip`: refuses while the slot's vaults hold anything unsettled, then empties it.
//!
//! The hook callbacks, `settle_equip` and `RaidLedger` are M3 (this crate is laid out so they are
//! added beside these).

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, set_return_data};
use anchor_lang::system_program;
use bordrless_hook::{AccountSource, ExtraAccount, HookAccountList, HOOK_ACCOUNTS_SEED};
use hookwars_common::{
    combine, manifest as compute_manifest, pda, template_id, validate, EquipConfig, Manifest,
    ParamsError, PARAM_FIELDS,
};

declare_id!("8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Hookwars items",
    project_url: "https://github.com/BordrlessDex/bordrless-programs",
    contacts: "link:https://github.com/BordrlessDex/bordrless-programs/security/advisories/new",
    policy: "https://github.com/BordrlessDex/bordrless-programs/blob/main/SECURITY.md",
    source_code: "https://github.com/BordrlessDex/bordrless-programs"
}

/// The armory's signer of every call here: `["armory"]` under the armory (02 section 2.2).
pub const ARMORY_SIGNER: Pubkey =
    Pubkey::from_str_const("2NSNJ4W51G5ZZSc4wVqkjyR8yr5yzuEjquKv7iprzSHP");
/// `EquipState` layout version.
pub const VERSION: u8 = 1;

/// Errors (04 section 6).
#[error_code]
pub enum ItemsError {
    /// The params break a template rule.
    #[msg("the params break a template rule")]
    BadParams,
    /// Unknown template.
    #[msg("unknown template")]
    UnknownTemplate,
    /// The two items cannot be forged together.
    #[msg("the two items cannot be forged together")]
    NotForgeable,
    /// The targets or role do not fit the template.
    #[msg("the targets or role do not fit the template")]
    BadTargets,
    /// A vault of the slot still holds something unsettled.
    #[msg("a vault of the slot still holds something unsettled")]
    VaultNotSettled,
    /// The slot already holds an item.
    #[msg("the slot already holds an item")]
    SlotNotEmpty,
    /// The slot holds no item.
    #[msg("the slot holds no item")]
    NotEquipped,
    /// A wrong account.
    #[msg("a wrong account")]
    WrongAccount,
}

fn map(e: ParamsError) -> Error {
    match e {
        ParamsError::UnknownTemplate => ItemsError::UnknownTemplate.into(),
        ParamsError::NotForgeable => ItemsError::NotForgeable.into(),
        ParamsError::OutOfRange | ParamsError::BadParams => ItemsError::BadParams.into(),
    }
}

/// One equip of a mint's slot (04 section 4). Also the owner of the slot's equip vault.
#[account]
#[derive(Debug)]
pub struct EquipState {
    /// Layout version.
    pub version: u8,
    /// Bump of `["equip", mint, slot]`.
    pub bump: u8,
    /// The token.
    pub mint: Pubkey,
    /// The slot.
    pub slot: u8,
    /// The equipped item (default when empty).
    pub item: Pubkey,
    /// Its template.
    pub template_id: u16,
    /// Targets and role.
    pub config: EquipConfig,
    /// When equipped.
    pub equipped_at: i64,
    /// Callbacks run (M3).
    pub runs: u64,
    /// Token-side cuts taken (M3).
    pub collected_token: u64,
    /// Pool-side cuts recorded (M3).
    pub pool_owed: u64,
    /// Pool-side cuts settled (M3).
    pub pool_settled: u64,
    /// Reserved.
    pub reserved: [u8; 32],
}

impl EquipState {
    /// Space for an `EquipState` with `targets` targets.
    pub fn space(targets: usize) -> usize {
        8 + 1 + 1 + 32 + 1 + 32 + 2 + (4 + 32 * targets + 1) + 8 + 8 + 8 + 8 + 8 + 32
    }
}

/// Emitted by `init_equip` (program log: called only by CPI).
#[event]
pub struct EquipInitialized {
    /// The token.
    pub mint: Pubkey,
    /// The slot.
    pub slot: u8,
    /// The item.
    pub item: Pubkey,
    /// Targets and role.
    pub config: EquipConfig,
}

/// Emitted by `close_equip`.
#[event]
pub struct EquipClosed {
    /// The token.
    pub mint: Pubkey,
    /// The slot.
    pub slot: u8,
    /// The item that left.
    pub item: Pubkey,
}

/// Instructions.
#[program]
pub mod hookwars_items {
    use super::*;

    /// Template rules beyond floor and ceiling (also re-checks floor and ceiling).
    pub fn validate_params(
        _ctx: Context<ArmoryOnly>,
        template_id: u16,
        field_min: [u32; PARAM_FIELDS],
        field_max: [u32; PARAM_FIELDS],
        params: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        hookwars_common::check_fields(template_id, &field_min, &field_max, &params).map_err(map)?;
        validate(template_id, &params).map_err(map)
    }

    /// The item's manifest, as return data (16 bytes).
    pub fn manifest(
        _ctx: Context<ArmoryOnly>,
        template_id: u16,
        params: [u32; PARAM_FIELDS],
        max_targets: u8,
    ) -> Result<()> {
        let m = compute_manifest(template_id, &params, max_targets).map_err(map)?;
        let mut v = Vec::new();
        m.serialize(&mut v)?;
        set_return_data(&v);
        Ok(())
    }

    /// The forged params, as return data.
    pub fn combine_params(
        _ctx: Context<ArmoryOnly>,
        template_id: u16,
        field_min: [u32; PARAM_FIELDS],
        field_max: [u32; PARAM_FIELDS],
        gain_bps: u16,
        a: [u32; PARAM_FIELDS],
        b: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        let out = combine(template_id, &field_min, &field_max, gain_bps, &a, &b).map_err(map)?;
        validate(template_id, &out).map_err(map)?;
        let mut v = Vec::new();
        out.serialize(&mut v)?;
        set_return_data(&v);
        Ok(())
    }

    /// Creates or resets the slot's `EquipState`, writes the item's registry, creates the equip
    /// vault for an item that may cut on the token side. Return data: the registry length (`u8`),
    /// which the armory passes to the token program as the slot's `extra_count`.
    pub fn init_equip(
        ctx: Context<InitEquip>,
        slot: u8,
        item: Pubkey,
        template_id: u16,
        manifest: hookwars_common::Manifest,
        config: EquipConfig,
        max_targets: u8,
    ) -> Result<()> {
        process_init_equip(ctx, slot, item, template_id, manifest, config, max_targets)
    }

    /// Empties the slot's `EquipState`; refused while its vaults hold anything unsettled.
    pub fn close_equip(ctx: Context<CloseEquip>, slot: u8) -> Result<()> {
        process_close_equip(ctx, slot)
    }
}

/// Accounts of the pure entry points: only the armory's signer.
#[derive(Accounts)]
pub struct ArmoryOnly<'info> {
    /// The armory's `["armory"]` PDA.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: Signer<'info>,
}

/// Accounts of `init_equip`.
#[derive(Accounts)]
pub struct InitEquip<'info> {
    /// The armory's `["armory"]` PDA.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: Signer<'info>,
    /// Pays rent.
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the token (a mint of the token program).
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: `["equip", mint, slot]` under this program, created or reset here.
    #[account(mut)]
    pub equip_state: UncheckedAccount<'info>,
    /// CHECK: `["bordrless-hook-accounts", mint, item]` under this program, written here.
    #[account(mut)]
    pub registry: UncheckedAccount<'info>,
    /// CHECK: the equip vault (`EquipState`'s holding of the mint), created here when the item may
    /// cut on the token side.
    #[account(mut)]
    pub equip_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Accounts of `close_equip`.
#[derive(Accounts)]
pub struct CloseEquip<'info> {
    /// The armory's `["armory"]` PDA.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: Signer<'info>,
    /// CHECK: the token.
    pub mint: UncheckedAccount<'info>,
    /// The slot's `EquipState`.
    #[account(mut)]
    pub equip_state: Account<'info, EquipState>,
    /// CHECK: the equip vault, when the equipped item may cut on the token side.
    pub equip_vault: Option<UncheckedAccount<'info>>,
}

fn check_targets(template_id: u16, config: &EquipConfig, max_targets: u8) -> Result<()> {
    let n = config.targets.len();
    require!(n <= usize::from(max_targets), ItemsError::BadTargets);
    let ok = match template_id {
        template_id::RAID | template_id::SHIELD => n >= 1 && config.role == 0,
        template_id::SPY | template_id::TREATY | template_id::TRANSFER_FEE => {
            n == 1 && config.role == 0
        }
        template_id::TRIBUTE => n == 1 && (config.role == 1 || config.role == 2),
        template_id::WALL | template_id::HALF_LIFE | template_id::WAR_ORDERS => {
            n == 0 && config.role == 0
        }
        _ => false,
    };
    require!(ok, ItemsError::BadTargets);
    let mut seen = config.targets.clone();
    seen.sort();
    seen.dedup();
    require!(seen.len() == n, ItemsError::BadTargets);
    Ok(())
}

/// The registry of an equipped item: the `Item` and `EquipState` first (04 section 2.2), then the
/// template's extras (04 section 3). M2 writes what each template's "Extras" names as fixed keys;
/// M3 owns the final lists (M2 implementation notes in 02).
fn registry_list(
    template_id: u16,
    mint: &Pubkey,
    item: &Pubkey,
    equip_state: &Pubkey,
    equip_vault: Option<Pubkey>,
    config: &EquipConfig,
) -> HookAccountList {
    let key = |k: Pubkey, writable: bool| ExtraAccount {
        writable,
        source: AccountSource::Key(k),
    };
    let mut v = vec![key(*item, false), key(*equip_state, true)];
    if let Some(vault) = equip_vault {
        v.push(key(vault, true));
    }
    let launches = |v: &mut Vec<ExtraAccount>| {
        for t in &config.targets {
            v.push(key(pda::launch(t).0, false));
        }
    };
    match template_id {
        template_id::RAID => {
            v.push(key(pda::raid_ledger(mint).0, true));
            v.push(key(pda::war_config().0, false));
            launches(&mut v);
        }
        template_id::SHIELD => {
            v.push(key(pda::raid_ledger(mint).0, true));
            v.push(key(pda::war_config().0, false));
            v.push(key(pda::war_state(mint).0, false));
            launches(&mut v);
        }
        template_id::WALL => v.push(key(pda::war_state(mint).0, false)),
        template_id::SPY => launches(&mut v),
        template_id::TREATY | template_id::TRIBUTE => {
            for t in &config.targets {
                v.push(key(*t, false));
            }
        }
        _ => {}
    }
    HookAccountList::new(v)
}

/// Creates `target` (a PDA of this program with `seeds`) with `space` bytes, or resizes it when
/// it exists and is smaller, topping up rent from `payer`. An address someone already funded is
/// allocated and assigned, as upstream `write_registry` does.
fn create_or_resize<'info>(
    payer: &AccountInfo<'info>,
    target: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    seeds: &[&[u8]],
    space: usize,
) -> Result<()> {
    let rent = Rent::get()?.minimum_balance(space);
    if *target.owner == system_program::ID {
        let current = target.lamports();
        if current == 0 {
            system_program::create_account(
                CpiContext::new_with_signer(
                    system.key(),
                    system_program::CreateAccount {
                        from: payer.clone(),
                        to: target.clone(),
                    },
                    &[seeds],
                ),
                rent,
                space as u64,
                &crate::ID,
            )?;
        } else {
            let top_up = rent.saturating_sub(current);
            if top_up > 0 {
                system_program::transfer(
                    CpiContext::new(
                        system.key(),
                        system_program::Transfer {
                            from: payer.clone(),
                            to: target.clone(),
                        },
                    ),
                    top_up,
                )?;
            }
            system_program::allocate(
                CpiContext::new_with_signer(
                    system.key(),
                    system_program::Allocate {
                        account_to_allocate: target.clone(),
                    },
                    &[seeds],
                ),
                space as u64,
            )?;
            system_program::assign(
                CpiContext::new_with_signer(
                    system.key(),
                    system_program::Assign {
                        account_to_assign: target.clone(),
                    },
                    &[seeds],
                ),
                &crate::ID,
            )?;
        }
        return Ok(());
    }
    require_keys_eq!(*target.owner, crate::ID, ItemsError::WrongAccount);
    if target.data_len() < space {
        let top_up = rent.saturating_sub(target.lamports());
        if top_up > 0 {
            system_program::transfer(
                CpiContext::new(
                    system.key(),
                    system_program::Transfer {
                        from: payer.clone(),
                        to: target.clone(),
                    },
                ),
                top_up,
            )?;
        }
        target.resize(space)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn process_init_equip(
    ctx: Context<InitEquip>,
    slot: u8,
    item: Pubkey,
    template_id: u16,
    manifest: Manifest,
    config: EquipConfig,
    max_targets: u8,
) -> Result<()> {
    check_targets(template_id, &config, max_targets)?;
    let mint = ctx.accounts.mint.key();
    let (state_key, state_bump) = pda::equip_state(&mint, slot);
    require_keys_eq!(ctx.accounts.equip_state.key(), state_key, ItemsError::WrongAccount);
    let payer = ctx.accounts.payer.to_account_info();
    let system = ctx.accounts.system_program.to_account_info();

    // EquipState: create, or reset an empty one.
    let state_info = ctx.accounts.equip_state.to_account_info();
    if *state_info.owner == crate::ID && state_info.data_len() > 8 {
        let existing = EquipState::try_deserialize(&mut &state_info.try_borrow_data()?[..])?;
        require!(existing.item == Pubkey::default(), ItemsError::SlotNotEmpty);
        require!(
            existing.pool_owed == existing.pool_settled,
            ItemsError::VaultNotSettled
        );
    }
    let slot_seed = [slot];
    let bump_seed = [state_bump];
    let seeds: &[&[u8]] = &[
        hookwars_common::seeds::EQUIP,
        mint.as_ref(),
        &slot_seed,
        &bump_seed,
    ];
    create_or_resize(
        &payer,
        &state_info,
        &system,
        seeds,
        EquipState::space(config.targets.len()),
    )?;
    let (pool_owed, pool_settled) = if state_info.data_len() > 8
        && state_info.try_borrow_data()?[..8] == *EquipState::DISCRIMINATOR
    {
        let s = EquipState::try_deserialize(&mut &state_info.try_borrow_data()?[..])?;
        (s.pool_owed, s.pool_settled)
    } else {
        (0, 0)
    };
    let state = EquipState {
        version: VERSION,
        bump: state_bump,
        mint,
        slot,
        item,
        template_id,
        config: config.clone(),
        equipped_at: Clock::get()?.unix_timestamp,
        runs: 0,
        collected_token: 0,
        pool_owed,
        pool_settled,
        reserved: [0; 32],
    };
    {
        let mut data = state_info.try_borrow_mut_data()?;
        let mut out: &mut [u8] = &mut data[..];
        state.try_serialize(&mut out)?;
    }

    // The equip vault, for an item that may cut on the token side.
    let vault = if manifest.token_cuts() {
        let vault = ctx
            .accounts
            .equip_vault
            .as_ref()
            .ok_or(ItemsError::WrongAccount)?;
        require_keys_eq!(
            vault.key(),
            pda::holding(&mint, &state_key),
            ItemsError::WrongAccount
        );
        let ix = bordrless_token::client::create_holding(payer.key(), mint, state_key);
        invoke(
            &ix,
            &[
                payer.clone(),
                ctx.accounts.mint.to_account_info(),
                state_info.clone(),
                vault.to_account_info(),
                system.clone(),
                ctx.accounts.token_event_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
        )?;
        Some(vault.key())
    } else {
        None
    };

    // The registry.
    let list = registry_list(template_id, &mint, &item, &state_key, vault, &config);
    let bytes = list.encode();
    let (reg_key, reg_bump) = Pubkey::find_program_address(
        &[HOOK_ACCOUNTS_SEED, mint.as_ref(), item.as_ref()],
        &crate::ID,
    );
    require_keys_eq!(ctx.accounts.registry.key(), reg_key, ItemsError::WrongAccount);
    let reg_info = ctx.accounts.registry.to_account_info();
    let reg_bump_seed = [reg_bump];
    let reg_seeds: &[&[u8]] = &[HOOK_ACCOUNTS_SEED, mint.as_ref(), item.as_ref(), &reg_bump_seed];
    create_or_resize(&payer, &reg_info, &system, reg_seeds, bytes.len())?;
    {
        let mut data = reg_info.try_borrow_mut_data()?;
        data[..bytes.len()].copy_from_slice(&bytes);
        for b in data[bytes.len()..].iter_mut() {
            *b = 0;
        }
    }
    let count = list.accounts.len() as u8;
    emit!(EquipInitialized {
        mint,
        slot,
        item,
        config,
    });
    set_return_data(&[count]);
    Ok(())
}

fn process_close_equip(ctx: Context<CloseEquip>, slot: u8) -> Result<()> {
    let mint = ctx.accounts.mint.key();
    let state_key = ctx.accounts.equip_state.key();
    require_keys_eq!(state_key, pda::equip_state(&mint, slot).0, ItemsError::WrongAccount);
    let state = &mut ctx.accounts.equip_state;
    require!(state.item != Pubkey::default(), ItemsError::NotEquipped);
    require!(state.pool_owed == state.pool_settled, ItemsError::VaultNotSettled);
    if let Some(vault) = ctx.accounts.equip_vault.as_ref() {
        require_keys_eq!(
            vault.key(),
            pda::holding(&mint, &state_key),
            ItemsError::WrongAccount
        );
        if *vault.owner == bordrless_token::ID && vault.data_len() > 0 {
            let h = bordrless_token::client::read_holding(&vault.to_account_info())?;
            require!(h.amount == 0, ItemsError::VaultNotSettled);
        }
    }
    let item = state.item;
    state.item = Pubkey::default();
    emit!(EquipClosed { mint, slot, item });
    Ok(())
}
