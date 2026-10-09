// Changed by Hookwars: security review 1: the siege always names the rival's war state (M-4), the raze waits below the TWAP floor (M-3)
//! Attack and defense paid from the chest (05 section 6): `siege`, `counter_strike`, `raze`,
//! `return_captured`. Each is permissionless, checks on chain that it is due, is capped per call
//! and per interval, and pays its sender at most the War orders' crank bounty (capped by the
//! config).
//!
//! The DEX, the launchpad and the token program write the pool, the launch and the mints these
//! steps touch, so those accounts are read by hand (never `Account<..>` with `mut`, whose exit
//! would write back what it read before the swap).

use anchor_lang::prelude::*;
use bordrless_core::swap_out;
use bordrless_launch::client::{self as launch_client, LaunchKeys};
use bordrless_launch::state::Launch;
use bordrless_swap::state::Pool;
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::Foreign;
use crate::foreign::{pool_twap, spot_q64, Item, RaidLedger, Template};
use crate::instructions::admin::read_own;
use crate::instructions::setup::{chest_balance, note_funding};
use crate::state::*;

/// The fee bound of a buy on a launch pool, in bps of the input (upstream companion `steps.rs`
/// 315-319): the LP fee, the creator and holder fees, Bordrless's share of them (rounded up), the
/// burn on buys.
pub fn buy_fee_bound(launch: &Launch) -> u64 {
    let r = &launch.rules;
    let cuts = u64::from(launch.creator_fee_bps) + u64::from(r.holder_fee_buy_bps);
    u64::from(launch.lp_fee_bps) + cuts + cuts.div_ceil(4) + u64::from(r.burn_buy_bps)
}

/// The same bound on a sell (sell rates).
pub fn sell_fee_bound(launch: &Launch) -> u64 {
    let r = &launch.rules;
    let cuts = u64::from(launch.creator_fee_bps) + u64::from(r.holder_fee_sell_bps);
    u64::from(launch.lp_fee_bps) + cuts + cuts.div_ceil(4) + u64::from(r.burn_sell_bps)
}

/// The companion's sandwich bound (`steps.rs` 326-333): the most a chest may move on `launch`'s
/// pool, in bps of a side.
pub fn pool_share_bps(launch: &Launch) -> u64 {
    bordrless_companion::pool_share_bps(launch)
}

fn read_launch(info: &AccountInfo, mint: &Pubkey) -> Result<Launch> {
    require_keys_eq!(
        *info.key,
        launch_client::launch_address(mint),
        WarError::WrongLaunch
    );
    let launch = read_own::<Launch>(info)?;
    require_keys_eq!(launch.mint, *mint, WarError::WrongLaunch);
    Ok(launch)
}

fn read_pool(info: &AccountInfo, launch: &Launch) -> Result<Pool> {
    require_keys_eq!(*info.key, launch.pool, WarError::WrongAccount);
    read_own::<Pool>(info)
}

/// The pool's quote side (real and virtual) times `bps`.
fn quote_side_cap(pool: &Pool, bps: u64) -> u64 {
    let side = u128::from(pool.quote_reserve) + u128::from(pool.virtual_quote);
    (side * u128::from(bps) / u128::from(BPS)).min(u128::from(u64::MAX)) as u64
}

fn chest_seeds(mint: &Pubkey, bump: u8) -> KeyedSeeds {
    KeyedSeeds::new(CHEST_SEED, *mint, bump)
}

// ---- siege -----------------------------------------------------------------------------------------

/// Arguments of `siege`, `counter_strike`, `raze` and `return_captured`: how many of the
/// remaining accounts are token slices the client resolved (in the order the step names).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default)]
pub struct SliceArgs {
    pub first: u8,
    pub second: u8,
}

