// Changed by Hookwars: observations written at the start and end of each swap; the swap body is
// one hop (`run_hop`) shared by `swap` and the new multi-hop `swap_route`; pool callbacks carry the
// DEX-filled route (spec 03 sections 3.1 to 3.3).
//! Swaps (hook protocol v2, `docs/hooks-v2.md` §3.1).
//!
//! A buy (quote in, base out): the `before_swap` answer's deltas, then its burn, leave the trader's
//! quote holding (the trader signed the swap); the rest reaches the quote vault (`received`), which
//! pays the LP fee and the protocol fee; the curve runs on what is left; the `after_swap` answer's
//! deltas, then its burn, leave the base vault (the pool signs); the rest goes to the recipient.
//!
//! A sell (base in, quote out): the input as for a buy; the LP fee on `received` (flat model;
//! under the share model it leaves the curve's output with the protocol fee); the curve; the
//! protocol fee from the curve's output, kept in the quote vault (so the protocol fee is always in
//! the quote token); the `after_swap` answer is taken from what is left of the output; the rest
//! goes to the recipient.
//!
//! Two protocol fee models (`Pool.fee_model`). Flat (ordinary pools): a rate of the quote, as
//! above. Share (launch pools): the pool's share of what the hooks cut and someone received, on
//! both sides, measured (`cuts_in = amount_in - burn_in - received`, `cuts_out = amount_out -
//! burn_out - held - delivered`), base-side cuts valued at the swap's own price, each side's
//! share rounded up and never above its value. A buy's input share leaves what reached the vault
//! before the curve; a sell's input share leaves the curve's output before the hook is told; a
//! sell's output share is held back from the delivery; a buy's output share (and a sell's share of
//! a quote hook's own cut on the delivery) is set aside from the quote reserve after the swap.
//! Burns are not cuts anyone collects, so they are not shared. Under the share model the LP fee is
//! Bordrless's too (a launch pool's liquidity is locked for ever, so nothing compounds): taken in
//! the quote, from a buy's `received` before the curve or a sell's curve output before the hook is
//! told, and added to the protocol fee; a pool whose hooks cut nothing pays the LP fee alone.
//!
//! Amounts are measured, never assumed: `received` is what the input vault gained, and
//! `min_amount_out` is checked against what the recipient's holding gained, so a token whose own
//! hook takes a cut still prices and settles correctly.

use anchor_lang::prelude::*;
use bordrless_core::{output_share, protocol_share, swap_amounts, swap_amounts_shared, Reserves};
use bordrless_hook::{
    discriminators, pool_flags, Allowed, Phase, PoolHookArgs, PoolOp, RouteContext, MAX_HOOK_DATA,
};

use crate::constants::*;
use crate::error::{swap_failure, SwapError};
use crate::events::*;
use crate::obs;
use crate::hooks::{split_extras, Cut, PoolHookCall};
use crate::instructions::pool::PoolSeeds;
use crate::state::*;
use crate::token::{balance, read_holding, TokenAccounts, TokenSide};

/// Arguments of `swap`.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapArgs {
    /// 0 base → quote (a sell), 1 quote → base (a buy).
    pub direction: u8,
    /// Exact input.
    pub amount_in: u64,
    /// The least the recipient's holding must gain.
    pub min_amount_out: u64,
    /// Remaining accounts of the input mint's token hook (its program, the token program's
    /// signer for it, its extras; none without a hook).
    pub in_hook_accounts: u8,
    /// Remaining accounts of the output mint's token hook (likewise).
    pub out_hook_accounts: u8,
    /// Opaque data for the pool hook.
    pub hook_data: Vec<u8>,
}

