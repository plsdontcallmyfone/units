// Changed by Hookwars: new file (hook economy, docs/spec/11-hook-economy.md section 5.5).
//! Faucet and sink simulation of the hook economy. Deterministic and seeded: the same parameter
//! set and seed always give the same run. The splits (recipe fee, licence) use the
//! `hookwars-common` functions the programs use, so the SOL flows here are the programs' flows.
//!
//! What it answers (11 section 5.5):
//! 1. Material supply per season: drops (capped, from activity) minus burns (crafts, repairs).
//! 2. Item supply in use: crafted items minus retired ones, and the share of items that are
//!    dormant.
//! 3. Builder income per item over its charge life, compared with the cost of a repair, for used
//!    and unused items.
//!
//! Every number in a parameter set is a TEST value for exploring the model, not a decision. Run:
//! `cargo run -p econ-sim --release -- [seeds] [days]`.

use hookwars_common::economy::{bps, licence_split};

/// SplitMix64: small, deterministic, good enough for a model.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform in [0, 1).
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
    /// Uniform in [lo, hi].
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
    /// Poisson with mean `m` (Knuth; `m` is small here).
    fn poisson(&mut self, m: f64) -> u64 {
        let l = (-m).exp();
        let (mut k, mut p) = (0u64, 1.0f64);
        loop {
            p *= self.unit();
            if p <= l {
                return k;
            }
            k += 1;
        }
    }
}

/// One named parameter set. Lamports are lamports; "units" are material units.
#[derive(Clone)]
struct Params {
    name: &'static str,
    season_days: u64,
    // Faucets (R42): drops from verified activity, capped per season.
    cap_a_per_season: u64,
    cap_b_per_season: u64,
    /// Material A per settled lamport of cuts: `num / den`.
    settle_num: u64,
    settle_den: u64,
    /// Material B per raid reveal.
    b_per_raid: u64,
    // Activity.
    traders: u64,
    /// Settled cut lamports per trader per day: uniform in [lo, hi].
    cut_lo: u64,
    cut_hi: u64,
    raiders: u64,
    raid_prob: f64,
    // Sinks.
    crafters: u64,
    craft_prob: f64,
    craft_a: u64,
    craft_b: u64,
    craft_fee: u64,
    repair_a: u64,
    repair_fee: u64,
    charges_restored: u64,
    recipe_protocol_bps: u16,
    // Items.
    max_charges: u64,
    /// Share of crafted items never equipped (no runs, so never wear).
    unused_share: f64,
    /// Runs per day of an equipped item: Poisson mean, low or high usage (half each).
    runs_low: f64,
    runs_high: f64,
    /// Holder royalty per run (after the 2.2 waterfall), lamports.
    royalty_per_run: u64,
    /// Licence sales per equipped item per day (probability) and price.
    licence_prob: f64,
    licence_price: u64,
    licence_protocol_bps: u16,
    author_bps: u16,
    /// Value a holder puts on one unit of material A when deciding to repair.
    material_a_value: u64,
    /// A dormant item not repaired within this many days is retired (stops counting as in use).
    retire_after_days: u64,
}

const BASELINE: Params = Params {
    name: "baseline",
    season_days: 7,
    cap_a_per_season: 20_000,
    cap_b_per_season: 2_000,
    settle_num: 1,
    settle_den: 1_000_000,
    b_per_raid: 1,
    traders: 200,
    cut_lo: 1_000_000,
    cut_hi: 40_000_000,
    raiders: 40,
    raid_prob: 0.3,
    crafters: 30,
    craft_prob: 0.5,
    craft_a: 20,
    craft_b: 2,
    craft_fee: 20_000_000,
    repair_a: 8,
    repair_fee: 5_000_000,
    charges_restored: 500,
    recipe_protocol_bps: 1_000,
    max_charges: 1_000,
    unused_share: 0.3,
    runs_low: 5.0,
    runs_high: 60.0,
    royalty_per_run: 20_000,
    licence_prob: 0.05,
    licence_price: 50_000_000,
    licence_protocol_bps: 500,
    author_bps: 1_000,
    material_a_value: 500_000,
    retire_after_days: 14,
};

