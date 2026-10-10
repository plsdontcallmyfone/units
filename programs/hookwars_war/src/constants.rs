// Changed by Hookwars: pass 4b: boss, coalition seeds and layouts, template ids war reads.
//! Seeds, layout constants and the programs the war program calls (and nothing else).
//!
//! Every number that is a policy (spends, intervals, windows, bounties, season lengths) is a field
//! of [`crate::state::WarParams`] in the config, changed only through the timelock (05 section 11).
//! The constants here are seeds, layouts and the upstream limits this program reuses by name.

use anchor_lang::prelude::Pubkey;

/// `["war-config"]`: the program's one config.
pub const WAR_CONFIG_SEED: &[u8] = b"war-config";
/// `["war", mint]`: a token's war state.
pub const WAR_SEED: &[u8] = b"war";
/// `["war-chest", mint]`: a token's war chest, a system-owned address only this program signs for.
pub const CHEST_SEED: &[u8] = b"war-chest";
/// `["treaty-inbox", mint]`: what partners' treaties pay a token, streamed to its holders.
pub const INBOX_SEED: &[u8] = b"treaty-inbox";
/// `["prize-vault"]`: the DEX's fee collector; split between the season's winner and the treasury.
pub const PRIZE_VAULT_SEED: &[u8] = b"prize-vault";
/// `["war-signer"]`: the authority of every `touch` carrying a war payload (04 2.10).
pub const WAR_SIGNER_SEED: &[u8] = b"war-signer";
/// `["loot-signer"]`: the only signer the armory's `mint_loot` accepts (02 4.3).
pub const LOOT_SIGNER_SEED: &[u8] = b"loot-signer";
/// `["season", number: u32 le]`.
pub const SEASON_SEED: &[u8] = b"season";
/// `["loot", season: u32 le]`.
pub const LOOT_SEED: &[u8] = b"loot";
/// `["roll", holding, nonce: u64 le]`.
pub const ROLL_SEED: &[u8] = b"roll";
/// `["quest", season: u32 le, mint, owner]`.
pub const QUEST_SEED: &[u8] = b"quest";

pub const VERSION: u8 = 1;
pub const BPS: u64 = 10_000;

/// Layout: captured rival holdings a chest tracks at once (`MAX_CAPTURED`, 00 section 6, set by
/// measurement; provisional until 07 records the measured account size and rent).
pub const MAX_CAPTURED: usize = 8;
/// Pass 4b: `["boss", season]`, `["coalition", id]`, `["coalition-chest", id]`.
pub const BOSS_SEED: &[u8] = b"boss";
pub const COALITION_SEED: &[u8] = b"coalition";
pub const COALITION_CHEST_SEED: &[u8] = b"coalition-chest";
/// Layout (10 section 8): `COALITION_MIN` and `COALITION_MAX` members.
pub const COALITION_MIN_MEMBERS: usize = 2;
pub const COALITION_MAX_MEMBERS: usize = 5;
/// Layout: rival holdings a coalition chest tracks at once.
pub const COALITION_CAPTURED: usize = 2;
/// Armory template ids war reads (10 sections 8, 11.3).
pub const COALITION_TEMPLATE: u16 = hookwars_common::template_id::COALITION;
pub const RIVALRY_TEMPLATE: u16 = hookwars_common::template_id::RIVALRY;

/// Layout: entries of a season's loot table (`LOOT_TABLE_LEN`, provisional as above).
pub const LOOT_TABLE_LEN: usize = 8;

/// Quest ids (05 section 9).
pub mod quest {
    /// Spend `quest_raid_points` raid points of the current season.
    pub const RAID: u8 = 1;
    /// Forge an item since the last `Forge` claim.
    pub const FORGE: u8 = 2;
}

/// The War orders template's fields, in 04's order (04 section 3.9).
pub mod orders {
    pub const SIEGE_THRESHOLD: usize = 0;
    pub const SIEGE_SPEND_BPS: usize = 1;
    pub const SIEGE_TWAP_SECS: usize = 2;
    pub const COUNTER_DROP_BPS: usize = 3;
    pub const COUNTER_SHORT_SECS: usize = 4;
    pub const COUNTER_LONG_SECS: usize = 5;
    pub const COUNTER_INTERVAL_SECS: usize = 6;
    pub const COUNTER_SPEND_BPS: usize = 7;
    pub const RAZE_ENABLED: usize = 8;
    pub const BOUNTY_RATE: usize = 9;
    pub const CRANK_BOUNTY_BPS: usize = 10;
}

/// The Treaty template's `returns_captured` field (04 section 3.5).
pub const TREATY_RETURNS_CAPTURED: usize = 2;

/// The programs the war program invokes or reads.
pub const TOKEN_ID: Pubkey = bordrless_token::ID;
pub const SWAP_ID: Pubkey = bordrless_swap::ID;
pub const LAUNCH_ID: Pubkey = bordrless_launch::ID;
pub const KIT_ID: Pubkey = bordrless_kit::ID;
pub const BRIDGE_ID: Pubkey = bordrless_bridge::ID;
pub const ITEMS_ID: Pubkey = bordrless_token::constants::ITEMS_ID;
pub const ARMORY_ID: Pubkey = bordrless_token::constants::ARMORY_ID;
/// Bridged SOL, every launch's quote and every chest's balance.
pub const BRIDGED_SOL_MINT: Pubkey = bordrless_swap::constants::BRIDGED_SOL_MINT;
/// The upgradeable loader (the config is created by this program's upgrade authority).
pub const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

/// Reused upstream limits (companion, 05 section 6): no counter-strike in a launch's first minute,
/// and a chest buy takes no less than the pool's own quote less the fees and this.
pub const BUYBACK_AFTER_LAUNCH: i64 = bordrless_companion::constants::BUYBACK_AFTER_LAUNCH;
pub const BUYBACK_SLIPPAGE_BPS: u64 = bordrless_companion::constants::BUYBACK_SLIPPAGE_BPS;
/// The highest crank bounty any war step may pay: the companion's (`MAX_CRANK_BOUNTY_BPS` in the
/// config may be set lower, never higher).
pub const BOUNTY_CEILING_BPS: u16 = bordrless_companion::constants::MAX_BOUNTY_BPS;
/// The kit's smallest share (a treaty inflow below it waits).
pub const MIN_SHARE_LAMPORTS: u64 = bordrless_kit::constants::MIN_SHARE_LAMPORTS;
