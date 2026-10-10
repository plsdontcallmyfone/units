// Changed by Hookwars: new file, M3b slot launches (spec 03 section 4).
//! Slot launches (Hookwars spec 03 section 4.3): a launch whose mint runs a slot table is made in
//! steps, because equipping items before the supply exists (R12) does not fit upstream's one
//! `create_launch` transaction.
//!
//! 1. `prepare_launch`: the slot mint, with the kit Locked in slot 0 when the rules install a kit
//!    module, no supply and the launch PDA as its only mint authority; `PreparedLaunch` records
//!    who prepared it and with which rules.
//! 2. `equip_prepared`, once per launch item (the armory equips one slot per call, 02 M2 notes):
//!    forwards the armory's `equip_launch`, signing as `["armory-caller", mint]`.
//! 3. `create_prepared_launch` (in `launch.rs`): upstream's `create_launch` on that mint.
//! 4. `refresh_pool_registry`, permissionless: the pool registry names each pool item's accounts.
//!
//! Upstream's own `create_launch` is unchanged for single-hook launches.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{
    hook_accounts_address, hook_signer_at, AccountSource, ExtraAccount, HookAccountList,
    HOOK_ACCOUNTS_SEED,
};
use bordrless_token::instructions::{CreateMintArgs, SlotInit};
use bordrless_token::state::SlotBounds;

use crate::constants::hookwars::*;
use crate::constants::*;
use crate::error::LaunchError;
use crate::events::{LaunchPrepared, PoolRegistryRefreshed};
use crate::instructions::launch::{check_rules, slot_registry_list};
use crate::state::*;

/// Arguments of `prepare_launch`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct PrepareLaunchArgs {
    /// Name.
    pub name: String,
    /// Symbol.
    pub symbol: String,
    /// Metadata URI.
    pub uri: String,
    /// The creator fee the launch will have (checked again at launch).
    pub creator_fee_bps: u16,
    /// The token rules the launch will have; when they install a kit module, the kit is Locked in
    /// slot 0 with exactly the flags those modules need.
    pub rules: LaunchRules,
    /// The item slots, after the kit's (none of them `Locked`). Their kind, rule, bounds and data
    /// range are fixed for ever.
    pub slots: Vec<SlotInit>,
}

/// Accounts of `prepare_launch`.
#[event_cpi]
#[derive(Accounts)]
pub struct PrepareLaunch<'info> {
    /// The creator: pays the rent; the only signer the later steps accept.
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(address = CONFIG_ADDRESS)]
    pub config: Box<Account<'info, Config>>,
    /// CHECK: the new mint; signs, created through the token program.
    #[account(mut)]
    pub mint: Signer<'info>,
    #[account(init, payer = creator, space = PreparedLaunch::LEN,
        seeds = [PREPARED_SEED, mint.key().as_ref()], bump)]
    pub prepared: Box<Account<'info, PreparedLaunch>>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID @ LaunchError::WrongProgram)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (address-checked).
    #[account(address = TOKEN_EVENT_AUTHORITY @ LaunchError::WrongProgram)]
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// The kit's Locked slot for `modules` (R8, R9: bytes `0..kit_data_len`, its two extras).
pub fn kit_slot(modules: u8) -> SlotInit {
    let flags = bordrless_kit::mint_flags(modules);
    let data_len = bordrless_kit::kit_data_len(flags);
    SlotInit {
        kind: bordrless_hook::slot_kind::LOCKED,
        equip_rule: bordrless_hook::equip_rule::LOCKED,
        bounds: SlotBounds {
            max_cut_bps: 0,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: false,
            may_burn: false,
        },
        data_len,
        locked_program: Some(KIT_ID),
        locked_flags: flags,
        locked_extra_count: bordrless_kit::setup::KIT_EXTRA_COUNT,
    }
}

