// Changed by Hookwars: new file (pass 4b, 10 section 8).
//! Coalitions with a shared chest. Tokens that each equip a Coalition item (template 43) naming
//! the same id form a `Coalition` (`COALITION_MIN_MEMBERS` to `COALITION_MAX_MEMBERS`, a term up
//! to `season_secs`). Each member's chest may contribute up to its item's bps of the chest (less a
//! live rivalry's budget) once per `contribute_interval_secs`. The shared chest spends only through
//! `coalition_siege` (a member's own siege guards: its War orders' threshold on its own raid
//! ledger, the coalition's siege interval, the premium wait, the sandwich bound, R10) and refills
//! through `coalition_raze` (the raze rate limit and the M-3 floor). After the term, and once
//! nothing captured is left, `dissolve_coalition` returns the chest to the members' chests pro rata
//! of what each contributed (rounding to the last member). Spot only: buys and sells at pool prices.

use anchor_lang::prelude::*;
use bordrless_core::swap_out;
use bordrless_launch::client::{self as launch_client, LaunchKeys};
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::{config_item, pool_twap, spot_q64, Foreign, RaidLedger};
use crate::instructions::attack::{
    buy_fee_bound, chest_seeds, mark_besieged, pool_share_bps, quote_side_cap, read_launch, read_pool,
    sell_fee_bound, SliceArgs,
};
use crate::instructions::setup::{chest_balance, note_funding};
use crate::state::*;

fn coalition_seeds(id: u32, bump: u8) -> ([u8; 4], [u8; 1]) {
    (id.to_le_bytes(), [bump])
}

/// Notes donations the shared chest received since its last write as funding.
fn note_coalition(c: &mut Coalition, balance: u64) {
    let inc = balance.saturating_sub(c.last_seen_balance);
    c.funded_total = c.funded_total.saturating_add(inc);
    c.last_seen_balance = balance;
}

// ---- form_coalition ---------------------------------------------------------------------------------

/// Accounts of `form_coalition`. Remaining: per member, `mint, Coalition item, its template, the
/// member's war state`.
#[event_cpi]
#[derive(Accounts)]
#[instruction(id: u32)]
pub struct FormCoalition<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(init, payer = payer, space = Coalition::LEN, seeds = [COALITION_SEED, &id.to_le_bytes()], bump)]
    pub coalition: Box<Account<'info, Coalition>>,
    pub system_program: Program<'info, System>,
}

/// `form_coalition(id, term_secs)`: permissionless; every member consented by equipping a
/// Coalition item naming `id`, and every member is a war token.
pub fn process_form_coalition<'info>(
    ctx: Context<'info, FormCoalition<'info>>,
    id: u32,
    term_secs: i64,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let rem = ctx.remaining_accounts;
    require!(rem.len() % 4 == 0, WarError::InvalidCoalition);
    let n = rem.len() / 4;
    require!(
        (COALITION_MIN_MEMBERS..=COALITION_MAX_MEMBERS).contains(&n),
        WarError::InvalidCoalition
    );
    require!(
        id > 0 && term_secs > 0 && term_secs <= ctx.accounts.config.params.season_secs,
        WarError::InvalidCoalition
    );
    let mut members = [Pubkey::default(); COALITION_MAX_MEMBERS];
    for (i, q) in rem.chunks(4).enumerate() {
        let mint_key = *q[0].key;
        require!(!members[..i].contains(&mint_key), WarError::InvalidCoalition);
        let mint = token_client::read_mint(&q[0])?;
        let (_, params) = config_item(&mint, &q[1], &q[2], COALITION_TEMPLATE)?;
        require!(params[0] == id, WarError::InvalidCoalition);
        require!(
            *q[3].key == WarState::address(&mint_key).0 && *q[3].owner == crate::ID,
            WarError::InvalidCoalition
        );
        members[i] = mint_key;
    }
    let c = &mut ctx.accounts.coalition;
    c.version = VERSION;
    c.bump = ctx.bumps.coalition;
    c.chest_bump = Coalition::chest(id).1;
    c.id = id;
    c.count = n as u8;
    c.members = members;
    c.created_at = now;
    c.ends_at = now.saturating_add(term_secs);
    emit_cpi!(CoalitionFormed {
        id,
        members: members[..n].to_vec(),
        ends_at: c.ends_at,
    });
    Ok(())
}

