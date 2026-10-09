// Changed by Hookwars: new file (M3b); security review 2 M-B: the season volume is net of raid
// points that leave holders (a raider selling back), counted in whole point units.
//! Raid (id 1, 04 section 3.1). Extras: `RaidLedger` (w), `WarConfig`, our `Launch`, each
//! target's `Launch`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};
use hookwars_common::raid::{RaidLedger, RaidRange, WarTouch};
use hookwars_common::{ids, pda};

use super::{fee, own_pool, Env, PoolOut, TokenOut, BUY};
use crate::{ItemsError, RaidMarked};

/// What the war configuration says (defaults when the war program is not set up).
#[derive(Clone, Copy, Debug, Default)]
pub struct WarView {
    pub season: u32,
    pub raid_window_secs: i64,
    pub point_unit_lamports: u64,
    pub loot_min_raid_lamports: u64,
}

/// Reads `WarConfig` (owned by the war program); defaults when absent.
pub fn war_view(info: &AccountInfo) -> WarView {
    if *info.owner != ids::WAR_ID || *info.key != pda::war_config().0 {
        return WarView {
            raid_window_secs: 1,
            ..Default::default()
        };
    }
    let Ok(data) = info.try_borrow_data() else {
        return WarView::default();
    };
    match hookwars_war::state::WarConfig::try_deserialize(&mut &data[..]) {
        Ok(c) => WarView {
            season: c.current_season,
            raid_window_secs: c.params.raid_window_secs.max(1),
            point_unit_lamports: c.params.point_unit_lamports,
            loot_min_raid_lamports: c.params.loot_min_raid_lamports,
        },
        Err(_) => WarView {
            raid_window_secs: 1,
            ..Default::default()
        },
    }
}

/// Loads `mint`'s ledger from `info`, if it is the ledger and exists.
pub fn load_ledger(info: &AccountInfo, mint: &Pubkey) -> Option<RaidLedger> {
    if *info.owner != crate::ID || *info.key != pda::raid_ledger(mint).0 {
        return None;
    }
    RaidLedger::decode(&info.try_borrow_data().ok()?)
}

/// Writes the ledger back.
pub fn save_ledger(info: &AccountInfo, l: &RaidLedger) -> Result<()> {
    require!(info.is_writable, ItemsError::WrongAccount);
    let mut data = info.try_borrow_mut_data()?;
    l.encode(&mut data).ok_or(ItemsError::WrongAccount)?;
    Ok(())
}

/// The raid test (04 section 3.1): a buy whose route sold one of `targets` on its own launch
/// pool first. `launches` are the targets' `Launch` accounts, in target order.
pub fn raid_target(env: &Env, args: &PoolHookArgs, launches: &[AccountInfo]) -> Option<usize> {
    let r = &args.route;
    if args.direction != BUY
        || r.hop_index < 1
        || r.route_input_mint == env.mint
        || r.route_input_mint == ids::BRIDGED_SOL_MINT
    {
        return None;
    }
    let i = env.targets.iter().position(|x| *x == r.route_input_mint)?;
    let pool = own_pool(launches.get(i)?, &r.route_input_mint)?;
    (pool == r.first_pool).then_some(i)
}

/// The part of a raid buy the season volume counts: whole point units, so that what a raider
/// later sells back can be taken out exactly through the points it carries (M-B).
pub fn counted_volume(amount_in: u64, war: WarView) -> u64 {
    match war.point_unit_lamports {
        0 => amount_in,
        u => amount_in / u * u,
    }
}

/// The season volume behind `points` raid points (the inverse of the stamp, rounding down).
pub fn volume_of_points(points: u64, war: WarView, points_per_unit: u32) -> u64 {
    if points_per_unit == 0 {
        return 0;
    }
    (u128::from(points) * u128::from(war.point_unit_lamports) / u128::from(points_per_unit)).min(u128::from(u64::MAX)) as u64
}

/// Writes the mark (R4) and rolls the inbound window, when `count_inbound`.
pub fn write_mark(env: &Env, ledger_info: &AccountInfo, war: WarView, args: &PoolHookArgs, rival: Pubkey, count_inbound: bool) -> Result<()> {
    let Some(mut l) = load_ledger(ledger_info, &env.mint) else {
        return Ok(());
    };
    l.roll_season(war.season);
    if count_inbound {
        l.add_inbound(&rival, env.now, war.raid_window_secs, args.amount_in);
        l.outbound_volume_season = l.outbound_volume_season.saturating_add(counted_volume(args.amount_in, war));
    }
    let fresh = l.mark.clock_slot != env.clock_slot || l.mark.recipient != args.recipient;
    if fresh || l.mark.quote_volume != args.amount_in {
        l.mark = hookwars_common::raid::Mark {
            clock_slot: env.clock_slot,
            recipient: args.recipient,
            rival,
            quote_volume: args.amount_in,
            stamped_slots: 0,
        };
    }
    save_ledger(ledger_info, &l)
}