/// Accounts of `swap`. The remaining accounts are the input mint's token-hook slice (its program,
/// the token program's signer for it, its extras), the output mint's, then the pool hook's
/// extras. A mint is passed writable only when the pool's hook may burn from that side (else
/// `MintNotWritable`); a transfer never write-locks a mint.
#[event_cpi]
#[derive(Accounts)]
pub struct Swap<'info> {
    /// The trader: owner (or delegate) of the input holding.
    pub trader: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut)]
    pub pool: Account<'info, Pool>,
    /// CHECK: address-checked; writable only for a burn of base.
    #[account(address = pool.base_mint @ SwapError::WrongHolding)]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: address-checked; writable only for a burn of quote.
    #[account(address = pool.quote_mint @ SwapError::WrongHolding)]
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: address-checked.
    #[account(mut, address = pool.base_vault @ SwapError::WrongVault)]
    pub base_vault: UncheckedAccount<'info>,
    /// CHECK: address-checked.
    #[account(mut, address = pool.quote_vault @ SwapError::WrongVault)]
    pub quote_vault: UncheckedAccount<'info>,
    /// CHECK: the trader's base holding, or on a buy any holding of the base mint to deliver to
    /// (checked by the token program).
    #[account(mut)]
    pub trader_base: UncheckedAccount<'info>,
    /// CHECK: the trader's quote holding, or on a sell any holding of the quote mint to deliver to.
    #[account(mut)]
    pub trader_quote: UncheckedAccount<'info>,
    /// CHECK: the pool's hook program (checked in the handler).
    pub hook_program: Option<UncheckedAccount<'info>>,
    /// CHECK: with a hook, this program's signer of its callbacks, `["hook-authority",
    /// hook_program]` (checked in the handler); absent (this program's id) without.
    pub hook_signer: Option<UncheckedAccount<'info>>,
    /// CHECK: the token program.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
}


/// One pool of a swap: the accounts upstream's `swap` takes, as account infos, so `swap` and each
/// hop of `swap_route` run the same code.
pub struct HopAccounts<'a, 'info> {
    /// The trader.
    pub trader: AccountInfo<'info>,
    /// The pool.
    pub pool: &'a mut Account<'info, Pool>,
    /// Base mint.
    pub base_mint: AccountInfo<'info>,
    /// Quote mint.
    pub quote_mint: AccountInfo<'info>,
    /// The pool's base vault.
    pub base_vault: AccountInfo<'info>,
    /// The pool's quote vault.
    pub quote_vault: AccountInfo<'info>,
    /// The trader's base holding (or, on a buy, any holding of the base mint to deliver to).
    pub trader_base: AccountInfo<'info>,
    /// The trader's quote holding (or, on a sell, any holding of the quote mint to deliver to).
    pub trader_quote: AccountInfo<'info>,
    /// The pool's hook program, when passed.
    pub hook_program: Option<AccountInfo<'info>>,
    /// This program's signer of its callbacks, when passed.
    pub hook_signer: Option<AccountInfo<'info>>,
    /// The input mint's token-hook slice.
    pub in_extras: &'a [AccountInfo<'info>],
    /// The output mint's token-hook slice.
    pub out_extras: &'a [AccountInfo<'info>],
    /// The pool hook's extras.
    pub pool_extras: &'a [AccountInfo<'info>],
}

/// What one hop is asked to do.
pub struct HopParams {
    /// 0 sell, 1 buy.
    pub direction: u8,
    /// Exact input.
    pub amount_in: u64,
    /// The least the recipient's holding must gain.
    pub min_amount_out: u64,
    /// Opaque data for the pool hook.
    pub hook_data: Vec<u8>,
    /// The route this hop belongs to; its `route_input_mint`, `route_output_mint`, `first_pool`
    /// and `route_amount_in` are filled by [`run_hop`] for a plain swap (`hop_count` 0 in).
    pub route: RouteContext,
}

/// `swap`.
pub fn process_swap<'info>(ctx: Context<'info, Swap<'info>>, args: SwapArgs) -> Result<()> {
    let clock = Clock::get()?;
    let SwapArgs {
        direction,
        amount_in,
        min_amount_out,
        in_hook_accounts,
        out_hook_accounts,
        hook_data,
    } = args;
    require!(!ctx.accounts.config.paused, SwapError::Paused);
    let token = TokenAccounts {
        program: ctx.accounts.token_program.to_account_info(),
        event_authority: ctx.accounts.token_event_authority.to_account_info(),
    };
    token.check()?;
    let (in_extras, out_extras, pool_extras) =
        split_extras(ctx.remaining_accounts, in_hook_accounts, out_hook_accounts)?;
    let accounts = &mut *ctx.accounts;
    let hop = HopAccounts {
        trader: accounts.trader.to_account_info(),
        base_mint: accounts.base_mint.to_account_info(),
        quote_mint: accounts.quote_mint.to_account_info(),
        base_vault: accounts.base_vault.to_account_info(),
        quote_vault: accounts.quote_vault.to_account_info(),
        trader_base: accounts.trader_base.to_account_info(),
        trader_quote: accounts.trader_quote.to_account_info(),
        hook_program: accounts.hook_program.as_ref().map(|a| a.to_account_info()),
        hook_signer: accounts.hook_signer.as_ref().map(|a| a.to_account_info()),
        pool: &mut accounts.pool,
        in_extras,
        out_extras,
        pool_extras,
    };
    let (event, _) = run_hop(
        &token,
        hop,
        HopParams {
            direction,
            amount_in,
            min_amount_out,
            hook_data,
            route: RouteContext::default(),
        },
        &clock,
    )?;
    emit_cpi!(event);
    Ok(())
}