// ---- contribute -------------------------------------------------------------------------------------

/// Accounts of `contribute`. Remaining: the bridge's `unwrap_sol` / `wrap_sol` accounts of both
/// chests and the token program's `create_holding` (the shared chest's holding, when missing).
#[event_cpi]
#[derive(Accounts)]
pub struct Contribute<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [COALITION_SEED, &coalition.id.to_le_bytes()], bump = coalition.bump)]
    pub coalition: Box<Account<'info, Coalition>>,
    /// CHECK: the shared chest (seeds-checked).
    #[account(mut, seeds = [COALITION_CHEST_SEED, &coalition.id.to_le_bytes()], bump = coalition.chest_bump)]
    pub coalition_chest: UncheckedAccount<'info>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the member's chest (seeds-checked).
    #[account(mut, seeds = [CHEST_SEED, mint.key().as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the member's Coalition `Item` (checked against its slots and template).
    pub item: UncheckedAccount<'info>,
    /// CHECK: its `Template`.
    pub template: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `contribute(amount)`: permissionless; moves `amount` of the member's chest into the shared chest.
pub fn process_contribute<'info>(ctx: Context<'info, Contribute<'info>>, amount: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let mint_key = ctx.accounts.mint.key();
    let c = &ctx.accounts.coalition;
    require!(!c.dissolved && now < c.ends_at, WarError::CoalitionClosed);
    let i = c.member_index(&mint_key).ok_or(WarError::InvalidCoalition)?;
    let (_, item) = config_item(&ctx.accounts.mint, &ctx.accounts.item, &ctx.accounts.template, COALITION_TEMPLATE)?;
    require!(item[0] == c.id, WarError::InvalidCoalition);
    require!(
        c.last_contribution_at[i] == 0
            || now >= c.last_contribution_at[i].saturating_add(params.contribute_interval_secs),
        WarError::ContributionLimit
    );

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let chest = ctx.accounts.war_chest.key();
    let shared = ctx.accounts.coalition_chest.key();
    let chest_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &chest);
    let shared_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &shared);
    let before = chest_balance(find(&all, &chest_holding)?)?;
    {
        let s = &mut ctx.accounts.war_state;
        s.roll(current);
        note_funding(s, before);
    }
    let room = before.saturating_sub(ctx.accounts.war_state.rivalry.reserved(now));
    let cap = bps_of(room, u64::from(item[1]).min(BPS));
    require!(amount > 0 && amount <= cap, WarError::ContributionLimit);
    let shared_before = chest_balance(find(&all, &shared_holding)?)?;
    note_coalition(&mut ctx.accounts.coalition, shared_before);

    let seeds = chest_seeds(&mint_key, ctx.accounts.war_state.chest_bump);
    invoke_built(
        &bordrless_bridge::client::unwrap_sol(chest, amount),
        &all,
        &[&seeds.seeds()],
    )?;
    transfer_lamports(&all, &seeds.seeds(), chest, &ctx.accounts.coalition_chest.to_account_info(), amount)?;
    ensure_holding(&all, ctx.accounts.cranker.key(), BRIDGED_SOL_MINT, shared)?;
    let id = ctx.accounts.coalition.id;
    let (idb, bump) = coalition_seeds(id, ctx.accounts.coalition.chest_bump);
    let shared_seeds: [&[u8]; 3] = [COALITION_CHEST_SEED, &idb, &bump];
    invoke_built(
        &bordrless_bridge::client::wrap_sol(shared, amount),
        &all,
        &[&shared_seeds],
    )?;
    let after = chest_balance(find(&all, &chest_holding)?)?;
    let shared_after = chest_balance(find(&all, &shared_holding)?)?;
    let s = &mut ctx.accounts.war_state;
    s.sent_coalition = s.sent_coalition.saturating_add(before.saturating_sub(after));
    s.last_seen_balance = after;
    let c = &mut ctx.accounts.coalition;
    c.contributed[i] = c.contributed[i].saturating_add(amount);
    c.funded_total = c.funded_total.saturating_add(shared_after.saturating_sub(c.last_seen_balance));
    c.last_seen_balance = shared_after;
    c.last_contribution_at[i] = now;
    emit_cpi!(CoalitionContributed {
        id,
        mint: mint_key,
        amount,
        contributed: c.contributed[i],
    });
    Ok(())
}