/// Accounts of `siege`. Remaining: the rival mint's slice for the delivery (`args.first`
/// accounts), then the DEX swap's accounts on the rival pool, the chest's rival holding (created
/// when missing, the cranker paying), the token program's and the bridge's `unwrap_sol`'s.
#[event_cpi]
#[derive(Accounts)]
pub struct Siege<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest (seeds-checked).
    #[account(mut, seeds = [CHEST_SEED, mint.key().as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the War orders `Item` (checked against the mint's War slot).
    pub orders_item: UncheckedAccount<'info>,
    /// CHECK: its `Template` (address- and kind-checked).
    pub orders_template: UncheckedAccount<'info>,
    /// CHECK: `["raid-ledger", mint]` under the items program (address-checked, decoded).
    #[account(address = crate::foreign::raid_ledger_address(&mint.key()) @ WarError::WrongAccount)]
    pub raid_ledger: UncheckedAccount<'info>,
    /// CHECK: the rival's mint (read by hand: the swap may write it).
    pub rival_mint: UncheckedAccount<'info>,
    /// CHECK: the rival's launch (read by hand).
    #[account(mut)]
    pub rival_launch: UncheckedAccount<'info>,
    /// CHECK: the rival's launch pool (read by hand).
    #[account(mut)]
    pub rival_pool: UncheckedAccount<'info>,
    /// CHECK: the rival's war state address, always passed (security review 1, M-4): when it holds
    /// a war state it is marked besieged; empty only while the rival has no war state.
    #[account(mut, address = WarState::address(&rival_mint.key()).0 @ WarError::WrongAccount)]
    pub rival_war_state: UncheckedAccount<'info>,
    /// CHECK: the rival's `KitConfig`, when the rival runs the kit (checked when read).
    pub rival_kit_config: Option<UncheckedAccount<'info>>,
    pub system_program: Program<'info, System>,
}

