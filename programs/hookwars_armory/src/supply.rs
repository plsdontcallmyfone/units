// Changed by Hookwars: new file (gating, docs/spec/18-gating.md part A): scarce templates.
//! Template supply: a per-template cap on new copies, the minter rule and lifetime counters.
//!
//! A template is tracked once its `Supply` at `["supply", template_id]` exists (bit 0 of
//! `Template.supply_flags`). Every path that makes a copy of a tracked template needs that `Supply`
//! among the leading accounts of its remaining accounts; a tracked template whose `Supply` is
//! missing is refused, so the cap cannot be skipped by leaving the account out. Untracked templates
//! (all registered before this part) behave as before.

use anchor_lang::prelude::*;

use crate::error::ArmoryError;
use crate::events::{MinterSet, SupplyChanged, SupplyInitialized};
use crate::state::Template;

/// `["supply", template_id (u16 LE)]`.
pub const SUPPLY_SEED: &[u8] = b"supply";

/// `Template.supply_flags` bits.
pub mod flags {
    /// A `Supply` exists for the template.
    pub const TRACKED: u8 = 1;
}

/// `Supply.minter_rule` values.
pub mod minter_rule {
    /// Only `Supply.minter` issues.
    pub const AUTHOR_ONLY: u8 = 0;
    /// Anyone the template's authoring rule allows, until the cap.
    pub const OPEN_UNTIL_CAP: u8 = 1;
}

/// A template's supply (18 section 1.1).
#[account]
#[derive(InitSpace, Debug)]
pub struct Supply {
    pub version: u8,
    pub bump: u8,
    pub template_id: u16,
    /// Lifetime cap on `issued + drops`; 0 = uncapped (counted only).
    pub max_supply: u32,
    /// Part of the cap kept for drops (loot and craft).
    pub loot_reserve: u32,
    /// Copies made by authors (`create_item`, composite modules).
    pub issued: u32,
    /// Copies made by `mint_loot` and `mint_crafted`.
    pub drops: u32,
    /// Items made by `forge` (a transformation, not counted against the cap).
    pub forged: u32,
    /// Items burned by `forge`.
    pub burned: u32,
    pub minter_rule: u8,
    pub minter: Pubkey,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

impl Supply {
    /// Copies in circulation: everything made minus everything burned.
    pub fn circulating(&self) -> u64 {
        (u64::from(self.issued) + u64::from(self.drops) + u64::from(self.forged)).saturating_sub(u64::from(self.burned))
    }

    /// Whether `max_supply` caps.
    pub fn capped(&self) -> bool {
        self.max_supply > 0
    }

    /// The most authors may issue.
    pub fn author_cap(&self) -> u32 {
        self.max_supply.saturating_sub(self.loot_reserve)
    }
}

/// The `Supply` address of a template.
pub fn supply_address(template_id: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SUPPLY_SEED, &template_id.to_le_bytes()], &crate::ID)
}

/// Whether a template is tracked.
pub fn tracked(t: &Template) -> bool {
    t.supply_flags & flags::TRACKED != 0
}

/// Splits the leading `Supply` accounts (owned here, with `Supply`'s discriminator) off `rem`.
pub fn take_supplies<'a, 'info>(rem: &'a [AccountInfo<'info>]) -> (&'a [AccountInfo<'info>], &'a [AccountInfo<'info>]) {
    let n = rem
        .iter()
        .take_while(|a| {
            *a.owner == crate::ID
                && a.try_borrow_data().map(|d| d.len() >= 8 && d[..8] == *Supply::DISCRIMINATOR).unwrap_or(false)
        })
        .count();
    rem.split_at(n)
}

/// The `Supply` of `template_id` among `supplies`; `SupplyMissing` when absent.
pub fn find<'a, 'info>(supplies: &'a [AccountInfo<'info>], template_id: u16) -> Result<&'a AccountInfo<'info>> {
    let key = supply_address(template_id).0;
    let info = supplies.iter().find(|a| *a.key == key).ok_or(ArmoryError::SupplyMissing)?;
    require!(info.is_writable, ArmoryError::SupplyMissing);
    Ok(info)
}

