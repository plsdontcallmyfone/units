// Changed by Hookwars: new file (arsenal wave B).
//! Streak (id 26, 08 section 4.3): buys on consecutive UTC days build a streak, up to
//! `max_streak`; any sell or send out resets it. Range 6 bytes: tag, `last_day: u16`,
//! `streak: u16`, `flags: u8`. Extras: our `Launch`. Pays nothing alone; it exports its bytes.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::daily_sell_cap::day;
use super::{exempt, own_pool, Env, TokenOut};

/// Range bytes.
pub const LEN: usize = 6;
/// Layout tag.
pub const TAG: u8 = 0x1a;

/// `(last_day, streak)`, if stamped.
pub fn read(b: &[u8]) -> Option<(u16, u16)> {
    (b.len() >= LEN && b[0] == TAG).then(|| {
        (
            u16::from_le_bytes(b[1..3].try_into().unwrap()),
            u16::from_le_bytes(b[3..5].try_into().unwrap()),
        )
    })
}

/// The encoded range.
pub fn write(last_day: u16, streak: u16) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&last_day.to_le_bytes());
    v.extend_from_slice(&streak.to_le_bytes());
    v.push(0);
    v
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let today = day(env.now);
    let max = u16::try_from(env.params[0]).unwrap_or(u16::MAX);
    let mut out = TokenOut::default();
    if Some(args.source_owner) == pool {
        if !exempt(&args.destination_owner, &env.mint, pool) {
            let streak = match read(dst) {
                Some((last, s)) if last == today => s.max(1),
                Some((last, s)) if last.wrapping_add(1) == today => s.saturating_add(1),
                _ => 1,
            };
            out.destination = Some(write(today, streak.min(max)));
        }
    } else if !exempt(&args.source_owner, &env.mint, pool) {
        if let Some((last, _)) = read(src) {
            out.source = Some(if args.source_balance == args.amount { vec![0; LEN] } else { write(last, 0) });
        }
    }
    Ok(out)
}