/// `siege(rival)`: the chest spot-buys the rival on its launch pool and keeps what it buys
/// (05 section 6.2).
pub fn process_siege<'info>(ctx: Context<'info, Siege<'info>>, args: SliceArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let mint_key = ctx.accounts.mint.key();
    let rival_key = ctx.accounts.rival_mint.key();
    require_keys_neq!(rival_key, mint_key, WarError::SelfSiege);
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let orders = war_orders(
        &ctx.accounts.mint,
        &ctx.accounts.orders_item,
        &ctx.accounts.orders_template,
    )?;
    let rival_mint = token_client::read_mint(&ctx.accounts.rival_mint)?;
    let kit = ctx.accounts.rival_kit_config.as_ref().map(|a| a.to_account_info());
    require!(
        !kit_rewards_on(&rival_key, &rival_mint, kit.as_ref())?,
        WarError::SiegeTargetHasRewards
    );
    let rival_launch = read_launch(&ctx.accounts.rival_launch, &rival_key)?;
    let pool = read_pool(&ctx.accounts.rival_pool, &rival_launch)?;

    // Due: rolling raid volume from the rival, spacing, room in the captured table.
    let ledger = RaidLedger::read(&ctx.accounts.raid_ledger)?;
    let threshold = u64::from(orders.get(orders::SIEGE_THRESHOLD))
        .checked_mul(params.siege_unit_lamports)
        .ok_or(WarError::MathOverflow)?;
    let rolling = ledger.rolling(&rival_key, now, params.raid_window_secs);
    require!(rolling > 0 && rolling >= threshold, WarError::SiegeNotDue);
    let state = &ctx.accounts.war_state;
    require!(
        state.last_siege_at == 0
            || now >= state.last_siege_at.saturating_add(params.siege_interval_secs),
        WarError::SiegeNotDue
    );
    require!(
        state.captured.iter().any(|c| c.rival_mint == rival_key || c.is_free()),
        WarError::CapturedTableFull
    );

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let chest = ctx.accounts.war_chest.key();
    let chest_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &chest);
    let balance_now = chest_balance(find(&all, &chest_holding)?)?;
    {
        let s = &mut ctx.accounts.war_state;
        s.roll(current);
        note_funding(s, balance_now);
    }

    // The premium check: the rival's spot price against its TWAP.
    let spot = spot_q64(
        pool.quote_reserve,
        pool.virtual_quote,
        pool.base_reserve,
        pool.virtual_base,
    )
    .ok_or(WarError::NoQuote)?;
    let window = i64::from(orders.get(orders::SIEGE_TWAP_SECS)).max(params.min_twap_secs);
    // Hookwars M3b: the ring is in the rival pool's own account (03 M3a notes).
    let twap = pool_twap(&ctx.accounts.rival_pool, now, window).ok_or(WarError::NoObservations)?;
    let ceiling = twap
        .checked_mul(u128::from(BPS + u64::from(params.siege_max_premium_bps)))
        .ok_or(WarError::MathOverflow)?
        / u128::from(BPS);
    if spot > ceiling {
        emit_cpi!(SiegeWaited {
            mint: mint_key,
            rival_mint: rival_key,
            rival_price: spot,
            rival_twap: twap,
        });
        return Ok(());
    }

    // Spend, bounty and the least the buy must bring.
    let spend_bps = orders.bps(orders::SIEGE_SPEND_BPS, params.siege_max_spend_bps);
    let total = bps_of(balance_now, spend_bps).min(quote_side_cap(&pool, pool_share_bps(&rival_launch)));
    require!(total > 0, WarError::NothingToDo);
    let bounty = bps_of(total, orders.crank_bps(&params));
    let spend = total - bounty;
    require!(spend > 0, WarError::NothingToDo);
    let net = bps_of(spend, BPS.saturating_sub(buy_fee_bound(&rival_launch)));
    let quote = swap_out(
        net,
        pool.quote_reserve,
        pool.virtual_quote,
        pool.base_reserve,
        pool.virtual_base,
    )
    .ok_or(WarError::NoQuote)?;
    let min_out = bps_of(quote, BPS - u64::from(params.siege_slippage_bps));
    require!(min_out > 0, WarError::NoQuote);

    let seeds = chest_seeds(&mint_key, ctx.accounts.war_state.chest_bump);
    let slice = slice_metas(ctx.remaining_accounts, usize::from(args.first))?;
    ensure_holding(&all, ctx.accounts.cranker.key(), rival_key, chest)?;
    let before = balance(&all, &rival_key, &chest)?;
    invoke_built(
        &launch_client::swap_with_base_slice(
            &LaunchKeys::of(&rival_launch),
            chest,
            chest,
            1,
            spend,
            min_out,
            slice,
        ),
        &all,
        &[&seeds.seeds()],
    )?;
    let bought = balance(&all, &rival_key, &chest)?
        .checked_sub(before)
        .ok_or(WarError::MathOverflow)?;
    pay_sol(
        &all,
        &seeds.seeds(),
        chest,
        &ctx.accounts.cranker.to_account_info(),
        bounty,
    )?;
    let balance_after = chest_balance(find(&all, &chest_holding)?)?;

    let s = &mut ctx.accounts.war_state;
    let entry = match s.captured.iter().position(|c| c.rival_mint == rival_key) {
        Some(i) => i,
        None => s
            .captured
            .iter()
            .position(|c| c.is_free())
            .ok_or(WarError::CapturedTableFull)?,
    };
    let c = &mut s.captured[entry];
    if c.is_free() {
        *c = Captured {
            rival_mint: rival_key,
            captured_at: now,
            ..Captured::default()
        };
    }
    c.amount = c.amount.checked_add(bought).ok_or(WarError::MathOverflow)?;
    c.cost = c.cost.saturating_add(spend);
    let captured_total = c.amount;
    s.season.sieges = s.season.sieges.saturating_add(1);
    s.season.siege_spend = s.season.siege_spend.saturating_add(spend);
    s.spent_siege = s.spent_siege.saturating_add(spend);
    s.paid_cranks = s.paid_cranks.saturating_add(bounty);
    s.last_siege_at = now;
    s.last_seen_balance = balance_after;

    let rival_info = ctx.accounts.rival_war_state.to_account_info();
    if *rival_info.owner == crate::ID && rival_info.data_len() > 0 {
        let mut rival_state = WarState::try_deserialize(&mut &rival_info.try_borrow_data()?[..])?;
        rival_state.roll(current);
        rival_state.under_siege_until = now.saturating_add(params.siege_interval_secs);
        rival_state.siege_by_chest = chest;
        rival_state.season.times_besieged = rival_state.season.times_besieged.saturating_add(1);
        rival_state.try_serialize(&mut &mut rival_info.try_borrow_mut_data()?[..])?;
    }
    emit_cpi!(SiegeExecuted {
        mint: mint_key,
        rival_mint: rival_key,
        spent: spend,
        bought,
        bounty,
        cranker: ctx.accounts.cranker.key(),
        captured_total,
    });
    Ok(())
}

