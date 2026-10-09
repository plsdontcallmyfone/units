// Changed by Hookwars: new file (arsenal wave E).
//! First Blood (id 38, 08 section 4.7): the first buy of at least `min_lamports` each UTC day pays
//! `discount_bps` less of our launch fees. Fields: `discount_bps`, `min_lamports`. Extras:
//! `["first-blood", mint]` (w; `init_first_blood`, in `crate::payouts`). Without it, nothing.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{Env, PoolOut, BUY};
use crate::payouts::FirstBlood;

/// `pool_before_swap`.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != BUY || ctx.side_amount < u64::from(env.params[1]) {
        return Ok(PoolOut::default());
    }
    let Some(info) = env.extras.first() else {
        return Ok(PoolOut::default());
    };
    if *info.key != hookwars_common::arsenal2::pda::first_blood(&env.mint).0
        || *info.owner != crate::ID
        || !info.is_writable
    {
        return Ok(PoolOut::default());
    }
    let mut s = FirstBlood::try_deserialize(&mut &info.try_borrow_data()?[..])?;
    let day = u32::try_from(env.now.max(0) / 86_400).unwrap_or(u32::MAX);
    if s.last_day == day && s.taken {
        return Ok(PoolOut::default());
    }
    s.last_day = day;
    s.taken = true;
    let mut data = info.try_borrow_mut_data()?;
    let mut out: &mut [u8] = &mut data[..];
    s.try_serialize(&mut out)?;
    Ok(PoolOut {
        discount_bps: env.params[0].min(10_000) as u16,
        ..Default::default()
    })
}
