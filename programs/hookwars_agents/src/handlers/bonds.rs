// Changed by Hookwars: new file (09).
//! Diplomat bonds (09 section 8). A bond ties two armory proposals that equip one Treaty or
//! Tribute item on two tokens. It returns on ratification, and is forfeited into the two tokens'
//! treaty inboxes only on a real rejection by holders (R29).

use anchor_lang::prelude::*;
use anchor_lang::system_program;
use bordrless_token::client as token;
use hookwars_armory::state::{proposal_status, ArmoryConfig, Item, Proposal};
use hookwars_common::template_id;

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::*;
use crate::handlers::common::*;
use crate::state::*;

// ---- post_bond -------------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct PostBond<'info> {
    #[account(mut)]
    pub agent_key: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, constraint = passport.agent_key == agent_key.key() @ AgentsError::NotThisAgent)]
    pub passport: Box<Account<'info, Passport>>,
    pub proposal_a: Box<Account<'info, Proposal>>,
    pub proposal_b: Box<Account<'info, Proposal>>,
    pub treaty_item: Box<Account<'info, Item>>,
    #[account(init, payer = agent_key, space = 8 + Bond::INIT_SPACE,
        seeds = [BOND_SEED, passport.key().as_ref(), proposal_a.key().as_ref()], bump)]
    pub bond: Box<Account<'info, Bond>>,
    #[account(init, payer = agent_key, space = 8 + BondMark::INIT_SPACE,
        seeds = [BOND_MARK_SEED, proposal_a.key().as_ref()], bump)]
    pub mark_a: Box<Account<'info, BondMark>>,
    #[account(init, payer = agent_key, space = 8 + BondMark::INIT_SPACE,
        seeds = [BOND_MARK_SEED, proposal_b.key().as_ref()], bump)]
    pub mark_b: Box<Account<'info, BondMark>>,
    pub system_program: Program<'info, System>,
}

pub fn process_post_bond(ctx: Context<PostBond>) -> Result<()> {
    let params = ctx.accounts.config.params;
    let key = ctx.accounts.passport.key();
    let t = now()?;
    {
        let p = &ctx.accounts.passport;
        require!(p.status == status::ACTIVE, AgentsError::AgentPaused);
        require!(p.kinds & kind::DIPLOMAT != 0, AgentsError::NotDiplomat);
        require!(
            t >= p.created_at.saturating_add(params.bond_min_passport_age_secs),
            AgentsError::PassportTooYoung
        );
    }
    let agent = ctx.accounts.agent_key.key();
    let vault = pda::vault(&key).0;
    let item = ctx.accounts.treaty_item.key();
    let (a, b) = (&ctx.accounts.proposal_a, &ctx.accounts.proposal_b);
    let by_agent = |p: &Proposal| p.proposer == agent || p.proposer == vault;
    require!(
        a.key() != b.key()
            && a.status == proposal_status::OPEN
            && b.status == proposal_status::OPEN
            && by_agent(a)
            && by_agent(b)
            && a.mint != b.mint
            && a.item == Some(item)
            && b.item == Some(item)
            && (ctx.accounts.treaty_item.template_id == template_id::TREATY
                || ctx.accounts.treaty_item.template_id == template_id::TRIBUTE),
        AgentsError::BadProposalPair
    );
    let (ka, kb, ma, mb) = (a.key(), b.key(), a.mint, b.mint);
    let amount = params.bond_lamports;
    system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.key(),
            system_program::Transfer {
                from: ctx.accounts.agent_key.to_account_info(),
                to: ctx.accounts.bond.to_account_info(),
            },
        ),
        amount,
    )?;
    let bond_key = ctx.accounts.bond.key();
    let bd = &mut ctx.accounts.bond;
    bd.passport = key;
    bd.proposal_a = ka;
    bd.proposal_b = kb;
    bd.mint_a = ma;
    bd.mint_b = mb;
    bd.treaty_item = item;
    bd.amount = amount;
    bd.posted_at = t;
    bd.ratified_at = 0;
    bd.status = bond_status::POSTED;
    bd.bump = ctx.bumps.bond;
    ctx.accounts.mark_a.bond = bond_key;
    ctx.accounts.mark_a.bump = ctx.bumps.mark_a;
    ctx.accounts.mark_b.bond = bond_key;
    ctx.accounts.mark_b.bump = ctx.bumps.mark_b;
    let r = &mut ctx.accounts.passport.record;
    r.treaties_proposed = r.treaties_proposed.saturating_add(1);
    emit_cpi!(BondPosted {
        bond: bond_key,
        passport: key,
        proposal_a: ka,
        proposal_b: kb,
        treaty_item: item,
        amount,
        ts: t
    });
    Ok(())
}