// ---- counter_strike --------------------------------------------------------------------------------

/// Accounts of `counter_strike`. Remaining: the mint's slice for the buy's delivery
/// (`args.first`), its slice for the burn (`args.second`), then the DEX swap's accounts, the
/// chest's token holding (created when missing), the token program's and the bridge's.
#[event_cpi]
#[derive(Accounts)]
pub struct CounterStrike<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, war_state.mint.as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest (seeds-checked).
    #[account(mut, seeds = [CHEST_SEED, war_state.mint.as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    /// CHECK: the mint (address-checked, read by hand: the burn writes it).
    #[account(mut, address = war_state.mint @ WarError::WrongAccount)]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: the War orders `Item`.
    pub orders_item: UncheckedAccount<'info>,
    /// CHECK: its `Template`.
    pub orders_template: UncheckedAccount<'info>,
    /// CHECK: the mint's launch (read by hand).
    #[account(mut)]
    pub launch: UncheckedAccount<'info>,
    /// CHECK: the launch pool (read by hand).
    #[account(mut)]
    pub pool: UncheckedAccount<'info>,
    /// CHECK: the mint's `KitConfig`, when it runs the kit.
    pub kit_config: Option<UncheckedAccount<'info>>,
    pub system_program: Program<'info, System>,
}

/// `counter_strike`: after a fall, the chest buys its own token on its launch pool and burns it
/// (05 section 6.3).
pub fn process_counter_strike<'info>(
    ctx: Context<'info, CounterStrike<'info>>,
    args: SliceArgs,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let mint_key = ctx.accounts.mint.key();
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let mint = token_client::read_mint(&ctx.accounts.mint)?;
    let kit = ctx.accounts.kit_config.as_ref().map(|a| a.to_account_info());
    require!(
        !kit_rewards_on(&mint_key, &mint, kit.as_ref())?,
        WarError::OwnTokenHasRewards
    );
    let orders = war_orders(&mint, &ctx.accounts.orders_item, &ctx.accounts.orders_template)?;
    let spend_bps = orders.bps(orders::COUNTER_SPEND_BPS, params.counter_max_spend_bps);
    require!(spend_bps > 0, WarError::NoWarOrders);
    let launch = read_launch(&ctx.accounts.launch, &mint_key)?;
    let pool = read_pool(&ctx.accounts.pool, &launch)?;

    // The trigger: the short TWAP at least `counter_drop_bps` below the long one.
    let short = i64::from(orders.get(orders::COUNTER_SHORT_SECS)).max(params.min_twap_secs);
    let long = i64::from(orders.get(orders::COUNTER_LONG_SECS)).max(params.min_twap_secs);
    require!(short < long, WarError::CounterNotDue);
    let twap_short = pool_twap(&ctx.accounts.pool, now, short).ok_or(WarError::NoObservations)?;
    let twap_long = pool_twap(&ctx.accounts.pool, now, long).ok_or(WarError::NoObservations)?;
    let drop = u64::from(orders.get(orders::COUNTER_DROP_BPS)).min(BPS);
    let lhs = twap_short.checked_mul(u128::from(BPS)).ok_or(WarError::MathOverflow)?;
    let rhs = twap_long
        .checked_mul(u128::from(BPS - drop))
        .ok_or(WarError::MathOverflow)?;
    require!(lhs <= rhs, WarError::CounterNotDue);
    let interval = i64::from(orders.get(orders::COUNTER_INTERVAL_SECS)).max(params.counter_min_interval_secs);
    let state = &ctx.accounts.war_state;
    require!(
        (state.last_counter_at == 0 || now >= state.last_counter_at.saturating_add(interval))
            && now >= launch.created_at.saturating_add(BUYBACK_AFTER_LAUNCH),
        WarError::CounterNotDue
    );

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let chest = ctx.accounts.war_chest.key();
    let chest_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &chest);
    let balance_now = chest_balance(find(&all, &chest_holding)?)?;
    {
        let s = &mut ctx.accounts.war_state;
        s.roll(current);
        note_funding(s, balance_now);
    }
    let total = bps_of(balance_now, spend_bps).min(quote_side_cap(&pool, pool_share_bps(&launch)));
    require!(total > 0, WarError::NothingToDo);
    let bounty = bps_of(total, orders.crank_bps(&params));
    let spend = total - bounty;
    require!(spend > 0, WarError::NothingToDo);
    let net = bps_of(spend, BPS.saturating_sub(buy_fee_bound(&launch)));
    let quote = swap_out(
        net,
        pool.quote_reserve,
        pool.virtual_quote,
        pool.base_reserve,
        pool.virtual_base,
    )
    .ok_or(WarError::NoQuote)?;
    let min_out = bps_of(quote, BPS - BUYBACK_SLIPPAGE_BPS);
    require!(min_out > 0, WarError::NoQuote);

    let seeds = chest_seeds(&mint_key, ctx.accounts.war_state.chest_bump);
    let buy_slice = slice_metas(ctx.remaining_accounts, usize::from(args.first))?;
    let burn_slice = slice_metas(
        &ctx.remaining_accounts[usize::from(args.first)..],
        usize::from(args.second),
    )?;
    ensure_holding(&all, ctx.accounts.cranker.key(), mint_key, chest)?;
    let before = balance(&all, &mint_key, &chest)?;
    invoke_built(
        &launch_client::swap_with_base_slice(
            &LaunchKeys::of(&launch),
            chest,
            chest,
            1,
            spend,
            min_out,
            buy_slice,
        ),
        &all,
        &[&seeds.seeds()],
    )?;
    let bought = balance(&all, &mint_key, &chest)?
        .checked_sub(before)
        .ok_or(WarError::MathOverflow)?;
    invoke_built(
        &token_client::burn_with(
            chest,
            token_client::holding_address(&mint_key, &chest),
            mint_key,
            token_hook(&mint),
            burn_slice,
            bought,
        ),
        &all,
        &[&seeds.seeds()],
    )?;
    pay_sol(
        &all,
        &seeds.seeds(),
        chest,
        &ctx.accounts.cranker.to_account_info(),
        bounty,
    )?;
    let balance_after = chest_balance(find(&all, &chest_holding)?)?;
    let s = &mut ctx.accounts.war_state;
    s.season.counter_strikes = s.season.counter_strikes.saturating_add(1);
    s.spent_counter = s.spent_counter.saturating_add(spend);
    s.paid_cranks = s.paid_cranks.saturating_add(bounty);
    s.last_counter_at = now;
    s.last_seen_balance = balance_after;
    emit_cpi!(CounterStrikeExecuted {
        mint: mint_key,
        spent: spend,
        burned: bought,
        bounty,
        cranker: ctx.accounts.cranker.key(),
    });
    Ok(())
}

