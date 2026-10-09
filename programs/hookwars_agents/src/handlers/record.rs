// Changed by Hookwars: new file (09).
//! The track record (09 section 6): `record` is a CPI from `["agents-caller"]` under the armory,
//! war or items, made after the calling instruction's own effects (R26).

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::AgentCredited;
use crate::handlers::common::*;
use crate::state::*;

#[event_cpi]
#[derive(Accounts)]
pub struct Record<'info> {
    /// `["agents-caller"]` under the armory, war or items.
    pub caller: Signer<'info>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    /// CHECK: who acted, as the calling program says: the agent key or its policy vault.
    pub actor: UncheckedAccount<'info>,
}

pub fn process_record(ctx: Context<Record>, kind: u8, value: u64) -> Result<()> {
    require!(
        RECORDERS.contains(&ctx.accounts.caller.key()),
        AgentsError::NotRecorder
    );
    let key = ctx.accounts.passport.key();
    let p = &mut ctx.accounts.passport;
    require!(p.status == status::ACTIVE, AgentsError::AgentPaused);
    let actor = ctx.accounts.actor.key();
    require!(
        actor == p.agent_key || actor == pda::vault(&key).0,
        AgentsError::NotThisAgent
    );
    require!(kind <= record_kind::MAX, AgentsError::BadRecordKind);
    let r = &mut p.record;
    match kind {
        record_kind::ITEMS_AUTHORED => r.items_authored = r.items_authored.saturating_add(1),
        record_kind::ITEMS_EQUIPPED => r.items_equipped = r.items_equipped.saturating_add(1),
        record_kind::ITEMS_FORGED => r.items_forged = r.items_forged.saturating_add(1),
        record_kind::ROYALTY_CLAIM => {
            r.royalty_claims = r.royalty_claims.saturating_add(1);
            r.royalties_claimed_sol = r.royalties_claimed_sol.saturating_add(value);
        }
        record_kind::CRANK => {
            r.cranks = r.cranks.saturating_add(1);
            r.crank_value_lamports = r.crank_value_lamports.saturating_add(value);
        }
        record_kind::BOUNTY => {
            r.bounties_claimed_lamports = r.bounties_claimed_lamports.saturating_add(value)
        }
        _ => r.loot_reveals = r.loot_reveals.saturating_add(1),
    }
    let t = now()?;
    p.last_active_at = t;
    let changed = refresh(p, key, t);
    emit_cpi!(AgentCredited {
        passport: key,
        kind,
        value,
        ts: t
    });
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    Ok(())
}
