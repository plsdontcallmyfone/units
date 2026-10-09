// Changed by Hookwars: new file (arsenal wave C).
//! Volatility Fee (id 15, 08 section 4.1): a flat cut while the short-window price is more than
//! `trigger_bps` away from the long-window price. Fields: `short_secs`, `long_secs`, `trigger_bps`,
//! `cut_bps`. Extras: our `Launch` and our launch pool. Both reads are over at least
//! `MIN_TWAP_SECS` (rule 7); no signal means no cut.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::velocity_fee::own_pool_account;
use super::{fee, quote_side, Env, PoolOut};

/// The short and long time-weighted prices (Q64.64), when the ring covers both windows.
pub fn twaps(env: &Env, short: i64, long: i64) -> Option<(u128, u128)> {
    let pool = own_pool_account(env)?;
    let min = bordrless_swap::constants::MIN_TWAP_SECS;
    let s = bordrless_swap::obs::window_read(pool, env.now, short, min).ok()?;
    let l = bordrless_swap::obs::window_read(pool, env.now, long, min).ok()?;
    (l.twap_q64 > 0).then_some((s.twap_q64, l.twap_q64))
}

/// The deviation of `short` from `long` in bps.
pub fn deviation_bps(short: u128, long: u128) -> u128 {
    short.abs_diff(long).saturating_mul(10_000) / long.max(1)
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let p = &env.params;
    let Some((s, l)) = twaps(env, i64::from(p[0]), i64::from(p[1])) else {
        return Ok(PoolOut::default());
    };
    if deviation_bps(s, l) < u128::from(p[2]) {
        return Ok(PoolOut::default());
    }
    Ok(PoolOut {
        cut: fee(ctx.side_amount, p[3]),
        ..Default::default()
    })
}