// ---- raze ------------------------------------------------------------------------------------------

/// Accounts of `raze`. Remaining: the rival mint's slice for the sell's input (`args.first`), then
/// the DEX swap's accounts, the chest's holdings, the token program's and the bridge's.
#[event_cpi]
#[derive(Accounts)]
pub struct Raze<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest (seeds-checked).
    #[account(mut, seeds = [CHEST_SEED, mint.key().as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the War orders `Item`.
    pub orders_item: UncheckedAccount<'info>,
    /// CHECK: its `Template`.
    pub orders_template: UncheckedAccount<'info>,
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

/// `raze(rival)`: sells captured rival tokens back into the rival's pool, slowly, when the War
/// orders allow it (05 section 6.4).
pub fn process_raze<'info>(ctx: Context<'info, Raze<'info>>, args: SliceArgs) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let mint_key = ctx.accounts.mint.key();
    let rival_key = ctx.accounts.rival_mint.key();
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let orders = war_orders(
        &ctx.accounts.mint,
        &ctx.accounts.orders_item,
        &ctx.accounts.orders_template,
    )?;
    require!(orders.get(orders::RAZE_ENABLED) == 1, WarError::RazeDisabled);
    token_client::read_mint(&ctx.accounts.rival_mint)?;
    let rival_launch = read_launch(&ctx.accounts.rival_launch, &rival_key)?;
    let pool = read_pool(&ctx.accounts.rival_pool, &rival_launch)?;

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let chest = ctx.accounts.war_chest.key();
    let chest_holding = token_client::holding_address(&BRIDGED_SOL_MINT, &chest);
    let balance_now = chest_balance(find(&all, &chest_holding)?)?;
    let sell = {
        let s = &mut ctx.accounts.war_state;
        s.roll(current);
        note_funding(s, balance_now);
        let c = s.captured_of(&rival_key).ok_or(WarError::NothingToDo)?;
        require!(c.amount > 0, WarError::NothingToDo);
        if c.raze_window_start == 0 || now >= c.raze_window_start.saturating_add(params.raze_interval_secs) {
            c.raze_window_start = now;
            c.raze_window_base = c.amount;
            c.razed_in_window = 0;
        }
        let allowance = bps_of(c.raze_window_base, u64::from(params.raze_max_bps_per_interval))
            .saturating_sub(c.razed_in_window);
        require!(allowance > 0, WarError::RazeLimit);
        let side = u128::from(pool.base_reserve) + u128::from(pool.virtual_base);
        let pool_cap = (side * u128::from(pool_share_bps(&rival_launch)) / u128::from(BPS))
            .min(u128::from(u64::MAX)) as u64;
        allowance.min(c.amount).min(pool_cap)
    };
    require!(sell > 0, WarError::NothingToDo);
    // Security review 1, M-3: the floor check, the mirror of the siege's premium check. A raze waits
    // while the rival's spot price is below its TWAP by more than `raze_max_discount_bps`, so a
    // searcher cannot depress the pool in the same transaction and buy the chest's tokens cheap.
    let spot = spot_q64(
        pool.quote_reserve,
        pool.virtual_quote,
        pool.base_reserve,
        pool.virtual_base,
    )
    .ok_or(WarError::NoQuote)?;
    let window = i64::from(orders.get(orders::SIEGE_TWAP_SECS)).max(params.min_twap_secs);
    let twap = pool_twap(&ctx.accounts.rival_pool, now, window).ok_or(WarError::NoObservations)?;
    let floor = twap
        .checked_mul(u128::from(BPS - u64::from(params.raze_max_discount_bps)))
        .ok_or(WarError::MathOverflow)?
        / u128::from(BPS);
    if spot < floor {
        emit_cpi!(RazeWaited {
            mint: mint_key,
            rival_mint: rival_key,
            rival_price: spot,
            rival_twap: twap,
        });
        return Ok(());
    }
    let gross = swap_out(
        sell,
        pool.base_reserve,
        pool.virtual_base,
        pool.quote_reserve,
        pool.virtual_quote,
    )
    .ok_or(WarError::NoQuote)?;
    let net = bps_of(gross, BPS.saturating_sub(sell_fee_bound(&rival_launch)));
    let min_out = bps_of(net, BPS - u64::from(params.siege_slippage_bps));
    require!(min_out > 0, WarError::NoQuote);

    let seeds = chest_seeds(&mint_key, ctx.accounts.war_state.chest_bump);
    let slice = slice_metas(ctx.remaining_accounts, usize::from(args.first))?;
    let before_tokens = balance(&all, &rival_key, &chest)?;
    invoke_built(
        &launch_client::swap_with_base_slice(
            &LaunchKeys::of(&rival_launch),
            chest,
            chest,
            0,
            sell,
            min_out,
            slice,
        ),
        &all,
        &[&seeds.seeds()],
    )?;
    let sold = before_tokens
        .checked_sub(balance(&all, &rival_key, &chest)?)
        .ok_or(WarError::MathOverflow)?;
    let got = chest_balance(find(&all, &chest_holding)?)?
        .checked_sub(balance_now)
        .ok_or(WarError::MathOverflow)?;
    let bounty = bps_of(got, orders.crank_bps(&params));
    pay_sol(
        &all,
        &seeds.seeds(),
        chest,
        &ctx.accounts.cranker.to_account_info(),
        bounty,
    )?;
    let balance_after = chest_balance(find(&all, &chest_holding)?)?;
    let s = &mut ctx.accounts.war_state;
    let c = s.captured_of(&rival_key).ok_or(WarError::NothingToDo)?;
    c.amount = c.amount.saturating_sub(sold);
    c.razed_in_window = c.razed_in_window.saturating_add(sold);
    let left = c.amount;
    if left == 0 {
        *c = Captured::default();
    }
    // Gross: the bounty is counted in `paid_cranks` (chest solvency, 07 invariant 5).
    s.razed_proceeds = s.razed_proceeds.saturating_add(got);
    s.paid_cranks = s.paid_cranks.saturating_add(bounty);
    s.last_seen_balance = balance_after;
    emit_cpi!(Razed {
        mint: mint_key,
        rival_mint: rival_key,
        sold,
        got,
        bounty,
        cranker: ctx.accounts.cranker.key(),
        captured_left: left,
    });
    Ok(())
}

