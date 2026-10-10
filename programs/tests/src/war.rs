// Changed by Hookwars: new file, the war program's harness; M3b: the armory's and the shared types; security review 1: TEST params, WarWorld::with_params; integration pass 3: new Template and Item fields; pass 4b: boss_share_bps, contribute_interval_secs, the new WarState fields.
// the observation ring in the pool account.
//! The war suites' world: the upstream world plus `hookwars_war` and its test-only stand-ins at the
//! items and armory ids (`war_items_stub`, `war_armory_stub`) and a randomness adapter
//! (`randomness_stub`). Foreign accounts (the War orders `Item` and its `Template`, the
//! `RaidLedger`, a pool's observation ring) are written with `put` in their owners' layouts (M3b),
//! and a launch's mint gets its slot table written the same way (the slot-aware launchpad is the
//! launchpad branch's).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AccountDeserialize, AccountSerialize};
use bordrless_hook::{slot_flags, slot_kind};
use bordrless_launch::client as launch;
use bordrless_launch::state::LaunchRules;
use bordrless_token::client as token;
use bordrless_token::state::{Holding, Mint, Slot, SlotBounds};
use hookwars_war::client as war;
use hookwars_war::foreign::{raid_ledger_address, spot_q64, template_address, PARAM_FIELDS};
use hookwars_war::instructions::ConfigArgs;
use hookwars_war::state::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

use crate::env::Tx;
use crate::fixture::World;
use crate::launch::VQ;

/// TEST values of every war parameter (00 section 6); none is a proposal for production.
pub const TEST_PARAMS: WarParams = WarParams {
    admin_timelock_secs: 3_600,
    max_crank_bounty_bps: 100,
    siege_interval_secs: 3_600,
    siege_max_spend_bps: 1_000,
    siege_max_premium_bps: 300,
    siege_slippage_bps: 200,
    siege_unit_lamports: 1_000_000,
    counter_max_spend_bps: 1_000,
    counter_min_interval_secs: 600,
    raze_max_bps_per_interval: 2_500,
    raze_interval_secs: 3_600,
    bounty_max_per_claim: 100_000_000,
    min_twap_secs: 60,
    raid_window_secs: 3_600,
    roll_expiry_secs: 600,
    quest_period_secs: 86_400,
    quest_raid_points: 10,
    season_secs: 7 * 86_400,
    challenge_secs: 86_400,
    season_prize_share_bps: 2_000,
    point_unit_lamports: 1_000_000,
    loot_min_raid_lamports: 10_000_000,
    // Security review 1: TEST values. A raze waits below the TWAP by more than 10% (M-3); the
    // bounty cap (M-6) is left wide open here (100% of a point's volume) so the suites' TEST rates
    // stand, and `tests/security.rs` sets a tight one.
    raze_max_discount_bps: 1_000,
    bounty_max_point_bps: 10_000,
    // Security review 2, M-B: TEST, raid volume scored per lamport the chest received in the season.
    raid_volume_per_funded: 1_000,
    // Pass 4b: TEST values (10 sections 8, 11.1).
    boss_share_bps: 1_000,
    contribute_interval_secs: 3_600,
    // Review 3: TEST values (L-3, M-6).
    coalition_min_term_secs: 3_600,
    coalition_grace_secs: 86_400,
};

/// TEST template ids (the armory numbers its templates densely; these are the suites').
pub const WAR_ORDERS_TEMPLATE: u16 = 9;
pub const TREATY_TEMPLATE: u16 = 5;
pub const RAID_TEMPLATE: u16 = 1;
/// The Raid slot's index in every war token the harness makes.
pub const RAID_SLOT: u8 = 1;
/// TEST creator fee of the launches.
pub const CREATOR_FEE_BPS: u16 = 100;

/// War orders (04 section 3.9), TEST values.
#[derive(Clone, Copy, Debug)]
pub struct OrdersSpec {
    pub siege_threshold: u32,
    pub siege_spend_bps: u32,
    pub siege_twap_secs: u32,
    pub counter_drop_bps: u32,
    pub counter_short_secs: u32,
    pub counter_long_secs: u32,
    pub counter_interval_secs: u32,
    pub counter_spend_bps: u32,
    pub raze_enabled: u32,
    pub bounty_rate: u32,
    pub crank_bounty_bps: u32,
}

