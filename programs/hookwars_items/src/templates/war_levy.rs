// Changed by Hookwars: new file (arsenal wave E).
//! War Levy (id 31, 08 section 4.5): while any rival's rolling inbound raid volume is at least
//! `trigger_lamports`, sells pay `sell_cut_bps` (of the quote out) to our war chest. Fields:
//! `trigger_lamports`, `sell_cut_bps`. Extras: our `RaidLedger`, `WarConfig`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::raid::{load_ledger, war_view};
use super::{fee, Env, PoolOut, SELL};

/// Whether a raid of at least `trigger` is under way.
pub fn active(env: &Env, trigger: u64) -> bool {
    let x = env.extras;
    if x.len() < 2 {
        return false;
    }
    let Some(l) = load_ledger(&x[0], &env.mint) else {
        return false;
    };
    let w = war_view(&x[1]).raid_window_secs;
    l.inbound
        .iter()
        .filter(|e| e.rival_mint != Pubkey::default())
        .any(|e| hookwars_common::raid::rolling_volume(e, env.now, w) >= trigger.max(1))
}

/// `pool_after_swap` (a sell's output is the quote).
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if before || args.direction != SELL || !active(env, u64::from(env.params[0])) {
        return Ok(PoolOut::default());
    }
    Ok(PoolOut {
        cut: fee(ctx.side_amount, env.params[1]),
        ..Default::default()
    })
}