/// What a create path does to a template's supply.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Change {
    /// An author's copy: `by` must satisfy the minter rule; counted against the author cap.
    Issue { by: Pubkey },
    /// A loot or craft drop: counted against the whole cap.
    Drop,
    /// A forge: one item made from two burned.
    Forge,
}

fn load(info: &AccountInfo) -> Result<Supply> {
    Supply::try_deserialize(&mut &info.try_borrow_data()?[..]).map_err(|_| error!(ArmoryError::SupplyMissing))
}

fn save(info: &AccountInfo, s: &Supply) -> Result<()> {
    let mut data = info.try_borrow_mut_data()?;
    let mut out: &mut [u8] = &mut data[..];
    s.try_serialize(&mut out)
}

/// Applies `change` to the template's `Supply` when the template is tracked (nothing otherwise).
pub fn apply(t: &Template, supplies: &[AccountInfo], change: Change) -> Result<()> {
    if !tracked(t) {
        return Ok(());
    }
    let info = find(supplies, t.id)?;
    let mut s = load(info)?;
    require!(s.template_id == t.id, ArmoryError::SupplyMissing);
    match change {
        Change::Issue { by } => {
            if s.minter_rule == minter_rule::AUTHOR_ONLY {
                require_keys_eq!(by, s.minter, ArmoryError::NotMinter);
            }
            let next = s.issued.checked_add(1).ok_or(ArmoryError::SupplyExhausted)?;
            if s.capped() {
                require!(next <= s.author_cap(), ArmoryError::SupplyExhausted);
            }
            s.issued = next;
        }
        Change::Drop => {
            let next = s.drops.checked_add(1).ok_or(ArmoryError::SupplyExhausted)?;
            if s.capped() {
                let total = u64::from(s.issued) + u64::from(next);
                require!(total <= u64::from(s.max_supply), ArmoryError::SupplyExhausted);
            }
            s.drops = next;
        }
        Change::Forge => {
            s.forged = s.forged.saturating_add(1);
            s.burned = s.burned.saturating_add(2);
        }
    }
    save(info, &s)?;
    emit!(SupplyChanged {
        template_id: s.template_id,
        max_supply: s.max_supply,
        loot_reserve: s.loot_reserve,
        issued: s.issued,
        drops: s.drops,
        forged: s.forged,
        burned: s.burned,
    });
    Ok(())
}

/// Whether `by` may issue copies of a tracked template whose `open_authoring` is off: it is the
/// `AUTHOR_ONLY` minter. Reads the `Supply` among `supplies` (false when absent or untracked).
pub fn is_minter(t: &Template, supplies: &[AccountInfo], by: &Pubkey) -> bool {
    if !tracked(t) {
        return false;
    }
    let Ok(info) = find(supplies, t.id) else { return false };
    let Ok(s) = load(info) else { return false };
    s.minter_rule == minter_rule::AUTHOR_ONLY && s.minter == *by
}

/// Checks a cap change (18 section 1.2): a cap only goes down (uncapped to any cap is down), never
/// below what was made, and the author cap still covers what authors issued.
pub fn check_cap(s: &Supply, max_supply: u32, loot_reserve: u32) -> Result<()> {
    if max_supply > 0 {
        require!(!s.capped() || max_supply <= s.max_supply, ArmoryError::CapMayOnlyFall);
        require!(loot_reserve <= max_supply, ArmoryError::InvalidSchema);
        require!(
            u64::from(s.issued) + u64::from(s.drops) <= u64::from(max_supply),
            ArmoryError::CapMayOnlyFall
        );
        require!(s.issued <= max_supply - loot_reserve, ArmoryError::CapMayOnlyFall);
    } else {
        // Uncapped only stays uncapped.
        require!(!s.capped(), ArmoryError::CapMayOnlyFall);
        require!(loot_reserve == 0, ArmoryError::InvalidSchema);
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Instructions (18 section 1.2).

use crate::gates::{action_hash_of, consume_queued};
use crate::state::{ArmoryConfig, QueuedAction};
use hookwars_common::seeds;

#[event_cpi]
#[derive(Accounts)]
#[instruction(template_id: u16)]
pub struct InitSupply<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, seeds = [seeds::TEMPLATE, &template_id.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
    #[account(init, payer = admin, space = 8 + Supply::INIT_SPACE,
        seeds = [SUPPLY_SEED, &template_id.to_le_bytes()], bump)]
    pub supply: Box<Account<'info, Supply>>,
    /// The queue entry (closed here).
    #[account(mut, close = admin)]
    pub queued: Box<Account<'info, QueuedAction>>,
    pub system_program: Program<'info, System>,
}

