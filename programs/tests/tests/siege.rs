// Changed by Hookwars: new file.
//! `siege`, `raze` and `return_captured` (05 sections 6.2, 6.4, 6.5): due and not due, the rival
//! premium wait, the holder-reward refusal (R10), the sandwich bound, the raze rate limit, peace,
//! and the chest's solvency after every step.

use bordrless_hook::slot_kind;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_token::client as token;
use hookwars_war::client as war;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use solana_signer::Signer;

/// A war token, a rival launch with some trading, the ledger showing `volume` raided from the
/// rival, flat observations at the rival's price, and `chest` lamports in the chest.
fn setup(volume: u64, chest: u64) -> (WarWorld, WarToken, anchor_lang::prelude::Pubkey) {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ATK", OrdersSpec::default());
    let rival = ww.launch("RIV", LaunchRules::NONE);
    ww.buyer(&rival, SOL);
    ww.w.env.warp(120);
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, volume, &[(rival, now, volume, 0)]);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    ww.fund_chest(&t.mint, chest);
    (ww, t, rival)
}

/// The threshold of the default orders: `siege_threshold * siege_unit_lamports`.
fn threshold() -> u64 {
    u64::from(OrdersSpec::default().siege_threshold) * TEST_PARAMS.siege_unit_lamports
}

#[test]
fn a_siege_buys_the_rival_and_keeps_it() {
    let (mut ww, t, rival) = setup(threshold(), 10 * SOL);
    let (tx, cranker) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    let e = tx.event::<SiegeExecuted>();
    assert!(e.bought > 0 && e.spent > 0);
    assert_eq!(e.cranker, cranker.pubkey());
    // The spend is capped by the companion's pool share, far below 10% of the chest here.
    assert!(e.spent < SOL);
    let s = ww.state(&t.mint);
    assert_eq!(s.season.sieges, 1);
    assert_eq!(s.spent_siege, e.spent);
    assert_eq!(s.captured[0].rival_mint, rival);
    assert_eq!(s.captured[0].amount, e.bought);
    assert_eq!(ww.w.env.holding(&rival, &WarWorld::chest(&t.mint)), e.bought);
    ww.assert_solvent(&t.mint);
    println!("siege: spent {} bought {} bounty {} cu {}", e.spent, e.bought, e.bounty, tx.cu());
}

#[test]
fn a_siege_waits_for_enough_raid_volume_and_its_interval() {
    let (mut ww, t, rival) = setup(threshold() - 1, 10 * SOL);
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    tx.expect_code(war_code(WarError::SiegeNotDue));
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, threshold(), &[(rival, now, threshold(), 0)]);
    ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None)).0.ok();
    // Again at once: the interval has not passed.
    ww.w.env.warp(1);
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    tx.expect_code(war_code(WarError::SiegeNotDue));
    ww.assert_solvent(&t.mint);
}

#[test]
fn raid_volume_rolls_out_of_the_window() {
    let (mut ww, t, rival) = setup(threshold(), 10 * SOL);
    // Two windows later the raid is gone.
    ww.w.env.warp(2 * TEST_PARAMS.raid_window_secs);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    tx.expect_code(war_code(WarError::SiegeNotDue));
}

#[test]
fn a_siege_waits_while_the_rival_trades_above_its_twap() {
    let (mut ww, t, rival) = setup(threshold(), 10 * SOL);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    // The TWAP is half the spot: far above the premium.
    ww.flat_observations(&pool, spot / 2, 3_600);
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    let e = tx.event::<SiegeWaited>();
    assert_eq!(e.rival_twap, spot / 2);
    let s = ww.state(&t.mint);
    assert_eq!(s.season.sieges, 0);
    assert_eq!(s.spent_siege, 0);
    ww.assert_solvent(&t.mint);
}

#[test]
fn a_siege_needs_observations() {
    let (mut ww, t, rival) = setup(threshold(), 10 * SOL);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    // History shorter than the orders' window.
    ww.flat_observations(&pool, spot, 60);
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    tx.expect_code(war_code(WarError::NoObservations));
}

#[test]
fn a_token_cannot_besiege_itself() {
    let (mut ww, t, _) = setup(threshold(), 10 * SOL);
    let mint = t.mint;
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &mint, false, None));
    tx.expect_code(war_code(WarError::SelfSiege));
}

