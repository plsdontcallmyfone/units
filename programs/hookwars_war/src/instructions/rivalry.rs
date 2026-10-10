// Changed by Hookwars: new file (pass 4b, 10 section 11.3, R36).
//! Rivalries, redesigned to stay spot: a Rivalry item (template 45, its one target the rival)
//! ring-fences a budget inside the token's own chest that sieges and counter-strikes against the
//! rival spend first, and that other spends (sieges of others, bounties, coalition contributions)
//! may not touch while the rivalry is live. At the end the result (raid volume each way, from both
//! `RaidLedger`s) adds to the season's `rivalry_wins` counter only. Nothing moves between the two
//! chests because of the result; unspent budget simply stays where it always was.

use anchor_lang::prelude::*;
use bordrless_token::client as token_client;
use bordrless_token::state::Mint;

use crate::common::*;
use crate::constants::*;
use crate::error::WarError;
use crate::events::*;
use crate::foreign::{config_item, equip_first_target, RaidLedger};
use crate::instructions::setup::{chest_balance, note_funding};
use crate::state::*;

/// Accounts of `open_rivalry`.
#[event_cpi]
#[derive(Accounts)]
pub struct OpenRivalry<'info> {
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: the Rivalry `Item` (checked against the mint's slots and its template).
    pub item: UncheckedAccount<'info>,
    /// CHECK: its `Template` (address- and program-checked).
    pub template: UncheckedAccount<'info>,
    /// CHECK: the items program's `EquipState` of the item's slot (decoded by hand for the target).
    pub equip_state: UncheckedAccount<'info>,
    /// CHECK: the chest's bridged-SOL holding (address-checked).
    #[account(address = token_client::holding_address(&BRIDGED_SOL_MINT, &chest_address(&mint.key()).0))]
    pub chest_holding: UncheckedAccount<'info>,
}

/// `open_rivalry`: permissionless once the token equips a Rivalry item; the budget is the item's
/// `budget_bps` of the chest now. One rivalry at a time per token.
pub fn process_open_rivalry(ctx: Context<OpenRivalry>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let mint_key = ctx.accounts.mint.key();
    let (slot, params) = config_item(&ctx.accounts.mint, &ctx.accounts.item, &ctx.accounts.template, RIVALRY_TEMPLATE)?;
    let item = ctx.accounts.item.key();
    let rival = equip_first_target(&ctx.accounts.equip_state, &mint_key, slot, &item).ok_or(WarError::RivalryState)?;
    require_keys_neq!(rival, mint_key, WarError::RivalryState);
    let starts_at = i64::from(params[0]);
    let ends_at = starts_at.saturating_add(i64::from(params[1]));
    let budget_bps = u64::from(params[2]).min(BPS);
    require!(now < ends_at, WarError::RivalryState);
    let balance = chest_balance(&ctx.accounts.chest_holding)?;
    let current = ctx.accounts.config.current_season;
    let s = &mut ctx.accounts.war_state;
    require!(!s.rivalry.is_open(), WarError::RivalryState);
    s.roll(current);
    note_funding(s, balance);
    let budget = bps_of(balance, budget_bps);
    s.rivalry = RivalryBudget {
        rival,
        rival_chest: chest_address(&rival).0,
        item,
        starts_at,
        ends_at,
        budget,
        spent: 0,
    };
    emit_cpi!(RivalryOpened {
        mint: mint_key,
        rival_mint: rival,
        budget,
        ends_at,
    });
    Ok(())
}

/// Accounts of `settle_rivalry`.
#[event_cpi]
#[derive(Accounts)]
pub struct SettleRivalry<'info> {
    #[account(seeds = [WAR_CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, WarConfig>>,
    #[account(mut, seeds = [WAR_SEED, mint.key().as_ref()], bump = war_state.bump)]
    pub war_state: Box<Account<'info, WarState>>,
    pub mint: Box<Account<'info, Mint>>,
    /// CHECK: this token's `RaidLedger` (address-checked; zero volume when it does not exist).
    #[account(address = crate::foreign::raid_ledger_address(&mint.key()) @ WarError::WrongAccount)]
    pub ledger: UncheckedAccount<'info>,
    /// CHECK: the rival's `RaidLedger` (address-checked; zero volume when it does not exist).
    #[account(address = crate::foreign::raid_ledger_address(&war_state.rivalry.rival) @ WarError::WrongAccount)]
    pub rival_ledger: UncheckedAccount<'info>,
}

/// Raid volume `ledger` recorded from `from` (both windows of its entry; 0 without a ledger).
fn volume_from(ledger: &AccountInfo, from: &Pubkey) -> u64 {
    if *ledger.owner != ITEMS_ID {
        return 0;
    }
    let Ok(data) = ledger.try_borrow_data() else { return 0 };
    RaidLedger::decode(&data)
        .and_then(|l| l.inbound.iter().find(|e| e.rival_mint == *from).map(|e| e.volume.saturating_add(e.prev_volume)))
        .unwrap_or(0)
}

/// `settle_rivalry`: permissionless after the rivalry's end, or at once when the token no longer
/// equips the item (an early end counts no win). A win adds one to the season's `rivalry_wins`.
pub fn process_settle_rivalry(ctx: Context<SettleRivalry>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let mint_key = ctx.accounts.mint.key();
    let r = ctx.accounts.war_state.rivalry;
    require!(r.is_open(), WarError::RivalryState);
    let early = !equips(&ctx.accounts.mint, &r.item);
    require!(early || now >= r.ends_at, WarError::RivalryState);
    let ours = volume_from(&ctx.accounts.ledger, &r.rival);
    let theirs = volume_from(&ctx.accounts.rival_ledger, &mint_key);
    let won = !early && ours > theirs;
    let current = ctx.accounts.config.current_season;
    let s = &mut ctx.accounts.war_state;
    s.roll(current);
    if won {
        s.season.rivalry_wins = s.season.rivalry_wins.saturating_add(1);
    }
    s.rivalry = RivalryBudget::default();
    emit_cpi!(RivalrySettled {
        mint: mint_key,
        rival_mint: r.rival,
        ours,
        theirs,
        won,
        spent: r.spent,
        early,
    });
    Ok(())
}
