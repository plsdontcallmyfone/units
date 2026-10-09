// Changed by Hookwars: new file (arsenal wave E).
//! Mercenary (id 29, 08 section 4.5): an untargeted Raid. Any buy whose route started on another
//! token (not ours, not the quote) is marked, and its delivery stamps `points_per_unit` raid points
//! per unit into the buyer's Raid-layout range. No discount, no cut, and it never counts inbound
//! siege volume (only an aimed Raid does). Field: `points_per_unit`. Extras: `RaidLedger` (w),
//! `WarConfig`, our `Launch` (Raid's, without targets). War touches spend its points as Raid's.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};
use hookwars_common::ids;

use super::raid::{self, war_view, write_mark};
use super::{Env, PoolOut, TokenOut, BUY};
use crate::ItemsError;

/// `pool_after_swap`: the mark.
pub fn pool(env: &Env, args: &PoolHookArgs, _ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    let r = &args.route;
    if before
        || args.direction != BUY
        || r.hop_index < 1
        || r.route_input_mint == env.mint
        || r.route_input_mint == ids::BRIDGED_SOL_MINT
    {
        return Ok(PoolOut::default());
    }
    require!(env.extras.len() >= 3, ItemsError::WrongAccount);
    let war = war_view(&env.extras[1]);
    write_mark(env, &env.extras[0], war, args, r.route_input_mint, false)?;
    Ok(PoolOut::default())
}

/// Raid's token half with Mercenary's points per unit.
fn as_raid<'a, 'info>(env: &Env<'a, 'info>) -> Env<'a, 'info> {
    let mut params = env.params;
    params[2] = params[0];
    Env {
        mint: env.mint,
        slot: env.slot,
        item: env.item,
        module: env.module,
        params,
        targets: env.targets,
        role: env.role,
        extras: env.extras,
        now: env.now,
        clock_slot: env.clock_slot,
    }
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    raid::token(&as_raid(env), args, src, dst)
}

/// `on_touch`: war payloads.
pub fn touch(env: &Env, args: &TokenSlotArgs, src: &[u8]) -> Result<Vec<u8>> {
    raid::touch(env, args, src)
}
