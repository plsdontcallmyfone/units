// Changed by Hookwars: new file (M4/M5); M3b: POINT_UNIT_LAMPORTS and LOOT_MIN_RAID_LAMPORTS in WarParams.
//! Accounts of the war program (05 section 2).

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::foreign::PARAM_FIELDS;

/// Every policy number of the war program (00 section 6, the parameters 05 uses). Set at
/// `init_config` and changed only through the timelock (`propose_config`, `apply_config`).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarParams {
    /// `ADMIN_TIMELOCK_SECS`: delay on every admin change (D-9).
    pub admin_timelock_secs: i64,
    /// `MAX_CRANK_BOUNTY_BPS`: ceiling on any war step's crank bounty (at most the companion's).
    pub max_crank_bounty_bps: u16,
    /// `SIEGE_INTERVAL_SECS`: spacing of a chest's sieges; also how long a besieged token is
    /// "under siege".
    pub siege_interval_secs: i64,
    /// `SIEGE_MAX_SPEND_BPS`: ceiling on a siege's spend, of the chest.
    pub siege_max_spend_bps: u16,
    /// `SIEGE_MAX_PREMIUM_BPS`: a siege waits while the rival trades above its TWAP by more.
    pub siege_max_premium_bps: u16,
    /// `SIEGE_SLIPPAGE_BPS`: a siege or raze takes no less than the quote less the fees and this.
    pub siege_slippage_bps: u16,
    /// `SIEGE_UNIT_LAMPORTS`: the unit of the War orders' `siege_threshold`.
    pub siege_unit_lamports: u64,
    /// `COUNTER_STRIKE_MAX_SPEND_BPS`: ceiling on a counter-strike's spend, of the chest.
    pub counter_max_spend_bps: u16,
    /// `COUNTER_STRIKE_MIN_INTERVAL_SECS`: floor on the War orders' counter-strike spacing.
    pub counter_min_interval_secs: i64,
    /// `RAZE_MAX_BPS_PER_INTERVAL`, `RAZE_INTERVAL_SECS`: how fast captured holdings may be sold.
    pub raze_max_bps_per_interval: u16,
    pub raze_interval_secs: i64,
    /// `BOUNTY_MAX_PER_CLAIM` (lamports).
    pub bounty_max_per_claim: u64,
    /// `MIN_TWAP_SECS`: the shortest window any price read may use.
    pub min_twap_secs: i64,
    /// `RAID_WINDOW_SECS`: the raid ledger's window (the items program writes with the same).
    pub raid_window_secs: i64,
    /// `ROLL_EXPIRY_SECS`: an unfulfilled roll may be cancelled after this.
    pub roll_expiry_secs: i64,
    /// `QUEST_PERIOD_SECS`, `QUEST_RAID_POINTS`.
    pub quest_period_secs: i64,
    pub quest_raid_points: u32,
    /// `SEASON_SECS`, `CHALLENGE_SECS`.
    pub season_secs: i64,
    pub challenge_secs: i64,
    /// `SEASON_PRIZE_SHARE_BPS`: of the prize vault, to the last winner's chest.
    pub season_prize_share_bps: u16,
    /// `POINT_UNIT_LAMPORTS` (M3b): raid quote volume per raid point unit (the Raid item reads it).
    pub point_unit_lamports: u64,
    /// `LOOT_MIN_RAID_LAMPORTS` (M3b): the smallest raid buy that earns a loot ticket.
    pub loot_min_raid_lamports: u64,
}

