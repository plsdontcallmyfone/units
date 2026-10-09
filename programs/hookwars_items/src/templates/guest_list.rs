// Changed by Hookwars: new file (arsenal wave D).
//! Guest List (id 23, 08 section 4.2): for the first `open_after_secs` after the launch, a buy is
//! refused unless the buyer holds at least `min_hold` of the target mint. Fields: `min_hold`,
//! `open_after_secs`. Extras: our `Launch`, the buyer's holding of the target (derived from the
//! actor, R22).

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{held, Env, PoolOut, BUY};
use crate::payouts::ArsenalError;

/// `pool_before_swap`.
pub fn pool(env: &Env, args: &PoolHookArgs, _ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != BUY {
        return Ok(PoolOut::default());
    }
    let x = env.extras;
    require!(x.len() >= 2, crate::ItemsError::WrongAccount);
    let created = x[0]
        .try_borrow_data()
        .ok()
        .and_then(|d| hookwars_common::launch_created_at(&d, &env.mint))
        .unwrap_or(0);
    if env.now >= created.saturating_add(i64::from(env.params[1])) {
        return Ok(PoolOut::default());
    }
    let target = env.targets.first().ok_or(crate::ItemsError::BadTargets)?;
    let amount = held(&x[1], target, &args.actor).unwrap_or(0);
    require!(amount >= u64::from(env.params[0]), ArsenalError::GuestListClosed);
    Ok(PoolOut::default())
}
