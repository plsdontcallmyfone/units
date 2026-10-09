// Changed by Hookwars: new file (M3b).
//! Side Skew (id 11, 08 section 4.1): separate buy and sell cuts. Fields: `buy_cut_bps`,
//! `sell_cut_bps`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, quote_side, Env, PoolOut, BUY};

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let bps = if args.direction == BUY { env.params[0] } else { env.params[1] };
    Ok(PoolOut {
        cut: fee(ctx.side_amount, bps),
        ..Default::default()
    })
}
