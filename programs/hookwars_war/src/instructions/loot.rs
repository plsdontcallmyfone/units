//! Loot (05 section 8): `roll` spends a ticket and asks the randomness adapter; `reveal` reads the
//! value and asks the armory to mint the item; `cancel_roll` returns the rent of an expired roll
//! (never the ticket).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::PARAM_FIELDS;
use crate::oracle::{request_randomness, Randomness};
use crate::state::*;

// ---- roll ------------------------------------------------------------------------------------------

/// Accounts of `roll`. Remaining: the Raid slot's extras, then the adapter's further accounts.
#[event_cpi]
#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct Roll<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the owner's holding of the mint (address-checked).
    #[account(mut, address = token_client::holding_address(&mint.key(), &owner.key()))]
    pub holding: UncheckedAccount<'info>,
    #[account(init, payer = owner, space = RollRequest::LEN,
              seeds = [ROLL_SEED, holding.key().as_ref(), &nonce.to_le_bytes()], bump)]
    pub roll_request: Box<Account<'info, RollRequest>>,
    /// CHECK: `["war-signer"]`.
    #[account(seeds = [WAR_SIGNER_SEED], bump)]
    pub war_signer: UncheckedAccount<'info>,
    /// CHECK: the items program.
    #[account(address = ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    /// CHECK: the token program's signer of the items program.
    #[account(address = token_client::hook_signer(&ITEMS_ID))]
    pub items_hook_signer: UncheckedAccount<'info>,
    /// CHECK: the randomness adapter the config names.
    #[account(address = config.randomness_program @ WarError::WrongRandomness)]
    pub oracle_program: UncheckedAccount<'info>,
    /// CHECK: the adapter's randomness account for this roll (the adapter checks it).
    #[account(mut)]
    pub oracle_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `roll(nonce, raid_slot)`: spends one ticket (`touch` `SpendTicket`), writes the request, then
/// asks the adapter for randomness bound to this request.
pub fn process_roll<'info>(
    ctx: Context<'info, Roll<'info>>,
    nonce: u64,
    raid_slot_index: u8,
) -> Result<()> {
    let current = ctx.accounts.config.current_season;
    require!(current > 0, WarError::SeasonNotOpen);
    let slot = raid_slot(&ctx.accounts.mint, raid_slot_index)?;
    let before = read_raid(&ctx.accounts.holding, &slot)?;
    require!(before.tickets > 0, WarError::NothingToDo);
    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let extras = slice_metas(ctx.remaining_accounts, usize::from(slot.extra_count))?;
    touch_raid(
        &all,
        ctx.accounts.war_signer.key(),
        ctx.bumps.war_signer,
        ctx.accounts.mint.key(),
        ctx.accounts.holding.key(),
        raid_slot_index,
        WarTouch::SpendTicket,
        extras,
    )?;
    let after = read_raid(&ctx.accounts.holding, &slot)?;
    require!(after.tickets + 1 == before.tickets, WarError::TouchMismatch);

    let clock = Clock::get()?;
    let roll_key = ctx.accounts.roll_request.key();
    let holding_key = ctx.accounts.holding.key();
    let bump = ctx.bumps.roll_request;
    let r = &mut ctx.accounts.roll_request;
    r.version = VERSION;
    r.bump = bump;
    r.owner = ctx.accounts.owner.key();
    r.mint = ctx.accounts.mint.key();
    r.holding = holding_key;
    r.nonce = nonce;
    r.season = current;
    r.requested_slot = clock.slot;
    r.requested_at = clock.unix_timestamp;
    r.oracle_program = ctx.accounts.oracle_program.key();
    r.oracle_account = ctx.accounts.oracle_account.key();

    let nonce_le = nonce.to_le_bytes();
    let bump_seed = [bump];
    let seeds: [&[u8]; 4] = [ROLL_SEED, holding_key.as_ref(), &nonce_le, &bump_seed];
    invoke_built(
        &request_randomness(
            ctx.accounts.oracle_program.key(),
            ctx.accounts.owner.key(),
            roll_key,
            ctx.accounts.oracle_account.key(),
        ),
        &all,
        &[&seeds],
    )?;
    emit_cpi!(RollRequested {
        roll: roll_key,
        mint: ctx.accounts.mint.key(),
        owner: ctx.accounts.owner.key(),
        holding: holding_key,
        season: current,
        requested_slot: clock.slot,
        oracle_program: ctx.accounts.oracle_program.key(),
        oracle_account: ctx.accounts.oracle_account.key(),
    });
    Ok(())
}

// ---- reveal ----------------------------------------------------------------------------------------

/// Accounts of `reveal`. Remaining: the armory `mint_loot`'s further accounts (as `create_item`
/// takes them), in its order.
#[event_cpi]
#[derive(Accounts)]
pub struct Reveal<'info> {
    /// Pays the item's rent; anyone.
    #[account(mut)]
    pub revealer: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    /// CHECK: the roll's owner (address-checked); receives the request's rent.
    #[account(mut, address = roll_request.owner @ WarError::WrongAccount)]
    pub owner: UncheckedAccount<'info>,
    #[account(mut, close = owner,
              seeds = [ROLL_SEED, roll_request.holding.as_ref(), &roll_request.nonce.to_le_bytes()],
              bump = roll_request.bump)]
    pub roll_request: Box<Account<'info, RollRequest>>,
    #[account(seeds = [LOOT_SEED, &roll_request.season.to_le_bytes()], bump = loot_table.bump)]
    pub loot_table: Box<Account<'info, LootTable>>,
    /// CHECK: exactly the randomness account the roll named.
    #[account(address = roll_request.oracle_account @ WarError::WrongRandomness)]
    pub oracle_account: UncheckedAccount<'info>,
    /// CHECK: `["loot-signer"]`, the only signer `mint_loot` accepts.
    #[account(seeds = [LOOT_SIGNER_SEED], bump)]
    pub loot_signer: UncheckedAccount<'info>,
    /// CHECK: the armory (address-checked).
    #[account(address = ARMORY_ID)]
    pub armory_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// What a 32-byte value draws from a loot table: a template by weight from bytes 0..8, then each
