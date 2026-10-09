// Changed by Hookwars: new file (arsenal wave B).
//! Daily Sell Cap (id 18, 08 section 4.2): a wallet may sell or send at most `cap_bps` of its
//! day's starting balance per UTC day. Range 11 bytes: tag, `day: u16`, `base: u64`. Extras: our
//! `Launch`. Receiving the same day raises `base` by the amount received (before other slots'
//! cuts, an upper bound).

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{exempt, own_pool, Env, TokenOut};
use crate::ItemsError;

/// Range bytes.
pub const LEN: usize = 11;
/// Layout tag.
pub const TAG: u8 = 0x12;

/// `(day, base)`, if stamped.
pub fn read(b: &[u8]) -> Option<(u16, u64)> {
    (b.len() >= LEN && b[0] == TAG).then(|| {
        (
            u16::from_le_bytes(b[1..3].try_into().unwrap()),
            u64::from_le_bytes(b[3..11].try_into().unwrap()),
        )
    })
}

/// The encoded range.
pub fn write(day: u16, base: u64) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&day.to_le_bytes());
    v.extend_from_slice(&base.to_le_bytes());
    v
}

/// The UTC day number of `now`, truncated to 16 bits.
pub fn day(now: i64) -> u16 {
    (now.max(0) / 86_400) as u16
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let today = day(env.now);
    let mut out = TokenOut::default();
    if Some(args.source_owner) != pool && !exempt(&args.source_owner, &env.mint, pool) {
        let base = match read(src) {
            Some((d, b)) if d == today => b,
            _ => args.source_balance,
        };
        let after = args.source_balance.saturating_sub(args.amount);
        let spent = base.saturating_sub(after);
        let cap = (u128::from(base) * u128::from(env.params[0]) / 10_000) as u64;
        require!(spent <= cap, ItemsError::DailyCapExceeded);
        out.source = Some(if after == 0 { vec![0; LEN] } else { write(today, base) });
    }
    if !exempt(&args.destination_owner, &env.mint, pool) {
        if let Some((d, b)) = read(dst) {
            if d == today {
                out.destination = Some(write(today, b.saturating_add(args.amount)));
            }
        }
    }
    Ok(out)
}
