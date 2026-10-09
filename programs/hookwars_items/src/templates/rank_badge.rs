// Changed by Hookwars: new file (arsenal wave B).
//! Rank Badge (id 35, 08 section 4.7): every `unit_lamports` bought earns a unit; every
//! `rank_step` units is a rank, up to `max_rank`. Range 6 bytes: tag, `volume_units: u32`,
//! `rank: u8`. Extras: our `Launch` and our launch pool. A buy's quote value is the tokens bought
//! times the pool's time-weighted price over `MIN_TWAP_SECS` (rule 7); no signal earns nothing.
//! Sends move nothing; emptying the holding clears it.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::velocity_fee::own_pool_account;
use super::{exempt, own_pool, Env, TokenOut};

/// Range bytes.
pub const LEN: usize = 6;
/// Layout tag.
pub const TAG: u8 = 0x23;

/// `(volume_units, rank)`, if stamped.
pub fn read(b: &[u8]) -> Option<(u32, u8)> {
    (b.len() >= LEN && b[0] == TAG).then(|| (u32::from_le_bytes(b[1..5].try_into().unwrap()), b[5]))
}

/// The encoded range.
pub fn write(units: u32, rank: u8) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&units.to_le_bytes());
    v.push(rank);
    v
}

/// The rank for `units`.
pub fn rank(p: &hookwars_common::Params, units: u32) -> u8 {
    (units / p[1].max(1)).min(p[2]).min(u32::from(u8::MAX)) as u8
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let pool = own_pool(&env.extras[0], &env.mint);
    let mut out = TokenOut::default();
    if Some(args.source_owner) == pool && !exempt(&args.destination_owner, &env.mint, pool) {
        let twap = own_pool_account(env).and_then(|p| {
            bordrless_swap::obs::window_read(
                p,
                env.now,
                bordrless_swap::constants::MIN_TWAP_SECS,
                bordrless_swap::constants::MIN_TWAP_SECS,
            )
            .ok()
        });
        if let Some(r) = twap {
            let quote = (u128::from(args.amount).saturating_mul(r.twap_q64)) >> 64;
            let earned = (quote / u128::from(env.params[0].max(1))).min(u128::from(u32::MAX)) as u32;
            let have = read(dst).map_or(0, |(u, _)| u);
            if earned > 0 || read(dst).is_some() {
                let units = have.saturating_add(earned);
                out.destination = Some(write(units, rank(&env.params, units)));
            }
        }
    } else if args.source_balance == args.amount && read(src).is_some() && Some(args.source_owner) != pool {
        out.source = Some(vec![0; LEN]);
    }
    Ok(out)
}
