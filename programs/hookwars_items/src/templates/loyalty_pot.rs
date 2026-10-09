// Changed by Hookwars: new file (arsenal wave E).
//! Loyalty Pot (id 24, 08 section 4.3): sells pay `sell_cut_bps` (of the quote out) into the pot
//! (`["loyalty", mint]`'s quote holding, through settlement); every `epoch_secs`, wallets that held
//! the whole previous epoch claim a share by balance (`claim_loyalty`, in `crate::payouts`).
//! Fields: `sell_cut_bps`, `epoch_secs`. Range 5 bytes: tag, `joined_epoch: u32`, rewritten to the
//! current epoch whenever the holding receives tokens. Extras: our `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};
use hookwars_common::arsenal2::epoch_of;

use super::{exempt, fee, own_pool, Env, PoolOut, TokenOut, SELL};

/// Range bytes.
pub const LEN: usize = 5;
/// Layout tag.
pub const TAG: u8 = 0x18;

/// The epoch a holding last received tokens in; `None` when never stamped (it held from before).
pub fn joined(b: &[u8]) -> Option<u32> {
    (b.len() >= LEN && b[0] == TAG).then(|| u32::from_le_bytes(b[1..5].try_into().unwrap()))
}

fn stamp(epoch: u32) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&epoch.to_le_bytes());
    v
}

/// `before_transfer`: receiving restamps the destination; emptying clears the source.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], _dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let wallet = |o: &Pubkey| o.is_on_curve() && !exempt(o, &env.mint, pool);
    let destination = (wallet(&args.destination_owner) && args.amount > 0)
        .then(|| stamp(epoch_of(env.now, env.params[1])));
    let source = (args.source_balance == args.amount && joined(src).is_some()).then(|| vec![0; LEN]);
    Ok(TokenOut {
        cut: 0,
        source,
        destination,
    })
}

/// `pool_after_swap`: the sell's cut (the quote out).
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if before || args.direction != SELL {
        return Ok(PoolOut::default());
    }
    Ok(PoolOut {
        cut: fee(ctx.side_amount, env.params[0]),
        ..Default::default()
    })
}
