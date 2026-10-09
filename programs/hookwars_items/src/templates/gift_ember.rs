// Changed by Hookwars: new file (arsenal wave E).
//! Gift Ember (id 34, 08 section 4.6): `cut_bps` of every wallet-to-wallet send is cut into the
//! equip vault and burned at settlement (a token callback never answers a burn). Buys, sells and
//! protocol transfers pay nothing. Field: `cut_bps`. Extras: our `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{exempt, fee, own_pool, Env, TokenOut};

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let wallet = |o: &Pubkey| o.is_on_curve() && !exempt(o, &env.mint, pool);
    if !wallet(&args.source_owner) || !wallet(&args.destination_owner) {
        return Ok(TokenOut::default());
    }
    Ok(TokenOut {
        cut: fee(args.amount, env.params[0]),
        ..Default::default()
    })
}
