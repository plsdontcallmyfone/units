// Changed by Hookwars: new file (M3b); expansion templates 43 to 45 (10); template 42 Soulbound dispatch (09); arsenal waves B and C; arsenal waves D and E.
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
use hookwars_common::{arsenal2 as a2, ids, pda, template_id as t, Params};

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
pub mod soulbound;
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
// Arsenal waves D and E (08 section 4).
pub mod ally_pass;
pub mod embargo;
pub mod first_blood;
pub mod garrison;
pub mod gift_ember;
pub mod guest_list;
pub mod holder_stream;
pub mod holdings;
pub mod loyalty_pot;
pub mod mercenary;
pub mod patience;
pub mod referral;
pub mod sell_ladder;
pub mod target_burn;
pub mod war_levy;

pub use holdings::held;

/// The registry entry a client resolves per trade (R22): the buyer's holding of another mint, or
/// the buyer's `Referred` account. `init_equip` writes fixed keys only (see the arsenal 2
/// integration requests in 08), so this key stands in for the derived account.
pub const DERIVED: Pubkey = Pubkey::new_from_array([0xD5; 32]);

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
        t::SOULBOUND => v.push((*mint, false)),
        t::SPY => {
            for r in targets {
                v.push(launch(r));
                v.push((launch_pool_address(r), false));
            }
            // Changed by Hookwars (security review 2 L-A): the armory config, for `min_twap_secs`.
            v.push((pda::config().0, false));
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
        // Arsenal waves D and E.
        a2::GUEST_LIST => {
            v.push(launch(mint));
            v.push((DERIVED, false));
        }
        a2::ALLY_PASS => v.push((DERIVED, false)),
        a2::REFERRAL => v.push((DERIVED, true)),
        a2::GARRISON => v.push((pda::war_state(mint).0, false)),
        a2::WAR_LEVY => {
            v.push((pda::raid_ledger(mint).0, false));
            v.push((pda::war_config().0, false));
        }
        a2::TARGET_BURN => {
            v.push(launch(mint));
            v.push((*mint, false));
        }
        a2::GIFT_EMBER | a2::SELL_LADDER | a2::LOYALTY_POT => v.push(launch(mint)),
        a2::MERCENARY => {
            v.push((pda::raid_ledger(mint).0, true));
            v.push((pda::war_config().0, false));
            v.push(launch(mint));
        }
        a2::PATIENCE => {
            v.push((pda::raid_ledger(mint).0, true));
            v.push(launch(mint));
        }
        a2::FIRST_BLOOD => v.push((a2::pda::first_blood(mint).0, true)),
        _ => {}
    }
    v
}

/// [`extras`] as registry entries (R22): the [`DERIVED`] placeholders become the PDAs a client
/// derives per trade from the pool prefix's actor (account 4). For `init_equip` to adopt (the
/// arsenal 2 integration requests in 08); [`extras`] keeps its fixed-key form until then.
pub fn extra_sources(template: u16, mint: &Pubkey, targets: &[Pubkey]) -> Vec<bordrless_hook::ExtraAccount> {
    use bordrless_hook::{AccountSource, ExtraAccount, Seed};
    const ACTOR: u8 = 4;
    let derived = |n: usize| -> ExtraAccount {
        match template {
            a2::GUEST_LIST | a2::ALLY_PASS => ExtraAccount {
                writable: false,
                source: AccountSource::Pda {
                    program: bordrless_token::ID,
                    seeds: vec![
                        Seed::Literal(b"holding".to_vec()),
                        Seed::Literal(targets.get(n).copied().unwrap_or_default().to_bytes().to_vec()),
                        Seed::Account(ACTOR),
                    ],
                },
            },
            _ => ExtraAccount {
                writable: true,
                source: AccountSource::Pda {
                    program: crate::ID,
                    seeds: vec![
                        Seed::Literal(a2::seeds::REFERRED.to_vec()),
                        Seed::Literal(mint.to_bytes().to_vec()),
                        Seed::Account(ACTOR),
                    ],
                },
            },
        }
    };
    let mut n = 0usize;
    extras(template, mint, targets)
        .into_iter()
        .map(|(k, writable)| {
            if k == DERIVED {
                n += 1;
                derived(n - 1)
            } else {
                ExtraAccount {
                    writable,
                    source: AccountSource::Key(k),
                }
            }
        })
        .collect()
}