impl WarParams {
    /// Every bps at most 10,000, the crank bounty at most the companion's, every duration
    /// positive, the units positive.
    pub fn valid(&self) -> bool {
        let bps = [
            self.siege_max_spend_bps,
            self.siege_max_premium_bps,
            self.siege_slippage_bps,
            self.counter_max_spend_bps,
            self.raze_max_bps_per_interval,
            self.season_prize_share_bps,
        ];
        let secs = [
            self.admin_timelock_secs,
            self.siege_interval_secs,
            self.counter_min_interval_secs,
            self.raze_interval_secs,
            self.min_twap_secs,
            self.raid_window_secs,
            self.roll_expiry_secs,
            self.quest_period_secs,
            self.season_secs,
            self.challenge_secs,
        ];
        bps.iter().all(|b| u64::from(*b) <= BPS)
            && self.siege_slippage_bps < 10_000
            && self.max_crank_bounty_bps <= BOUNTY_CEILING_BPS
            && secs.iter().all(|s| *s > 0)
            && self.siege_unit_lamports > 0
    }
}

/// A proposed change of the config, applied by anyone after `eta` (05 section 11).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingConfig {
    pub admin: Pubkey,
    pub protocol_treasury: Pubkey,
    pub randomness_program: Pubkey,
    pub treaty_template_id: Option<u16>,
    pub params: WarParams,
    pub eta: i64,
}

/// `WarConfig` at `["war-config"]` (05 section 2.1).
#[account]
#[derive(InitSpace, Debug)]
pub struct WarConfig {
    pub version: u8,
    pub bump: u8,
    pub prize_vault_bump: u8,
    pub admin: Pubkey,
    /// Receives protocol fees after the season prize share.
    pub protocol_treasury: Pubkey,
    /// The randomness adapter (D-4; section 8.2).
    pub randomness_program: Pubkey,
    /// The armory template id of Treaty items (`return_captured`, `accrue_treaty_time`); `None`
    /// until the admin names it. War orders are recognised by their kind, `War` (one template).
    pub treaty_template_id: Option<u16>,
    /// Season now running (0 before the first).
    pub current_season: u32,
    /// Mint that won the last finalized season, and that season.
    pub last_winner: Option<Pubkey>,
    pub last_winner_season: u32,
    pub params: WarParams,
    pub pending: Option<PendingConfig>,
    pub reserved: [u8; 64],
}

impl WarConfig {
    pub const LEN: usize = 8 + Self::INIT_SPACE;

    pub fn address() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[WAR_CONFIG_SEED], &crate::ID)
    }
}

/// A rival holding a chest captured.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Captured {
    pub rival_mint: Pubkey,
    pub amount: u64,
    /// Bridged SOL spent on it.
    pub cost: u64,
    pub captured_at: i64,
    pub raze_window_start: i64,
    pub raze_window_base: u64,
    pub razed_in_window: u64,
}

impl Captured {
    pub fn is_free(&self) -> bool {
        self.rival_mint == Pubkey::default()
    }
}

/// A season's counters of one token: values only, never marked to market (05 section 2.4).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeasonCounters {
    pub raid_volume_won: u64,
    pub sieges: u32,
    pub siege_spend: u64,
    pub times_besieged: u32,
    pub counter_strikes: u32,
    pub treaty_secs: u64,
}

/// `WarState` at `["war", mint]`: war-side facts only (05 section 2.4).
#[account]
#[derive(InitSpace, Debug)]
pub struct WarState {
    pub version: u8,
    pub bump: u8,
    pub chest_bump: u8,
    pub inbox_bump: u8,
    pub mint: Pubkey,
    pub launch: Pubkey,
    /// The chest's bridged-SOL balance after the last write.
    pub last_seen_balance: u64,
    pub funded_total: u64,
    pub spent_siege: u64,
    pub spent_counter: u64,
    pub paid_bounties: u64,
    pub paid_cranks: u64,
    pub razed_proceeds: u64,
    pub treaty_shared_total: u64,
    pub last_siege_at: i64,
    pub last_counter_at: i64,
    pub last_treaty_tick: i64,
    /// Set when another token's chest besieges this one; read by Defense items (04).
    pub under_siege_until: i64,
    /// The besieging chest while `under_siege_until > now`.
    pub siege_by_chest: Pubkey,
    pub captured: [Captured; MAX_CAPTURED],
    /// Season the `season` counters belong to.
    pub season_id: u32,
    pub season: SeasonCounters,
    /// The frozen previous season (10.1).
    pub prev_season: SeasonCounters,
    pub reserved: [u8; 64],
}