// ---- coalition_siege --------------------------------------------------------------------------------

/// Accounts of `coalition_siege`. Remaining: the rival mint's slice for the delivery (`args.first`),
/// then the DEX swap's accounts, the shared chest's rival holding (created when missing), the token
/// program's and the bridge's `unwrap_sol`'s.
#[event_cpi]
#[derive(Accounts)]
pub struct CoalitionSiege<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [COALITION_SEED, &coalition.id.to_le_bytes()], bump = coalition.bump)]
    pub coalition: Box<Account<'info, Coalition>>,
    /// CHECK: the shared chest (seeds-checked).
    #[account(mut, seeds = [COALITION_CHEST_SEED, &coalition.id.to_le_bytes()], bump = coalition.chest_bump)]
    pub coalition_chest: UncheckedAccount<'info>,
    /// The member whose War orders and raid ledger make the siege due.
    pub member_mint: Box<Account<'info, Mint>>,
    /// CHECK: the member's War orders `Item`.
    pub orders_item: UncheckedAccount<'info>,
    /// CHECK: its `Template`.
    pub orders_template: UncheckedAccount<'info>,
    /// CHECK: the member's `RaidLedger` (address-checked, decoded).
    #[account(address = crate::foreign::raid_ledger_address(&member_mint.key()) @ WarError::WrongAccount)]
    pub raid_ledger: UncheckedAccount<'info>,
    /// CHECK: the rival's mint (read by hand).
    pub rival_mint: UncheckedAccount<'info>,
    /// CHECK: the rival's launch (read by hand).
    #[account(mut)]
    pub rival_launch: UncheckedAccount<'info>,
    /// CHECK: the rival's launch pool (read by hand).
    #[account(mut)]
    pub rival_pool: UncheckedAccount<'info>,
    /// CHECK: the rival's war state address (marked besieged when it holds one, M-4).
    #[account(mut, address = WarState::address(&rival_mint.key()).0 @ WarError::WrongAccount)]
    pub rival_war_state: UncheckedAccount<'info>,
    /// CHECK: the rival's `KitConfig`, when the rival runs the kit (R10).
    pub rival_kit_config: Option<UncheckedAccount<'info>>,
    pub system_program: Program<'info, System>,
}

