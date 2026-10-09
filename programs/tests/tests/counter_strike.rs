// Changed by Hookwars: new file.
//! `counter_strike` (05 section 6.3): after a fall, the chest buys its own token on its launch pool
//! and burns it; due and not due, the companion's limits, the holder-reward refusal (section 3),
//! the sandwich bound, the chest's solvency.

use bordrless_hook::slot_kind;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_token::state::{Mint, Slot};
use hookwars_war::client as war;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use solana_signer::Signer;

/// A war token with trading, past its launch's first minute, `chest` lamports in its chest and
/// observations where the price fell from twice the spot to the spot `short` seconds ago.
fn fallen(chest: u64) -> (WarWorld, WarToken) {
    let mut ww = WarWorld::new();
    let t = ww.war_token("CTR", OrdersSpec::default());
    ww.buyer(&t.mint, 2 * SOL);
    ww.w.env.warp(3_600);
    let spot = ww.spot(&t.pool);
    let o = OrdersSpec::default();
    ww.falling_observations(
        &t.pool,
        2 * spot,
        spot,
        i64::from(o.counter_short_secs),
        i64::from(o.counter_long_secs),
    );
    ww.fund_chest(&t.mint, chest);
    (ww, t)
}

#[test]
fn a_counter_strike_buys_back_and_burns_after_a_fall() {
    let (mut ww, t) = fallen(10 * SOL);
    let supply = ww.w.env.read::<Mint>(&t.mint).supply;
    let (tx, cranker) = ww.crank(|c, ww| ww.counter_ix(c, &t));
    let e = tx.event::<CounterStrikeExecuted>();
    assert!(e.spent > 0 && e.burned > 0);
    assert_eq!(e.cranker, cranker.pubkey());
    assert_eq!(ww.w.env.read::<Mint>(&t.mint).supply, supply - e.burned);
    assert_eq!(ww.w.env.holding(&t.mint, &WarWorld::chest(&t.mint)), 0);
    let s = ww.state(&t.mint);
    assert_eq!(s.season.counter_strikes, 1);
    assert_eq!(s.spent_counter, e.spent);
    ww.assert_solvent(&t.mint);
    println!("counter-strike: spent {} burned {} bounty {} cu {}", e.spent, e.burned, e.bounty, tx.cu());
}

#[test]
fn no_counter_strike_without_a_fall() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("FLAT", OrdersSpec::default());
    ww.buyer(&t.mint, SOL);
    ww.w.env.warp(3_600);
    let spot = ww.spot(&t.pool);
    ww.flat_observations(&t.pool, spot, 3_600);
    ww.fund_chest(&t.mint, 10 * SOL);
    let (tx, _) = ww.crank(|c, ww| ww.counter_ix(c, &t));
    tx.expect_code(war_code(WarError::CounterNotDue));
    ww.assert_solvent(&t.mint);
}

#[test]
fn counter_strikes_keep_their_interval() {
    let (mut ww, t) = fallen(10 * SOL);
    ww.crank(|c, ww| ww.counter_ix(c, &t)).0.ok();
    ww.w.env.warp(1);
    let spot = ww.spot(&t.pool);
    let o = OrdersSpec::default();
    ww.falling_observations(&t.pool, 2 * spot, spot, i64::from(o.counter_short_secs), i64::from(o.counter_long_secs));
    let (tx, _) = ww.crank(|c, ww| ww.counter_ix(c, &t));
    tx.expect_code(war_code(WarError::CounterNotDue));
    ww.w.env.warp(i64::from(o.counter_interval_secs));
    let spot = ww.spot(&t.pool);
    ww.falling_observations(&t.pool, 2 * spot, spot, i64::from(o.counter_short_secs), i64::from(o.counter_long_secs));
    ww.crank(|c, ww| ww.counter_ix(c, &t)).0.ok();
    assert_eq!(ww.state(&t.mint).season.counter_strikes, 2);
    ww.assert_solvent(&t.mint);
}

