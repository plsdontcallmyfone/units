// Changed by Hookwars: integration pass 3 (E-3): the registry ends with the craft Wear of an item that wears.
// Changed by Hookwars: integration pass 2 (08 arsenal 2 request 1): registries carry derived extra sources;
// kit tokens refuse off-curve token-side payees (review 1 H-2 carried to settle).
// Changed by Hookwars: new file (M3b), the armory-facing entry points moved out of lib.rs (M2) and; agents branch: template 42 takes no targets.
// extended with the M3b registries and composites.
// Changed by Hookwars (arsenal waves D and E): one arm in `check_module_targets`.
//! The entry points the armory calls (02 section 4, 04 section 2.6).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, set_return_data};
use anchor_lang::system_program;
use bordrless_hook::{AccountSource, ExtraAccount, HookAccountList, HOOK_ACCOUNTS_SEED};
use hookwars_common::composite::{CompositeItem, Module};
use hookwars_common::{
    combine, manifest as compute_manifest, pda, template_id, validate, EquipConfig, Manifest,
    Params, MAX_MODULES,
};

use crate::{map, CloseEquip, EquipClosed, EquipInitialized, EquipState, InitEquip, ItemsError, VERSION};

/// Template rules beyond floor and ceiling (also re-checks floor and ceiling).
pub fn validate_params(template_id: u16, field_min: Params, field_max: Params, params: Params) -> Result<()> {
    hookwars_common::check_fields(template_id, &field_min, &field_max, &params).map_err(map)?;
    validate(template_id, &params).map_err(map)
}

/// The item's manifest, as return data (16 bytes).
pub fn manifest(template_id: u16, params: Params, max_targets: u8) -> Result<()> {
    let m = compute_manifest(template_id, &params, max_targets).map_err(map)?;
    let mut v = Vec::new();
    m.serialize(&mut v)?;
    set_return_data(&v);
    Ok(())
}

/// The forged params, as return data.
pub fn combine_params(
    template_id: u16,
    field_min: Params,
    field_max: Params,
    gain_bps: u16,
    a: Params,
    b: Params,
) -> Result<()> {
    let out = combine(template_id, &field_min, &field_max, gain_bps, &a, &b).map_err(map)?;
    validate(template_id, &out).map_err(map)?;
    let mut v = Vec::new();
    out.serialize(&mut v)?;
    set_return_data(&v);
    Ok(())
}

/// The targets one module of `template_id` may be aimed at (04 section 3, 08 section 4).
pub fn check_module_targets(template_id: u16, targets: &[Pubkey], role: u8) -> Result<()> {
    let n = targets.len();
    let ok = match template_id {
        template_id::RAID | template_id::SHIELD => n >= 1 && role == 0,
        template_id::SPY | template_id::TREATY | template_id::TRANSFER_FEE => n == 1 && role == 0,
        template_id::TRIBUTE => n == 1 && (role == 1 || role == 2),
        template_id::WALL
        | template_id::HALF_LIFE
        | template_id::WAR_ORDERS
        | template_id::SIZE_TIERS
        | template_id::SIDE_SKEW
        | template_id::LAUNCH_DECAY
        | template_id::MAX_TRANSACTION
        | template_id::DUST_GUARD
        | template_id::SELL_BURN
        | template_id::SOULBOUND => n == 0,
        // Arsenal waves B and C: no targets.
        template_id::VELOCITY_FEE
        | template_id::IMPACT_FEE
        | template_id::VOLATILITY_FEE
        | template_id::RUSH_HOUR
        | template_id::COOLDOWN
        | template_id::DAILY_SELL_CAP
        | template_id::FLASH_GUARD
        | template_id::DUMP_BRAKE
        | template_id::STREAK
        | template_id::RANK_BADGE
        | template_id::GUILD_TAG => n == 0,
        // Expansion templates (10): Coalition and Boss name no target; Rivalry names its one rival.
        template_id::COALITION | template_id::BOSS => n == 0,
        template_id::RIVALRY => n == 1 && role == 0,
        // Arsenal waves D and E (one arm; their rules live in `hookwars_common::arsenal2`).
        id if hookwars_common::arsenal2::is(id) => hookwars_common::arsenal2::targets_ok(id, n, role),
        _ => false,
    };
    require!(ok, ItemsError::BadTargets);
    let mut seen = targets.to_vec();
    seen.sort();
    seen.dedup();
    require!(seen.len() == n, ItemsError::BadTargets);
    Ok(())
}

