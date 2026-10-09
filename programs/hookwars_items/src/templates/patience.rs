// Changed by Hookwars: new file (arsenal wave D).
//! Patience (id 40, 08 section 4.7): wallets whose tokens are at least `min_age_secs` old pay
//! `discount_bps` less of our launch fees when they sell. Fields: `min_age_secs`, `discount_bps`,
//! `source` (as built only 1: the module's own age stamp). Range 5 bytes: tag, `since: u32`, kept
//! like Half-Life's (age travels with tokens, blends on receive). Extras: `RaidLedger` (w), our
//! `Launch`.
//!
//! A sell's fees are taken after the swap, but by then the seller's tokens have moved; so the
//! token half checks the age on the seller's transfer into the pool and marks the seller in the
//! ledger's `sell_mark` (this slot's bit, the convention Shield uses), and `pool_after_swap`
//! grants the discount from that mark.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};

use super::raid::{load_ledger, save_ledger};
use super::shield::blend;
use super::{exempt, own_pool, Env, PoolOut, TokenOut, SELL};
use crate::ItemsError;

/// Range bytes.
pub const LEN: usize = 5;
/// Layout tag.
pub const TAG: u8 = 0x28;

fn since(b: &[u8]) -> Option<u32> {
    (b.len() >= LEN && b[0] == TAG).then(|| u32::from_le_bytes(b[1..5].try_into().unwrap()))
}

fn stamp(at: u32) -> Vec<u8> {
    let mut v = vec![TAG];
    v.extend_from_slice(&at.to_le_bytes());
    v
}

fn bit(slot: u8) -> u8 {
    1u8.checked_shl(u32::from(slot)).unwrap_or(0)
}

/// `before_transfer`.
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let x = env.extras;
    require!(x.len() >= 2, ItemsError::WrongAccount);
    let pool = own_pool(&x[1], &env.mint);
    let now = u32::try_from(env.now.max(0)).unwrap_or(u32::MAX);
    let source_exempt = exempt(&args.source_owner, &env.mint, pool);
    let destination_exempt = exempt(&args.destination_owner, &env.mint, pool) || !args.destination_owner.is_on_curve();
    let s = since(src);
    if !source_exempt && Some(args.destination_owner) == pool {
        if let Some(at) = s {
            if env.now - i64::from(at) >= i64::from(env.params[0]) {
                if let Some(mut l) = load_ledger(&x[0], &env.mint) {
                    if l.sell_mark.clock_slot != env.clock_slot || l.sell_mark.seller != args.source_owner {
                        l.sell_mark = hookwars_common::raid::SellMark {
                            clock_slot: env.clock_slot,
                            seller: args.source_owner,
                            marked_slots: 0,
                        };
                    }
                    l.sell_mark.marked_slots |= bit(env.slot);
                    save_ledger(&x[0], &l)?;
                }
            }
        }
    }
    let destination = if destination_exempt || args.amount == 0 {
        None
    } else {
        let incoming = if source_exempt { now } else { s.unwrap_or(now) };
        let held = since(dst).unwrap_or(0);
        Some(stamp(blend(args.destination_balance, held, args.amount, incoming)))
    };
    let source = (!source_exempt && args.source_balance == args.amount && s.is_some()).then(|| vec![0; LEN]);
    Ok(TokenOut {
        cut: 0,
        source,
        destination,
    })
}

/// `pool_after_swap`: the discount for a marked seller.
pub fn pool(env: &Env, args: &PoolHookArgs, _ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    if before || args.direction != SELL {
        return Ok(PoolOut::default());
    }
    let Some(info) = env.extras.first() else {
        return Ok(PoolOut::default());
    };
    let Some(mut l) = load_ledger(info, &env.mint) else {
        return Ok(PoolOut::default());
    };
    let b = bit(env.slot);
    let m = l.sell_mark;
    if m.clock_slot != env.clock_slot || m.seller != args.actor || m.marked_slots & b == 0 {
        return Ok(PoolOut::default());
    }
    l.sell_mark.marked_slots &= !b;
    save_ledger(info, &l)?;
    Ok(PoolOut {
        discount_bps: env.params[1].min(10_000) as u16,
        ..Default::default()
    })
}
