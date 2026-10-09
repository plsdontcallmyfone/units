// Changed by Hookwars: new file (M3b).
//! Half-Life (id 7, 04 section 3.7): upstream `half_life` as a template. Fields: `max_fee_ppm`,
//! `half_life_secs`, `zero_after_halvings`. Range 5 bytes: tag, `since: u32`. Extras: our
//! `Launch`. The fee is one cut into the equip vault, burned at settlement.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::shield::blend;
use super::{exempt, own_pool, Env, TokenOut};

/// Range bytes.
pub const LEN: usize = 5;
/// Layout tag.
pub const TAG: u8 = 0x07;

/// The exit fee in ppm for tokens `age` seconds old (upstream `fee_ppm` with the fields).
pub fn fee_ppm(p: &hookwars_common::Params, age: i64) -> u64 {
    let (max, hl, zero) = (u64::from(p[0]), i64::from(p[1]).max(1), i64::from(p[2]));
    if age <= 0 {
        return max;
    }
    let halvings = age / hl;
    if halvings >= zero || halvings >= 63 {
        return 0;
    }
    let into = (age % hl) as u64;
    let hi = max >> halvings;
    let lo = max >> (halvings + 1);
    hi - (hi - lo) * into / hl as u64
}

fn since(b: &[u8]) -> Option<u32> {
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
    let launch = hookwars_common::pda::launch(&env.mint).0;
    let now = u32::try_from(env.now.max(0)).unwrap_or(u32::MAX);
    let source_exempt = exempt(&args.source_owner, &env.mint, pool);
    let destination_exempt = exempt(&args.destination_owner, &env.mint, pool) || args.source_owner == launch;
    let s = since(src);
    let cut = if source_exempt {
        0
    } else {
        let age = env.now - i64::from(s.unwrap_or(now));
        let ppm = fee_ppm(&env.params, age);
        ((u128::from(args.amount) * u128::from(ppm) / 1_000_000) as u64).min(args.amount)
    };
    let received = args.amount - cut;
    let destination = if destination_exempt || received == 0 {
        None
    } else {
        let incoming = if source_exempt { now } else { s.unwrap_or(now) };
        let held = since(dst).unwrap_or(now);
        Some(stamp(blend(args.destination_balance, held, received, incoming)))
    };
    let source = (!source_exempt && args.source_balance == args.amount && s.is_some()).then(|| vec![0; LEN]);
    Ok(TokenOut {
        cut,
        source,
        destination,
    })
}