/// Pool callbacks.
pub fn pool(env: &Env, args: &PoolHookArgs, ctx: &ItemPoolContext, before: bool) -> Result<PoolOut> {
    let x = env.extras;
    require!(x.len() >= 3 + env.targets.len(), ItemsError::WrongAccount);
    let Some(i) = raid_target(env, args, &x[3..]) else {
        return Ok(PoolOut::default());
    };
    if before {
        return Ok(PoolOut {
            discount_bps: env.params[0].min(10_000) as u16,
            cut: fee(ctx.side_amount, env.params[1]),
            burn: 0,
        });
    }
    let war = war_view(&x[1]);
    write_mark(env, &x[0], war, args, env.targets[i], true)?;
    Ok(PoolOut::default())
}

/// `before_transfer` (the token half).
pub fn token(env: &Env, args: &TokenSlotArgs, src: &[u8], dst: &[u8]) -> Result<TokenOut> {
    if args.source == args.destination {
        return Ok(TokenOut::default());
    }
    let x = env.extras;
    require!(x.len() >= 3, ItemsError::WrongAccount);
    let war = war_view(&x[1]);
    let pool = own_pool(&x[2], &env.mint);
    let launch = pda::launch(&env.mint).0;
    let mut s = RaidRange::read(src, war.season);
    let mut d = RaidRange::read(dst, war.season);
    let (s0, d0) = (s, d);
    let holder = |o: &Pubkey| Some(*o) != pool && *o != launch && o.is_on_curve();

    let mut stamped = false;
    let mut stamped_pts = 0u32;
    if let (Some(mut l), Some(p)) = (load_ledger(&x[0], &env.mint), pool) {
        let bit = 1u8.checked_shl(u32::from(env.slot)).unwrap_or(0);
        if l.mark.clock_slot == env.clock_slot
            && l.mark.clock_slot != 0
            && args.destination_owner == l.mark.recipient
            && args.source_owner == p
            && l.mark.stamped_slots & bit == 0
        {
            let units = l.mark.quote_volume.checked_div(war.point_unit_lamports).unwrap_or(0);
            let pts = u32::try_from(units.saturating_mul(u64::from(env.params[2]))).unwrap_or(u32::MAX);
            let tik = u16::from(l.mark.quote_volume >= war.loot_min_raid_lamports && war.season > 0);
            d.raid_points = d.raid_points.saturating_add(pts);
            d.tickets = d.tickets.saturating_add(tik);
            l.mark.stamped_slots |= bit;
            stamped_pts = pts;
            save_ledger(&x[0], &l)?;
            emit!(RaidMarked {
                mint: env.mint,
                rival: l.mark.rival,
                trader: l.mark.recipient,
                volume: l.mark.quote_volume,
                points: pts,
                loot_ticket: tik == 1,
            });
            stamped = true;
        }
    }
    if !stamped && args.source_balance > 0 {
        let share = |v: u64| -> u64 {
            (u128::from(v) * u128::from(args.amount) / u128::from(args.source_balance)) as u64
        };
        let moved_pts = share(u64::from(s.raid_points)) as u32;
        let moved_tik = share(u64::from(s.tickets)) as u16;
        if holder(&args.source_owner) && holder(&args.destination_owner) {
            s.raid_points -= moved_pts;
            s.tickets -= moved_tik;
            d.raid_points = d.raid_points.saturating_add(moved_pts);
            d.tickets = d.tickets.saturating_add(moved_tik);
        } else if Some(args.destination_owner) == pool {
            s.raid_points -= moved_pts;
            s.tickets -= moved_tik;
        }
    }
    if args.source_balance == args.amount && holder(&args.source_owner) {
        s = RaidRange::default();
    }
    // M-B: points that left holders (sold back to the pool, or dropped) take their volume out of
    // the season volume, so a raid bought and sold back nets to zero.
    let before = u64::from(s0.raid_points) + u64::from(d0.raid_points) + u64::from(stamped_pts);
    let after = u64::from(s.raid_points) + u64::from(d.raid_points);
    if before > after {
        if let Some(mut l) = load_ledger(&x[0], &env.mint) {
            l.roll_season(war.season);
            let v = volume_of_points(before - after, war, env.params[2]);
            l.outbound_volume_season = l.outbound_volume_season.saturating_sub(v);
            save_ledger(&x[0], &l)?;
        }
    }
    Ok(TokenOut {
        cut: 0,
        source: (s != s0).then(|| s.write()),
        destination: (d != d0).then(|| d.write()),
    })
}

/// `on_touch`: war payloads, from the war program's signer only (04 section 2.10).
pub fn touch(env: &Env, args: &TokenSlotArgs, src: &[u8]) -> Result<Vec<u8>> {
    require_keys_eq!(args.authority, crate::WAR_SIGNER, ItemsError::NotWarSigner);
    let war = war_view(&env.extras[1]);
    let payload = WarTouch::deserialize(&mut args.payload.as_slice())
        .map_err(|_| error!(ItemsError::BadParams))?;
    let mut r = RaidRange::read(src, war.season);
    match payload {
        WarTouch::SpendRaidPoints { amount } => {
            r.raid_points = r.raid_points.checked_sub(amount).ok_or(ItemsError::NotEnoughPoints)?;
        }
        WarTouch::SpendTicket => {
            r.tickets = r.tickets.checked_sub(1).ok_or(ItemsError::NoTicket)?;
        }
        WarTouch::AddTicket { amount } => r.tickets = r.tickets.saturating_add(amount),
    }
    Ok(r.write())
}
