// Changed by Hookwars: new file (arsenal wave C).
//! Velocity Fee (id 13, 08 section 4.1): a cut that grows with the number of trades in a window.
//! Fields: `window_secs`, `swaps_threshold`, `cut_per_excess_bps`, `max_cut_bps`. Extras: our
//! `Launch` and our launch pool (whose ring the DEX keeps, 03 M3a notes). The trade count comes
//! from the ring over at least `MIN_TWAP_SECS` (rule 7); no signal means no cut.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, own_pool, quote_side, Env, PoolOut};

/// Our launch pool account, when `extras` are our `Launch` and the pool it names.
pub fn own_pool_account<'a, 'info>(env: &'a Env<'a, 'info>) -> Option<&'a AccountInfo<'info>> {
    let launch = env.extras.first()?;
    let pool = env.extras.get(1)?;
    (own_pool(launch, &env.mint) == Some(pool.key())).then_some(pool)
}

/// The cut rate for `swaps` trades in the window.
pub fn rate(p: &hookwars_common::Params, swaps: u64) -> u32 {
    let excess = swaps.saturating_sub(u64::from(p[1]));
    let r = excess.saturating_mul(u64::from(p[2]));
    r.min(u64::from(p[3])) as u32
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let Some(pool) = own_pool_account(env) else {
        return Ok(PoolOut::default());
    };
    let window = i64::from(env.params[0]);
    let Ok(r) = bordrless_swap::obs::window_read(pool, env.now, window, bordrless_swap::constants::MIN_TWAP_SECS)
    else {
        return Ok(PoolOut::default());
    };
    Ok(PoolOut {
        cut: fee(ctx.side_amount, rate(&env.params, r.swaps)),
        ..Default::default()
    })
}