impl WarState {
    pub const LEN: usize = 8 + Self::INIT_SPACE;

    pub fn address(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[WAR_SEED, mint.as_ref()], &crate::ID)
    }

    /// Rolls the counters into season `current` (05 section 10.1).
    pub fn roll(&mut self, current: u32) {
        if self.season_id >= current {
            return;
        }
        self.prev_season = if self.season_id + 1 == current {
            self.season
        } else {
            SeasonCounters::default()
        };
        self.season = SeasonCounters::default();
        self.season_id = current;
    }

    /// The counters of season `n` as this state holds them (zero when it holds neither).
    pub fn counters_of(&self, n: u32) -> SeasonCounters {
        if self.season_id == n {
            self.season
        } else if self.season_id == n + 1 {
            self.prev_season
        } else {
            SeasonCounters::default()
        }
    }

    /// The captured entry of `rival`, if any.
    pub fn captured_of(&mut self, rival: &Pubkey) -> Option<&mut Captured> {
        self.captured.iter_mut().find(|c| c.rival_mint == *rival)
    }

    /// What the chest's bridged-SOL balance must be by the totals (chest solvency, 07 invariant
    /// 5): funding and raze proceeds in, sieges, counter-strikes, bounties and chest-paid cranks
    /// out. `None` if the totals ever spent more than came in.
    pub fn expected_balance(&self) -> Option<u64> {
        self.funded_total
            .checked_add(self.razed_proceeds)?
            .checked_sub(self.spent_siege)?
            .checked_sub(self.spent_counter)?
            .checked_sub(self.paid_bounties)?
            .checked_sub(self.paid_cranks)
    }
}

/// `chest`, `inbox` and `prize vault` addresses.
pub fn chest_address(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CHEST_SEED, mint.as_ref()], &crate::ID)
}

pub fn inbox_address(mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[INBOX_SEED, mint.as_ref()], &crate::ID)
}

pub fn prize_vault_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[PRIZE_VAULT_SEED], &crate::ID)
}

pub fn war_signer_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WAR_SIGNER_SEED], &crate::ID)
}

pub fn loot_signer_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[LOOT_SIGNER_SEED], &crate::ID)
}

/// One `u64` weight per counter (05 section 10.2).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScoreWeights {
    pub raid_volume_won: u64,
    pub sieges: u64,
    pub siege_spend: u64,
    pub times_besieged: u64,
    pub counter_strikes: u64,
    pub treaty_secs: u64,
}

/// `Season` at `["season", number: u32 le]` (05 section 10.2).
#[account]
#[derive(InitSpace, Debug)]
pub struct Season {
    pub version: u8,
    pub bump: u8,
    pub number: u32,
    pub starts_at: i64,
    pub ends_at: i64,
    pub weights: ScoreWeights,
    pub penalize_besieged: bool,
    /// From the timelock.
    pub eta: i64,
    pub opened: bool,
    pub leader: Option<Pubkey>,
    pub leader_score: i128,
    pub finalized: bool,
    /// Prize paid to this season's winner so far.
    pub prize_paid: u64,
    pub reserved: [u8; 32],
}

impl Season {
    pub const LEN: usize = 8 + Self::INIT_SPACE;

