// Changed by Hookwars: new file (arsenal wave D).
//! Referral (id 36, 08 section 4.7): a buyer who named a referrer (`set_referrer`) pays `cut_bps`
//! of each buy; the cut is recorded as owed to that referrer in the buyer's `Referred` account and
//! paid by `settle_referral` (in `crate::payouts`) from the Referral vault. Field: `cut_bps`.
//! Extras: the buyer's `["referred", mint, buyer]` (w, derived from the actor, R22). A buyer
//! without one pays nothing.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;

use super::{fee, Env, PoolOut, BUY};
use crate::payouts::Referred;

/// `pool_before_swap`.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if !before || args.direction != BUY {
        return Ok(PoolOut::default());
    }
    let Some(info) = env.extras.first() else {
        return Ok(PoolOut::default());
    };
    let expected = hookwars_common::arsenal2::pda::referred(&env.mint, &args.actor).0;
    if *info.key != expected || *info.owner != crate::ID || !info.is_writable {
        return Ok(PoolOut::default());
    }
    let mut r = Referred::try_deserialize(&mut &info.try_borrow_data()?[..])?;
    let cut = fee(ctx.side_amount, env.params[0]);
    r.owed = r.owed.checked_add(cut).ok_or(crate::ItemsError::Overflow)?;
    let mut data = info.try_borrow_mut_data()?;
    let mut out: &mut [u8] = &mut data[..];
    r.try_serialize(&mut out)?;
    Ok(PoolOut {
        cut,
        ..Default::default()
    })
}
