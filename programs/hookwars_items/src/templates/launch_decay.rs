// Changed by Hookwars: new file (M3b).
//! Launch Decay (id 12, 08 section 4.1): a cut falling linearly from launch. Fields:
//! `start_cut_bps`, `end_cut_bps`, `decay_secs`. Extras: our `Launch` (its `created_at`).

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, quote_side, Env, PoolOut};

/// The cut rate `elapsed` seconds after launch.
pub fn rate(p: &hookwars_common::Params, elapsed: i64) -> u32 {
    let (start, end, d) = (p[0], p[1], i64::from(p[2]).max(1));
    if elapsed >= d {
        return end;
    }
    let e = elapsed.max(0) as u64;
    start - ((u64::from(start - end) * e) / d as u64) as u32
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !quote_side(args, before) {
        return Ok(PoolOut::default());
    }
    let Some(created) = env.extras.first().and_then(|l| {
        (*l.owner == hookwars_common::ids::LAUNCH_ID && *l.key == hookwars_common::pda::launch(&env.mint).0)
            .then(|| l.try_borrow_data().ok().and_then(|d| hookwars_common::launch_created_at(&d, &env.mint)))
            .flatten()
    }) else {
        return Ok(PoolOut::default());
    };
    Ok(PoolOut {
        cut: fee(ctx.side_amount, rate(&env.params, env.now - created)),
        ..Default::default()
    })
}
