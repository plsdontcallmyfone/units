// Changed by Hookwars: new file (M3b).
//! Size Tiers (id 10, 08 section 4.1): a cut by trade size. Fields: `t1_lamports`, `t2_lamports`,
//! `cut1_bps`, `cut2_bps`, `cut3_bps`. Cuts on the quote side: a buy's input, a sell's output.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, quote_side, Env, PoolOut};

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let p = &env.params;
    let q = ctx.side_amount;
    let bps = if q < u64::from(p[0]) {
        p[2]
    } else if q < u64::from(p[1]) {
        p[3]
    } else {
        p[4]
    };
    Ok(PoolOut {
        cut: fee(q, bps),
        ..Default::default()
    })
}