/// One hop: upstream's swap, steps 1 to 7, with the observation ring written before the reserves
/// move and after, and the route in every pool callback. Answers the `Swapped` event and what the
/// recipient's holding gained.
/// A mint's single hook program, read in its own frame: a slot `Mint` is over a kilobyte, and
/// keeping it out of `run_hop`'s frame keeps that frame under the 4,096-byte SBF stack limit.
#[inline(never)]
fn mint_hook_program(info: &AccountInfo) -> Result<Option<Pubkey>> {
    Ok(bordrless_token::client::read_mint(info)?.hook_program)
}

pub fn run_hop<'a, 'info>(
    token: &TokenAccounts<'info>,
    hop: HopAccounts<'a, 'info>,
    params: HopParams,
    clock: &Clock,
) -> Result<(Swapped, u64)> {
    let HopParams {
        direction,
        amount_in,
        min_amount_out,
        hook_data,
        mut route,
    } = params;
    require!(amount_in > 0, SwapError::ZeroAmount);
    require!(direction <= 1, SwapError::InvalidDirection);
    require!(hook_data.len() <= MAX_HOOK_DATA, SwapError::HookDataTooLong);
    let buy = direction == 1;
    let HopAccounts {
        trader,
        pool: pool_account,
        base_mint,
        quote_mint,
        base_vault,
        quote_vault,
        trader_base,
        trader_quote,
        hook_program,
        hook_signer,
        in_extras,
        out_extras,
        pool_extras,
    } = hop;
    let pool_key = pool_account.key();

    // Hookwars: the ring accrues the price the previous instruction left, before anything moves.
    obs::begin(&pool_account.to_account_info(), pool_account, clock.unix_timestamp)?;

    // The sides.
    let (in_mint, out_mint, vault_in, vault_out, trader_in, trader_out) = if buy {
        (
            quote_mint.clone(),
            base_mint.clone(),
            quote_vault.clone(),
            base_vault.clone(),
            trader_quote.clone(),
            trader_base.clone(),
        )
    } else {
        (
            base_mint.clone(),
            quote_mint.clone(),
            base_vault.clone(),
            quote_vault.clone(),
            trader_base.clone(),
            trader_quote.clone(),
        )
    };
    if route.hop_count == 0 {
        route = RouteContext::single(*in_mint.key, *out_mint.key, pool_key, amount_in);
    }
    let in_hook = mint_hook_program(&in_mint)?;
    let out_hook = mint_hook_program(&out_mint)?;
    let recipient = read_holding(&trader_out)?.owner;
    // No delta may go to a vault or to either of the trader's holdings.
    let forbidden = [
        *vault_in.key,
        *vault_out.key,
        *trader_in.key,
        *trader_out.key,
    ];

    let call = PoolHookCall::of(
        pool_account.hook_program,
        pool_account.hook_signer_bump,
        hook_program,
        hook_signer,
        [
            pool_account.to_account_info(),
            base_mint.clone(),
            quote_mint.clone(),
            trader.clone(),
        ],
        pool_extras,
    )?;
    let flags = pool_account.hook_flags;
    let mut hook_args = {
        let pool = &*pool_account;
        PoolHookArgs {
            op: PoolOp::Swap,
            phase: Phase::Before,
            pool: pool_key,
            base_mint: pool.base_mint,
            quote_mint: pool.quote_mint,
            actor: trader.key(),
            recipient,
            direction,
            amount_in,
            amount_out: 0,
            base_reserve: pool.base_reserve,
            quote_reserve: pool.quote_reserve,
            virtual_base: pool.virtual_base,
            virtual_quote: pool.virtual_quote,
            lp_fee_bps: pool.lp_fee_bps,
            protocol_fee_bps: pool.protocol_fee_bps,
            swap_count: pool.swap_count,
            created_at: pool.created_at,
            lp_amount: 0,
            hook_data,
            route,
        }
    };

    // 1. Before: the hook may set the LP fee and cut the input (deltas and a burn), as the pool's
    //    flags allow; the answer is checked before anything moves.
    let mut lp_fee_bps = pool_account.lp_fee_bps;
    let mut cut_in = Cut::none();
    if let Some(call) = &call {
        if pool_account.runs(pool_flags::BEFORE_SWAP) {
            let allowed = Allowed::pool(PoolOp::Swap, Phase::Before, flags);
            if let Some((ret, taken)) =
                call.invoke(discriminators::BEFORE_SWAP, &hook_args, allowed)?
            {
                if let Some(fee) = ret.lp_fee_bps {
                    require!(fee <= MAX_LP_FEE_BPS, SwapError::FeeTooHigh);
                    lp_fee_bps = fee;
                }
                cut_in = call.cut(&ret, taken, amount_in, &in_mint, &forbidden)?;
            }
        }
    }

    // 2. The input: the cut from the trader's holding (each delta, then the burn), then the rest
    //    to the vault, measured there.
    let in_side = TokenSide::of(in_extras, in_hook.is_some());
    for (holding, amount) in &cut_in.deltas {
        token.transfer(&trader, &trader_in, holding, &in_mint, &in_side, *amount, &[])?;
    }
    if cut_in.burn > 0 {
        token.burn(&trader, &trader_in, &in_mint, &in_side, cut_in.burn, &[])?;
    }
    let vault_in_before = balance(&vault_in)?;
    token.transfer(
        &trader,
        &trader_in,
        &vault_in,
        &in_mint,
        &in_side,
        amount_in - cut_in.taken,
        &[],
    )?;
    let received = balance(&vault_in)?
        .checked_sub(vault_in_before)
        .ok_or(SwapError::MathOverflow)?;
    require!(received > 0, SwapError::NothingReceived);
    // What the hooks took from the input and someone received: the pool hook's deltas and any
    // cut the input mint's own hook took from those transfers (measured, in the input token).
    let cuts_in = amount_in
        .checked_sub(cut_in.burn)
        .and_then(|rest| rest.checked_sub(received))
        .ok_or(SwapError::MathOverflow)?;

    // 3. The fees and the curve; the protocol fee is in quote (upstream comments, unchanged).
    let (amounts, shared, share_bps) = {
        let pool = &*pool_account;
        let reserves = Reserves {
            base_reserve: pool.base_reserve,
            quote_reserve: pool.quote_reserve,
            virtual_base: pool.virtual_base,
            virtual_quote: pool.virtual_quote,
        };
        let shared = pool.shares_cuts();
        let share_bps = pool.protocol_share_bps;
        let amounts = if shared {
            swap_amounts_shared(buy, received, lp_fee_bps, share_bps, cuts_in, &reserves)
        } else {
            swap_amounts(buy, received, lp_fee_bps, pool.protocol_fee_bps, &reserves)
        }
        .map_err(swap_failure)?;
        (amounts, shared, share_bps)
    };

    // 4. Reserves: `to_reserve_in` enters the input reserve; the curve's whole output leaves the
    //    output reserve; the protocol fee is set aside in the quote vault.
    {
        let pool = &mut **pool_account;
        if buy {
            pool.quote_reserve = pool
                .quote_reserve
                .checked_add(amounts.to_reserve_in)
                .ok_or(SwapError::MathOverflow)?;
            pool.base_reserve = pool
                .base_reserve
                .checked_sub(amounts.out_gross)
                .ok_or(SwapError::InsufficientLiquidity)?;
            pool.quote_volume = pool
                .quote_volume
                .checked_add(u128::from(received))
                .ok_or(SwapError::MathOverflow)?;
            pool.base_volume = pool
                .base_volume
                .checked_add(u128::from(amounts.out_gross))
                .ok_or(SwapError::MathOverflow)?;
        } else {
            pool.base_reserve = pool
                .base_reserve
                .checked_add(amounts.to_reserve_in)
                .ok_or(SwapError::MathOverflow)?;
            pool.quote_reserve = pool
                .quote_reserve
                .checked_sub(amounts.out_gross)
                .ok_or(SwapError::InsufficientLiquidity)?;
            pool.base_volume = pool
                .base_volume
                .checked_add(u128::from(received))
                .ok_or(SwapError::MathOverflow)?;
            pool.quote_volume = pool
                .quote_volume
                .checked_add(u128::from(amounts.out_gross))
                .ok_or(SwapError::MathOverflow)?;
        }
        pool.protocol_fees_quote = pool
            .protocol_fees_quote
            .checked_add(amounts.protocol_fee)
            .ok_or(SwapError::MathOverflow)?;
        pool.swap_count = pool
            .swap_count
            .checked_add(1)
            .ok_or(SwapError::MathOverflow)?;
        pool.last_swap_at = clock.unix_timestamp;
    }

    // 5. After: the hook sees the result (the pool written as it now stands) and may cut the
    //    output it is told: the curve's output, less a sell's protocol fee.
    let mut cut_out = Cut::none();
    if let Some(call) = &call {
        if pool_account.runs(pool_flags::AFTER_SWAP) {
            pool_account.exit(&crate::ID)?;
            let pool = &*pool_account;
            hook_args.phase = Phase::After;
            hook_args.amount_in = received;
            hook_args.amount_out = amounts.amount_out;
            hook_args.base_reserve = pool.base_reserve;
            hook_args.quote_reserve = pool.quote_reserve;
            hook_args.lp_fee_bps = lp_fee_bps;
            let allowed = Allowed::pool(PoolOp::Swap, Phase::After, flags);
            if let Some((ret, taken)) =
                call.invoke(discriminators::AFTER_SWAP, &hook_args, allowed)?
            {
                cut_out = call.cut(&ret, taken, amounts.amount_out, &out_mint, &forbidden)?;
            }
        }
    }

    // 6. The output: the cut from the vault (each delta, then the burn; the pool signs), then the
    //    rest to the recipient, measured there. Under the share model a sell holds back the pool's
    //    share of the hook's quote deltas from the delivery (the trader pays it).
    let held_back = if shared && !buy {
        let deltas_sum = cut_out.taken - cut_out.burn;
        protocol_share(deltas_sum, share_bps).ok_or(SwapError::MathOverflow)?
    } else {
        0
    };
    let to_deliver = amounts
        .amount_out
        .checked_sub(cut_out.taken)
        .and_then(|rest| rest.checked_sub(held_back))
        .filter(|rest| *rest > 0)
        .ok_or(SwapError::FeeExceedsOutput)?;
    let pool_info = pool_account.to_account_info();
    let seeds = PoolSeeds::of(pool_account);
    let pool_seeds = seeds.seeds();
    let out_side = TokenSide::of(out_extras, out_hook.is_some());
    for (holding, amount) in &cut_out.deltas {
        token.transfer(
            &pool_info,
            &vault_out,
            holding,
            &out_mint,
            &out_side,
            *amount,
            &[&pool_seeds],
        )?;
    }
    if cut_out.burn > 0 {
        token.burn(
            &pool_info,
            &vault_out,
            &out_mint,
            &out_side,
            cut_out.burn,
            &[&pool_seeds],
        )?;
    }
    let trader_out_before = balance(&trader_out)?;
    token.transfer(
        &pool_info,
        &vault_out,
        &trader_out,
        &out_mint,
        &out_side,
        to_deliver,
        &[&pool_seeds],
    )?;
    let delivered = balance(&trader_out)?
        .checked_sub(trader_out_before)
        .ok_or(SwapError::MathOverflow)?;
    require!(delivered >= min_amount_out, SwapError::Slippage);
    // What the hooks took from the output and someone received (measured, in the output token).
    let cuts_out = amounts
        .amount_out
        .checked_sub(cut_out.burn)
        .and_then(|rest| rest.checked_sub(held_back))
        .and_then(|rest| rest.checked_sub(delivered))
        .ok_or(SwapError::MathOverflow)?;

    // 7. Share model: the pool's share of the output side's cuts (upstream comments, unchanged).
    let protocol_out = if shared {
        output_share(buy, share_bps, cuts_out, amounts.net_in, amounts.out_gross)
            .ok_or(SwapError::MathOverflow)?
    } else {
        0
    };
    let from_reserve = protocol_out
        .checked_sub(held_back)
        .ok_or(SwapError::MathOverflow)?;
    {
        let pool = &mut **pool_account;
        pool.quote_reserve = pool
            .quote_reserve
            .checked_sub(from_reserve)
            .ok_or(SwapError::InsufficientLiquidity)?;
        pool.protocol_fees_quote = pool
            .protocol_fees_quote
            .checked_add(protocol_out)
            .ok_or(SwapError::MathOverflow)?;
    }
    let protocol_fee = amounts
        .protocol_fee
        .checked_add(protocol_out)
        .ok_or(SwapError::MathOverflow)?;

    // Hookwars: the price this swap left, for the next accrual.
    obs::end(&pool_info, pool_account)?;
    // Written now, so a later hop or the caller reads the pool as it stands.
    pool_account.exit(&crate::ID)?;

    let pool = &*pool_account;
    let event = Swapped {
        pool: pool_key,
        trader: trader.key(),
        recipient,
        direction,
        amount_in,
        deltas_in: cut_in.paid(),
        burn_in: cut_in.burn,
        cuts_in,
        received_in: received,
        lp_fee: amounts.lp_fee,
        protocol_fee,
        lp_fee_bps,
        amount_out: amounts.out_gross,
        deltas_out: cut_out.paid(),
        burn_out: cut_out.burn,
        cuts_out,
        delivered_out: delivered,
        base_reserve: pool.base_reserve,
        quote_reserve: pool.quote_reserve,
        virtual_base: pool.virtual_base,
        virtual_quote: pool.virtual_quote,
        swap_count: pool.swap_count,
        slot: clock.slot,
        ts: clock.unix_timestamp,
        route,
    };
    Ok((event, delivered))
}