/// `coalition_siege(rival)`: the shared chest spot-buys a rival that is not a member.
pub fn process_coalition_siege<'info>(ctx: Context<'info, CoalitionSiege<'info>>, args: SliceArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let member = ctx.accounts.member_mint.key();
    let rival_key = ctx.accounts.rival_mint.key();
    {
        let c = &ctx.accounts.coalition;
        require!(!c.dissolved && now < c.ends_at, WarError::CoalitionClosed);
        require!(c.member_index(&member).is_some(), WarError::InvalidCoalition);
        require!(c.member_index(&rival_key).is_none(), WarError::SelfSiege);
        require!(
            c.last_siege_at == 0 || now >= c.last_siege_at.saturating_add(params.siege_interval_secs),
            WarError::SiegeNotDue
        );
        require!(
            c.captured.iter().any(|x| x.rival_mint == rival_key || x.is_free()),
            WarError::CapturedTableFull
        );
    }
    let orders = war_orders(&ctx.accounts.member_mint, &ctx.accounts.orders_item, &ctx.accounts.orders_template)?;
    let rival_mint = token_client::read_mint(&ctx.accounts.rival_mint)?;
    let kit = ctx.accounts.rival_kit_config.as_ref().map(|a| a.to_account_info());
    require!(
        !kit_rewards_on(&rival_key, &rival_mint, kit.as_ref())?,
        WarError::SiegeTargetHasRewards
    );
    let rival_launch = read_launch(&ctx.accounts.rival_launch, &rival_key)?;
    let pool = read_pool(&ctx.accounts.rival_pool, &rival_launch)?;
    let ledger = RaidLedger::read(&ctx.accounts.raid_ledger)?;
    let threshold = u64::from(orders.get(orders::SIEGE_THRESHOLD))
        .checked_mul(params.siege_unit_lamports)
        .ok_or(WarError::MathOverflow)?;
    let rolling = ledger.rolling(&rival_key, now, params.raid_window_secs);
    require!(rolling > 0 && rolling >= threshold, WarError::SiegeNotDue);

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let shared = ctx.accounts.coalition_chest.key();
    let shared_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &shared);
    let balance_now = chest_balance(find(&all, &shared_holding)?)?;
    note_coalition(&mut ctx.accounts.coalition, balance_now);

    let spot = spot_q64(pool.quote_reserve, pool.virtual_quote, pool.base_reserve, pool.virtual_base)
        .ok_or(WarError::NoQuote)?;
    let window = i64::from(orders.get(orders::SIEGE_TWAP_SECS)).max(params.min_twap_secs);
    let twap = pool_twap(&ctx.accounts.rival_pool, now, window).ok_or(WarError::NoObservations)?;
    let ceiling = twap
        .checked_mul(u128::from(BPS + u64::from(params.siege_max_premium_bps)))
        .ok_or(WarError::MathOverflow)?
        / u128::from(BPS);
    if spot > ceiling {
        emit_cpi!(SiegeWaited {
            mint: shared,
            rival_mint: rival_key,
            rival_price: spot,
            rival_twap: twap,
        });
        return Ok(());
    }
    let spend_bps = orders.bps(orders::SIEGE_SPEND_BPS, params.siege_max_spend_bps);
    let total = bps_of(balance_now, spend_bps).min(quote_side_cap(&pool, pool_share_bps(&rival_launch)));
    require!(total > 0, WarError::NothingToDo);
    let bounty = bps_of(total, orders.crank_bps(&params));
    let spend = total - bounty;
    require!(spend > 0, WarError::NothingToDo);
    let net = bps_of(spend, BPS.saturating_sub(buy_fee_bound(&rival_launch)));
    let quote = swap_out(net, pool.quote_reserve, pool.virtual_quote, pool.base_reserve, pool.virtual_base)
        .ok_or(WarError::NoQuote)?;
    let min_out = bps_of(quote, BPS - u64::from(params.siege_slippage_bps));
    require!(min_out > 0, WarError::NoQuote);

    let id = ctx.accounts.coalition.id;
    let (idb, bump) = coalition_seeds(id, ctx.accounts.coalition.chest_bump);
    let seeds: [&[u8]; 3] = [COALITION_CHEST_SEED, &idb, &bump];
    let slice = slice_metas(ctx.remaining_accounts, usize::from(args.first))?;
    ensure_holding(&all, ctx.accounts.cranker.key(), rival_key, shared)?;
    let before = balance(&all, &rival_key, &shared)?;
    invoke_built(
        &launch_client::swap_with_base_slice(&LaunchKeys::of(&rival_launch), shared, shared, 1, spend, min_out, slice),
        &all,
        &[&seeds],
    )?;
    let bought = balance(&all, &rival_key, &shared)?
        .checked_sub(before)
        .ok_or(WarError::MathOverflow)?;
    pay_sol(&all, &seeds, shared, &ctx.accounts.cranker.to_account_info(), bounty)?;
    let balance_after = chest_balance(find(&all, &shared_holding)?)?;
    let c = &mut ctx.accounts.coalition;
    let entry = match c.captured.iter().position(|x| x.rival_mint == rival_key) {
        Some(i) => i,
        None => c.captured.iter().position(|x| x.is_free()).ok_or(WarError::CapturedTableFull)?,
    };
    let e = &mut c.captured[entry];
    if e.is_free() {
        *e = Captured {
            rival_mint: rival_key,
            captured_at: now,
            ..Captured::default()
        };
    }
    e.amount = e.amount.checked_add(bought).ok_or(WarError::MathOverflow)?;
    e.cost = e.cost.saturating_add(spend);
    c.spent_siege = c.spent_siege.saturating_add(spend);
    c.paid_cranks = c.paid_cranks.saturating_add(bounty);
    c.last_siege_at = now;
    c.last_seen_balance = balance_after;
    mark_besieged(
        &ctx.accounts.rival_war_state.to_account_info(),
        current,
        now.saturating_add(params.siege_interval_secs),
        shared,
    )?;
    emit_cpi!(CoalitionSiegeExecuted {
        id,
        rival_mint: rival_key,
        spent: spend,
        bought,
        bounty,
        cranker: ctx.accounts.cranker.key(),
    });
    Ok(())
}

