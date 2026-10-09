// Changed by Hookwars: new file (arsenal wave D).
//! Embargo (id 28, 08 section 4.4): buyers whose route started by selling one of the target mints
//! pay `cut_bps` extra, to our war chest. Field: `cut_bps`. Targets: 1 or more mints. No extras:
//! the route comes from the DEX (R5); a forged first pool only charges the forger.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, Env, PoolOut, BUY};

/// `pool_before_swap`.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    let r = &args.route;
    if !before || args.direction != BUY || r.hop_index < 1 || !env.targets.contains(&r.route_input_mint) {
        return Ok(PoolOut::default());
    }
    Ok(PoolOut {
        cut: fee(ctx.side_amount, env.params[0]),
        ..Default::default()
    })
}