fn sets() -> Vec<Params> {
    vec![
        BASELINE,
        Params { name: "tight-caps", cap_a_per_season: 6_000, cap_b_per_season: 600, ..BASELINE },
        Params { name: "loose-caps", cap_a_per_season: 80_000, cap_b_per_season: 8_000, ..BASELINE },
        Params { name: "high-wear", max_charges: 300, charges_restored: 150, ..BASELINE },
        Params { name: "low-activity", traders: 50, raiders: 10, ..BASELINE },
        Params { name: "pricey-repair", repair_fee: 40_000_000, repair_a: 30, ..BASELINE },
        // Caps sized to the sinks: material B (raids) limits crafting, so A's cap is set near
        // what crafts and repairs can burn per season.
        Params { name: "balanced", cap_a_per_season: 1_600, cap_b_per_season: 2_000, ..BASELINE },
        Params { name: "balanced-high-wear", cap_a_per_season: 1_600, max_charges: 300, charges_restored: 300, repair_a: 4, repair_fee: 1_000_000, ..BASELINE },
    ]
}

struct Item {
    runs_mean: f64,
    used: u64,
    dormant_since: Option<u64>,
    retired: bool,
    income: u64,
    repairs: u64,
    repair_cost: u64,
}

struct Holder {
    a: u64,
    b: u64,
}

#[derive(Default, Clone)]
struct SeasonRow {
    supply_a: u64,
    supply_b: u64,
    dropped_a: u64,
    dropped_b: u64,
    burned_a: u64,
    burned_b: u64,
    items_alive: u64,
    items_dormant: u64,
    crafts: u64,
    repairs: u64,
    protocol: u64,
    season_pool: u64,
    holders: u64,
    authors: u64,
}

struct Run {
    seasons: Vec<SeasonRow>,
    /// Over all items that ever wore out: mean income per charge life, mean repair cost.
    used_income_per_life: f64,
    unused_income_per_life: f64,
    repair_cost: f64,
    repaired_share: f64,
    min_supply_after_first_season: u64,
}

