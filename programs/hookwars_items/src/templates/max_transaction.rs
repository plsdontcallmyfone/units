// Changed by Hookwars: new file (M3b).
//! Max Transaction (id 19) and Dust Guard (id 22), 08 section 4.2. Extras: our `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{exempt, own_pool, Env, TokenOut};
use crate::ItemsError;

/// Max Transaction: refuses a transfer above `max_tx_bps` of supply, except protocol transfers
/// (the launch, its pool, the vaults: exempt owners as Half-Life's).
pub fn max_tx(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    let pool = own_pool(&env.extras[0], &env.mint);
    let launch = hookwars_common::pda::launch(&env.mint).0;
    let protocol = args.source_owner == launch
        || args.destination_owner == launch
        || (exempt(&args.source_owner, &env.mint, pool) && Some(args.source_owner) != pool)
        || (exempt(&args.destination_owner, &env.mint, pool) && Some(args.destination_owner) != pool);
    if !protocol {
        let cap = u128::from(args.supply) * u128::from(env.params[0]) / 10_000;
        require!(u128::from(args.amount) <= cap, ItemsError::TransferTooLarge);
    }
    Ok(TokenOut::default())
}

/// Dust Guard: refuses a wallet-to-wallet send under `min_amount` (buys, sells and an exact
/// empty-out are exempt).
pub fn dust(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    let pool = own_pool(&env.extras[0], &env.mint);
    let wallets = args.source_owner.is_on_curve()
        && args.destination_owner.is_on_curve()
        && Some(args.source_owner) != pool
        && Some(args.destination_owner) != pool;
    if wallets && args.amount != args.source_balance {
        require!(u64::from(env.params[0]) <= args.amount, ItemsError::DustRefused);
    }
    Ok(TokenOut::default())
}