// ---- coalition_raze ---------------------------------------------------------------------------------

/// Accounts of `coalition_raze`. Remaining: as `raze`'s, for the shared chest.
#[event_cpi]
#[derive(Accounts)]
pub struct CoalitionRaze<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [COALITION_SEED, &coalition.id.to_le_bytes()], bump = coalition.bump)]
    pub coalition: Box<Account<'info, Coalition>>,
    /// CHECK: the shared chest (seeds-checked).
    #[account(mut, seeds = [COALITION_CHEST_SEED, &coalition.id.to_le_bytes()], bump = coalition.chest_bump)]
    pub coalition_chest: UncheckedAccount<'info>,
    /// CHECK: the rival's mint (read by hand).
    pub rival_mint: UncheckedAccount<'info>,
    /// CHECK: the rival's launch (read by hand).
    #[account(mut)]
    pub rival_launch: UncheckedAccount<'info>,
    /// CHECK: the rival's launch pool (read by hand).
    #[account(mut)]
    pub rival_pool: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `coalition_raze(rival)`: sells captured rival tokens of the shared chest back into the rival's
/// pool at the raze rate limit, waiting below the M-3 floor (window `min_twap_secs`); the crank
/// bounty is `max_crank_bounty_bps` of the proceeds.
pub fn process_coalition_raze<'info>(ctx: Context<'info, CoalitionRaze<'info>>, args: SliceArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let params = ctx.accounts.config.params;
    let rival_key = ctx.accounts.rival_mint.key();
    token_client::read_mint(&ctx.accounts.rival_mint)?;
    let rival_launch = read_launch(&ctx.accounts.rival_launch, &rival_key)?;
    let pool = read_pool(&ctx.accounts.rival_pool, &rival_launch)?;
    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let shared = ctx.accounts.coalition_chest.key();
    let shared_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &shared);
    let balance_now = chest_balance(find(&all, &shared_holding)?)?;
    let sell = {
        let c = &mut ctx.accounts.coalition;
        note_coalition(c, balance_now);
        let e = c
            .captured
            .iter_mut()
            .find(|x| x.rival_mint == rival_key && x.amount > 0)
            .ok_or(WarError::NothingToDo)?;
        if e.raze_window_start == 0 || now >= e.raze_window_start.saturating_add(params.raze_interval_secs) {
            e.raze_window_start = now;
            e.raze_window_base = e.amount;
            e.razed_in_window = 0;
        }
        let allowance = bps_of(e.raze_window_base, u64::from(params.raze_max_bps_per_interval))
            .saturating_sub(e.razed_in_window);
        require!(allowance > 0, WarError::RazeLimit);
        let side = u128::from(pool.base_reserve) + u128::from(pool.virtual_base);
        let pool_cap = (side * u128::from(pool_share_bps(&rival_launch)) / u128::from(BPS)).min(u128::from(u64::MAX)) as u64;
        allowance.min(e.amount).min(pool_cap)
    };
    require!(sell > 0, WarError::NothingToDo);
    let spot = spot_q64(pool.quote_reserve, pool.virtual_quote, pool.base_reserve, pool.virtual_base)
        .ok_or(WarError::NoQuote)?;
    let twap = pool_twap(&ctx.accounts.rival_pool, now, params.min_twap_secs).ok_or(WarError::NoObservations)?;
    let floor = twap
        .checked_mul(u128::from(BPS - u64::from(params.raze_max_discount_bps)))
        .ok_or(WarError::MathOverflow)?
        / u128::from(BPS);
    if spot < floor {
        emit_cpi!(RazeWaited {
            mint: shared,
            rival_mint: rival_key,
            rival_price: spot,
            rival_twap: twap,
        });
        return Ok(());
    }
    let gross = swap_out(sell, pool.base_reserve, pool.virtual_base, pool.quote_reserve, pool.virtual_quote)
        .ok_or(WarError::NoQuote)?;
    let net = bps_of(gross, BPS.saturating_sub(sell_fee_bound(&rival_launch)));
    let min_out = bps_of(net, BPS - u64::from(params.siege_slippage_bps));
    require!(min_out > 0, WarError::NoQuote);
    let id = ctx.accounts.coalition.id;
    let (idb, bump) = coalition_seeds(id, ctx.accounts.coalition.chest_bump);
    let seeds: [&[u8]; 3] = [COALITION_CHEST_SEED, &idb, &bump];
    let slice = slice_metas(ctx.remaining_accounts, usize::from(args.first))?;
    let before_tokens = balance(&all, &rival_key, &shared)?;
    invoke_built(
        &launch_client::swap_with_base_slice(&LaunchKeys::of(&rival_launch), shared, shared, 0, sell, min_out, slice),
        &all,
        &[&seeds],
    )?;
    let sold = before_tokens
        .checked_sub(balance(&all, &rival_key, &shared)?)
        .ok_or(WarError::MathOverflow)?;
    let got = chest_balance(find(&all, &shared_holding)?)?
        .checked_sub(balance_now)
        .ok_or(WarError::MathOverflow)?;
    let bounty = bps_of(got, u64::from(params.max_crank_bounty_bps));
    pay_sol(&all, &seeds, shared, &ctx.accounts.cranker.to_account_info(), bounty)?;
    let balance_after = chest_balance(find(&all, &shared_holding)?)?;
    let c = &mut ctx.accounts.coalition;
    let e = c
        .captured
        .iter_mut()
        .find(|x| x.rival_mint == rival_key)
        .ok_or(WarError::NothingToDo)?;
    e.amount = e.amount.saturating_sub(sold);
    e.razed_in_window = e.razed_in_window.saturating_add(sold);
    if e.amount == 0 {
        *e = Captured::default();
    }
    c.razed_proceeds = c.razed_proceeds.saturating_add(got);
    c.paid_cranks = c.paid_cranks.saturating_add(bounty);
    c.last_seen_balance = balance_after;
    emit_cpi!(CoalitionRazed {
        id,
        rival_mint: rival_key,
        sold,
        got,
        bounty,
    });
    Ok(())
}