// ---- resolve_bond ----------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct ResolveBond<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, has_one = passport @ AgentsError::WrongAccount,
        constraint = bond.proposal_a == proposal_a.key() && bond.proposal_b == proposal_b.key() @ AgentsError::WrongAccount)]
    pub bond: Box<Account<'info, Bond>>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    pub proposal_a: Box<Account<'info, Proposal>>,
    pub proposal_b: Box<Account<'info, Proposal>>,
    /// CHECK: the first token (supply read for the quorum).
    #[account(address = bond.mint_a @ AgentsError::WrongAccount)]
    pub mint_a: UncheckedAccount<'info>,
    /// CHECK: the second token.
    #[account(address = bond.mint_b @ AgentsError::WrongAccount)]
    pub mint_b: UncheckedAccount<'info>,
    #[account(address = hookwars_common::pda::config().0 @ AgentsError::WrongAccount)]
    pub armory_config: Box<Account<'info, ArmoryConfig>>,
    /// CHECK: the agent key (a returned bond goes back to it).
    #[account(mut, address = passport.agent_key @ AgentsError::WrongAccount)]
    pub agent_key: UncheckedAccount<'info>,
    /// CHECK: `["treaty-inbox", mint_a]` under war.
    #[account(mut, address = pda::treaty_inbox(&bond.mint_a) @ AgentsError::WrongAccount)]
    pub inbox_a: UncheckedAccount<'info>,
    /// CHECK: `["treaty-inbox", mint_b]` under war.
    #[account(mut, address = pda::treaty_inbox(&bond.mint_b) @ AgentsError::WrongAccount)]
    pub inbox_b: UncheckedAccount<'info>,
}

/// A real rejection: more votes against than for, and the quorum met against the whole supply
/// (stricter than the armory's eligible supply, so a bond is never forfeited where the armory's
/// own quorum was not met).
fn rejected(p: &Proposal, supply: u64, quorum_bps: u16) -> bool {
    let total = u128::from(p.votes_for) + u128::from(p.votes_against);
    p.status == proposal_status::FAILED
        && p.votes_against > p.votes_for
        && total * 10_000 >= u128::from(supply) * u128::from(quorum_bps)
}

