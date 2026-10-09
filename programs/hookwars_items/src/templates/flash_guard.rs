// Changed by Hookwars: new file (arsenal wave B).
//! Flash Guard (id 20, 08 section 4.2): tokens cannot be sold within `min_slots` slots of being
//! bought. Range 5 bytes: tag, `buy_slot_low: u32` (the low 32 bits of `Clock::slot`, compared
//! with wrapping arithmetic). Extras: our `Launch`. Sends are not refused (Cooldown covers them).

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{exempt, own_pool, Env, TokenOut};
use crate::ItemsError;

/// Range bytes.
pub const LEN: usize = 5;
/// Layout tag.
pub const TAG: u8 = 0x14;

/// The stamped slot, if any.
pub fn buy_slot(b: &[u8]) -> Option<u32> {
    (b.len() >= LEN && b[0] == TAG).then(|| u32::from_le_bytes(b[1..5].try_into().unwrap()))
}

fn stamp(at: u32) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&at.to_le_bytes());
    v
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], _dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let low = env.clock_slot as u32;
    let mut out = TokenOut::default();
    if Some(args.source_owner) == pool {
        if !exempt(&args.destination_owner, &env.mint, pool) {
            out.destination = Some(stamp(low));
        }
    } else if !exempt(&args.source_owner, &env.mint, pool) {
        if let Some(at) = buy_slot(src) {
            if Some(args.destination_owner) == pool {
                require!(low.wrapping_sub(at) >= env.params[0], ItemsError::FlashSellTooSoon);
            }
            if args.source_balance == args.amount {
                out.source = Some(vec![0; LEN]);
            }
        }
    }
    Ok(out)
}
