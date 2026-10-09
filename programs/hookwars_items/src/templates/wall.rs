// Changed by Hookwars: new file (M3b).
//! Wall (id 3, 04 section 3.3): a temporary max wallet while under siege. Extras: our `WarState`.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::shield::under_siege;
use super::{Env, TokenOut};
use crate::ItemsError;

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    let (siege, chest) = under_siege(&env.extras[0], &env.mint, env.now);
    if !siege {
        return Ok(TokenOut::default());
    }
    let capped = args.destination_owner.is_on_curve() || args.destination_owner == chest;
    if !capped || args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let cap = u128::from(args.supply) * u128::from(env.params[0]) / 10_000;
    let after = u128::from(args.destination_balance) + u128::from(args.amount);
    require!(after <= cap, ItemsError::WallHolds);
    Ok(TokenOut::default())
}
