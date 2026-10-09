// Changed by Hookwars: window vectors from the pool ring reader (bordrless_core::observations).
//! Vectors for the app's TypeScript mirror of the units math (app/INTEGRATION.md section 4):
//! `programs/tests/vectors/hookwars-math.json`, copied to `app/packages/shared/vectors/`. Rendered
//! from the Rust the programs run: `bordrless_core::observations::window_read`, `hookwars_common::{shape, combine, manifest,
//! PerformanceRule::holds}` and `hookwars_war::{state::Season::score, instructions::loot::draw,
//! state::bps_of}`. As in `vectors.rs`, when a file differs from what the Rust computes the test
//! rewrites it and fails, so a stale file never passes.

use std::path::PathBuf;

use anchor_lang::prelude::Pubkey;
use hookwars_common::{
    combine, manifest, shape, ForgeRule, ParamsError, PerformanceRule, WindowRead, PARAM_FIELDS,
};
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use hookwars_war::client as war;
use hookwars_war::constants::LOOT_TABLE_LEN;
use hookwars_war::instructions::loot::draw;
use hookwars_war::state::{bps_of, LootEntry, LootTable, ParamRange, ScoreWeights, Season, SeasonCounters};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
    fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + self.below(u64::from(hi - lo) + 1) as u32
    }
}

fn arr(v: &[u32]) -> String {
    format!("[{}]", v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(","))
}
fn s(v: impl ToString) -> String {
    format!("\"{}\"", v.to_string())
}
fn hex(b: &[u8]) -> String {
    s(b.iter().map(|x| format!("{x:02x}")).collect::<String>())
}
fn rule_name(r: ForgeRule) -> &'static str {
    match r {
        ForgeRule::Unused => "unused",
        ForgeRule::TowardCeiling => "towardCeiling",
        ForgeRule::TowardFloor => "towardFloor",
        ForgeRule::Keep => "keep",
        ForgeRule::FloorWhenBothOn => "floorWhenBothOn",
    }
}
fn err_name(e: ParamsError) -> &'static str {
    match e {
        ParamsError::UnknownTemplate => "UnknownTemplate",
        ParamsError::OutOfRange => "OutOfRange",
        ParamsError::BadParams => "BadParams",
        ParamsError::NotForgeable => "NotForgeable",
    }
}
fn read_json(r: &Option<WindowRead>) -> String {
    match r {
        None => "null".into(),
        Some(w) => format!(
            "{{\"twapQ64\":{},\"quoteVolume\":{},\"swaps\":{},\"seconds\":{}}}",
            s(w.twap_q64), s(w.quote_volume), s(w.swaps), s(w.seconds)
        ),
    }
}