impl Default for OrdersSpec {
    fn default() -> Self {
        Self {
            siege_threshold: 100,
            siege_spend_bps: 1_000,
            siege_twap_secs: 600,
            counter_drop_bps: 1_000,
            counter_short_secs: 300,
            counter_long_secs: 3_600,
            counter_interval_secs: 600,
            counter_spend_bps: 1_000,
            raze_enabled: 1,
            bounty_rate: 10_000,
            crank_bounty_bps: 50,
        }
    }
}

impl OrdersSpec {
    pub fn params(&self) -> [u32; PARAM_FIELDS] {
        [
            self.siege_threshold,
            self.siege_spend_bps,
            self.siege_twap_secs,
            self.counter_drop_bps,
            self.counter_short_secs,
            self.counter_long_secs,
            self.counter_interval_secs,
            self.counter_spend_bps,
            self.raze_enabled,
            self.bounty_rate,
            self.crank_bounty_bps,
        ]
    }
}

/// A token with a war chest.
#[derive(Clone, Copy, Debug)]
pub struct WarToken {
    pub mint: Pubkey,
    pub orders: war::Orders,
    pub pool: Pubkey,
    pub raid_item: Pubkey,
}

/// The war world.
pub struct WarWorld {
    pub w: World,
}

impl Default for WarWorld {
    fn default() -> Self {
        Self::new()
    }
}

impl WarWorld {
    /// The world, the war program and its stand-ins loaded, its config created with
    /// [`TEST_PARAMS`], the War orders and Treaty templates written.
    pub fn new() -> Self {
        Self::with_params(TEST_PARAMS)
    }

    /// As [`WarWorld::new`] with the war config created with `params` (security review 1 tests).
    pub fn with_params(params: WarParams) -> Self {
        let mut w = World::new();
        let loads = [
            ("hookwars_war", hookwars_war::ID),
            ("war_items_stub", war_items_stub::ID),
            ("war_armory_stub", war_armory_stub::ID),
            ("randomness_stub", randomness_stub::ID),
        ];
        for (name, id) in loads {
            w.env
                .svm
                .add_program(id, &crate::program_bytes(name))
                .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
        }
        let deployer = w.env.deployer.insecure_clone();
        w.env
            .set_upgrade_authority(hookwars_war::ID, Some(deployer.pubkey()));
        let mut ww = Self { w };
        ww.init_config(params).ok();
        ww.put_template(WAR_ORDERS_TEMPLATE, slot_kind::WAR, 11, [0; PARAM_FIELDS], [u32::MAX; PARAM_FIELDS], true);
        let mut max = [0u32; PARAM_FIELDS];
        max[..3].copy_from_slice(&[10_000, 10_000, 1]);
        ww.put_template(TREATY_TEMPLATE, slot_kind::RELATION, 3, [0; PARAM_FIELDS], max, false);
        let mut max = [0u32; PARAM_FIELDS];
        max[..3].copy_from_slice(&[10_000, 1_000, 100]);
        ww.put_template(RAID_TEMPLATE, slot_kind::POOL, 3, [0; PARAM_FIELDS], max, true);
        ww
    }

    /// The TEST loot table: Raid items three times as often as War orders.
    pub fn loot_entries() -> Vec<LootEntry> {
        let mut raid = [ParamRange::default(); PARAM_FIELDS];
        raid[0] = ParamRange { min: 100, max: 5_000 };
        raid[1] = ParamRange { min: 0, max: 500 };
        raid[2] = ParamRange { min: 1, max: 100 };
        let mut orders = [ParamRange::default(); PARAM_FIELDS];
        for (i, r) in orders.iter_mut().enumerate() {
            *r = ParamRange {
                min: 1,
                max: 1_000 + i as u32,
            };
        }
        vec![
            LootEntry {
                template_id: RAID_TEMPLATE,
                weight: 3,
                ranges: raid,
            },
            LootEntry {
                template_id: WAR_ORDERS_TEMPLATE,
                weight: 1,
                ranges: orders,
            },
        ]
    }

