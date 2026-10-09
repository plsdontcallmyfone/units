// Changed by Hookwars: new file (arsenal wave E).
//! Garrison (id 30, 08 section 4.5): while we are under siege, buyers pay `discount_bps` less of
//! our launch fees. Field: `discount_bps`. Extras: our `WarState`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::shield::under_siege;
use super::{Env, PoolOut, BUY};

/// `pool_before_swap`.
pub fn pool(env: &Env, args: &PoolHookArgs, _ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != BUY {
        return Ok(PoolOut::default());
    }
    let Some(ws) = env.extras.first() else {
        return Ok(PoolOut::default());
    };
    if !under_siege(ws, &env.mint, env.now).0 {
        return Ok(PoolOut::default());
    }
    Ok(PoolOut {
        discount_bps: env.params[0].min(10_000) as u16,
        ..Default::default()
    })
}