// ---- return_captured -------------------------------------------------------------------------------

/// Accounts of `return_captured`. Remaining: the rival mint's `Locked` slice for a protocol
/// transfer (`args.first`; empty without a Locked slot), then the holdings and the token program's.
#[event_cpi]
#[derive(Accounts)]
pub struct ReturnCaptured<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest (seeds-checked).
    #[account(seeds = [CHEST_SEED, mint.key().as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the rival's mint (read by hand).
    pub rival_mint: UncheckedAccount<'info>,
    /// CHECK: the Treaty `Item` both mints equip.
    pub treaty_item: UncheckedAccount<'info>,
    /// CHECK: its `Template`.
    pub treaty_template: UncheckedAccount<'info>,
    /// CHECK: the rival's chest (address-checked).
    #[account(address = chest_address(&rival_mint.key()).0 @ WarError::WrongAccount)]
    pub rival_war_chest: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `return_captured(rival)` (peace): every captured rival token to the rival's chest, when the
/// Treaty both mints equip returns captured holdings (05 section 6.5; 04 section 3.5 field 2).
pub fn process_return_captured<'info>(
    ctx: Context<'info, ReturnCaptured<'info>>,
    args: SliceArgs,
) -> Result<()> {
    let mint_key = ctx.accounts.mint.key();
    let rival_key = ctx.accounts.rival_mint.key();
    let treaty_id = ctx
        .accounts
        .config
        .treaty_template_id
        .ok_or(WarError::NoTreatyTemplate)?;
    let item_key = ctx.accounts.treaty_item.key();
    let item = Item::read(&ctx.accounts.treaty_item)?;
    require!(item.template_id == treaty_id, WarError::NoTreaty);
    require_keys_eq!(
        ctx.accounts.treaty_template.key(),
        crate::foreign::template_address(treaty_id),
        WarError::NoTreaty
    );
    let template = Template::read(&ctx.accounts.treaty_template)?;
    require!(template.program == ITEMS_ID, WarError::NoTreaty);
    require!(
        item.params[TREATY_RETURNS_CAPTURED] == 1,
        WarError::PeaceReturnsOff
    );
    let rival_mint = token_client::read_mint(&ctx.accounts.rival_mint)?;
    require!(
        equips(&ctx.accounts.mint, &item_key) && equips(&rival_mint, &item_key),
        WarError::NoTreaty
    );
    let amount = ctx
        .accounts
        .war_state
        .captured
        .iter()
        .find(|c| c.rival_mint == rival_key)
        .map(|c| c.amount)
        .ok_or(WarError::NothingToDo)?;
    require!(amount > 0, WarError::NothingToDo);

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let chest = ctx.accounts.war_chest.key();
    let rival_chest = ctx.accounts.rival_war_chest.key();
    let bump = ctx.accounts.war_state.chest_bump;
    let seeds = chest_seeds(&mint_key, bump);
    let slice = slice_metas(ctx.remaining_accounts, usize::from(args.first))?;
    ensure_holding(&all, ctx.accounts.cranker.key(), rival_key, rival_chest)?;
    invoke_built(
        &token_client::transfer_from_protocol(
            chest,
            token_client::holding_address(&rival_key, &chest),
            token_client::holding_address(&rival_key, &rival_chest),
            rival_key,
            slice,
            amount,
            crate::ID,
            vec![CHEST_SEED.to_vec(), mint_key.to_bytes().to_vec(), vec![bump]],
        ),
        &all,
        &[&seeds.seeds()],
    )?;
    let s = &mut ctx.accounts.war_state;
    if let Some(c) = s.captured_of(&rival_key) {
        *c = Captured::default();
    }
    emit_cpi!(CapturedReturned {
        mint: mint_key,
        rival_mint: rival_key,
        amount,
        treaty_item: item_key,
    });
    Ok(())
}
