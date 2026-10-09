// Changed by Hookwars: new file (M3b); expansion templates 43 to 45 (10); arsenal waves B and C.
//! Every template's behaviour (04 section 3, 08 section 4), one file per template. A template is a
//! set of pure functions over an [`Env`] (who it is, its params and targets, its own extras) and
//! the callback's arguments; the engine (`crate::engine`) checks signers and accounts, runs each
//! module, merges answers and records cuts. Templates never make a CPI (R3).
//!
//! Adding a template: a new file here, an arm in [`extras`], [`token_before`], [`pool`] and
//! [`touch`] where it subscribes, and its shape in `hookwars_common`.

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};
use hookwars_common::{ids, pda, template_id as t, Params};

pub mod boss;
pub mod coalition;
pub mod half_life;
pub mod launch_decay;
pub mod max_transaction;
pub mod raid;
pub mod rivalry;
pub mod sell_burn;
pub mod shield;
pub mod side_skew;
pub mod size_tiers;
pub mod spy;
pub mod transfer_fee;
pub mod treaty;
pub mod wall;
// Arsenal waves B and C.
pub mod cooldown;
pub mod daily_sell_cap;
pub mod dump_brake;
pub mod flash_guard;
pub mod guild_tag;
pub mod impact_fee;
pub mod rank_badge;
pub mod rush_hour;
pub mod streak;
pub mod velocity_fee;
pub mod volatility_fee;

/// What a module sees.
pub struct Env<'a, 'info> {
    /// The token.
    pub mint: Pubkey,
    /// The slot.
    pub slot: u8,
    /// The item.
    pub item: Pubkey,
    /// The module's index (0 for a plain item).
    pub module: u8,
    /// The module's params.
    pub params: Params,
    /// The module's targets.
    pub targets: &'a [Pubkey],
    /// The equip's role.
    pub role: u8,
    /// The module's own extras, in [`extras`] order.
    pub extras: &'a [AccountInfo<'info>],
    /// `Clock::unix_timestamp`.
    pub now: i64,
    /// `Clock::slot`.
    pub clock_slot: u64,
}

/// A module's token-side answer: a cut (into the equip vault) and new bytes of its sub-range.
#[derive(Default)]
pub struct TokenOut {
    pub cut: u64,
    pub source: Option<Vec<u8>>,
    pub destination: Option<Vec<u8>>,
}

/// A module's pool-side answer.
#[derive(Default, Clone, Copy)]
pub struct PoolOut {
    pub discount_bps: u16,
    pub cut: u64,
    pub burn: u64,
}

/// Swap directions (`PoolHookArgs.direction`).
pub const SELL: u8 = 0;
/// A buy: quote in, base out.
pub const BUY: u8 = 1;

/// A module's own extras (04 section 3 "Extras", as built: see the M3b notes in 04).
pub fn extras(template: u16, mint: &Pubkey, targets: &[Pubkey]) -> Vec<(Pubkey, bool)> {
    let launch = |m: &Pubkey| (pda::launch(m).0, false);
    let mut v = Vec::new();
    match template {
        t::RAID => {
            v.push((pda::raid_ledger(mint).0, true));
            v.push((pda::war_config().0, false));
            v.push(launch(mint));
            v.extend(targets.iter().map(launch));
        }
        t::SHIELD => {
            v.push((pda::raid_ledger(mint).0, true));
            v.push((pda::war_config().0, false));
            v.push((pda::war_state(mint).0, false));
            v.push(launch(mint));
            v.extend(targets.iter().map(launch));
        }
        t::WALL => v.push((pda::war_state(mint).0, false)),
        t::BOSS => {
            v.push((pda::raid_ledger(mint).0, true));
            v.push((pda::war_config().0, false));
        }
        t::SPY => {
            for r in targets {
                v.push(launch(r));
                v.push((launch_pool_address(r), false));
            }
        }
        t::TREATY | t::TRIBUTE => {
            for p in targets {
                v.push((*p, false));
                for s in 0..bordrless_token::constants::MAX_SLOTS as u8 {
                    v.push((pda::equip_state(p, s).0, false));
                }
            }
        }
        t::HALF_LIFE | t::TRANSFER_FEE | t::LAUNCH_DECAY | t::MAX_TRANSACTION | t::DUST_GUARD => {
            v.push(launch(mint))
        }
        // Arsenal waves B and C: our Launch (buy and sell detection), and our launch pool where
        // the template reads its ring.
        t::COOLDOWN | t::FLASH_GUARD | t::DAILY_SELL_CAP | t::STREAK => v.push(launch(mint)),
        t::VELOCITY_FEE | t::VOLATILITY_FEE | t::DUMP_BRAKE | t::RANK_BADGE => {
            v.push(launch(mint));
            v.push((launch_pool_address(mint), false));
        }
        _ => {}
    }
    v
}

