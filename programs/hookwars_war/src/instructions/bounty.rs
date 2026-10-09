//! `claim_bounty`: a holder turns raid points into SOL from the chest (05 section 7).

use anchor_lang::prelude::*;
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::instructions::setup::{chest_balance, note_funding};
use crate::state::*;

/// Accounts of `claim_bounty`. Remaining: the Raid slot's extras (`extra_count` of them), then the
/// bridge's `unwrap_sol` accounts and the token program's.
#[event_cpi]
#[derive(Accounts)]
pub struct ClaimBounty<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest (seeds-checked).
    #[account(mut, seeds = [CHEST_SEED, mint.key().as_ref()], bump = war_state.chest_bump)]
    pub war_chest: UncheckedAccount<'info>,
    /// CHECK: the chest's bridged-SOL holding (address-checked).
    #[account(mut, address = token_client::holding_address(&BRIDGED_SOL_MINT, &war_chest.key()))]
    pub chest_holding: UncheckedAccount<'info>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the War orders `Item`.
    pub orders_item: UncheckedAccount<'info>,
    /// CHECK: its `Template`.
    pub orders_template: UncheckedAccount<'info>,
    /// CHECK: the owner's holding of the mint (address-checked; the token program checks it).
    #[account(mut, address = token_client::holding_address(&mint.key(), &owner.key()))]
    pub holding: UncheckedAccount<'info>,
    /// CHECK: `["war-signer"]`, signing the touch.
    #[account(seeds = [WAR_SIGNER_SEED], bump)]
    pub war_signer: UncheckedAccount<'info>,
    /// CHECK: the items program (address-checked).
    #[account(address = ITEMS_ID)]
    pub items_program: UncheckedAccount<'info>,
    /// CHECK: the token program's signer of the items program's callbacks (address-checked).
    #[account(address = token_client::hook_signer(&ITEMS_ID))]
    pub items_hook_signer: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `claim_bounty(raid_slot)`: `pay = min(points * bounty_rate, BOUNTY_MAX_PER_CLAIM, chest)`; the
/// points it costs are spent through `touch` before anything is paid, and checked after.
pub fn process_claim_bounty<'info>(
    ctx: Context<'info, ClaimBounty<'info>>,
    raid_slot_index: u8,
) -> Result<()> {
    let params = ctx.accounts.config.params;
    let current = ctx.accounts.config.current_season;
    let orders = war_orders(
        &ctx.accounts.mint,
        &ctx.accounts.orders_item,
        &ctx.accounts.orders_template,
    )?;
    let rate = u64::from(orders.get(orders::BOUNTY_RATE));
    require!(rate > 0, WarError::NoWarOrders);
    let slot = raid_slot(&ctx.accounts.mint, raid_slot_index)?;
    let before = read_raid(&ctx.accounts.holding, &slot)?;
    let points = before.points_in(current);
    require!(points > 0, WarError::NothingToDo);
    let balance_now = chest_balance(&ctx.accounts.chest_holding)?;
    {
        let s = &mut ctx.accounts.war_state;
        s.roll(current);
        note_funding(s, balance_now);
    }
    let pay = u64::from(points)
        .saturating_mul(rate)
        .min(params.bounty_max_per_claim)
        .min(balance_now);
    require!(pay > 0, WarError::ChestInsufficient);
    let spent = pay.div_ceil(rate).min(u64::from(points)) as u32;

    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let extras = slice_metas(ctx.remaining_accounts, usize::from(slot.extra_count))?;
    touch_raid(
        &all,
        ctx.accounts.war_signer.key(),
        ctx.bumps.war_signer,
        ctx.accounts.mint.key(),
        ctx.accounts.holding.key(),
        raid_slot_index,
        WarTouch::SpendRaidPoints { amount: spent },
        extras,
    )?;
    let after = read_raid(&ctx.accounts.holding, &slot)?;
    require!(
        after.points_in(current) == points - spent,
        WarError::TouchMismatch
    );
    let mint_key = ctx.accounts.mint.key();
    let chest = ctx.accounts.war_chest.key();
    let seeds = KeyedSeeds::new(CHEST_SEED, mint_key, ctx.accounts.war_state.chest_bump);
    pay_sol(
        &all,
        &seeds.seeds(),
        chest,
        &ctx.accounts.owner.to_account_info(),
        pay,
    )?;
    let balance_after = chest_balance(&ctx.accounts.chest_holding)?;
    let s = &mut ctx.accounts.war_state;
    s.paid_bounties = s.paid_bounties.saturating_add(pay);
    s.last_seen_balance = balance_after;
    emit_cpi!(BountyClaimed {
        mint: mint_key,
        owner: ctx.accounts.owner.key(),
        points: spent,
        paid: pay,
    });
    Ok(())
}
