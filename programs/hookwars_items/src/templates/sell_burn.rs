// Changed by Hookwars: new file (M3b).
//! Sell Burn (id 32, 08 section 4.6): a share of every sell's base, burned. Field: `burn_bps`.
//! As built: answered in `pool_before_swap` on a sell, where the side is base (the launchpad's
//! burn rule, 03 section 5.2); 08 names `pool_after_swap`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{Env, PoolOut, SELL};

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != SELL {
        return Ok(PoolOut::default());
    }
    let burn = (u128::from(ctx.side_amount) * u128::from(env.params[0]) / 10_000) as u64;
    Ok(PoolOut {
        burn,
        ..Default::default()
    })
}