/// How many extras [`extras`] gives a module of `template` with `targets` targets.
pub fn extra_count(template: u16, targets: usize) -> usize {
    match template {
        t::RAID => 3 + targets,
        t::SHIELD => 4 + targets,
        t::WALL => 1,
        t::BOSS => 2,
        t::SPY => 2 * targets,
        t::TREATY | t::TRIBUTE => targets * (1 + bordrless_token::constants::MAX_SLOTS),
        t::HALF_LIFE | t::TRANSFER_FEE | t::LAUNCH_DECAY | t::MAX_TRANSACTION | t::DUST_GUARD => 1,
        t::COOLDOWN | t::FLASH_GUARD | t::DAILY_SELL_CAP | t::STREAK => 1,
        t::VELOCITY_FEE | t::VOLATILITY_FEE | t::DUMP_BRAKE | t::RANK_BADGE => 2,
        _ => 0,
    }
}

/// The launch pool of `mint` as the launchpad opens it (bridged SOL quote, the policy LP fee, the
/// launchpad as hook). Readers check it against the `Launch` account they are also given.
pub fn launch_pool_address(mint: &Pubkey) -> Pubkey {
    bordrless_swap::state::Pool::address(
        mint,
        &ids::BRIDGED_SOL_MINT,
        bordrless_core::policy::LP_FEE_BPS,
        Some(ids::LAUNCH_ID),
    )
    .0
}

/// The launch pool a `Launch` account names, if `info` is `mint`'s launch.
pub fn own_pool(info: &AccountInfo, mint: &Pubkey) -> Option<Pubkey> {
    if *info.owner != ids::LAUNCH_ID || *info.key != pda::launch(mint).0 {
        return None;
    }
    hookwars_common::launch_pool(&info.try_borrow_data().ok()?, mint)
}

/// `["war-chest", mint]` under the war program.
pub fn war_chest(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"war-chest", mint.as_ref()], &ids::WAR_ID).0
}

/// `["treaty-inbox", mint]` under the war program.
pub fn treaty_inbox(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"treaty-inbox", mint.as_ref()], &ids::WAR_ID).0
}

/// Whether `owner` is a protocol account of `mint` that never pays a holder fee: the launch, its
/// pool, the war chest, the treaty inbox, the pool cuts, an equip vault (04 section 3.7). Wallets
/// (on the curve) are never exempt, which also spares the derivations for them.
pub fn exempt(owner: &Pubkey, mint: &Pubkey, pool: Option<Pubkey>) -> bool {
    if Some(*owner) == pool {
        return true;
    }
    if owner.is_on_curve() {
        return false;
    }
    if *owner == pda::launch(mint).0 {
        return true;
    }
    *owner == war_chest(mint)
        || *owner == treaty_inbox(mint)
        || *owner == pda::pool_cuts(mint).0
        || (0..bordrless_token::constants::MAX_SLOTS as u8).any(|s| *owner == pda::equip_state(mint, s).0)
}

/// Whether this callback acts on the quote side: a buy's `pool_before_swap` (the input) or a
/// sell's `pool_after_swap` (the output) (03 section 5.2).
pub fn quote_side(args: &PoolHookArgs, before: bool) -> bool {
    (before && args.direction == BUY) || (!before && args.direction == SELL)
}

/// `bordrless_core::fee_amount`, saturating at the amount.
pub fn fee(amount: u64, bps: u32) -> u64 {
    let bps = u16::try_from(bps).unwrap_or(u16::MAX).min(10_000);
    bordrless_core::fee_amount(amount, bps).unwrap_or(amount).min(amount)
}