#[test]
fn a_rival_with_holder_rewards_cannot_be_besieged() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ATK", OrdersSpec::default());
    let rules = LaunchRules {
        holder_fee_buy_bps: 100,
        holder_fee_sell_bps: 100,
        ..LaunchRules::NONE
    };
    let rival = ww.launch("RWD", rules);
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, threshold(), &[(rival, now, threshold(), 0)]);
    ww.fund_chest(&t.mint, 10 * SOL);
    let kit = bordrless_kit::client::kit_config_address(&rival);
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, Some(kit)));
    tx.expect_code(war_code(WarError::SiegeTargetHasRewards));
    // Without its kit config the step cannot tell: refused too.
    let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
    tx.expect_code(war_code(WarError::MissingAccount));
}

#[test]
fn a_siege_marks_a_besieged_war_token() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ATK", OrdersSpec::default());
    let r = ww.war_token("DEF", OrdersSpec::default());
    ww.buyer(&r.mint, SOL);
    ww.w.env.warp(120);
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, threshold(), &[(r.mint, now, threshold(), 0)]);
    let spot = ww.spot(&r.pool);
    ww.flat_observations(&r.pool, spot, 3_600);
    ww.fund_chest(&t.mint, 10 * SOL);
    let rival = r.mint;
    ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, true, None)).0.ok();
    let d = ww.state(&r.mint);
    assert_eq!(d.siege_by_chest, WarWorld::chest(&t.mint));
    assert_eq!(d.under_siege_until, ww.now() + TEST_PARAMS.siege_interval_secs);
    assert_eq!(d.season.times_besieged, 1);
    ww.assert_solvent(&t.mint);
    ww.assert_solvent(&r.mint);
}

/// The companion's sandwich argument (05 6.2, upstream `steps.rs` 321-325) on a siege: an attacker
/// who buys the rival right before the siege and sells right after ends with less bridged SOL.
#[test]
fn a_sandwich_around_a_siege_loses() {
    for pump in [SOL / 10, SOL / 2, SOL] {
        let (mut ww, t, rival) = setup(threshold(), 50 * SOL);
        let attacker = ww.w.wallet_with_sol(10 * SOL);
        let before = ww.w.env.holding(&ww.w.sol, &attacker.pubkey());
        ww.w.buy(&attacker, &rival, pump).ok();
        let (tx, _) = ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None));
        let sieged = tx.events::<SiegeExecuted>().len() == 1;
        let held = ww.w.env.holding(&rival, &attacker.pubkey());
        ww.w.sell(&attacker, &rival, held).ok();
        let after = ww.w.env.holding(&ww.w.sol, &attacker.pubkey());
        println!("sandwich pump {pump}: sieged {sieged}, attacker {before} -> {after}");
        assert!(after <= before, "a sandwich of {pump} gained {}", after - before);
        ww.assert_solvent(&t.mint);
    }
}

fn besieged() -> (WarWorld, WarToken, anchor_lang::prelude::Pubkey) {
    let (mut ww, t, rival) = setup(threshold(), 10 * SOL);
    ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None)).0.ok();
    (ww, t, rival)
}

#[test]
fn a_raze_sells_captured_tokens_at_the_window_rate() {
    let (mut ww, t, rival) = besieged();
    let captured = ww.state(&t.mint).captured[0].amount;
    let (tx, _) = ww.crank(|c, ww| ww.raze_ix(c, &t, &rival));
    let e = tx.event::<Razed>();
    // At most a quarter (TEST raze_max_bps_per_interval) of what was held at the window's start.
    assert!(e.sold > 0 && e.sold <= captured * u64::from(TEST_PARAMS.raze_max_bps_per_interval) / 10_000);
    assert!(e.got > 0);
    assert_eq!(e.captured_left, captured - e.sold);
    ww.assert_solvent(&t.mint);
    // The window's allowance is used: refused until the next window (or the pool share caps it).
    let mut refused = false;
    for _ in 0..8 {
        ww.w.env.warp(1);
        let (tx, _) = ww.crank(|c, ww| ww.raze_ix(c, &t, &rival));
        if tx.result.is_err() {
            tx.expect_code(war_code(WarError::RazeLimit));
            refused = true;
            break;
        }
    }
    assert!(refused, "the window's allowance never ran out");
    ww.w.env.warp(TEST_PARAMS.raze_interval_secs);
    ww.crank(|c, ww| ww.raze_ix(c, &t, &rival)).0.ok();
    ww.assert_solvent(&t.mint);
}