/// field uniformly in its range from its own two bytes (8 + 2i), disjoint from the others.
pub fn draw(table: &LootTable, value: &[u8; 32]) -> Option<(u16, [u32; PARAM_FIELDS])> {
    let entries = table.active();
    let total: u64 = entries.iter().map(|e| u64::from(e.weight)).sum();
    if total == 0 {
        return None;
    }
    let mut pick = u64::from_le_bytes(value[..8].try_into().ok()?) % total;
    let entry = entries.iter().find(|e| {
        let w = u64::from(e.weight);
        if pick < w {
            true
        } else {
            pick -= w;
            false
        }
    })?;
    let mut params = [0u32; PARAM_FIELDS];
    for (i, p) in params.iter_mut().enumerate() {
        let at = 8 + 2 * i;
        let r = u64::from(u16::from_le_bytes(value[at..at + 2].try_into().ok()?));
        let range = entry.ranges[i];
        let span = u64::from(range.max - range.min) + 1;
        *p = range.min + ((r * span) >> 16) as u32;
    }
    Some((entry.template_id, params))
}

/// `sha256("global:mint_loot")[..8]`.
pub const MINT_LOOT: [u8; 8] = [0x91, 0x7e, 0x90, 0xcd, 0xed, 0x39, 0x2f, 0xd3];

/// The armory's `mint_loot(owner, template_id, params)` (02 section 4.3): `loot_signer` (signer),
/// `payer` (signer, writable), `owner`, then `extra` in order.
pub fn mint_loot_ix(
    loot_signer: Pubkey,
    payer: Pubkey,
    owner: Pubkey,
    template_id: u16,
    params: [u32; PARAM_FIELDS],
    extra: Vec<AccountMeta>,
) -> Result<Instruction> {
    let mut data = MINT_LOOT.to_vec();
    owner.serialize(&mut data)?;
    template_id.serialize(&mut data)?;
    params.serialize(&mut data)?;
    let mut accounts = vec![
        AccountMeta::new_readonly(loot_signer, true),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(owner, false),
    ];
    accounts.extend(extra);
    Ok(Instruction {
        program_id: ARMORY_ID,
        accounts,
        data,
    })
}

/// `reveal`: permissionless; the value must be fulfilled after the request and bound to it.
pub fn process_reveal<'info>(ctx: Context<'info, Reveal<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let r = &ctx.accounts.roll_request;
    let roll_key = r.key();
    let randomness = Randomness::read(&ctx.accounts.oracle_account, &r.oracle_program)?;
    require_keys_eq!(randomness.requester, roll_key, WarError::WrongRandomness);
    require!(
        randomness.fulfilled && randomness.fulfilled_slot > r.requested_slot,
        WarError::RandomnessNotReady
    );
    let table = &ctx.accounts.loot_table;
    require!(now >= table.eta, WarError::LootTableNotReady);
    let (template_id, params) =
        draw(table, &randomness.value).ok_or(WarError::LootTableNotReady)?;
    let (owner, mint, season) = (r.owner, r.mint, r.season);
    let extra = ctx
        .remaining_accounts
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: false,
            is_writable: a.is_writable,
        })
        .collect();
    let ix = mint_loot_ix(
        ctx.accounts.loot_signer.key(),
        ctx.accounts.revealer.key(),
        owner,
        template_id,
        params,
        extra,
    )?;
    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let bump = [ctx.bumps.loot_signer];
    let seeds: [&[u8]; 2] = [LOOT_SIGNER_SEED, &bump];
    invoke_built(&ix, &all, &[&seeds])?;
    emit_cpi!(RollRevealed {
        roll: roll_key,
        mint,
        owner,
        item: Pubkey::default(),
        template_id,
        params,
        season,
    });
    // Integration pass 2 (09 section 21 item 4): optional agent attribution, after the effects.
    let (_, rec) = hookwars_common::agents_record::split(ctx.remaining_accounts, &crate::ID);
    hookwars_common::agents_record::record(rec, &crate::ID, &ctx.accounts.revealer.key(), hookwars_common::agents_record::LOOT_REVEAL, 0)?;
    Ok(())
}

// ---- cancel_roll -----------------------------------------------------------------------------------

/// Accounts of `cancel_roll`.
#[event_cpi]
#[derive(Accounts)]
pub struct CancelRoll<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, close = owner, has_one = owner @ WarError::WrongAccount,
              seeds = [ROLL_SEED, roll_request.holding.as_ref(), &roll_request.nonce.to_le_bytes()],
              bump = roll_request.bump)]
    pub roll_request: Box<Account<'info, RollRequest>>,
}

/// `cancel_roll`: the owner, after `roll_expiry_secs`; rent returned, the ticket not (a refund
/// would let a trader cancel rolls whose value showed up badly).
pub fn process_cancel_roll(ctx: Context<CancelRoll>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let r = &ctx.accounts.roll_request;
    require!(
        now >= r
            .requested_at
            .saturating_add(ctx.accounts.config.params.roll_expiry_secs),
        WarError::RollNotExpired
    );
    emit_cpi!(RollCancelled {
        roll: r.key(),
        mint: r.mint,
        owner: r.owner,
    });
    Ok(())
}