    /// Proposes season `number` (starting at the timelock's end) with `weights` and the TEST loot
    /// table, waits the timelock, and opens it. Answers the season.
    pub fn open_season(&mut self, weights: ScoreWeights) -> Season {
        let admin = self.w.env.deployer.insecure_clone();
        let number = self.config().current_season + 1;
        let starts_at = self.now() + TEST_PARAMS.admin_timelock_secs;
        let args = hookwars_war::instructions::SeasonArgs {
            number,
            starts_at,
            weights,
            penalize_besieged: false,
        };
        let templates = [template_address(RAID_TEMPLATE), template_address(WAR_ORDERS_TEMPLATE)];
        let ixs = [
            war::propose_season(admin.pubkey(), args),
            war::propose_loot_table(admin.pubkey(), number, Self::loot_entries(), &templates),
        ];
        self.w.env.send(&ixs, &[&admin]).ok();
        self.w.env.warp(TEST_PARAMS.admin_timelock_secs);
        self.w.env.send(&[war::open_season(number - 1)], &[]).ok();
        self.w.env.read(&Season::address(number).0)
    }

    pub fn config_args(params: WarParams, admin: Pubkey, treasury: Pubkey) -> ConfigArgs {
        ConfigArgs {
            admin,
            protocol_treasury: treasury,
            randomness_program: randomness_stub::ID,
            treaty_template_id: Some(TREATY_TEMPLATE),
            params,
        }
    }

    /// `init_config` by the deployer (the program's upgrade authority).
    pub fn init_config(&mut self, params: WarParams) -> Tx {
        let deployer = self.w.env.deployer.insecure_clone();
        let args = Self::config_args(params, deployer.pubkey(), self.w.env.treasury.pubkey());
        self.w
            .env
            .send(&[war::init_config(deployer.pubkey(), args)], &[&deployer])
    }

    pub fn config(&self) -> WarConfig {
        self.w.env.read(&war::config_address())
    }

    pub fn now(&self) -> i64 {
        self.w.env.now
    }

    // ---------------------------------------------------------------------------- foreign accounts

    pub fn put_anchor<T: AccountSerialize>(&mut self, key: Pubkey, owner: Pubkey, value: &T, len: usize) {
        let mut data = Vec::with_capacity(len);
        value.try_serialize(&mut data).expect("serialize");
        data.resize(data.len().max(len), 0);
        let lamports = self.w.env.rent(data.len());
        self.w.env.put(
            key,
            Account {
                lamports,
                data,
                owner,
                executable: false,
                rent_epoch: 0,
            },
        );
    }

    /// Writes a `Template` (02 section 2.3) in the armory's own layout.
    pub fn put_template(
        &mut self,
        id: u16,
        kind: u8,
        field_count: u8,
        field_min: [u32; PARAM_FIELDS],
        field_max: [u32; PARAM_FIELDS],
        loot_enabled: bool,
    ) {
        let t = hookwars_armory::state::Template {
            version: 1,
            bump: 0,
            id,
            program: bordrless_token::constants::ITEMS_ID,
            code_hash: [7; 32],
            deploy_slot: None,
            kind,
            field_count,
            field_min,
            field_max,
            open_authoring: false,
            loot_enabled,
            forge_enabled: false,
            max_level: 1,
            loot_royalty_bps: 0,
            max_targets: 0,
            status: 0,
            name: format!("TEST template {id}"),
            registered_by: Pubkey::default(),
            created_at: 0,
            author_bps: 0,
            default_access: 0,
            allowed_access: 0,
            charges_on_create: 0,
            reserved: [0; 24],
        };
        self.put_anchor(template_address(id), war_armory_stub::ID, &t, 0);
    }