/// Hookwars: arguments of `swap_route` (spec 03 section 3.3).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct SwapRouteArgs {
    /// Exact input of the first hop.
    pub amount_in: u64,
    /// The least the trader's final holding must gain.
    pub min_amount_out: u64,
    /// Per hop, in order; 1 ..= `MAX_ROUTE_HOPS`.
    pub hops: Vec<HopArgs>,
    /// Opaque data for every hop's pool hook (never read for the route).
    pub hook_data: Vec<u8>,
}

/// One hop of a route.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug)]
pub struct HopArgs {
    /// 0 sell (base in), 1 buy (quote in), of this hop's pool.
    pub direction: u8,
    /// Remaining accounts this hop takes, the fixed ten included (`HOP_FIXED_ACCOUNTS`).
    pub accounts: u8,
    /// Of those, the input mint's token-hook slice.
    pub in_hook_accounts: u8,
    /// And the output mint's.
    pub out_hook_accounts: u8,
}

/// Accounts every hop group starts with: pool (its observation ring inside), base mint, quote
/// mint, base vault, quote vault, trader base holding, trader quote holding, hook program (this
/// program's id for none), hook signer (likewise). Then the input mint's token-hook slice, the
/// output mint's, and the pool hook's extras.
pub const HOP_FIXED_ACCOUNTS: usize = 9;

