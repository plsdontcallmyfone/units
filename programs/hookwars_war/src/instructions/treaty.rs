//! What treaties pay a token, streamed to its holders (05 section 6.6, R13), and the time treaties
//! held, for the season score (05 section 6.7).

use anchor_lang::prelude::*;
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::Item;
use crate::instructions::admin::read_own;
use crate::state::*;

/// Accounts of `share_treaty_inflow`. Remaining: the kit `share`'s accounts and the bridge's
/// `unwrap_sol`'s.
#[event_cpi]
#[derive(Accounts)]
pub struct ShareTreatyInflow<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, war_state.mint.as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the treaty inbox (seeds-checked).
    #[account(mut, seeds = [INBOX_SEED, war_state.mint.as_ref()], bump = war_state.inbox_bump)]
    pub treaty_inbox: UncheckedAccount<'info>,
    /// CHECK: its bridged-SOL holding (address-checked).
    #[account(mut, address = token_client::holding_address(&BRIDGED_SOL_MINT, &treaty_inbox.key()))]
    pub inbox_holding: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `share_treaty_inflow`: the inbox, less a crank bounty of at most `max_crank_bounty_bps`, shared
/// with holders through the kit (streamed over its hour, so a buy just before cannot capture it).
pub fn process_share_treaty_inflow<'info>(
    ctx: Context<'info, ShareTreatyInflow<'info>>,
) -> Result<()> {
    let mint = ctx.accounts.war_state.mint;
    let inbox = ctx.accounts.treaty_inbox.key();
    let amount = crate::instructions::setup::chest_balance(&ctx.accounts.inbox_holding)?;
    let bounty = bps_of(amount, u64::from(ctx.accounts.config.params.max_crank_bounty_bps));
    let share = amount.saturating_sub(bounty);
    require!(share >= MIN_SHARE_LAMPORTS, WarError::NothingToDo);
    let all = available(ctx.accounts.to_account_infos(), ctx.remaining_accounts);
    let seeds = KeyedSeeds::new(INBOX_SEED, mint, ctx.accounts.war_state.inbox_bump);
    invoke_built(
        &bordrless_kit::client::share(
            inbox,
            mint,
            ctx.accounts.inbox_holding.key(),
            BRIDGED_SOL_MINT,
            share,
        ),
        &all,
        &[&seeds.seeds()],
    )?;
    pay_sol(
        &all,
        &seeds.seeds(),
        inbox,
        &ctx.accounts.cranker.to_account_info(),
        bounty,
    )?;
    let s = &mut ctx.accounts.war_state;
    // The bounty came from the inbox, not the chest: `paid_cranks` counts chest money only.
    s.treaty_shared_total = s.treaty_shared_total.saturating_add(share);
    emit_cpi!(TreatyInflowShared {
        mint,
        amount: share,
        bounty,
        cranker: ctx.accounts.cranker.key(),
    });
    Ok(())
}

/// Accounts of `accrue_treaty_time`. Remaining: pairs `(treaty item, partner mint)`.
#[event_cpi]
#[derive(Accounts)]
pub struct AccrueTreatyTime<'info> {
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the current `Season` (seeds-checked against the config's current season).
    pub season: UncheckedAccount<'info>,
}

/// `accrue_treaty_time`: for each Treaty item equipped by the mint and by its partner, adds the
/// season time since the last tick to `treaty_secs`; then sets the tick. No bounty.
pub fn process_accrue_treaty_time<'info>(
    ctx: Context<'info, AccrueTreatyTime<'info>>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let current = ctx.accounts.config.current_season;
    require!(current > 0, WarError::SeasonNotOpen);
    require_keys_eq!(
        ctx.accounts.season.key(),
        Season::address(current).0,
        WarError::WrongAccount
    );
    let season = read_own::<Season>(&ctx.accounts.season)?;
    let treaty_id = ctx
        .accounts
        .config
        .treaty_template_id
        .ok_or(WarError::NoTreatyTemplate)?;
    let rem = ctx.remaining_accounts;
    require!(rem.len() % 2 == 0, WarError::MissingAccount);
    let mint_key = ctx.accounts.mint.key();
    let s = &mut ctx.accounts.war_state;
    s.roll(current);
    let from = s.last_treaty_tick.max(season.starts_at);
    let to = now.min(season.ends_at);
    let span = u64::try_from(to.saturating_sub(from)).unwrap_or(0);
    let mut seen: Vec<Pubkey> = Vec::new();
    for pair in rem.chunks(2) {
        let (item_info, partner_info) = (&pair[0], &pair[1]);
        if seen.contains(item_info.key) {
            continue;
        }
        let item = Item::read(item_info)?;
        require!(item.template_id == treaty_id, WarError::NoTreaty);
        let partner = token_client::read_mint(partner_info)?;
        require_keys_neq!(*partner_info.key, mint_key, WarError::NoTreaty);
        require!(
            equips(&ctx.accounts.mint, item_info.key) && equips(&partner, item_info.key),
            WarError::NoTreaty
        );
        seen.push(*item_info.key);
        s.season.treaty_secs = s.season.treaty_secs.saturating_add(span);
    }
    s.last_treaty_tick = now;
    for item in seen {
        emit_cpi!(TreatyTimeAccrued {
            mint: mint_key,
            treaty_item: item,
            secs: span,
            season: current,
        });
    }
    Ok(())
}