    /// Writes an `Item` (02 section 2.4) of `template_id` with `params` at a fresh key.
    pub fn put_item(&mut self, template_id: u16, params: [u32; PARAM_FIELDS]) -> Pubkey {
        let key = Pubkey::new_unique();
        let it = hookwars_armory::state::Item {
            version: 1,
            bump: 0,
            item_mint: Pubkey::new_unique(),
            template_id,
            params,
            manifest: hookwars_common::Manifest::default(),
            author: Pubkey::default(),
            royalty_bps: 0,
            level: 1,
            source: 0,
            equipped_count: 1,
            royalty_owner_bump: 0,
            created_at: 0,
            has_wear: false,
            reserved: [0; 31],
        };
        self.put_anchor(key, war_armory_stub::ID, &it, 0);
        key
    }

    /// Writes the armory's `ForgeCounter` of `wallet`.
    pub fn put_forge_counter(&mut self, wallet: Pubkey, count: u64) {
        let c = hookwars_armory::state::ForgeCounter {
            wallet,
            count,
            bump: 0,
        };
        self.put_anchor(
            hookwars_war::foreign::forge_counter_address(&wallet),
            war_armory_stub::ID,
            &c,
            0,
        );
    }

    /// Writes `mint`'s `RaidLedger` (04 section 2.9) in the shared layout.
    pub fn put_ledger(
        &mut self,
        mint: &Pubkey,
        season_id: u32,
        season_volume: u64,
        windows: &[(Pubkey, i64, u64, u64)],
    ) {
        use hookwars_common::raid::{RaidLedger, RaidWindow, RAID_TABLE_LEN};
        let mut inbound = [RaidWindow::default(); RAID_TABLE_LEN];
        for (i, (rival, start, volume, prev)) in windows.iter().enumerate() {
            inbound[i] = RaidWindow {
                rival_mint: *rival,
                window_start: *start,
                volume: *volume,
                prev_volume: *prev,
            };
        }
        let l = RaidLedger {
            version: 1,
            bump: 0,
            mint: *mint,
            season_id,
            outbound_volume_season: season_volume,
            inbound,
            ..Default::default()
        };
        let mut data = vec![0u8; RaidLedger::LEN];
        l.encode(&mut data).unwrap();
        let lamports = self.w.env.rent(data.len());
        self.w.env.put(
            raid_ledger_address(mint),
            Account {
                lamports,
                data,
                owner: war_items_stub::ID,
                executable: false,
                rent_epoch: 0,
            },
        );
    }

    /// Writes `pool`'s observation ring (03 section 3.1, M3a: in the pool account's tail, layout
    /// `bordrless_core::observations`): `entries` as `(ts, price_cumulative)`, the header's last
    /// price and time, its cumulative the newest entry's.
    pub fn put_observations(&mut self, pool: &Pubkey, last_price: u128, last_ts: i64, entries: &[(i64, u128)]) {
        let mut account = self.w.env.account(pool).expect("pool");
        let e: Vec<crate::ring::RingEntry> = entries.iter().map(|(t, c)| (*t, *c, 0, 0)).collect();
        account.data = crate::ring::with_ring(account.data, &crate::ring::ring(pool, last_price, last_ts, &e));
        account.lamports = account.lamports.max(self.w.env.rent(account.data.len()));
        self.w.env.put(*pool, account);
    }

    /// The pool's spot price (Q64.64).
    pub fn spot(&self, pool: &Pubkey) -> u128 {
        let p: bordrless_swap::state::Pool = self.w.env.read(pool);
        spot_q64(p.quote_reserve, p.virtual_quote, p.base_reserve, p.virtual_base).unwrap()
    }

    /// Observations under which every TWAP over the last `span` seconds equals `price`.
    pub fn flat_observations(&mut self, pool: &Pubkey, price: u128, span: i64) {
        let t0 = self.now() - span;
        self.put_observations(pool, price, t0, &[(t0, 0)]);
    }

    /// Observations where the price was `high` until `short` seconds ago and `low` since, over
    /// `long` seconds.
    pub fn falling_observations(&mut self, pool: &Pubkey, high: u128, low: u128, short: i64, long: i64) {
        let now = self.now();
        let (t0, t1) = (now - long, now - short);
        let c1 = high * (long - short) as u128;
        self.put_observations(pool, low, t1, &[(t0, 0), (t1, c1)]);
    }

    // ---------------------------------------------------------------------------- mints and slots