// ---- dissolve_coalition -----------------------------------------------------------------------------

/// Accounts of `dissolve_coalition`. Remaining: per member in the coalition's order, `war state
/// (mut), chest (mut), chest's bridged-SOL holding (mut)`; then the bridge's `unwrap_sol` /
/// `wrap_sol` accounts.
#[event_cpi]
#[derive(Accounts)]
pub struct DissolveCoalition<'info> {
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [COALITION_SEED, &coalition.id.to_le_bytes()], bump = coalition.bump)]
    pub coalition: Box<Account<'info, Coalition>>,
    /// CHECK: the shared chest (seeds-checked).
    #[account(mut, seeds = [COALITION_CHEST_SEED, &coalition.id.to_le_bytes()], bump = coalition.chest_bump)]
    pub coalition_chest: UncheckedAccount<'info>,
    /// CHECK: its bridged-SOL holding (address-checked; may not exist).
    #[account(mut, address = token_client::holding_address(&BRIDGED_SOL_MINT, &coalition_chest.key()))]
    pub coalition_holding: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Pays `share` (lamports already in `chest`) into the member's chest holding and books it.
#[inline(never)]
fn return_share<'info>(
    all: &[AccountInfo<'info>],
    q: &[AccountInfo<'info>],
    mint: &Pubkey,
    current: u32,
    share: u64,
) -> Result<()> {
    require!(*q[0].key == WarState::address(mint).0 && *q[0].owner == crate::ID, WarError::WrongAccount);
    let mut s = Box::new(WarState::try_deserialize(&mut &q[0].try_borrow_data()?[..])?);
    require!(*q[1].key == chest_address(mint).0, WarError::WrongAccount);
    require!(
        *q[2].key == token_client::holding_address(&BRIDGED_SOL_MINT, q[1].key),
        WarError::WrongAccount
    );
    let before = chest_balance(&q[2])?;
    s.roll(current);
    note_funding(&mut s, before);
    if share > 0 {
        let seeds = chest_seeds(mint, s.chest_bump);
        invoke_built(&bordrless_bridge::client::wrap_sol(*q[1].key, share), all, &[&seeds.seeds()])?;
    }
    let after = chest_balance(&q[2])?;
    s.received_other = s.received_other.saturating_add(after.saturating_sub(before));
    s.last_seen_balance = after;
    s.try_serialize(&mut &mut q[0].try_borrow_mut_data()?[..])?;
    Ok(())
}

/// `dissolve_coalition`: permissionless after the term once nothing captured is left; returns the
/// shared chest pro rata of contributions (equal shares when nobody contributed), the rounding to
/// the last member. Booked by each member as `received_other`, never as season funding.
pub fn process_dissolve_coalition<'info>(ctx: Context<'info, DissolveCoalition<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let current = ctx.accounts.config.current_season;
    let (n, id, members, contributed) = {
        let c = &ctx.accounts.coalition;
        require!(!c.dissolved, WarError::CoalitionClosed);
        require!(now >= c.ends_at && c.captured.iter().all(|x| x.amount == 0), WarError::CoalitionNotDone);
        (usize::from(c.count), c.id, c.members, c.contributed)
    };
    let rem = ctx.remaining_accounts;
    require!(rem.len() >= 3 * n, WarError::MissingAccount);
    let total_balance = chest_balance(&ctx.accounts.coalition_holding)?;
    note_coalition(&mut ctx.accounts.coalition, total_balance);
    let all = available(ctx.accounts.to_account_infos(), rem);
    let shared = ctx.accounts.coalition_chest.key();
    let (idb, bump) = coalition_seeds(id, ctx.accounts.coalition.chest_bump);
    let seeds: [&[u8]; 3] = [COALITION_CHEST_SEED, &idb, &bump];
    if total_balance > 0 {
        invoke_built(&bordrless_bridge::client::unwrap_sol(shared, total_balance), &all, &[&seeds])?;
    }
    let weight_total: u128 = contributed[..n].iter().map(|v| u128::from(*v)).sum();
    let mut left = total_balance;
    let mut returned = Vec::with_capacity(n);
    for i in 0..n {
        let share = if i + 1 == n {
            left
        } else if weight_total == 0 {
            total_balance / n as u64
        } else {
            (u128::from(total_balance) * u128::from(contributed[i]) / weight_total) as u64
        };
        left -= share;
        let q = &rem[3 * i..3 * i + 3];
        transfer_lamports(&all, &seeds, shared, &q[1], share)?;
        return_share(&all, q, &members[i], current, share)?;
        returned.push(share);
    }
    let c = &mut ctx.accounts.coalition;
    c.returned = c.returned.saturating_add(total_balance);
    c.last_seen_balance = chest_balance(&ctx.accounts.coalition_holding)?;
    c.dissolved = true;
    emit_cpi!(CoalitionDissolved { id, returned });
    Ok(())
}
