// Changed by Hookwars: new file (arsenal wave E).
//! Holder Stream (id 25, 08 section 4.3): `buy_cut_bps` of buys and `sell_cut_bps` of sells (the
//! quote side) go to our treaty inbox, which the war program streams to holders through the kit's
//! `share` (R13). Fields: `buy_cut_bps`, `sell_cut_bps`. No extras.

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