    pub fn write_mint(&mut self, key: &Pubkey, f: impl FnOnce(&mut Mint)) {
        let mut account = self.w.env.account(key).expect("mint");
        let mut mint = Mint::try_deserialize(&mut &account.data[..]).expect("mint decodes");
        f(&mut mint);
        let mut data = Vec::with_capacity(account.data.len());
        mint.try_serialize(&mut data).expect("serialize mint");
        data.resize(account.data.len(), 0);
        account.data = data;
        self.w.env.put(*key, account);
    }

    /// Appends `slot` to `mint`'s slot table (the slot-aware launchpad is not in this branch).
    pub fn add_slot(&mut self, mint: &Pubkey, slot: Slot) -> u8 {
        let mut index = 0;
        self.write_mint(mint, |m| {
            index = m.slot_count;
            m.slots[usize::from(m.slot_count)] = slot;
            m.slot_count += 1;
        });
        index
    }

    pub fn war_slot(item: Pubkey) -> Slot {
        Slot {
            kind: slot_kind::WAR,
            item,
            program: bordrless_token::constants::ITEMS_ID,
            data_epoch: 1,
            ..Slot::default()
        }
    }

    /// The Raid slot: the items program's, answering touch, 11 bytes (12 with the epoch byte).
    pub fn raid_slot(item: Pubkey, offset: u8) -> Slot {
        let signer = bordrless_hook::hook_signer(&bordrless_token::ID, &bordrless_token::constants::ITEMS_ID);
        Slot {
            kind: slot_kind::POOL,
            bounds: SlotBounds {
                may_write_data: true,
                may_answer_touch: true,
                ..SlotBounds::default()
            },
            data_offset: offset,
            data_len: 12,
            item,
            program: bordrless_token::constants::ITEMS_ID,
            flags: slot_flags::ANSWERS_TOUCH | slot_flags::WRITES_HOOK_DATA,
            signer_bump: signer.1,
            data_epoch: 1,
            ..Slot::default()
        }
    }

    /// A slot that only names `item` (never called): how the suites mark a Treaty as equipped.
    pub fn named_slot(kind: u8, item: Pubkey) -> Slot {
        Slot {
            kind,
            item,
            program: bordrless_token::constants::ITEMS_ID,
            data_epoch: 1,
            ..Slot::default()
        }
    }

    /// A launch by a fresh creator with `rules`.
    pub fn launch(&mut self, symbol: &str, rules: LaunchRules) -> Pubkey {
        let creator = self.w.env.funded(100_000_000_000);
        let (mint, tx) = self.w.create_launch_with(&creator, symbol, CREATOR_FEE_BPS, VQ, rules);
        tx.ok();
        mint
    }

    /// A launch made a war token: a War slot holding War orders of `orders`, a Raid slot, then
    /// `init_war`. Its launch is warped past the launchpad's first minute.
    pub fn war_token(&mut self, symbol: &str, orders: OrdersSpec) -> WarToken {
        let mint = self.launch(symbol, LaunchRules::NONE);
        self.make_war(mint, orders)
    }

    /// `mint` (a launch with no kit) made a war token.
    pub fn make_war(&mut self, mint: Pubkey, orders: OrdersSpec) -> WarToken {
        let item = self.put_item(WAR_ORDERS_TEMPLATE, orders.params());
        let raid_item = Pubkey::new_unique();
        self.add_slot(&mint, Self::war_slot(item));
        let idx = self.add_slot(&mint, Self::raid_slot(raid_item, 0));
        assert_eq!(idx, RAID_SLOT);
        let payer = self.w.env.payer.insecure_clone();
        self.w.env.send(&[war::init_war(payer.pubkey(), mint)], &[]).ok();
        WarToken {
            mint,
            orders: war::Orders {
                item,
                template: template_address(WAR_ORDERS_TEMPLATE),
            },
            pool: self.w.launch_pool_key(&mint),
            raid_item,
        }
    }

