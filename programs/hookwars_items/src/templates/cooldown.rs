// Changed by Hookwars: new file (arsenal wave B).
//! Cooldown (id 17, 08 section 4.2): after a buy, a wallet waits `cooldown_secs` before selling
//! or sending. Range 5 bytes: tag, `last_buy: u32` (unix seconds). Extras: our `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{exempt, own_pool, Env, TokenOut};
use crate::ItemsError;

/// Range bytes.
pub const LEN: usize = 5;
/// Layout tag.
pub const TAG: u8 = 0x11;

/// The stamped `last_buy`, if any.
pub fn last_buy(b: &[u8]) -> Option<u32> {
    (b.len() >= LEN && b[0] == TAG).then(|| u32::from_le_bytes(b[1..5].try_into().unwrap()))
}

fn stamp(at: u32) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&at.to_le_bytes());
    v
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let now = u32::try_from(env.now.max(0)).unwrap_or(u32::MAX);
    let mut out = TokenOut::default();
    if Some(args.source_owner) == pool {
        // A buy: stamp the buyer, keeping the later stamp.
        if !exempt(&args.destination_owner, &env.mint, pool) {
            let at = last_buy(dst).map_or(now, |l| l.max(now));
            out.destination = Some(stamp(at));
        }
    } else if !exempt(&args.source_owner, &env.mint, pool) {
        // A sell or a send out of a wallet.
        if let Some(l) = last_buy(src) {
            let until = i64::from(l) + i64::from(env.params[0]);
            require!(env.now >= until, ItemsError::CooldownActive);
            if args.source_balance == args.amount {
                out.source = Some(vec![0; LEN]);
            }
        }
    }
    Ok(out)
}
