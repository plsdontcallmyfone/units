// Changed by Hookwars: new file (M3b).
//! Transfer Fee (id 8, 04 section 3.8): upstream `tax_hook` as a template. Fields: `fee_bps`,
//! `max_wallet_bps` (0 off). Target: the collector (paid at settlement). Extras: our `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{exempt, fee, own_pool, Env, TokenOut};
use crate::ItemsError;

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let collector = env.targets.first().copied().unwrap_or_default();
    let pool = own_pool(&env.extras[0], &env.mint);
    let free = args.source_owner == collector
        || args.destination_owner == collector
        || exempt(&args.source_owner, &env.mint, pool) && args.source_owner != pool.unwrap_or_default()
        || exempt(&args.destination_owner, &env.mint, pool) && args.destination_owner != pool.unwrap_or_default();
    let cut = if free { 0 } else { fee(args.amount, env.params[0]) };
    let cap_bps = env.params[1];
    if cap_bps > 0
        && cap_bps < 10_000
        && args.destination_owner != collector
        && !exempt(&args.destination_owner, &env.mint, pool)
    {
        let cap = u128::from(args.supply) * u128::from(cap_bps) / 10_000;
        let after = u128::from(args.destination_balance) + u128::from(args.amount - cut);
        require!(after <= cap, ItemsError::WalletTooLarge);
    }
    Ok(TokenOut {
        cut,
        ..Default::default()
    })
}