    /// Writes a fresh `WarState` for `mint` directly (for a token whose mint cannot carry a slot
    /// table in this branch, such as a kit launch with the upstream single hook), and creates its
    /// treaty inbox's holding.
    pub fn put_war_state(&mut self, mint: &Pubkey) {
        let (key, bump) = WarState::address(mint);
        let s = WarState {
            version: 1,
            bump,
            chest_bump: chest_address(mint).1,
            inbox_bump: inbox_address(mint).1,
            mint: *mint,
            launch: launch::launch_address(mint),
            last_seen_balance: 0,
            funded_total: 0,
            spent_siege: 0,
            spent_counter: 0,
            paid_bounties: 0,
            paid_cranks: 0,
            razed_proceeds: 0,
            treaty_shared_total: 0,
            last_siege_at: 0,
            last_counter_at: 0,
            last_treaty_tick: 0,
            under_siege_until: 0,
            siege_by_chest: Pubkey::default(),
            captured: [Captured::default(); hookwars_war::constants::MAX_CAPTURED],
            season_id: 0,
            season: SeasonCounters::default(),
            prev_season: SeasonCounters::default(),
            received_other: 0,
            sent_coalition: 0,
            rivalry: RivalryBudget::default(),
            reserved: [0; 64],
        };
        self.put_anchor(key, hookwars_war::ID, &s, WarState::LEN);
        let payer = self.w.env.payer.insecure_clone();
        let inbox = inbox_address(mint).0;
        self.w
            .env
            .send(&[token::create_holding(payer.pubkey(), self.w.sol, inbox)], &[])
            .ok();
    }

    /// Writes `owner`'s Raid range in its holding of `mint` (the holding must exist).
    pub fn set_raid(&mut self, mint: &Pubkey, owner: &Pubkey, season: u32, points: u32, tickets: u16) {
        let key = token::holding_address(mint, owner);
        let mut account = self.w.env.account(&key).expect("holding");
        let mut h = Holding::try_deserialize(&mut &account.data[..]).expect("holding decodes");
        let mut r = [0u8; 12];
        r[0] = 1; // the slot's epoch
        r[1] = 0x01; // the Raid tag
        r[2..6].copy_from_slice(&season.to_le_bytes());
        r[6..10].copy_from_slice(&points.to_le_bytes());
        r[10..12].copy_from_slice(&tickets.to_le_bytes());
        h.hook_data[..12].copy_from_slice(&r);
        let mut data = Vec::with_capacity(account.data.len());
        h.try_serialize(&mut data).unwrap();
        data.resize(account.data.len(), 0);
        account.data = data;
        self.w.env.put(key, account);
    }

    /// `owner`'s Raid range: (season, points, tickets).
    pub fn raid(&self, mint: &Pubkey, owner: &Pubkey) -> (u32, u32, u16) {
        let h: Holding = self.w.env.read(&token::holding_address(mint, owner));
        let r = &h.hook_data[..12];
        if r[0] != 1 || r[1] != 0x01 {
            return (0, 0, 0);
        }
        (
            u32::from_le_bytes(r[2..6].try_into().unwrap()),
            u32::from_le_bytes(r[6..10].try_into().unwrap()),
            u16::from_le_bytes(r[10..12].try_into().unwrap()),
        )
    }

    // ---------------------------------------------------------------------------- chests

    pub fn chest(mint: &Pubkey) -> Pubkey {
        chest_address(mint).0
    }

    pub fn chest_holding(mint: &Pubkey) -> Pubkey {
        token::holding_address(&bordrless_swap::constants::BRIDGED_SOL_MINT, &Self::chest(mint))
    }

    pub fn chest_balance(&self, mint: &Pubkey) -> u64 {
        self.w.env.holding(&self.w.sol, &Self::chest(mint))
    }

    pub fn state(&self, mint: &Pubkey) -> WarState {
        self.w.env.read(&WarState::address(mint).0)
    }

    /// Sends `lamports` of bridged SOL into the chest's holding (a plain transfer, as anyone may),
    /// then `record_funding`.
    pub fn fund_chest(&mut self, mint: &Pubkey, lamports: u64) {
        let donor = self.w.wallet_with_sol(lamports);
        let ixs = [
            token::transfer(
                donor.pubkey(),
                token::holding_address(&self.w.sol, &donor.pubkey()),
                Self::chest_holding(mint),
                self.w.sol,
                None,
                vec![],
                lamports,
            ),
            war::record_funding(*mint),
        ];
        self.w.env.send_paid_by(&ixs, &donor, &[]).ok();
    }