/// How many extras [`extras`] gives a module of `template` with `targets` targets.
pub fn extra_count(template: u16, targets: usize) -> usize {
    match template {
        t::RAID => 3 + targets,
        t::SHIELD => 4 + targets,
        t::WALL => 1,
        t::BOSS => 2,
        t::SOULBOUND => 1,
        t::SPY => 2 * targets + 1,
        t::TREATY | t::TRIBUTE => targets * (1 + bordrless_token::constants::MAX_SLOTS),
        t::HALF_LIFE | t::TRANSFER_FEE | t::LAUNCH_DECAY | t::MAX_TRANSACTION | t::DUST_GUARD => 1,
        t::COOLDOWN | t::FLASH_GUARD | t::DAILY_SELL_CAP | t::STREAK => 1,
        t::VELOCITY_FEE | t::VOLATILITY_FEE | t::DUMP_BRAKE | t::RANK_BADGE => 2,
        // Arsenal waves D and E.
        a2::GUEST_LIST | a2::WAR_LEVY | a2::TARGET_BURN | a2::PATIENCE => 2,
        a2::MERCENARY => 3,
        a2::ALLY_PASS
        | a2::REFERRAL
        | a2::GARRISON
        | a2::GIFT_EMBER
        | a2::SELL_LADDER
        | a2::LOYALTY_POT
        | a2::FIRST_BLOOD => 1,
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
        t::SOULBOUND => soulbound::token(env, args),
        t::COOLDOWN => cooldown::token(env, args, src, dst),
        t::FLASH_GUARD => flash_guard::token(env, args, src, dst),
        t::DAILY_SELL_CAP => daily_sell_cap::token(env, args, src, dst),
        t::STREAK => streak::token(env, args, src, dst),
        t::RANK_BADGE => rank_badge::token(env, args, src, dst),
        t::GUILD_TAG => guild_tag::token(env, args, src, dst),
        // Arsenal waves D and E.
        a2::GIFT_EMBER => gift_ember::token(env, args),
        a2::SELL_LADDER => sell_ladder::token(env, args),
        a2::LOYALTY_POT => loyalty_pot::token(env, args, src, dst),
        a2::MERCENARY => mercenary::token(env, args, src, dst),
        a2::PATIENCE => patience::token(env, args, src, dst),
        _ => Ok(TokenOut::default()),
    }
}

/// Token side, `on_touch`: new bytes of the module's sub-range, if any.
pub fn touch(template: u16, env: &Env, args: &TokenSlotArgs, src: &[u8]) -> Result<Option<Vec<u8>>> {
    match template {
        t::RAID => raid::touch(env, args, src).map(Some),
        t::GUILD_TAG => guild_tag::touch(env, args, src).map(Some),
        a2::MERCENARY => mercenary::touch(env, args, src).map(Some),
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
        // Arsenal waves D and E.
        a2::GUEST_LIST => guest_list::pool(env, args, ctx, before),
        a2::ALLY_PASS => ally_pass::pool(env, args, ctx, before),
        a2::EMBARGO => embargo::pool(env, args, ctx, before),
        a2::HOLDER_STREAM => holder_stream::pool(env, args, ctx, before),
        a2::GARRISON => garrison::pool(env, args, ctx, before),
        a2::WAR_LEVY => war_levy::pool(env, args, ctx, before),
        a2::TARGET_BURN => target_burn::pool(env, args, ctx, before),
        a2::MERCENARY => mercenary::pool(env, args, ctx, before),
        a2::PATIENCE => patience::pool(env, args, ctx, before),
        a2::LOYALTY_POT => loyalty_pot::pool(env, args, ctx, before),
        a2::REFERRAL => referral::pool(env, args, ctx, before),
        a2::FIRST_BLOOD => first_blood::pool(env, args, ctx, before),
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
        a2::GIFT_EMBER | a2::SELL_LADDER => Destination::Burn,
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
        // Arsenal waves D and E.
        a2::EMBARGO | a2::WAR_LEVY => Destination::Owner(war_chest(mint)),
        a2::HOLDER_STREAM => Destination::Owner(treaty_inbox(mint)),
        a2::LOYALTY_POT => Destination::Owner(a2::pda::loyalty(mint).0),
        a2::REFERRAL => Destination::Owner(a2::pda::referral_owner(mint).0),
        _ => Destination::None,
    }
}
