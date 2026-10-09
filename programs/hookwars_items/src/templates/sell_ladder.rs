// Changed by Hookwars: new file (arsenal wave E).
//! Sell Ladder (id 37, 08 section 4.7): a sell pays more the bigger the share of the seller's
//! holding it moves: `cut_per_step_bps` per `step_bps` sold, up to `max_cut_bps`, cut from the
//! tokens sold into the equip vault and burned at settlement. Fields: `step_bps`,
//! `cut_per_step_bps`, `max_cut_bps`. Extras: our `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;
use hookwars_common::arsenal2::ladder_bps;

use super::{exempt, fee, own_pool, Env, TokenOut};

/// `before_transfer`: a sell is a wallet's transfer into the launch pool.
pub fn token(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    let pool = own_pool(&env.extras[0], &env.mint);
    if pool != Some(args.destination_owner) || exempt(&args.source_owner, &env.mint, pool) {
        return Ok(TokenOut::default());
    }
    let bps = ladder_bps(&env.params, args.amount, args.source_balance);
    Ok(TokenOut {
        cut: fee(args.amount, bps),
        ..Default::default()
    })
}