/// Accounts of `swap_route`; the hop groups are the remaining accounts.
#[event_cpi]
#[derive(Accounts)]
pub struct SwapRoute<'info> {
    /// The trader: owner (or delegate) of every input holding.
    pub trader: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    /// CHECK: the token program.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
}

fn optional_info<'info>(info: &AccountInfo<'info>) -> Option<AccountInfo<'info>> {
    if *info.key == crate::ID {
        None
    } else {
        Some(info.clone())
    }
}

/// `swap_route`: hops run one after another inside this instruction (not as CPIs), so a route is
/// no deeper than a swap. Each hop's input is what the previous hop delivered, measured on the
/// holding; no pool appears twice; only the last hop checks `min_amount_out`.
pub fn process_swap_route<'info>(
    ctx: Context<'info, SwapRoute<'info>>,
    args: SwapRouteArgs,
) -> Result<()> {
    let clock = Clock::get()?;
    require!(!ctx.accounts.config.paused, SwapError::Paused);
    require!(!args.hops.is_empty(), SwapError::EmptyRoute);
    require!(args.hops.len() <= MAX_ROUTE_HOPS, SwapError::RouteTooLong);
    require!(args.hook_data.len() <= MAX_HOOK_DATA, SwapError::HookDataTooLong);
    let token = TokenAccounts {
        program: ctx.accounts.token_program.to_account_info(),
        event_authority: ctx.accounts.token_event_authority.to_account_info(),
    };
    token.check()?;
    let trader = ctx.accounts.trader.to_account_info();
    let remaining: &'info [AccountInfo<'info>] = ctx.remaining_accounts;

    // The groups, and the route's shape, checked before anything moves.
    let mut groups: Vec<&'info [AccountInfo<'info>]> = Vec::with_capacity(args.hops.len());
    let mut at = 0usize;
    for hop in &args.hops {
        let n = usize::from(hop.accounts);
        require!(hop.direction <= 1, SwapError::InvalidDirection);
        require!(
            n >= HOP_FIXED_ACCOUNTS
                + usize::from(hop.in_hook_accounts)
                + usize::from(hop.out_hook_accounts),
            SwapError::AccountCounts
        );
        require!(remaining.len() >= at + n, SwapError::AccountCounts);
        groups.push(&remaining[at..at + n]);
        at += n;
    }
    require!(at == remaining.len(), SwapError::AccountCounts);
    // (in mint, out mint, trader in holding, trader out holding) per hop.
    let sides: Vec<(Pubkey, Pubkey, Pubkey, Pubkey)> = groups
        .iter()
        .zip(&args.hops)
        .map(|(g, hop)| {
            let (base, quote, tb, tq) = (*g[1].key, *g[2].key, *g[5].key, *g[6].key);
            if hop.direction == 1 {
                (quote, base, tq, tb)
            } else {
                (base, quote, tb, tq)
            }
        })
        .collect();
    for i in 0..groups.len() {
        for j in (i + 1)..groups.len() {
            require_keys_neq!(*groups[i][0].key, *groups[j][0].key, SwapError::RoutePoolRepeated);
        }
        if i + 1 < groups.len() {
            require_keys_eq!(sides[i].1, sides[i + 1].0, SwapError::RouteBroken);
            require_keys_eq!(sides[i].3, sides[i + 1].2, SwapError::RouteBroken);
        }
    }
    let hop_count = u8::try_from(groups.len()).map_err(|_| SwapError::RouteTooLong)?;
    let route_input_mint = sides[0].0;
    let route_output_mint = sides[groups.len() - 1].1;
    let first_pool = *groups[0][0].key;

    let mut amount = args.amount_in;
    let mut pools = Vec::with_capacity(groups.len());
    for (i, (group, hop)) in groups.iter().zip(&args.hops).enumerate() {
        let mut pool: Account<'info, Pool> = Account::try_from(&group[0])?;
        require_keys_eq!(*group[1].key, pool.base_mint, SwapError::WrongHolding);
        require_keys_eq!(*group[2].key, pool.quote_mint, SwapError::WrongHolding);
        require_keys_eq!(*group[3].key, pool.base_vault, SwapError::WrongVault);
        require_keys_eq!(*group[4].key, pool.quote_vault, SwapError::WrongVault);
        let (in_n, out_n) = (
            usize::from(hop.in_hook_accounts),
            usize::from(hop.out_hook_accounts),
        );
        let slices = &group[HOP_FIXED_ACCOUNTS..];
        let last = i + 1 == groups.len();
        let hop_accounts = HopAccounts {
            trader: trader.clone(),
            base_mint: group[1].clone(),
            quote_mint: group[2].clone(),
            base_vault: group[3].clone(),
            quote_vault: group[4].clone(),
            trader_base: group[5].clone(),
            trader_quote: group[6].clone(),
            hook_program: optional_info(&group[7]),
            hook_signer: optional_info(&group[8]),
            pool: &mut pool,
            in_extras: &slices[..in_n],
            out_extras: &slices[in_n..in_n + out_n],
            pool_extras: &slices[in_n + out_n..],
        };
        let (event, delivered) = run_hop(
            &token,
            hop_accounts,
            HopParams {
                direction: hop.direction,
                amount_in: amount,
                min_amount_out: if last { args.min_amount_out } else { 0 },
                hook_data: args.hook_data.clone(),
                route: RouteContext {
                    route_input_mint,
                    route_output_mint,
                    first_pool,
                    route_amount_in: args.amount_in,
                    hop_index: i as u8,
                    hop_count,
                },
            },
            &clock,
        )?;
        emit_cpi!(event);
        pools.push(pool.key());
        amount = delivered;
    }
    emit_cpi!(RouteSwapped {
        trader: trader.key(),
        route_input_mint,
        route_output_mint,
        amount_in: args.amount_in,
        amount_out: amount,
        pools,
        slot: clock.slot,
        ts: clock.unix_timestamp,
    });
    Ok(())
}