    /// Chest solvency (07 invariant 5): the balance is exactly what the totals say, and what the
    /// state last saw.
    #[track_caller]
    pub fn assert_solvent(&self, mint: &Pubkey) {
        let s = self.state(mint);
        let balance = self.chest_balance(mint);
        assert_eq!(s.expected_balance(), Some(balance), "chest totals vs balance");
        assert_eq!(s.last_seen_balance, balance, "last seen vs balance");
    }

    // ---------------------------------------------------------------------------- step builders

    /// The instructions a chest buy or sell on `mint`'s launch pool makes (for their accounts).
    pub fn chest_swap_inner(&self, chest_mint: &Pubkey, pool_mint: &Pubkey, payer: &Pubkey, sell: bool) -> Vec<Instruction> {
        let chest = Self::chest(chest_mint);
        let keys = self.w.launch_keys(pool_mint);
        vec![
            launch::swap_with_base_slice(&keys, chest, chest, if sell { 0 } else { 1 }, 1, 0, vec![]),
            token::create_holding(*payer, *pool_mint, chest),
            bordrless_bridge::client::unwrap_sol(chest, 0),
        ]
    }

    pub fn siege_ix(&self, cranker: &Pubkey, t: &WarToken, rival: &Pubkey, rival_has_war: bool, rival_kit: Option<Pubkey>) -> Instruction {
        let pool = self.w.launch_pool_key(rival);
        let inner = self.chest_swap_inner(&t.mint, rival, cranker, false);
        war::siege(*cranker, t.mint, t.orders, *rival, pool, rival_has_war, rival_kit, vec![], &inner)
    }

    pub fn counter_ix(&self, cranker: &Pubkey, t: &WarToken) -> Instruction {
        let chest = Self::chest(&t.mint);
        let mut inner = self.chest_swap_inner(&t.mint, &t.mint, cranker, false);
        inner.push(token::burn_with(chest, token::holding_address(&t.mint, &chest), t.mint, None, vec![], 1));
        war::counter_strike(*cranker, t.mint, t.orders, t.pool, None, vec![], vec![], &inner)
    }

    pub fn raze_ix(&self, cranker: &Pubkey, t: &WarToken, rival: &Pubkey) -> Instruction {
        let pool = self.w.launch_pool_key(rival);
        let inner = self.chest_swap_inner(&t.mint, rival, cranker, true);
        war::raze(*cranker, t.mint, t.orders, *rival, pool, vec![], &inner)
    }

    pub fn bounty_ix(&self, owner: &Pubkey, t: &WarToken) -> Instruction {
        let inner = [bordrless_bridge::client::unwrap_sol(Self::chest(&t.mint), 0)];
        war::claim_bounty(*owner, t.mint, t.orders, RAID_SLOT, vec![], &inner)
    }

    /// Sends `ix` paid by a fresh funded cranker; answers the transaction and the cranker.
    pub fn crank(&mut self, ix: impl FnOnce(&Pubkey, &Self) -> Instruction) -> (Tx, Keypair) {
        let cranker = self.w.env.funded(10_000_000_000);
        let i = ix(&cranker.pubkey(), self);
        let tx = self.w.env.send_paid_by(&[i], &cranker, &[]);
        (tx, cranker)
    }

    /// A wallet that bought `lamports` of `mint` (bridged SOL, a holding).
    pub fn buyer(&mut self, mint: &Pubkey, lamports: u64) -> Keypair {
        let wallet = self.w.wallet_with_sol(lamports * 2);
        self.w.buy(&wallet, mint, lamports).ok();
        wallet
    }

    /// The mint's metas helper for touch-only extras (none in the harness).
    pub fn no_extras() -> Vec<AccountMeta> {
        vec![]
    }
}

/// The war program's error code.
pub fn war_code(e: hookwars_war::error::WarError) -> u32 {
    u32::from(e)
}
