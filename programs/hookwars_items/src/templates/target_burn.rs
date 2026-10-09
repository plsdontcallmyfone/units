// Changed by Hookwars: new file (arsenal wave E).
//! Target Burn (id 33, 08 section 4.6): `burn_bps` of the base side of every trade is burned
//! (a sell's input, a buy's output, R21) until the supply reaches `target_supply_bps` of what the
//! launch started with. Fields: `burn_bps`, `target_supply_bps`. Extras: our `Launch`, our `Mint`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;
use hookwars_common::arsenal2::{launch_initial_supply, target_burn};

use super::{quote_side, Env, PoolOut};

/// Pool callbacks: the base side only.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    let x = env.extras;
    if quote_side(args, before) || x.len() < 2 || *x[1].key != env.mint {
        return Ok(PoolOut::default());
    }
    let Some(initial) = x[0].try_borrow_data().ok().and_then(|d| launch_initial_supply(&d, &env.mint)) else {
        return Ok(PoolOut::default());
    };
    let supply = bordrless_token::client::read_mint(&x[1])?.supply;
    Ok(PoolOut {
        burn: target_burn(&env.params, ctx.side_amount, supply, initial),
        ..Default::default()
    })
}
