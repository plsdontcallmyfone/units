// Changed by Hookwars: new file (M3b); security review 2 L-B: the sell mark names who signed the
// sell (owner or delegate, the swap's actor), and the origin travels to every destination but the
// launch's own accounts.
//! Shield (id 2, 04 section 3.2). Extras: `RaidLedger` (w), `WarConfig`, our `WarState`, our
//! `Launch`, each target's `Launch`.
//!
//! As built (04 M3b notes): the token half marks a sell of a recently raided holding in the
//! ledger's `sell_mark`, and `pool_after_swap` cuts from that mark, so the pool half needs no read
//! of the seller's holding. Shield writes the raid mark as Raid does but never counts inbound
//! volume (Raid does), so a swap is never counted twice.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};
use hookwars_common::pda;

use super::raid::{load_ledger, raid_target, save_ledger, war_view, write_mark};
use super::{fee, own_pool, Env, PoolOut, TokenOut, SELL};
use crate::{ItemsError, ShieldTaken};

/// Range bytes.
pub const LEN: usize = 6;
/// Layout tag.
pub const TAG: u8 = 0x02;

fn read(b: &[u8]) -> (u8, u32) {
    if b.len() < LEN || b[0] != TAG {
        return (0, 0);
    }
    (b[1], u32::from_le_bytes(b[2..6].try_into().unwrap()))
}

fn write(origin: u8, at: u32) -> Vec<u8> {
    if origin == 0 {
        return vec![0; LEN];
    }
    let mut v = vec![TAG, origin];
    v.extend_from_slice(&at.to_le_bytes());
    v
}

/// Whether the token is under siege now (05 `WarState.under_siege_until`).
pub fn under_siege(info: &AccountInfo, mint: &Pubkey, now: i64) -> (bool, Pubkey) {
    if *info.owner != hookwars_common::ids::WAR_ID || *info.key != pda::war_state(mint).0 {
        return (false, Pubkey::default());
    }
    let Ok(data) = info.try_borrow_data() else {
        return (false, Pubkey::default());
    };
    match hookwars_war::state::WarState::try_deserialize(&mut &data[..]) {
        Ok(s) if s.under_siege_until > now => (true, s.siege_by_chest),
        _ => (false, Pubkey::default()),
    }
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    let x = env.extras;
    require!(x.len() >= 4 + env.targets.len(), ItemsError::WrongAccount);
    if before {
        return Ok(PoolOut::default());
    }
    if args.direction == SELL {
        let only_under_siege = env.params[2] == 1;
        if only_under_siege && !under_siege(&x[2], &env.mint, env.now).0 {
            return Ok(PoolOut::default());
        }
        let Some(mut l) = load_ledger(&x[0], &env.mint) else {
            return Ok(PoolOut::default());
        };
        let bit = 1u8.checked_shl(u32::from(env.slot)).unwrap_or(0);
        let m = l.sell_mark;
        if m.clock_slot == env.clock_slot && m.seller == args.actor && m.marked_slots & bit != 0 {
            l.sell_mark.marked_slots &= !bit;
            save_ledger(&x[0], &l)?;
            let cut = fee(ctx.side_amount, env.params[0]);
            emit!(ShieldTaken {
                mint: env.mint,
                owner: args.actor,
                cut,
            });
            return Ok(PoolOut {
                cut,
                ..Default::default()
            });
        }
        return Ok(PoolOut::default());
    }
    if let Some(i) = raid_target(env, args, &x[4..]) {
        let war = war_view(&x[1]);
        write_mark(env, &x[0], war, args, env.targets[i], false)?;
    }
    Ok(PoolOut::default())
}

/// `before_transfer` (the token half).
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let x = env.extras;
    require!(x.len() >= 4, ItemsError::WrongAccount);
    let pool = own_pool(&x[3], &env.mint);
    let now = u32::try_from(env.now.max(0)).unwrap_or(u32::MAX);
    let (so, sat) = read(src);
    let (dor, dat) = read(dst);
    let mut out = TokenOut::default();
    let Some(p) = pool else {
        return Ok(out);
    };
    if args.source_owner == p {
        // A buy: a raid mark for this recipient this slot gives the origin.
        if let Some(l) = load_ledger(&x[0], &env.mint) {
            let m = l.mark;
            if m.clock_slot == env.clock_slot && m.clock_slot != 0 && m.recipient == args.destination_owner {
                if let Some(i) = env.targets.iter().position(|t| *t == m.rival) {
                    let at = blend(args.destination_balance, dat, args.amount, now);
                    out.destination = Some(write(i as u8 + 1, at));
                }
            }
        }
        return Ok(out);
    }
    if args.destination_owner == p {
        // A sell: a recent origin marks the seller for the pool half.
        let window = i64::from(env.params[1]);
        if so != 0 && env.now - i64::from(sat) < window {
            if let Some(mut l) = load_ledger(&x[0], &env.mint) {
                let bit = 1u8.checked_shl(u32::from(env.slot)).unwrap_or(0);
                // The swap's actor is whoever signed the input transfer: the owner, or a delegate
                // selling the owner's tokens.
                if l.sell_mark.clock_slot != env.clock_slot || l.sell_mark.seller != args.authority {
                    l.sell_mark = hookwars_common::raid::SellMark {
                        clock_slot: env.clock_slot,
                        seller: args.authority,
                        marked_slots: 0,
                    };
                }
                l.sell_mark.marked_slots |= bit;
                save_ledger(&x[0], &l)?;
            }
        }
    } else if so != 0 && args.destination_owner != pda::launch(&env.mint).0 {
        // The origin travels with the tokens, to off-curve holders too.
        let at = blend(args.destination_balance, if dor == 0 { sat } else { dat }, args.amount, sat);
        out.destination = Some(write(so, at));
    }
    if args.source_balance == args.amount && so != 0 {
        out.source = Some(vec![0; LEN]);
    }
    Ok(out)
}

/// Weighted arrival time (upstream Half-Life `blend`).
pub fn blend(balance: u64, since: u32, added: u64, added_since: u32) -> u32 {
    if balance == 0 || since == 0 {
        return added_since;
    }
    let total = u128::from(balance) + u128::from(added);
    ((u128::from(balance) * u128::from(since) + u128::from(added) * u128::from(added_since)) / total) as u32
}