fn run(p: &Params, seed: u64, days: u64) -> Run {
    let mut r = Rng(seed.wrapping_mul(0x2545_F491_4F6C_DD1D) ^ 0xA5A5);
    let mut holders: Vec<Holder> = (0..p.crafters).map(|_| Holder { a: 0, b: 0 }).collect();
    let mut items: Vec<Item> = Vec::new();
    let mut seasons: Vec<SeasonRow> = Vec::new();
    let mut row = SeasonRow::default();
    let (mut emitted_a, mut emitted_b) = (0u64, 0u64);
    let (mut supply_a, mut supply_b) = (0u64, 0u64);
    let mut min_supply = u64::MAX;
    let (mut used_income, mut used_lives, mut unused_income, mut unused_lives) = (0u64, 0u64, 0u64, 0u64);
    let (mut decisions, mut repaired) = (0u64, 0u64);
    for day in 0..days {
        if day % p.season_days == 0 {
            emitted_a = 0;
            emitted_b = 0;
        }
        // Faucets: settled cuts and raid reveals, dropped to random crafters (the activity's
        // wallets are the same population here), capped per season.
        for _ in 0..p.traders {
            let cut = r.range(p.cut_lo, p.cut_hi);
            let want = u64::try_from(u128::from(cut) * u128::from(p.settle_num) / u128::from(p.settle_den)).unwrap_or(u64::MAX);
            let amt = want.min(p.cap_a_per_season - emitted_a);
            if amt > 0 {
                emitted_a += amt;
                supply_a += amt;
                row.dropped_a += amt;
                let i = (r.next() % p.crafters) as usize;
                holders[i].a += amt;
            }
        }
        for _ in 0..p.raiders {
            if r.chance(p.raid_prob) {
                let amt = p.b_per_raid.min(p.cap_b_per_season - emitted_b);
                if amt > 0 {
                    emitted_b += amt;
                    supply_b += amt;
                    row.dropped_b += amt;
                    let i = (r.next() % p.crafters) as usize;
                    holders[i].b += amt;
                }
            }
        }
        // Crafts: burn inputs, pay the fee (protocol share, rest to the season pool).
        for h in holders.iter_mut() {
            if h.a >= p.craft_a && h.b >= p.craft_b && r.chance(p.craft_prob) {
                h.a -= p.craft_a;
                h.b -= p.craft_b;
                supply_a -= p.craft_a;
                supply_b -= p.craft_b;
                row.burned_a += p.craft_a;
                row.burned_b += p.craft_b;
                let protocol = bps(p.craft_fee, p.recipe_protocol_bps);
                row.protocol += protocol;
                row.season_pool += p.craft_fee - protocol;
                row.crafts += 1;
                let runs_mean = if r.chance(p.unused_share) {
                    0.0
                } else if r.chance(0.5) {
                    p.runs_low
                } else {
                    p.runs_high
                };
                items.push(Item { runs_mean, used: 0, dormant_since: None, retired: false, income: 0, repairs: 0, repair_cost: 0 });
            }
        }
        // Items run, wear, earn, and are repaired or retired.
        let repair_cost = p.repair_fee + p.repair_a * p.material_a_value;
        for it in items.iter_mut().filter(|i| !i.retired) {
            if it.dormant_since.is_none() && it.runs_mean > 0.0 {
                let runs = r.poisson(it.runs_mean).min(p.max_charges - it.used);
                it.used += runs;
                it.income += runs * p.royalty_per_run;
                row.holders += runs * p.royalty_per_run;
                if r.chance(p.licence_prob) {
                    let (pr, au, ho) = licence_split(p.licence_price, p.licence_protocol_bps, p.author_bps);
                    row.protocol += pr;
                    row.authors += au;
                    row.holders += ho;
                    it.income += ho;
                }
                if it.used >= p.max_charges {
                    it.dormant_since = Some(day);
                    if it.repairs == 0 {
                        used_income += it.income;
                        used_lives += 1;
                    }
                }
            }
            if let Some(since) = it.dormant_since {
                // Repair when the restored charges are expected to earn at least the repair's cost
                // and a holder has the material (any crafter sells it at its value here).
                let expected = p.charges_restored * p.royalty_per_run;
                let seller = holders.iter_mut().find(|h| h.a >= p.repair_a);
                if day == since {
                    decisions += 1;
                }
                if let (true, Some(h)) = (expected >= repair_cost, seller) {
                    h.a -= p.repair_a;
                    supply_a -= p.repair_a;
                    row.burned_a += p.repair_a;
                    let protocol = bps(p.repair_fee, p.recipe_protocol_bps);
                    row.protocol += protocol;
                    row.season_pool += p.repair_fee - protocol;
                    row.repairs += 1;
                    it.repairs += 1;
                    it.repair_cost += repair_cost;
                    it.used = it.used.saturating_sub(p.charges_restored);
                    it.dormant_since = None;
                    if day == since {
                        repaired += 1;
                    }
                } else if day >= since + p.retire_after_days {
                    it.retired = true;
                }
            }
        }
        if day >= p.season_days {
            min_supply = min_supply.min(supply_a);
        }
        if (day + 1) % p.season_days == 0 {
            row.supply_a = supply_a;
            row.supply_b = supply_b;
            row.items_alive = items.iter().filter(|i| !i.retired).count() as u64;
            row.items_dormant = items.iter().filter(|i| !i.retired && i.dormant_since.is_some()).count() as u64;
            seasons.push(std::mem::take(&mut row));
        }
    }
    // Unused items earn only licences they never get (no runs): their income per life is 0.
    for it in &items {
        if it.runs_mean == 0.0 {
            unused_income += it.income;
            unused_lives += 1;
        }
    }
    let mean = |a: u64, n: u64| if n == 0 { 0.0 } else { a as f64 / n as f64 };
    Run {
        seasons,
        used_income_per_life: mean(used_income, used_lives),
        unused_income_per_life: mean(unused_income, unused_lives),
        repair_cost: (p.repair_fee + p.repair_a * p.material_a_value) as f64,
        repaired_share: mean(repaired, decisions),
        min_supply_after_first_season: if min_supply == u64::MAX { 0 } else { min_supply },
    }
}