#[test]
fn a_raze_needs_the_orders_to_allow_it() {
    let mut ww = WarWorld::new();
    let orders = OrdersSpec {
        raze_enabled: 0,
        ..OrdersSpec::default()
    };
    let t = ww.war_token("NORZ", orders);
    let rival = ww.launch("RIV", LaunchRules::NONE);
    let (tx, _) = ww.crank(|c, ww| ww.raze_ix(c, &t, &rival));
    tx.expect_code(war_code(WarError::RazeDisabled));
}

#[test]
fn peace_returns_captured_tokens_under_a_treaty_both_equip() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ATK", OrdersSpec::default());
    let r = ww.war_token("DEF", OrdersSpec::default());
    ww.buyer(&r.mint, SOL);
    ww.w.env.warp(120);
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, threshold(), &[(r.mint, now, threshold(), 0)]);
    let spot = ww.spot(&r.pool);
    ww.flat_observations(&r.pool, spot, 3_600);
    ww.fund_chest(&t.mint, 10 * SOL);
    let rival = r.mint;
    ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, true, None)).0.ok();
    let captured = ww.state(&t.mint).captured[0].amount;

    let mut params = [0u32; hookwars_war::foreign::PARAM_FIELDS];
    params[2] = 1; // returns_captured
    let treaty = ww.put_item(TREATY_TEMPLATE, params);
    let treaty_template = hookwars_war::foreign::template_address(TREATY_TEMPLATE);
    let peace = |ww: &WarWorld, c: &anchor_lang::prelude::Pubkey| {
        let chest = WarWorld::chest(&t.mint);
        let rival_chest = WarWorld::chest(&rival);
        let inner = [
            token::create_holding(*c, rival, rival_chest),
            token::transfer_from_protocol(
                chest,
                token::holding_address(&rival, &chest),
                token::holding_address(&rival, &rival_chest),
                rival,
                vec![],
                1,
                hookwars_war::ID,
                vec![],
            ),
        ];
        let _ = ww;
        war::return_captured(*c, t.mint, rival, treaty, treaty_template, vec![], &inner)
    };
    // Only one side equips it: no treaty.
    ww.add_slot(&t.mint, WarWorld::named_slot(slot_kind::RELATION, treaty));
    let (tx, _) = ww.crank(|c, ww| peace(ww, c));
    tx.expect_code(war_code(WarError::NoTreaty));
    // Both equip it: returned.
    ww.add_slot(&rival, WarWorld::named_slot(slot_kind::RELATION, treaty));
    let (tx, _) = ww.crank(|c, ww| peace(ww, c));
    let e = tx.event::<CapturedReturned>();
    assert_eq!(e.amount, captured);
    assert_eq!(ww.w.env.holding(&rival, &WarWorld::chest(&rival)), captured);
    assert_eq!(ww.w.env.holding(&rival, &WarWorld::chest(&t.mint)), 0);
    assert!(ww.state(&t.mint).captured[0].is_free());
    ww.assert_solvent(&t.mint);
}

#[test]
fn peace_needs_a_treaty_that_returns_captured_holdings() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ATK", OrdersSpec::default());
    let rival = ww.launch("RIV", LaunchRules::NONE);
    let params = [0u32; hookwars_war::foreign::PARAM_FIELDS]; // returns_captured 0
    let treaty = ww.put_item(TREATY_TEMPLATE, params);
    ww.add_slot(&t.mint, WarWorld::named_slot(slot_kind::RELATION, treaty));
    ww.add_slot(&rival, WarWorld::named_slot(slot_kind::RELATION, treaty));
    let template = hookwars_war::foreign::template_address(TREATY_TEMPLATE);
    let (tx, _) = ww.crank(|c, _| war::return_captured(*c, t.mint, rival, treaty, template, vec![], &[]));
    tx.expect_code(war_code(WarError::PeaceReturnsOff));
}