pub fn process_init_supply(
    ctx: Context<InitSupply>,
    template_id: u16,
    max_supply: u32,
    loot_reserve: u32,
    rule: u8,
    minter: Pubkey,
) -> Result<()> {
    let hash = action_hash_of(
        &crate::instruction::InitSupply { template_id, max_supply, loot_reserve, minter_rule: rule, minter },
        &[],
    );
    consume_queued(&ctx.accounts.queued, hash, &ctx.accounts.admin.key())?;
    require!(rule <= minter_rule::OPEN_UNTIL_CAP, ArmoryError::InvalidSchema);
    require!(loot_reserve <= max_supply, ArmoryError::InvalidSchema);
    let ts = Clock::get()?.unix_timestamp;
    let s = &mut ctx.accounts.supply;
    s.version = 1;
    s.bump = ctx.bumps.supply;
    s.template_id = template_id;
    s.max_supply = max_supply;
    s.loot_reserve = loot_reserve;
    s.minter_rule = rule;
    s.minter = minter;
    s.created_at = ts;
    ctx.accounts.template.supply_flags |= flags::TRACKED;
    emit_cpi!(SupplyInitialized { template_id, max_supply, loot_reserve, minter_rule: rule, minter, ts });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct SetSupply<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, seeds = [SUPPLY_SEED, &supply.template_id.to_le_bytes()], bump = supply.bump)]
    pub supply: Box<Account<'info, Supply>>,
    /// The queue entry (closed here).
    #[account(mut, close = admin)]
    pub queued: Box<Account<'info, QueuedAction>>,
}

pub fn process_set_supply_cap(ctx: Context<SetSupply>, template_id: u16, max_supply: u32, loot_reserve: u32) -> Result<()> {
    let hash = action_hash_of(&crate::instruction::SetSupplyCap { template_id, max_supply, loot_reserve }, &[]);
    consume_queued(&ctx.accounts.queued, hash, &ctx.accounts.admin.key())?;
    let s = &mut ctx.accounts.supply;
    require!(s.template_id == template_id, ArmoryError::WrongAccount);
    check_cap(s, max_supply, loot_reserve)?;
    s.max_supply = max_supply;
    s.loot_reserve = loot_reserve;
    emit!(SupplyChanged {
        template_id,
        max_supply,
        loot_reserve,
        issued: s.issued,
        drops: s.drops,
        forged: s.forged,
        burned: s.burned,
    });
    Ok(())
}

pub fn process_set_minter_rule(ctx: Context<SetSupply>, template_id: u16, rule: u8) -> Result<()> {
    let hash = action_hash_of(&crate::instruction::SetMinterRule { template_id, minter_rule: rule }, &[]);
    consume_queued(&ctx.accounts.queued, hash, &ctx.accounts.admin.key())?;
    require!(rule <= minter_rule::OPEN_UNTIL_CAP, ArmoryError::InvalidSchema);
    let s = &mut ctx.accounts.supply;
    require!(s.template_id == template_id, ArmoryError::WrongAccount);
    s.minter_rule = rule;
    emit_cpi!(MinterSet { template_id, minter_rule: rule, minter: s.minter, ts: Clock::get()?.unix_timestamp });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct HandOverMinter<'info> {
    pub minter: Signer<'info>,
    #[account(mut, seeds = [SUPPLY_SEED, &supply.template_id.to_le_bytes()], bump = supply.bump,
        has_one = minter @ ArmoryError::NotMinter)]
    pub supply: Box<Account<'info, Supply>>,
}

pub fn process_hand_over_minter(ctx: Context<HandOverMinter>, new_minter: Pubkey) -> Result<()> {
    let s = &mut ctx.accounts.supply;
    s.minter = new_minter;
    emit_cpi!(MinterSet {
        template_id: s.template_id,
        minter_rule: s.minter_rule,
        minter: new_minter,
        ts: Clock::get()?.unix_timestamp
    });
    Ok(())
}