pub fn process_resolve_bond(ctx: Context<ResolveBond>) -> Result<()> {
    require!(
        ctx.accounts.bond.status == bond_status::POSTED,
        AgentsError::BadBondStatus
    );
    let t = now()?;
    let grace = ctx.accounts.config.params.bond_cancel_grace_secs;
    let quorum = ctx.accounts.armory_config.params.vote_quorum_bps;
    let supply_a = token::read_mint(&ctx.accounts.mint_a.to_account_info())?.supply;
    let supply_b = token::read_mint(&ctx.accounts.mint_b.to_account_info())?.supply;
    let (a, b) = (&ctx.accounts.proposal_a, &ctx.accounts.proposal_b);
    let ratified = a.status == proposal_status::EXECUTED && b.status == proposal_status::EXECUTED;
    let forfeit = rejected(a, supply_a, quorum) || rejected(b, supply_b, quorum);
    let failed = a.status == proposal_status::FAILED || b.status == proposal_status::FAILED;
    let cancelled = a.status == proposal_status::CANCELLED || b.status == proposal_status::CANCELLED;
    let posted = ctx.accounts.bond.posted_at;
    let final_return = failed || (cancelled && t >= posted.saturating_add(grace));
    require!(ratified || forfeit || final_return, AgentsError::BondNotFinal);
    let bond_info = ctx.accounts.bond.to_account_info();
    let amount = ctx.accounts.bond.amount;
    let key = ctx.accounts.passport.key();
    let bond_key = ctx.accounts.bond.key();
    if forfeit && !ratified {
        let half = amount / 2;
        move_lamports(&bond_info, &ctx.accounts.inbox_a.to_account_info(), half)?;
        move_lamports(&bond_info, &ctx.accounts.inbox_b.to_account_info(), amount - half)?;
        ctx.accounts.bond.status = bond_status::REJECTED;
        let r = &mut ctx.accounts.passport.record;
        r.bonds_forfeited = r.bonds_forfeited.saturating_add(1);
        emit_cpi!(BondForfeited {
            bond: bond_key,
            passport: key,
            inbox_a: ctx.accounts.inbox_a.key(),
            inbox_b: ctx.accounts.inbox_b.key(),
            amount,
            ts: t
        });
        return Ok(());
    }
    move_lamports(&bond_info, &ctx.accounts.agent_key.to_account_info(), amount)?;
    if ratified {
        ctx.accounts.bond.status = bond_status::RATIFIED;
        ctx.accounts.bond.ratified_at = t;
        let r = &mut ctx.accounts.passport.record;
        r.treaties_ratified = r.treaties_ratified.saturating_add(1);
    } else {
        ctx.accounts.bond.status = bond_status::RETURNED;
    }
    emit_cpi!(BondReturned {
        bond: bond_key,
        passport: key,
        ratified,
        amount,
        ts: t
    });
    Ok(())
}

// ---- mark_treaty_outcome ---------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct MarkTreatyOutcome<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, has_one = passport @ AgentsError::WrongAccount)]
    pub bond: Box<Account<'info, Bond>>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    /// CHECK: the first token (its slot table is read).
    #[account(address = bond.mint_a @ AgentsError::WrongAccount)]
    pub mint_a: UncheckedAccount<'info>,
    /// CHECK: the second token.
    #[account(address = bond.mint_b @ AgentsError::WrongAccount)]
    pub mint_b: UncheckedAccount<'info>,
}

/// Whether `info`'s slot table holds `item`.
fn holds(info: &AccountInfo, item: &Pubkey) -> Result<bool> {
    let m = token::read_mint(info)?;
    Ok(m.slots[..usize::from(m.slot_count).min(m.slots.len())]
        .iter()
        .any(|s| s.item == *item))
}

pub fn process_mark_outcome(ctx: Context<MarkTreatyOutcome>) -> Result<()> {
    require!(
        ctx.accounts.bond.status == bond_status::RATIFIED,
        AgentsError::BadBondStatus
    );
    let t = now()?;
    let item = ctx.accounts.bond.treaty_item;
    let held_now = holds(&ctx.accounts.mint_a.to_account_info(), &item)?
        && holds(&ctx.accounts.mint_b.to_account_info(), &item)?;
    let key = ctx.accounts.passport.key();
    let bond_key = ctx.accounts.bond.key();
    if !held_now {
        ctx.accounts.bond.status = bond_status::BROKEN;
        let r = &mut ctx.accounts.passport.record;
        r.treaties_broken = r.treaties_broken.saturating_add(1);
        emit_cpi!(TreatyBroken {
            bond: bond_key,
            passport: key,
            ts: t
        });
        return Ok(());
    }
    let hold = ctx.accounts.config.params.treaty_hold_secs;
    require!(
        t >= ctx.accounts.bond.ratified_at.saturating_add(hold),
        AgentsError::TooEarly
    );
    ctx.accounts.bond.status = bond_status::HELD;
    let r = &mut ctx.accounts.passport.record;
    r.treaties_held = r.treaties_held.saturating_add(1);
    emit_cpi!(TreatyHeld {
        bond: bond_key,
        passport: key,
        ts: t
    });
    Ok(())
}