fn render() -> String {
    let mut rng = Rng(0x756e_6974_735f_7631); // "units_v1"
    let mut out = String::from("{\n\"paramFields\":");
    out += &PARAM_FIELDS.to_string();

    // ---- shapes
    out += ",\n\"shapes\":[";
    let mut first = true;
    for id in 1u16..=9 {
        let sh = shape(id).unwrap();
        if !first { out += ","; }
        first = false;
        out += &format!(
            "\n{{\"id\":{id},\"kind\":{},\"fieldCount\":{},\"forgeable\":{},\"rules\":[{}],\"zeroOff\":[{}]}}",
            sh.kind,
            sh.field_count,
            sh.forgeable,
            sh.rules.iter().map(|r| s(rule_name(*r))).collect::<Vec<_>>().join(","),
            sh.zero_off.iter().map(|b| b.to_string()).collect::<Vec<_>>().join(",")
        );
    }

    // ---- combine (forge) and manifest
    out += "],\n\"combine\":[";
    first = true;
    for id in 1u16..=9 {
        let sh = shape(id).unwrap();
        for _ in 0..12 {
            let mut min = [0u32; PARAM_FIELDS];
            let mut max = [0u32; PARAM_FIELDS];
            let mut a = [0u32; PARAM_FIELDS];
            let mut b = [0u32; PARAM_FIELDS];
            for i in 0..usize::from(sh.field_count) {
                let lo = rng.range(0, 2_000);
                let hi = lo + rng.range(1, 8_000);
                min[i] = lo;
                max[i] = hi;
                a[i] = if sh.zero_off[i] && rng.below(4) == 0 { 0 } else { rng.range(lo, hi) };
                b[i] = if rng.below(5) == 0 { a[i] } else if sh.zero_off[i] && rng.below(4) == 0 { 0 } else { rng.range(lo, hi) };
            }
            let gain = rng.range(0, 10_000) as u16;
            let res = combine(id, &min, &max, gain, &a, &b);
            if !first { out += ","; }
            first = false;
            out += &format!(
                "\n{{\"id\":{id},\"gainBps\":{gain},\"min\":{},\"max\":{},\"a\":{},\"b\":{},\"out\":{},\"error\":{}}}",
                arr(&min), arr(&max), arr(&a), arr(&b),
                res.map(|p| arr(&p)).unwrap_or_else(|_| "null".into()),
                res.err().map(|e| s(err_name(e))).unwrap_or_else(|| "null".into())
            );
        }
    }
    out += "],\n\"manifest\":[";
    first = true;
    for id in 1u16..=9 {
        let sh = shape(id).unwrap();
        for _ in 0..4 {
            let mut p = [0u32; PARAM_FIELDS];
            for i in 0..usize::from(sh.field_count) {
                p[i] = rng.range(0, 3_000);
            }
            let targets = rng.range(0, 4) as u8;
            let m = manifest(id, &p, targets);
            if !first { out += ","; }
            first = false;
            out += &format!("\n{{\"id\":{id},\"params\":{},\"maxTargets\":{targets},\"manifest\":", arr(&p));
            out += &match m {
                Ok(m) => format!(
                    "{{\"kind\":{},\"tokenFlags\":{},\"poolFlags\":{},\"maxCutBuyBps\":{},\"maxCutSellBps\":{},\"maxCutTransferBps\":{},\"maxDiscountBps\":{},\"mayRefuse\":{},\"mayBurn\":{},\"dataBytes\":{},\"readsOtherPools\":{}}},\"error\":null}}",
                    m.kind, m.token_flags, m.pool_flags, m.max_cut_buy_bps, m.max_cut_sell_bps, m.max_cut_transfer_bps,
                    m.max_discount_bps, m.may_refuse, m.may_burn, m.data_bytes, m.reads_other_pools
                ),
                Err(e) => format!("null,\"error\":{}}}", s(err_name(e))),
            };
        }
    }

    // ---- window_read and the performance rule over the observation ring in a pool account
    // (bordrless_core::observations, the bytes after the `Pool` fields; M3a). Built with the
    // program's own init / accumulate / set_price, read with the program's own window_read.
    out += "],\n\"windowRead\":[";
    first = true;
    let disc = bordrless_swap::obs::OBSERVATIONS_DISCRIMINATOR;
    let ring_read = |data: &[u8], now: i64, w: i64, vol: u128, swaps: u64| {
        bordrless_core::observations::window_read(data, &disc, now, w, 1, vol, swaps)
            .ok()
            .map(|r| WindowRead { twap_q64: r.twap_q64, quote_volume: r.quote_volume, swaps: r.swaps, seconds: r.span_secs })
    };
    for case in 0..24 {
        let len = 2 + rng.below(6) as u16;
        let spacing = 1 + rng.below(120) as u32;
        let mut data = vec![0u8; bordrless_core::observations::account_len(len)];
        let mut ts = 1_000_000 + rng.below(1_000) as i64;
        let mut vol: u128 = u128::from(rng.below(1_000_000));
        let mut swaps: u64 = rng.below(10);
        let price0 = (u128::from(rng.next()) << 8) | 1;
        bordrless_core::observations::init(&mut data, &disc, 0, [7u8; 32], len, spacing, ts, price0, vol, swaps).unwrap();
        let steps = 1 + rng.below(3 * u64::from(len));
        for _ in 0..steps {
            ts += 1 + rng.below(400) as i64;
            vol += u128::from(rng.below(5_000_000_000));
            swaps += 1 + rng.below(5);
            bordrless_core::observations::accumulate(&mut data, &disc, ts, vol, swaps).unwrap();
            let p = (u128::from(rng.next()) << 8) | 1;
            bordrless_core::observations::set_price(&mut data, &disc, p).unwrap();
        }
        let pool_vol = vol + u128::from(rng.below(1_000_000_000));
        let pool_swaps = swaps + rng.below(10);
        let now = ts + rng.below(900) as i64;
        let window = if case % 7 == 0 { 0 } else { 1 + rng.below(3_000) as i64 };
        let base = window + 1 + rng.below(3_000) as i64;
        let short_read = ring_read(&data, now, window, pool_vol, pool_swaps);
        let base_read = ring_read(&data, now, base, pool_vol, pool_swaps);
        let rule = PerformanceRule {
            metric: rng.below(3) as u8,
            window_secs: window.max(0) as u32,
            base_window_secs: base as u32,
            op: rng.below(2) as u8,
            ratio_bps: rng.range(1, 20_000) as u16,
            hold_secs: 0,
        };
        if !first { out += ","; }
        first = false;
        out += &format!(
            "\n{{\"data\":{},\"poolQuoteVolume\":{},\"poolSwapCount\":{},\"now\":{},\"window\":{},\"baseWindow\":{},\"read\":{},\"baseRead\":{},\"rule\":{{\"metric\":{},\"op\":{},\"ratioBps\":{}}},\"holds\":{}}}",
            hex(&data), s(pool_vol), s(pool_swaps), s(now), s(window), s(base),
            read_json(&short_read), read_json(&base_read), rule.metric, rule.op, rule.ratio_bps,
            rule.holds(short_read, base_read)
        );
    }

    // ---- season score
    out += "],\n\"seasonScore\":[";
    first = true;
    for i in 0..16 {
        let big = i % 5 == 0;
        let w = |r: &mut Rng| if big { r.next() } else { r.below(1_000) };
        let weights = ScoreWeights {
            raid_volume_won: w(&mut rng), sieges: w(&mut rng), siege_spend: w(&mut rng),
            times_besieged: w(&mut rng), counter_strikes: w(&mut rng), treaty_secs: w(&mut rng),
        };
        let c = SeasonCounters {
            raid_volume_won: if big { rng.next() } else { rng.below(1_000_000_000_000) },
            sieges: rng.below(50) as u32,
            siege_spend: rng.below(1_000_000_000_000),
            times_besieged: rng.below(50) as u32,
            counter_strikes: rng.below(50) as u32,
            treaty_secs: rng.below(10_000_000),
        };
        let penalize = rng.below(2) == 1;
        let season = Season {
            version: 1, bump: 0, number: 1, starts_at: 0, ends_at: 0, weights, penalize_besieged: penalize, eta: 0,
            opened: true, leader: None, leader_score: 0, finalized: false, prize_paid: 0, reserved: Default::default(),
        };
        let score = season.score(&c);
        if !first { out += ","; }
        first = false;
        out += &format!(
            "\n{{\"weights\":[{}],\"counters\":{{\"raidVolumeWon\":{},\"sieges\":{},\"siegeSpend\":{},\"timesBesieged\":{},\"counterStrikes\":{},\"treatySecs\":{}}},\"penalizeBesieged\":{penalize},\"score\":{}}}",
            [weights.raid_volume_won, weights.sieges, weights.siege_spend, weights.times_besieged, weights.counter_strikes, weights.treaty_secs]
                .iter().map(|x| s(x)).collect::<Vec<_>>().join(","),
            s(c.raid_volume_won), c.sieges, s(c.siege_spend), c.times_besieged, c.counter_strikes, s(c.treaty_secs),
            score.map(s).unwrap_or_else(|| "null".into())
        );
    }

    // ---- loot draw
    out += "],\n\"lootDraw\":[";
    first = true;
    for _ in 0..16 {
        let count = rng.below(LOOT_TABLE_LEN as u64 + 1) as u8;
        let mut entries = [LootEntry::default(); LOOT_TABLE_LEN];
        for e in entries.iter_mut().take(usize::from(count)) {
            e.template_id = rng.range(1, 9) as u16;
            e.weight = if rng.below(6) == 0 { 0 } else { rng.range(1, 1_000) };
            for r in e.ranges.iter_mut() {
                let lo = rng.range(0, 5_000);
                *r = ParamRange { min: lo, max: lo + rng.range(0, 5_000) };
            }
        }
        let table = LootTable { version: 1, bump: 0, season: 1, count, entries, eta: 0 };
        let mut value = [0u8; 32];
        for b in value.iter_mut() {
            *b = rng.below(256) as u8;
        }
        let res = draw(&table, &value);
        if !first { out += ","; }
        first = false;
        out += &format!(
            "\n{{\"entries\":[{}],\"value\":{},\"templateId\":{},\"params\":{}}}",
            table.active().iter().map(|e| format!(
                "{{\"templateId\":{},\"weight\":{},\"ranges\":[{}]}}",
                e.template_id, e.weight,
                e.ranges.iter().map(|r| format!("[{},{}]", r.min, r.max)).collect::<Vec<_>>().join(",")
            )).collect::<Vec<_>>().join(","),
            hex(&value),
            res.map(|r| r.0.to_string()).unwrap_or_else(|| "null".into()),
            res.map(|r| arr(&r.1)).unwrap_or_else(|| "null".into())
        );
    }

    // ---- bps_of
    out += "],\n\"bpsOf\":[";
    first = true;
    for _ in 0..16 {
        let amount = rng.next();
        let part = rng.below(10_001);
        if !first { out += ","; }
        first = false;
        out += &format!("\n{{\"amount\":{},\"part\":{part},\"out\":{}}}", s(amount), s(bps_of(amount, part)));
    }

    // ---- instructions, rendered by the programs' own Rust clients
    out += "],\n\"instructions\":[";
    let key = |n: u8| Pubkey::new_from_array([n; 32]);
    let meta = |n: u8, w: bool| if w { AccountMeta::new(key(n), false) } else { AccountMeta::new_readonly(key(n), false) };
    let inner = Instruction { program_id: key(200), accounts: vec![meta(201, true), meta(202, false)], data: vec![] };
    let orders = war::Orders { item: key(30), template: key(31) };
    let cases: Vec<(&str, Instruction)> = vec![
        ("initWar", war::init_war(key(1), key(2))),
        ("recordFunding", war::record_funding(key(2))),
        ("claimBounty", war::claim_bounty(key(3), key(2), orders, 1, vec![meta(40, true), meta(41, false)], &[])),
        ("claimQuestRaid", war::claim_quest(key(3), key(2), 4, 1, 9, 1, vec![meta(40, true)])),
        ("claimQuestForge", war::claim_quest(key(3), key(2), 4, 2, 9, 1, vec![])),
        ("roll", war::roll(key(3), key(2), 7, 1, key(50), key(51), vec![meta(40, true)])),
        ("cancelRoll", war::cancel_roll(key(3), bordrless_token::client::holding_address(&key(2), &key(3)), 7)),
        ("openSeason0", war::open_season(0)),
        ("openSeason3", war::open_season(3)),
        ("submitCandidateLedger", war::submit_candidate(key(3), 5, key(2), true)),
        ("submitCandidateNoLedger", war::submit_candidate(key(3), 5, key(2), false)),
        ("finalizeSeason", war::finalize_season(5)),
        ("splitNoWinner", war::split_protocol_fees(key(3), key(60), None, &[])),
        ("splitWinner", war::split_protocol_fees(key(3), key(60), Some((key(2), 4)), &[inner.clone()])),
        ("accrueTreatyTime", war::accrue_treaty_time(key(2), 4, &[(key(70), key(71)), (key(72), key(73))])),
        ("siege", war::siege(key(3), key(2), orders, key(80), key(81), true, Some(key(82)), vec![meta(83, true), meta(84, false)], &[inner.clone()])),
        ("siegeNoWar", war::siege(key(3), key(2), orders, key(80), key(81), false, None, vec![], &[])),
        ("counterStrike", war::counter_strike(key(3), key(2), orders, key(81), None, vec![meta(83, true)], vec![meta(84, true), meta(85, false)], &[inner.clone()])),
        ("raze", war::raze(key(3), key(2), orders, key(80), key(81), vec![meta(83, true)], &[])),
        ("returnCaptured", war::return_captured(key(3), key(2), key(80), key(90), key(91), vec![meta(83, true)], &[])),
        ("shareTreatyInflow", war::share_treaty_inflow(key(3), key(2), &[inner.clone()])),
        ("touch", bordrless_token::client::touch(key(3), key(2), key(4), key(5), 1, vec![1, 2, 3], vec![meta(40, true)])),
    ];
    first = true;
    for (name, ix) in cases {
        if !first { out += ","; }
        first = false;
        out += &format!(
            "\n{{\"name\":{},\"programId\":{},\"keys\":[{}],\"data\":{}}}",
            s(name), s(ix.program_id),
            ix.accounts.iter().map(|m| format!("[{},{},{}]", s(m.pubkey), m.is_signer, m.is_writable)).collect::<Vec<_>>().join(","),
            hex(&ix.data)
        );
    }
    out += "]\n}\n";
    let _ = Pubkey::default();
    out
}

#[test]
fn hookwars_math_vectors_match_the_rust() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rendered = render();
    let mut stale = Vec::new();
    for path in [
        root.join("vectors/hookwars-math.json"),
        root.join("../../app/packages/shared/vectors/hookwars-math.json"),
    ] {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != rendered {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir).unwrap();
            }
            std::fs::write(&path, &rendered).unwrap();
            stale.push(path.display().to_string());
        }
    }
    assert!(stale.is_empty(), "vectors were stale and have been rewritten: {stale:?}");
}