/// Token side, `before_transfer`.
pub fn token_before(
    template: u16,
    env: &Env,
    args: &TokenSlotArgs,
    src: &[u8],
    dst: &[u8],
) -> Result<TokenOut> {
    match template {
        t::RAID => raid::token(env, args, src, dst),
        t::SHIELD => shield::token(env, args, src, dst),
        t::WALL => wall::token(env, args),
        t::HALF_LIFE => half_life::token(env, args, src, dst),
        t::TRANSFER_FEE => transfer_fee::token(env, args),
        t::MAX_TRANSACTION => max_transaction::max_tx(env, args),
        t::DUST_GUARD => max_transaction::dust(env, args),
        t::COOLDOWN => cooldown::token(env, args, src, dst),
        t::FLASH_GUARD => flash_guard::token(env, args, src, dst),
        t::DAILY_SELL_CAP => daily_sell_cap::token(env, args, src, dst),
        t::STREAK => streak::token(env, args, src, dst),
        t::RANK_BADGE => rank_badge::token(env, args, src, dst),
        t::GUILD_TAG => guild_tag::token(env, args, src, dst),
        _ => Ok(TokenOut::default()),
    }
}

/// Token side, `on_touch`: new bytes of the module's sub-range, if any.
pub fn touch(template: u16, env: &Env, args: &TokenSlotArgs, src: &[u8]) -> Result<Option<Vec<u8>>> {
    match template {
        t::RAID => raid::touch(env, args, src).map(Some),
        t::GUILD_TAG => guild_tag::touch(env, args, src).map(Some),
        _ => Ok(None),
    }
}

/// Pool side.
pub fn pool(
    template: u16,
    env: &Env,
    args: &PoolHookArgs,
    ctx: &ItemPoolContext,
    before: bool,
) -> Result<PoolOut> {
    match template {
        t::RAID => raid::pool(env, args, ctx, before),
        t::SHIELD => shield::pool(env, args, ctx, before),
        t::SPY => spy::pool(env, args, ctx, before),
        t::TREATY | t::TRIBUTE => treaty::pool(template, env, args, ctx, before),
        t::SIZE_TIERS => size_tiers::pool(env, args, ctx, before),
        t::SIDE_SKEW => side_skew::pool(env, args, ctx, before),
        t::LAUNCH_DECAY => launch_decay::pool(env, args, ctx, before),
        t::SELL_BURN => sell_burn::pool(env, args, ctx, before),
        t::BOSS => boss::pool(env, args, ctx, before),
        t::VELOCITY_FEE => velocity_fee::pool(env, args, ctx, before),
        t::IMPACT_FEE => impact_fee::pool(env, args, ctx, before),
        t::VOLATILITY_FEE => volatility_fee::pool(env, args, ctx, before),
        t::RUSH_HOUR => rush_hour::pool(env, args, ctx, before),
        t::DUMP_BRAKE => dump_brake::pool(env, args, ctx, before),
        _ => Ok(PoolOut::default()),
    }
}

/// Where a module's collected cuts go at settlement (04 section 2.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Destination {
    /// Nothing is ever collected on this side.
    None,
    /// Burned (token side).
    Burn,
    /// The holding of this owner (of the side's mint).
    Owner(Pubkey),
}

/// A module's token-side destination.
pub fn token_destination(template: u16, targets: &[Pubkey]) -> Destination {
    match template {
        t::HALF_LIFE => Destination::Burn,
        t::TRANSFER_FEE => targets.first().map(|c| Destination::Owner(*c)).unwrap_or(Destination::None),
        _ => Destination::None,
    }
}

/// A module's pool-side destination.
pub fn pool_destination(template: u16, mint: &Pubkey, targets: &[Pubkey]) -> Destination {
    match template {
        t::RAID | t::SHIELD | t::SPY | t::SIZE_TIERS | t::SIDE_SKEW | t::LAUNCH_DECAY => {
            Destination::Owner(war_chest(mint))
        }
        // Arsenal wave C: the war chest, as wave A's fee templates.
        t::VELOCITY_FEE | t::IMPACT_FEE | t::VOLATILITY_FEE | t::RUSH_HOUR | t::DUMP_BRAKE => {
            Destination::Owner(war_chest(mint))
        }
        t::TREATY | t::TRIBUTE => targets
            .first()
            .map(|p| Destination::Owner(treaty_inbox(p)))
            .unwrap_or(Destination::None),
        _ => Destination::None,
    }
}