/// `prepare_launch`.
pub fn process_prepare_launch(ctx: Context<PrepareLaunch>, args: PrepareLaunchArgs) -> Result<()> {
    let clock = Clock::get()?;
    let config = &ctx.accounts.config;
    require!(!config.paused, LaunchError::Paused);
    require!(
        !args.name.is_empty()
            && args.name.len() <= 32
            && !args.symbol.is_empty()
            && args.symbol.len() <= 10
            && args.uri.len() <= 200,
        LaunchError::InvalidMetadata
    );
    require!(
        args.creator_fee_bps <= config.max_creator_fee_bps,
        LaunchError::CreatorFeeTooHigh
    );
    check_rules(&args.rules, args.creator_fee_bps, config)?;
    require!(
        !args.slots.is_empty()
            && args.slots.iter().all(|s| {
                s.kind != bordrless_hook::slot_kind::LOCKED && s.locked_program.is_none()
            }),
        LaunchError::InvalidSlots
    );
    let modules = args.rules.modules();
    let mut slots = Vec::with_capacity(args.slots.len() + 1);
    if modules != 0 {
        slots.push(kit_slot(modules));
    }
    slots.extend(args.slots.iter().cloned());
    require!(
        slots.len() <= bordrless_token::constants::MAX_SLOTS,
        LaunchError::InvalidSlots
    );
    let mint = ctx.accounts.mint.key();
    let (launch, _) = Launch::address(&mint);
    let (slot_authority, _) = bordrless_hook::slot_authority(&ARMORY_ID, &mint);
    // The launch PDA is the only mint authority: only `create_prepared_launch` can mint the
    // supply, once, and revokes it then. Nothing else about the mint can change.
    let ix = bordrless_token::client::create_slot_mint(
        ctx.accounts.creator.key(),
        mint,
        CreateMintArgs {
            decimals: config.decimals,
            name: args.name.clone(),
            symbol: args.symbol.clone(),
            uri: args.uri.clone(),
            max_supply: config.supply,
            mint_authority: Some(launch),
            freeze_authority: None,
            hook_program: None,
            hook_flags: 0,
            hook_authority: None,
            metadata_authority: None,
        },
        Some(slot_authority),
        slots.clone(),
    );
    invoke_signed(
        &ix,
        &[
            ctx.accounts.creator.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_event_authority.to_account_info(),
            ctx.accounts.token_program.to_account_info(),
        ],
        &[],
    )?;
    let p = &mut ctx.accounts.prepared;
    p.version = VERSION;
    p.bump = ctx.bumps.prepared;
    p.mint = mint;
    p.creator = ctx.accounts.creator.key();
    p.rules = args.rules;
    p.creator_fee_bps = args.creator_fee_bps;
    p.slot_count = slots.len() as u8;
    p.prepared_at = clock.unix_timestamp;
    p.launched = false;
    p.reserved = [0; 32];
    emit_cpi!(LaunchPrepared {
        mint,
        creator: ctx.accounts.creator.key(),
        slot_count: slots.len() as u8,
        kit_slot: modules != 0,
        slot: clock.slot,
        ts: clock.unix_timestamp,
    });
    Ok(())
}

/// Accounts of `equip_prepared`. The remaining accounts are the armory's `equip_launch` accounts,
/// as its client builds them (the armory caller among them, which this program signs for).
#[derive(Accounts)]
pub struct EquipPrepared<'info> {
    /// The creator who prepared the launch.
    pub creator: Signer<'info>,
    #[account(seeds = [PREPARED_SEED, mint.key().as_ref()], bump = prepared.bump,
        constraint = !prepared.launched @ LaunchError::NotPrepared,
        constraint = prepared.creator == creator.key() @ LaunchError::WrongCreator)]
    pub prepared: Box<Account<'info, PreparedLaunch>>,
    /// CHECK: the prepared mint (bound by `prepared`'s seeds).
    pub mint: UncheckedAccount<'info>,
    /// CHECK: the armory.
    #[account(address = ARMORY_ID @ LaunchError::WrongProgram)]
    pub armory: UncheckedAccount<'info>,
}

