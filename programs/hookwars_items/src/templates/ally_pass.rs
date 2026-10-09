// Changed by Hookwars: new file (arsenal wave D).
//! Ally Pass (id 27, 08 section 4.4): buyers who hold at least `min_hold` of the ally mint get
//! `discount_bps` off this token's own launch fees. Fields: `min_hold`, `discount_bps`. Extras: the
//! buyer's holding of the ally (derived from the actor, R22).

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{held, Env, PoolOut, BUY};

/// `pool_before_swap`.
pub fn pool(env: &Env, args: &PoolHookArgs, _ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != BUY {
        return Ok(PoolOut::default());
    }
    let (Some(ally), Some(h)) = (env.targets.first(), env.extras.first()) else {
        return Ok(PoolOut::default());
    };
    match held(h, ally, &args.actor) {
        Some(a) if a >= u64::from(env.params[0]) => Ok(PoolOut {
            discount_bps: env.params[1].min(10_000) as u16,
            ..Default::default()
        }),
        _ => Ok(PoolOut::default()),
    }
}
