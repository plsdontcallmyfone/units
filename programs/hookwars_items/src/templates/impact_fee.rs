// Changed by Hookwars: new file (arsenal wave C).
//! Impact Fee (id 14, 08 section 4.1): a cut that scales with the trade's own price move.
//! Fields: `cut_per_impact_bps` (bps of cut per 100 bps of price moved), `max_cut_bps`. No extras:
//! the move is computed from the reserves and virtual reserves the DEX passes in `PoolHookArgs`.
//! A buy is measured at its `before` callback (reserves before the swap, quote in); a sell at its
//! `after` callback, where the DEX passes the reserves after the swap and the swap's own amounts.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, quote_side, Env, PoolOut, BUY};

/// The price move of this trade in bps, from the effective reserves.
pub fn impact_bps(args: &PoolHookArgs, side_amount: u64, before: bool) -> u128 {
    let x = u128::from(args.base_reserve) + u128::from(args.virtual_base);
    let y = u128::from(args.quote_reserve) + u128::from(args.virtual_quote);
    if x == 0 || y == 0 {
        return 0;
    }
    if before && args.direction == BUY {
        // Constant product: the price ratio after / before is (y + q)^2 / y^2.
        let y1 = y + u128::from(side_amount);
        let num = y1.saturating_mul(y1).saturating_sub(y.saturating_mul(y));
        num.saturating_mul(10_000) / y.saturating_mul(y).max(1)
    } else {
        // A sell after the swap: x and y are after; before was (x - base in, y + quote out).
        let x0 = x.saturating_sub(u128::from(args.amount_in));
        let y0 = y.saturating_add(u128::from(args.amount_out));
        let a = y0.saturating_mul(x);
        let b = y.saturating_mul(x0);
        if a == 0 {
            return 0;
        }
        a.saturating_sub(b).saturating_mul(10_000) / a
    }
}

/// The cut rate for an impact of `impact` bps.
pub fn rate(p: &hookwars_common::Params, impact: u128) -> u32 {
    let r = impact.saturating_mul(u128::from(p[0])) / 100;
    r.min(u128::from(p[1])) as u32
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let impact = impact_bps(args, ctx.side_amount, before);
    Ok(PoolOut {
        cut: fee(ctx.side_amount, rate(&env.params, impact)),
        ..Default::default()
    })
}
