// Changed by Hookwars: new file (M3b).
//! Spy (id 4, 04 section 3.4): our fees move with a rival's time-weighted price. Fields: `mode`
//! (1 Rivalry, 2 Momentum), `window_secs`, `trigger_bps`, `effect_bps`. Extras: the rival's
//! `Launch` and its launch pool (whose ring the DEX keeps, 03 M3a notes).

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, own_pool, Env, PoolOut, BUY, SELL};
use crate::ItemsError;

/// The change of the rival's time-weighted price, in bps, between the window before last and the
/// last window; `None` when the ring does not cover both.
pub fn change_bps(pool: &AccountInfo, now: i64, window: i64) -> Option<i128> {
    let one = bordrless_swap::obs::window_read(pool, now, window, 1).ok()?;
    let two = bordrless_swap::obs::window_read(pool, now, 2 * window, 1).ok()?;
    if two.span_secs <= one.span_secs {
        return None;
    }
    let s1 = one.span_secs as u128;
    let s2 = two.span_secs as u128;
    let prev = (two.twap_q64.checked_mul(s2)?.checked_sub(one.twap_q64.checked_mul(s1)?)?) / (s2 - s1);
    if prev == 0 {
        return None;
    }
    let now_w = one.twap_q64 as i128;
    let prev = prev as i128;
    Some((now_w - prev) * 10_000 / prev)
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    require!(env.extras.len() >= 2, ItemsError::WrongAccount);
    let rival = env.targets.first().copied().unwrap_or_default();
    // The pool must be the one the rival's launch names.
    if own_pool(&env.extras[0], &rival) != Some(env.extras[1].key()) {
        return Ok(PoolOut::default());
    }
    let (mode, window, trigger, effect) = (env.params[0], i64::from(env.params[1]), i128::from(env.params[2]), env.params[3]);
    let Some(c) = change_bps(&env.extras[1], env.now, window) else {
        return Ok(PoolOut::default());
    };
    match (mode, before, args.direction) {
        (1, false, SELL) if c >= trigger => Ok(PoolOut {
            cut: fee(ctx.side_amount, effect),
            ..Default::default()
        }),
        (2, true, BUY) if c <= -trigger => Ok(PoolOut {
            discount_bps: effect.min(10_000) as u16,
            ..Default::default()
        }),
        _ => Ok(PoolOut::default()),
    }
}