fn check_targets(template_id: u16, config: &EquipConfig, max_targets: u8, modules: Option<&[Module]>) -> Result<()> {
    require!(config.targets.len() <= usize::from(max_targets), ItemsError::BadTargets);
    match modules {
        None => check_module_targets(template_id, &config.targets, config.role),
        Some(ms) => {
            let mut used = 0usize;
            for m in ms {
                let start = usize::from(m.target_start);
                let end = start + usize::from(m.target_count);
                require!(end <= config.targets.len(), ItemsError::BadTargets);
                let role = if m.template_id == template_id::TRIBUTE { config.role } else { 0 };
                check_module_targets(m.template_id, &config.targets[start..end], role)?;
                used += usize::from(m.target_count);
            }
            require!(used == config.targets.len(), ItemsError::BadTargets);
            Ok(())
        }
    }
}

/// The registry of an equipped item (04 section 2.2): `Item`, `EquipState`, the equip vault when
/// it cuts on the token side, the module list for a composite, then each module's own extras,
/// then (integration pass 3, E-3) the item's craft `Wear` when it wears.
pub fn registry_list(
    template_id: u16,
    mint: &Pubkey,
    item: &Pubkey,
    equip_state: &Pubkey,
    equip_vault: Option<Pubkey>,
    config: &EquipConfig,
    composite: Option<(&Pubkey, &[Module])>,
    wear: Option<Pubkey>,
) -> HookAccountList {
    let key = |k: Pubkey, writable: bool| ExtraAccount {
        writable,
        source: AccountSource::Key(k),
    };
    let mut v = vec![key(*item, false), key(*equip_state, true)];
    if let Some(vault) = equip_vault {
        v.push(key(vault, true));
    }
    match composite {
        None => {
            // Integration pass 2 (08 arsenal 2 request 1): derived entries (holdings, `Referred`)
            // are written as seeds the token resolves per trade, not as a placeholder key.
            v.extend(crate::templates::extra_sources(template_id, mint, &config.targets));
        }
        Some((list, modules)) => {
            v.push(key(*list, false));
            for m in modules {
                let start = usize::from(m.target_start);
                let end = start + usize::from(m.target_count);
                v.extend(crate::templates::extra_sources(m.template_id, mint, &config.targets[start..end]));
            }
        }
    }
    // Integration pass 3 (E-3): an item that wears reads its craft `Wear` last.
    if let Some(w) = wear {
        v.push(key(w, false));
    }
    HookAccountList::new(v)
}

/// Creates `target` (a PDA of this program with `seeds`) with `space` bytes, or resizes it when
/// it exists and is smaller, topping up rent from `payer`.
pub fn create_or_resize<'info>(
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

/// Reads a composite's module list from `info` (owned by the armory, keyed by `item`).
pub fn read_composite(info: &AccountInfo, item: &Pubkey) -> Result<CompositeItem> {
    require_keys_eq!(*info.owner, hookwars_common::ids::ARMORY_ID, ItemsError::BadComposite);
    require_keys_eq!(*info.key, CompositeItem::address(item).0, ItemsError::BadComposite);
    let c = CompositeItem::decode(&info.try_borrow_data()?).ok_or(ItemsError::BadComposite)?;
    require_keys_eq!(c.item, *item, ItemsError::BadComposite);
    require!(
        !c.modules.is_empty() && c.modules.len() <= MAX_MODULES,
        ItemsError::BadComposite
    );
    Ok(c)
}

