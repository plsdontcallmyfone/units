//! `init_war` and `record_funding` (05 sections 4 and 5).

use anchor_lang::prelude::*;
use bordrless_launch::state::Launch;
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::state::*;

/// Accounts of `init_war`.
#[event_cpi]
#[derive(Accounts)]
pub struct InitWar<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    pub mint: Box<Account<'info, Mint>>,
    /// The mint's launch: a war chest buys back on the launch pool.
    #[account(address = bordrless_launch::client::launch_address(&mint.key()) @ WarError::WrongLaunch,
              constraint = launch.mint == mint.key() @ WarError::WrongLaunch)]
    pub launch: Box<Account<'info, Launch>>,
    #[account(init, payer = payer, space = WarState::LEN, seeds = [WAR_SEED, mint.key().as_ref()], bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest, system-owned, signed for by this program.
    #[account(seeds = [CHEST_SEED, mint.key().as_ref()], bump)]
    pub war_chest: UncheckedAccount<'info>,
    /// CHECK: the treaty inbox, system-owned, signed for by this program.
    #[account(seeds = [INBOX_SEED, mint.key().as_ref()], bump)]
    pub treaty_inbox: UncheckedAccount<'info>,
    /// CHECK: bridged SOL (address-checked).
    #[account(address = BRIDGED_SOL_MINT)]
    pub bridged_sol_mint: UncheckedAccount<'info>,
    /// CHECK: the chest's holding of bridged SOL, created here (the token program checks it).
    #[account(mut)]
    pub chest_holding: UncheckedAccount<'info>,
    /// CHECK: the inbox's holding of bridged SOL, created here (the token program checks it).
    #[account(mut)]
    pub inbox_holding: UncheckedAccount<'info>,
    /// CHECK: the token program.
    #[account(address = TOKEN_ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (the token program checks it).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `init_war`: a token's war state, chest and treaty inbox, with their bridged-SOL holdings. The
/// mint must have a `War` slot (00 4.1); a second call fails at `init`.
pub fn process_init_war(ctx: Context<InitWar>) -> Result<()> {
    war_slot(&ctx.accounts.mint)?;
    let mint = ctx.accounts.mint.key();
    let payer = ctx.accounts.payer.key();
    let (chest, inbox) = (ctx.accounts.war_chest.key(), ctx.accounts.treaty_inbox.key());
    let all = ctx.accounts.to_account_infos();
    ensure_holding(&all, payer, BRIDGED_SOL_MINT, chest)?;
    ensure_holding(&all, payer, BRIDGED_SOL_MINT, inbox)?;
    let current = ctx.accounts.config.current_season;
    let s = &mut ctx.accounts.war_state;
    s.version = VERSION;
    s.bump = ctx.bumps.war_state;
    s.chest_bump = ctx.bumps.war_chest;
    s.inbox_bump = ctx.bumps.treaty_inbox;
    s.mint = mint;
    s.launch = ctx.accounts.launch.key();
    s.season_id = current;
    s.last_treaty_tick = Clock::get()?.unix_timestamp;
    emit_cpi!(WarChestCreated {
        mint,
        chest,
        treaty_inbox: inbox,
        war_state: s.key(),
    });
    Ok(())
}

/// Counts the chest balance's increase since `last_seen_balance` as funding (05 section 4). Every
/// spend calls it first and sets `last_seen_balance` after, so the totals stay exact without
/// trusting any sender. Answers the increase.
pub fn note_funding(state: &mut WarState, balance: u64) -> u64 {
    let increase = balance.saturating_sub(state.last_seen_balance);
    state.funded_total = state.funded_total.saturating_add(increase);
    state.last_seen_balance = balance;
    increase
}

/// The chest's bridged-SOL balance.
pub fn chest_balance(chest_holding: &AccountInfo) -> Result<u64> {
    if chest_holding.owner != &TOKEN_ID {
        return Ok(0);
    }
    Ok(token_client::read_holding(chest_holding)?.amount)
}

/// Accounts of `record_funding`.
#[event_cpi]
#[derive(Accounts)]
pub struct RecordFunding<'info> {
    #[account(mut, seeds = [WAR_SEED, war_state.mint.as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    /// CHECK: the chest's holding of bridged SOL (address-checked).
    #[account(address = token_client::holding_address(&BRIDGED_SOL_MINT, &chest_address(&war_state.mint).0))]
    pub chest_holding: UncheckedAccount<'info>,
}

/// `record_funding`: permissionless, no bounty.
pub fn process_record_funding(ctx: Context<RecordFunding>) -> Result<()> {
    let balance = chest_balance(&ctx.accounts.chest_holding)?;
    let s = &mut ctx.accounts.war_state;
    let amount = note_funding(s, balance);
    emit_cpi!(WarFunded {
        mint: s.mint,
        amount,
        balance,
        funded_total: s.funded_total,
    });
    Ok(())
}
