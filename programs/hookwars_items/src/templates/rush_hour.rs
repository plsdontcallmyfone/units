// Changed by Hookwars: new file (arsenal wave C).
//! Rush Hour (id 16, 08 section 4.1): a cut by the hour of the day (UTC). Fields: `start_hour`,
//! `hours`, `inside_cut_bps`, `outside_cut_bps`. No extras.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, quote_side, Env, PoolOut};

/// The UTC hour of `now`.
pub fn hour(now: i64) -> u32 {
    (now.rem_euclid(86_400) / 3_600) as u32
}

/// Whether `now` falls in the window.
pub fn inside(p: &hookwars_common::Params, now: i64) -> bool {
    (hour(now) + 24 - p[0] % 24) % 24 < p[1]
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let p = &env.params;
    let bps = if inside(p, env.now) { p[2] } else { p[3] };
    Ok(PoolOut {
        cut: fee(ctx.side_amount, bps),
        ..Default::default()
    })
}