/// `equip_prepared(data)`: the armory's `equip_launch` with `data`, signed as
/// `["armory-caller", mint]`. Only `equip_launch` is forwarded, only for a prepared mint that has
/// not launched, only by its creator. The armory checks the rest (a fresh mint: no supply, the
/// slot empty; the item fits the slot).
pub fn process_equip_prepared<'info>(
    ctx: Context<'info, EquipPrepared<'info>>,
    data: Vec<u8>,
) -> Result<()> {
    require!(
        data.len() >= 8 && data[..8] == EQUIP_LAUNCH_DISCRIMINATOR,
        LaunchError::NotEquipLaunch
    );
    let mint = ctx.accounts.mint.key();
    let (caller, bump) =
        Pubkey::find_program_address(&[ARMORY_CALLER_SEED, mint.as_ref()], &crate::ID);
    let metas = ctx
        .remaining_accounts
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: *a.key == caller || a.is_signer,
            is_writable: a.is_writable,
        })
        .collect();
    let mut infos: Vec<AccountInfo> = ctx.remaining_accounts.to_vec();
    infos.push(ctx.accounts.armory.to_account_info());
    let ix = Instruction {
        program_id: ARMORY_ID,
        accounts: metas,
        data,
    };
    invoke_signed(&ix, &infos, &[&[ARMORY_CALLER_SEED, mint.as_ref(), &[bump]]])?;
    Ok(())
}

/// Accounts of `refresh_pool_registry`. The remaining accounts are, for each pool slot of the
/// mint in slot order (03 section 5.5), its item's registry
/// (`["bordrless-hook-accounts", mint, item]` under the slot's program).
#[event_cpi]
#[derive(Accounts)]
pub struct RefreshPoolRegistry<'info> {
    /// Pays any rent the registry grows by.
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(constraint = launch.is_slot_launch() @ LaunchError::NotPrepared)]
    pub launch: Box<Account<'info, Launch>>,
    /// CHECK: the launch's mint (address-checked), read for its slot table.
    #[account(address = launch.mint @ LaunchError::WrongHolding)]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: this program's registry for the launch's pool (address-checked in the handler).
    #[account(mut)]
    pub registry: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Whether a slot's item gets forwarded pool callbacks: a filled `Pool` or `Relation` slot whose
/// item subscribes to either.
pub fn forwards(s: &bordrless_token::state::Slot) -> bool {
    s.is_filled()
        && (s.kind == bordrless_hook::slot_kind::POOL
            || s.kind == bordrless_hook::slot_kind::RELATION)
        && s.pool_flags & (ITEM_POOL_BEFORE | ITEM_POOL_AFTER) != 0
}

/// `refresh_pool_registry`: rewrites the pool registry of a slot launch from its mint's slot
/// table: upstream's four extras, the `PoolCuts` holding, then per forwarded slot its program,
/// this program's signer for it and its item's registry extras. Permissionless; a swap built from
/// a stale registry fails the callbacks' checks (`StaleRegistry`, `WrongItemProgram`) rather than
/// running a removed item.
pub fn process_refresh_pool_registry<'info>(
    ctx: Context<'info, RefreshPoolRegistry<'info>>,
) -> Result<()> {
    let launch = &ctx.accounts.launch;
    let mint = Box::new(bordrless_token::client::read_mint(
        &ctx.accounts.mint.to_account_info(),
    )?);
    let (registry, bump) = hook_accounts_address(&crate::ID, &launch.pool);
    require_keys_eq!(
        ctx.accounts.registry.key(),
        registry,
        LaunchError::WrongHolding
    );
    require_keys_eq!(
        *ctx.accounts.registry.owner,
        crate::ID,
        LaunchError::WrongHolding
    );
    let (pool_cuts_owner, _) =
        Pubkey::find_program_address(&[POOL_CUTS_SEED, launch.mint.as_ref()], &ITEMS_ID);
    let pool_cuts =
        bordrless_token::client::holding_address(&launch.quote_mint, &pool_cuts_owner);
    let mut list = slot_registry_list(launch.holder_vault, launch.kit_config, pool_cuts);
    let items = append_pool_items(&mut list, &mint, &launch.mint, ctx.remaining_accounts)?;
    rewrite_registry(
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.registry.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &list,
        bump,
    )?;
    emit_cpi!(PoolRegistryRefreshed {
        mint: launch.mint,
        pool: launch.pool,
        items,
        accounts: list.accounts.len() as u16,
        ts: Clock::get()?.unix_timestamp,
    });
    Ok(())
}

