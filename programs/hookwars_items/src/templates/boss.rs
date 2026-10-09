// Changed by Hookwars: new file (expansion, 10 section 11.1).
//! Boss (id 44, kind Pool). Equipped on the protocol's boss token. Every buy of the boss whose route
//! started by selling another units token on that token's own launch pool counts as a raid from
//! that source: the volume is added to the boss's `RaidLedger` per source mint, over windows of
//! `params[0]` seconds (set to the season length), and to the ledger's season total. War pays each
//! source's war chest its share of the boss pool after the season (`claim_boss_share`, an
//! integration request in 10). The item never cuts, discounts or burns. Extras: the boss token's
//! `RaidLedger` (w).

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::PoolHookArgs;
use hookwars_common::ids;

use super::raid::{load_ledger, save_ledger, war_view};
use super::{launch_pool_address, Env, PoolOut, BUY};
use crate::ItemsError;

/// The source a boss buy counts for, if any: a buy whose first hop sold a units token (not the
/// boss itself, not bridged SOL) on that token's canonical launch pool. The launch pool address
/// is a PDA of the source mint, so no attacker-made pool can stand in for it (review 1 M-5).
pub fn source(env: &Env, args: &PoolHookArgs) -> Option<Pubkey> {
    let r = &args.route;
    if args.direction != BUY
        || r.hop_index < 1
        || r.route_input_mint == env.mint
        || r.route_input_mint == ids::BRIDGED_SOL_MINT
        || r.first_pool != launch_pool_address(&r.route_input_mint)
    {
        return None;
    }
    Some(r.route_input_mint)
}

/// Pool callbacks: counts in `pool_after_swap` only.
pub fn pool(env: &Env, args: &PoolHookArgs, _ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if before {
        return Ok(PoolOut::default());
    }
    let Some(src) = source(env, args) else {
        return Ok(PoolOut::default());
    };
    let x = env.extras;
    require!(!x.is_empty(), ItemsError::WrongAccount);
    let Some(mut l) = load_ledger(&x[0], &env.mint) else {
        return Ok(PoolOut::default());
    };
    let season = x.get(1).map(war_view).unwrap_or_default().season;
    l.roll_season(season);
    let window = i64::from(env.params[0]).max(1);
    l.add_inbound(&src, env.now, window, args.amount_in);
    l.outbound_volume_season = l.outbound_volume_season.saturating_add(args.amount_in);
    save_ledger(&x[0], &l)?;
    Ok(PoolOut::default())
}