    pub fn address(number: u32) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[SEASON_SEED, &number.to_le_bytes()], &crate::ID)
    }

    /// The score of `c` under this season's weights, in `i128` with checked arithmetic.
    pub fn score(&self, c: &SeasonCounters) -> Option<i128> {
        let w = &self.weights;
        let term = |v: u64, weight: u64| i128::from(v).checked_mul(i128::from(weight));
        let besieged = term(u64::from(c.times_besieged), w.times_besieged)?;
        let mut s = term(c.raid_volume_won, w.raid_volume_won)?;
        s = s.checked_add(term(u64::from(c.sieges), w.sieges)?)?;
        s = s.checked_add(term(c.siege_spend, w.siege_spend)?)?;
        s = s.checked_add(term(u64::from(c.counter_strikes), w.counter_strikes)?)?;
        s = s.checked_add(term(c.treaty_secs, w.treaty_secs)?)?;
        if self.penalize_besieged {
            s.checked_sub(besieged)
        } else {
            s.checked_add(besieged)
        }
    }
}

/// A parameter's range in a loot entry.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParamRange {
    pub min: u32,
    pub max: u32,
}

/// One template a roll may mint, with its weight and a range per field (05 section 8.3).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LootEntry {
    pub template_id: u16,
    pub weight: u32,
    pub ranges: [ParamRange; PARAM_FIELDS],
}

/// `LootTable` at `["loot", season: u32 le]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct LootTable {
    pub version: u8,
    pub bump: u8,
    pub season: u32,
    pub count: u8,
    pub entries: [LootEntry; LOOT_TABLE_LEN],
    pub eta: i64,
}

impl LootTable {
    pub const LEN: usize = 8 + Self::INIT_SPACE;

    pub fn address(season: u32) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[LOOT_SEED, &season.to_le_bytes()], &crate::ID)
    }

    pub fn active(&self) -> &[LootEntry] {
        &self.entries[..usize::from(self.count).min(LOOT_TABLE_LEN)]
    }
}

/// `RollRequest` at `["roll", holding, nonce: u64 le]` (05 section 8.2).
#[account]
#[derive(InitSpace, Debug)]
pub struct RollRequest {
    pub version: u8,
    pub bump: u8,
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub holding: Pubkey,
    pub nonce: u64,
    pub season: u32,
    pub requested_slot: u64,
    pub requested_at: i64,
    pub oracle_program: Pubkey,
    pub oracle_account: Pubkey,
}

impl RollRequest {
    pub const LEN: usize = 8 + Self::INIT_SPACE;

    pub fn address(holding: &Pubkey, nonce: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[ROLL_SEED, holding.as_ref(), &nonce.to_le_bytes()], &crate::ID)
    }
}

/// `QuestMark` at `["quest", season: u32 le, mint, owner]` (05 section 9; for `Forge` the mint is
/// the default key).
#[account]
#[derive(InitSpace, Debug)]
pub struct QuestMark {
    pub version: u8,
    pub bump: u8,
    pub season: u32,
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub last_period: u32,
    pub last_forge_count: u64,
}

impl QuestMark {
    pub const LEN: usize = 8 + Self::INIT_SPACE;

    pub fn address(season: u32, mint: &Pubkey, owner: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[QUEST_SEED, &season.to_le_bytes(), mint.as_ref(), owner.as_ref()],
            &crate::ID,
        )
    }
}

/// The War orders' parameters, read from the item in a mint's `War` slot (05 section 6.0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WarOrders {
    pub params: [u32; PARAM_FIELDS],
}

impl WarOrders {
    pub fn get(&self, field: usize) -> u32 {
        self.params[field]
    }

    /// A bps field capped by a config ceiling.
    pub fn bps(&self, field: usize, ceiling: u16) -> u64 {
        u64::from(self.params[field]).min(u64::from(ceiling))
    }

    /// The crank bounty bps, capped by the config.
    pub fn crank_bps(&self, params: &WarParams) -> u64 {
        self.bps(orders::CRANK_BOUNTY_BPS, params.max_crank_bounty_bps)
    }
}

/// `part` basis points of `amount`, rounded down.
pub fn bps_of(amount: u64, part: u64) -> u64 {
    (u128::from(amount) * u128::from(part) / u128::from(BPS)) as u64
}