/// Changed by Hookwars (security review 2 L-D): the forwarded slots' part of a slot launch's pool
/// registry, shared by `refresh_pool_registry` and `create_prepared_launch` (which now writes the
/// full registry, so no swap reverts between the launch and a refresh). `infos` are the items'
/// registries, one per forwarded slot in slot order, and nothing else.
pub fn append_pool_items(
    list: &mut HookAccountList,
    mint: &bordrless_token::state::Mint,
    launch_mint: &Pubkey,
    infos: &[AccountInfo],
) -> Result<Vec<Pubkey>> {
    let mut items = Vec::new();
    let mut r = 0usize;
    for s in mint.active_slots() {
        if !forwards(s) {
            continue;
        }
        let info = infos.get(r).ok_or(LaunchError::ItemAccountsMissing)?;
        r += 1;
        // Changed by Hookwars (protocol pass 4a): an external template's registry is the items
        // program's (written at the armory's equip), not its own program's.
        let owner = if *info.owner == ITEMS_ID { ITEMS_ID } else { s.program };
        let (expected, _) = Pubkey::find_program_address(
            &[HOOK_ACCOUNTS_SEED, launch_mint.as_ref(), s.item.as_ref()],
            &owner,
        );
        require_keys_eq!(*info.key, expected, LaunchError::StaleRegistry);
        require_keys_eq!(*info.owner, owner, LaunchError::StaleRegistry);
        let item_list = HookAccountList::decode(&info.try_borrow_data()?)
            .ok_or(LaunchError::StaleRegistry)?;
        require!(
            item_list.accounts.len() == usize::from(s.extra_count),
            LaunchError::StaleRegistry
        );
        let signer = hook_signer_at(&crate::ID, &s.program, s.launch_signer_bump)
            .ok_or(LaunchError::WrongItemProgram)?;
        list.accounts.push(ExtraAccount {
            writable: false,
            source: AccountSource::Key(s.program),
        });
        list.accounts.push(ExtraAccount {
            writable: false,
            source: AccountSource::Key(signer),
        });
        list.accounts.extend(item_list.accounts);
        items.push(s.item);
    }
    require!(r == infos.len(), LaunchError::StaleRegistry);
    Ok(items)
}

/// How many slots of `mint` the launch pool forwards to (one item registry each).
pub fn forwarded_count(mint: &bordrless_token::state::Mint) -> usize {
    mint.active_slots().iter().filter(|s| forwards(s)).count()
}

/// Rewrites a registry this program owns with `list`, growing or shrinking it (the payer pays the
/// rent a larger one needs; a smaller one keeps its lamports).
fn rewrite_registry<'info>(
    payer: &AccountInfo<'info>,
    registry: &AccountInfo<'info>,
    system_program: &AccountInfo<'info>,
    list: &HookAccountList,
    _bump: u8,
) -> Result<()> {
    let bytes = list.encode();
    let rent = Rent::get()?.minimum_balance(bytes.len());
    let current = registry.lamports();
    if rent > current {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                system_program.key(),
                anchor_lang::system_program::Transfer {
                    from: payer.clone(),
                    to: registry.clone(),
                },
            ),
            rent - current,
        )?;
    }
    registry.resize(bytes.len())?;
    registry.try_borrow_mut_data()?.copy_from_slice(&bytes);
    Ok(())
}
