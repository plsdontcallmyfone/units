// Changed by Hookwars: new file (arsenal wave C).
//! Dump Brake (id 21, 08 section 4.2): sells pay a cut while the short-window price is more than
//! `drop_bps` under the long-window price. Fields: `short_secs`, `long_secs`, `drop_bps`,
//! `sell_cut_bps`. Extras: our `Launch` and our launch pool. The cut is in the quote, so it runs
//! on a sell's `after` callback (the quote side, as Size Tiers' sells).

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::volatility_fee::twaps;
use super::{fee, Env, PoolOut, SELL};

/// Whether `short` is more than `drop_bps` under `long`.
pub fn braking(short: u128, long: u128, drop_bps: u32) -> bool {
    let floor = long.saturating_mul(u128::from(10_000u32.saturating_sub(drop_bps.min(10_000)))) / 10_000;
    short < floor
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if before || args.direction != SELL {
        return Ok(PoolOut::default());
    }
    let p = &env.params;
    let Some((s, l)) = twaps(env, i64::from(p[0]), i64::from(p[1])) else {
        return Ok(PoolOut::default());
    };
    if !braking(s, l, p[2]) {
        return Ok(PoolOut::default());
    }
    Ok(PoolOut {
        cut: fee(ctx.side_amount, p[3]),
        ..Default::default()
    })
}