fn stats(v: &[f64]) -> (f64, f64, f64) {
    let n = v.len().max(1) as f64;
    let mean = v.iter().sum::<f64>() / n;
    let min = v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (mean, min, max)
}

fn sol(l: f64) -> f64 {
    l / 1e9
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seeds: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(16);
    let days: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(84);
    println!("units econ-sim | seeds 1..={seeds} | days {days} | every parameter is a TEST value");
    for p in sets() {
        let runs: Vec<Run> = (1..=seeds).map(|s| run(&p, s, days)).collect();
        let n_seasons = runs[0].seasons.len();
        println!();
        println!(
            "== {} | caps A/B {}/{} per {}-day season | craft {}A+{}B + {} SOL | repair {}A + {} SOL restores {} of {} charges",
            p.name,
            p.cap_a_per_season,
            p.cap_b_per_season,
            p.season_days,
            p.craft_a,
            p.craft_b,
            sol(p.craft_fee as f64),
            p.repair_a,
            sol(p.repair_fee as f64),
            p.charges_restored,
            p.max_charges
        );
        println!("season | supply A mean (min..max) | supply B mean | dropped A | burned A | crafts | repairs | items alive | dormant share | protocol SOL | season pool SOL | holders SOL | authors SOL");
        for s in 0..n_seasons {
            let col = |f: &dyn Fn(&SeasonRow) -> f64| stats(&runs.iter().map(|r| f(&r.seasons[s])).collect::<Vec<_>>());
            let (sa, sa_min, sa_max) = col(&|x| x.supply_a as f64);
            let (sb, _, _) = col(&|x| x.supply_b as f64);
            let (da, _, _) = col(&|x| x.dropped_a as f64);
            let (ba, _, _) = col(&|x| x.burned_a as f64);
            let (cr, _, _) = col(&|x| x.crafts as f64);
            let (rp, _, _) = col(&|x| x.repairs as f64);
            let (al, _, _) = col(&|x| x.items_alive as f64);
            let (ds, _, _) = col(&|x| if x.items_alive == 0 { 0.0 } else { x.items_dormant as f64 / x.items_alive as f64 });
            let (pr, _, _) = col(&|x| x.protocol as f64);
            let (sp, _, _) = col(&|x| x.season_pool as f64);
            let (ho, _, _) = col(&|x| x.holders as f64);
            let (au, _, _) = col(&|x| x.authors as f64);
            println!(
                "{:>6} | {:>8.0} ({:.0}..{:.0}) | {:>7.0} | {:>9.0} | {:>8.0} | {:>6.1} | {:>7.1} | {:>11.1} | {:>13.3} | {:>12.3} | {:>15.3} | {:>11.3} | {:>11.3}",
                s + 1,
                sa,
                sa_min,
                sa_max,
                sb,
                da,
                ba,
                cr,
                rp,
                al,
                ds,
                sol(pr),
                sol(sp),
                sol(ho),
                sol(au)
            );
        }
        let (ui, _, _) = stats(&runs.iter().map(|r| r.used_income_per_life).collect::<Vec<_>>());
        let (ni, _, _) = stats(&runs.iter().map(|r| r.unused_income_per_life).collect::<Vec<_>>());
        let (rs, _, _) = stats(&runs.iter().map(|r| r.repaired_share).collect::<Vec<_>>());
        let (ms, ms_min, _) = stats(&runs.iter().map(|r| r.min_supply_after_first_season as f64).collect::<Vec<_>>());
        let last = n_seasons - 1;
        let growth = stats(
            &runs
                .iter()
                .map(|r| r.seasons[last].supply_a as f64 - r.seasons[last - 1].supply_a as f64)
                .collect::<Vec<_>>(),
        );
        println!(
            "summary | used item income per charge life {:.3} SOL vs repair cost {:.3} SOL | unused item income per life {:.3} SOL | worn items repaired {:.0}% | min A supply after season 1: mean {:.0}, lowest {:.0} | A supply change in the last season: mean {:.0} (min {:.0}, max {:.0})",
            sol(ui),
            sol(runs[0].repair_cost),
            sol(ni),
            rs * 100.0,
            ms,
            ms_min,
            growth.0,
            growth.1,
            growth.2
        );
    }
}