#[allow(clippy::too_many_arguments)]
pub fn process_init_equip<'info>(
    ctx: Context<'info, InitEquip<'info>>,
    slot: u8,
    item: Pubkey,
    template_id: u16,
    manifest: Manifest,
    config: EquipConfig,
    max_targets: u8,
    wear: bool,
) -> Result<()> {
    let composite = if template_id == template_id::COMPOSITE {
        let info = ctx.remaining_accounts.first().ok_or(ItemsError::BadComposite)?;
        Some((info.key(), read_composite(info, &item)?))
    } else {
        None
    };
    check_targets(
        template_id,
        &config,
        max_targets,
        composite.as_ref().map(|(_, c)| c.modules.as_slice()),
    )?;
    let mint = ctx.accounts.mint.key();
    // Integration pass 2 (security cross-branch, review 1 H-2): on a kit token a token-side payout
    // goes only to a wallet on the curve or a vault derivable from the mint.
    {
        let m = bordrless_token::client::read_mint(&ctx.accounts.mint.to_account_info())?;
        if crate::templates::runs_kit(&m) {
            let modules: Vec<(u16, usize, usize)> = match &composite {
                Some((_, c)) => c
                    .modules
                    .iter()
                    .map(|md| (md.template_id, usize::from(md.target_start), usize::from(md.target_count)))
                    .collect(),
                None => vec![(template_id, 0, config.targets.len())],
            };
            for (id, start, count) in modules {
                let end = (start + count).min(config.targets.len());
                if let crate::templates::Destination::Owner(o) =
                    crate::templates::token_destination(id, &config.targets[start.min(end)..end])
                {
                    require!(crate::templates::kit_payee_ok(&mint, &o), ItemsError::BadTargets);
                }
            }
        }
    }
    let (state_key, state_bump) = pda::equip_state(&mint, slot);
    require_keys_eq!(ctx.accounts.equip_state.key(), state_key, ItemsError::WrongAccount);
    let payer = ctx.accounts.payer.to_account_info();
    let system = ctx.accounts.system_program.to_account_info();

    // EquipState: create, or reset an empty one.
    let state_info = ctx.accounts.equip_state.to_account_info();
    let mut carried = (0u64, 0u64);
    if *state_info.owner == crate::ID && state_info.data_len() > 8 {
        let existing = EquipState::try_deserialize(&mut &state_info.try_borrow_data()?[..])?;
        require!(existing.item == Pubkey::default(), ItemsError::SlotNotEmpty);
        require!(
            existing.pool_owed == existing.pool_settled
                && existing.token_unsettled.iter().all(|x| *x == 0)
                && existing.pool_unsettled.iter().all(|x| *x == 0),
            ItemsError::VaultNotSettled
        );
        carried = (existing.pool_owed, existing.pool_settled);
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
        pool_owed: carried.0,
        pool_settled: carried.1,
        token_unsettled: [0; MAX_MODULES],
        pool_unsettled: [0; MAX_MODULES],
        runs_at_settle: 0,
        reserved: [0; 24],
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
        if *vault.owner != bordrless_token::ID {
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
        }
        Some(vault.key())
    } else {
        None
    };

    // The registry.
    let list = registry_list(
        template_id,
        &mint,
        &item,
        &state_key,
        vault,
        &config,
        composite.as_ref().map(|(k, c)| (k, c.modules.as_slice())),
        wear.then(|| hookwars_common::eco_cpi::wear_address(&item)),
    );
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

pub fn process_close_equip(ctx: Context<CloseEquip>, slot: u8) -> Result<()> {
    let mint = ctx.accounts.mint.key();
    let state_key = ctx.accounts.equip_state.key();
    require_keys_eq!(state_key, pda::equip_state(&mint, slot).0, ItemsError::WrongAccount);
    let state = &mut ctx.accounts.equip_state;
    require!(state.item != Pubkey::default(), ItemsError::NotEquipped);
    require!(
        state.pool_owed == state.pool_settled
            && state.token_unsettled.iter().all(|x| *x == 0)
            && state.pool_unsettled.iter().all(|x| *x == 0),
        ItemsError::VaultNotSettled
    );
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