#[test]
fn no_counter_strike_in_the_launchs_first_minute() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("NEW", OrdersSpec::default());
    let spot = ww.spot(&t.pool);
    let o = OrdersSpec::default();
    ww.falling_observations(&t.pool, 2 * spot, spot, i64::from(o.counter_short_secs), i64::from(o.counter_long_secs));
    ww.fund_chest(&t.mint, 10 * SOL);
    let (tx, _) = ww.crank(|c, ww| ww.counter_ix(c, &t));
    tx.expect_code(war_code(WarError::CounterNotDue));
}

#[test]
fn no_counter_strike_when_the_orders_spend_nothing() {
    let mut ww = WarWorld::new();
    let orders = OrdersSpec {
        counter_spend_bps: 0,
        ..OrdersSpec::default()
    };
    let t = ww.war_token("OFF", orders);
    let (tx, _) = ww.crank(|c, ww| ww.counter_ix(c, &t));
    tx.expect_code(war_code(WarError::NoWarOrders));
}

#[test]
fn a_token_with_holder_rewards_cannot_counter_strike() {
    let mut ww = WarWorld::new();
    let rules = LaunchRules {
        holder_fee_buy_bps: 100,
        holder_fee_sell_bps: 100,
        ..LaunchRules::NONE
    };
    let mint = ww.launch("RWD", rules);
    // The kit in a Locked slot (R8), as a Hookwars launch carries it, then the war slots.
    ww.add_slot(
        &mint,
        Slot {
            kind: slot_kind::LOCKED,
            program: bordrless_kit::ID,
            data_len: 32,
            data_epoch: 1,
            ..Slot::default()
        },
    );
    let item = ww.put_item(WAR_ORDERS_TEMPLATE, OrdersSpec::default().params());
    ww.add_slot(&mint, WarWorld::war_slot(item));
    let payer = ww.w.env.payer.insecure_clone();
    ww.w.env.send(&[war::init_war(payer.pubkey(), mint)], &[]).ok();
    let orders = war::Orders {
        item,
        template: hookwars_war::foreign::Template::address(WAR_ORDERS_TEMPLATE),
    };
    let pool = ww.w.launch_pool_key(&mint);
    let kit = bordrless_kit::client::kit_config_address(&mint);
    let (tx, _) = ww.crank(|c, _| war::counter_strike(*c, mint, orders, pool, Some(kit), vec![], vec![], &[]));
    tx.expect_code(war_code(WarError::OwnTokenHasRewards));
}

#[test]
fn a_counter_strike_needs_observations_over_its_long_window() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("SHORT", OrdersSpec::default());
    ww.w.env.warp(3_600);
    let spot = ww.spot(&t.pool);
    ww.flat_observations(&t.pool, spot, 600); // shorter than counter_long_secs
    let (tx, _) = ww.crank(|c, ww| ww.counter_ix(c, &t));
    tx.expect_code(war_code(WarError::NoObservations));
}

/// The companion's sandwich bound on the token's own pool: a front-runner who buys before the
/// counter-strike and sells after ends with less bridged SOL.
#[test]
fn a_sandwich_around_a_counter_strike_loses() {
    for pump in [SOL / 10, SOL / 2, 2 * SOL] {
        let (mut ww, t) = fallen(50 * SOL);
        let attacker = ww.w.wallet_with_sol(10 * SOL);
        let before = ww.w.env.holding(&ww.w.sol, &attacker.pubkey());
        ww.w.buy(&attacker, &t.mint, pump).ok();
        let (tx, _) = ww.crank(|c, ww| ww.counter_ix(c, &t));
        let struck = tx.result.is_ok();
        let held = ww.w.env.holding(&t.mint, &attacker.pubkey());
        ww.w.sell(&attacker, &t.mint, held).ok();
        let after = ww.w.env.holding(&ww.w.sol, &attacker.pubkey());
        println!("counter sandwich pump {pump}: struck {struck}, attacker {before} -> {after}");
        assert!(after <= before, "a sandwich of {pump} gained {}", after - before);
        ww.assert_solvent(&t.mint);
    }
}
