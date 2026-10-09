// Changed by Hookwars: new file.
//! The war program: its config and the timelock, `init_war`, funding, and bounties paid through
//! `touch` (05 sections 4, 5, 7, 11).

use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_launch::state::LaunchRules;
use bordrless_token::client as token;
use hookwars_war::client as war;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use solana_signer::Signer;

#[test]
fn the_config_is_made_once_by_the_upgrade_authority() {
    let mut ww = WarWorld::new();
    let c = ww.config();
    assert_eq!(c.params, TEST_PARAMS);
    assert_eq!(c.current_season, 0);
    // A second init fails (the account exists); a stranger cannot make it either.
    ww.init_config(TEST_PARAMS).expect_fail();
}

#[test]
fn a_stranger_cannot_init_the_config() {
    let mut ww = WarWorld::new();
    let stranger = ww.w.env.funded(SOL);
    // The config already exists in the world, so check the authority rule on its own:
    // the ProgramData's authority is the deployer, not the stranger.
    let args = WarWorld::config_args(TEST_PARAMS, stranger.pubkey(), stranger.pubkey());
    let tx = ww
        .w
        .env
        .send(&[war::init_config(stranger.pubkey(), args)], &[&stranger]);
    tx.expect_fail();
}

#[test]
fn config_changes_wait_for_the_timelock_and_can_be_cancelled() {
    let mut ww = WarWorld::new();
    let admin = ww.w.env.deployer.insecure_clone();
    let mut next = TEST_PARAMS;
    next.siege_max_spend_bps = 500;
    let args = WarWorld::config_args(next, admin.pubkey(), ww.w.env.treasury.pubkey());
    let tx = ww.w.env.send(&[war::propose_config(admin.pubkey(), args)], &[&admin]);
    let proposed = tx.event::<ConfigProposed>();
    assert_eq!(proposed.eta, ww.now() + TEST_PARAMS.admin_timelock_secs);
    // Too early: refused.
    ww.w.env
        .send(&[war::apply_config()], &[])
        .expect_code(war_code(WarError::TimelockNotPassed));
    // A stranger cannot propose.
    let stranger = ww.w.env.funded(SOL);
    ww.w.env
        .send(&[war::propose_config(stranger.pubkey(), args)], &[&stranger])
        .expect_code(war_code(WarError::NotAdmin));
    // After the eta, anyone applies.
    ww.w.env.warp(TEST_PARAMS.admin_timelock_secs);
    ww.w.env.send(&[war::apply_config()], &[]).ok();
    assert_eq!(ww.config().params.siege_max_spend_bps, 500);
    assert!(ww.config().pending.is_none());
    // A second proposal, cancelled by the admin: nothing applies.
    let tx = ww.w.env.send(&[war::propose_config(admin.pubkey(), args)], &[&admin]);
    tx.ok();
    ww.w.env
        .send(&[war::cancel_pending(admin.pubkey(), 0, 0)], &[&admin])
        .ok();
    ww.w.env
        .send(&[war::apply_config()], &[])
        .expect_code(war_code(WarError::NothingPending));
}

#[test]
fn invalid_params_are_refused() {
    let mut ww = WarWorld::new();
    let admin = ww.w.env.deployer.insecure_clone();
    let mut bad = TEST_PARAMS;
    bad.max_crank_bounty_bps = 101; // above the companion's ceiling
    let args = WarWorld::config_args(bad, admin.pubkey(), admin.pubkey());
    ww.w.env
        .send(&[war::propose_config(admin.pubkey(), args)], &[&admin])
        .expect_code(war_code(WarError::InvalidParams));
    let mut bad = TEST_PARAMS;
    bad.season_secs = 0;
    let args = WarWorld::config_args(bad, admin.pubkey(), admin.pubkey());
    ww.w.env
        .send(&[war::propose_config(admin.pubkey(), args)], &[&admin])
        .expect_code(war_code(WarError::InvalidParams));
}

#[test]
fn init_war_needs_a_war_slot_and_runs_once() {
    let mut ww = WarWorld::new();
    let plain = ww.launch("PLAIN", LaunchRules::NONE);
    let payer = ww.w.env.payer.insecure_clone();
    ww.w.env
        .send(&[war::init_war(payer.pubkey(), plain)], &[])
        .expect_code(war_code(WarError::MissingWarSlot));
    let t = ww.war_token("WAR", OrdersSpec::default());
    let s = ww.state(&t.mint);
    assert_eq!(s.mint, t.mint);
    assert_eq!(s.funded_total, 0);
    // The chest's and the inbox's holdings exist, empty.
    assert_eq!(ww.chest_balance(&t.mint), 0);
    // A second init fails.
    ww.w.env
        .send(&[war::init_war(payer.pubkey(), t.mint)], &[])
        .expect_fail();
}

#[test]
fn funding_is_counted_from_the_balance_whoever_sends_it() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("FUND", OrdersSpec::default());
    ww.fund_chest(&t.mint, 2 * SOL);
    let s = ww.state(&t.mint);
    assert_eq!(s.funded_total, 2 * SOL);
    ww.assert_solvent(&t.mint);
    // Another plain transfer, then a record: counted once.
    ww.fund_chest(&t.mint, SOL);
    let tx = ww.w.env.send(&[war::record_funding(t.mint)], &[]);
    let e = tx.event::<WarFunded>();
    assert_eq!(e.amount, 0);
    assert_eq!(ww.state(&t.mint).funded_total, 3 * SOL);
    ww.assert_solvent(&t.mint);
}

#[test]
fn a_bounty_spends_raid_points_through_touch_and_pays_sol() {
    let mut ww = WarWorld::new();
    let orders = OrdersSpec {
        bounty_rate: 1_000_000, // TEST: 0.001 SOL a point
        ..OrdersSpec::default()
    };
    let t = ww.war_token("BNTY", orders);
    ww.fund_chest(&t.mint, 5 * SOL);
    let holder = ww.buyer(&t.mint, SOL);
    let season = ww.config().current_season;
    ww.set_raid(&t.mint, &holder.pubkey(), season, 300, 0);
    let sol_before = ww.w.env.lamports(&holder.pubkey());
    let ix = ww.bounty_ix(&holder.pubkey(), &t);
    let tx = ww.w.env.send(&[ix], &[&holder]);
    let e = tx.event::<BountyClaimed>();
    // 300 points are worth 0.3 SOL; one claim pays at most `bounty_max_per_claim` (TEST 0.1 SOL),
    // so it spends exactly the 100 points that pays for.
    assert_eq!((e.points, e.paid), (100, TEST_PARAMS.bounty_max_per_claim));
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).1, 200);
    assert_eq!(ww.w.env.lamports(&holder.pubkey()), sol_before + TEST_PARAMS.bounty_max_per_claim);
    ww.assert_solvent(&t.mint);
    // Two more claims spend the rest.
    for _ in 0..2 {
        let ix = ww.bounty_ix(&holder.pubkey(), &t);
        ww.w.env.send(&[ix], &[&holder]).ok();
    }
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).1, 0);
    assert_eq!(ww.state(&t.mint).paid_bounties, 300_000_000);
    ww.assert_solvent(&t.mint);
    // Nothing left to claim.
    let ix = ww.bounty_ix(&holder.pubkey(), &t);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::NothingToDo));
}

#[test]
fn a_bounty_is_capped_per_claim_and_by_the_chest() {
    let mut ww = WarWorld::new();
    let orders = OrdersSpec {
        bounty_rate: 1_000_000,
        ..OrdersSpec::default()
    };
    let t = ww.war_token("CAP", orders);
    ww.fund_chest(&t.mint, 50_000_000); // less than the claim's worth
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 0, 1_000, 0);
    let ix = ww.bounty_ix(&holder.pubkey(), &t);
    let e = ww.w.env.send(&[ix], &[&holder]).event::<BountyClaimed>();
    // The chest held 0.05 SOL: 50 points paid, 950 left.
    assert_eq!((e.points, e.paid), (50, 50_000_000));
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).1, 950);
    ww.assert_solvent(&t.mint);
}

#[test]
fn points_of_another_season_count_zero() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("OLD", OrdersSpec::default());
    ww.fund_chest(&t.mint, SOL);
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 7, 500, 0);
    let ix = ww.bounty_ix(&holder.pubkey(), &t);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::NothingToDo));
}

#[test]
fn only_the_war_signer_can_spend_raid_points() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("SIGN", OrdersSpec::default());
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 0, 100, 0);
    // The holder touches its own holding directly with a war payload: the Raid item refuses.
    let mut payload = Vec::new();
    anchor_lang::AnchorSerialize::serialize(
        &hookwars_war::common::WarTouch::SpendRaidPoints { amount: 100 },
        &mut payload,
    )
    .unwrap();
    let ix = token::touch(
        holder.pubkey(),
        t.mint,
        token::holding_address(&t.mint, &holder.pubkey()),
        bordrless_token::constants::ITEMS_ID,
        RAID_SLOT,
        payload,
        vec![],
    );
    ww.w.env.send(&[ix], &[&holder]).expect_fail();
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).1, 100);
}

#[test]
fn a_bounty_names_the_raid_slot_or_fails() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("SLOT", OrdersSpec::default());
    ww.fund_chest(&t.mint, SOL);
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 0, 100, 0);
    // Slot 0 is the War slot, not a Raid slot.
    let inner = [bordrless_bridge::client::unwrap_sol(WarWorld::chest(&t.mint), 0)];
    let ix = war::claim_bounty(holder.pubkey(), t.mint, t.orders, 0, vec![], &inner);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::WrongRaidSlot));
}

#[test]
fn war_orders_must_be_the_ones_the_war_slot_names() {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ORDR", OrdersSpec::default());
    ww.fund_chest(&t.mint, SOL);
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 0, 100, 0);
    // Another War orders item, not equipped: refused.
    let rich = OrdersSpec {
        bounty_rate: 1_000_000_000,
        ..OrdersSpec::default()
    };
    let other = ww.put_item(WAR_ORDERS_TEMPLATE, rich.params());
    let orders = war::Orders {
        item: other,
        template: t.orders.template,
    };
    let inner = [bordrless_bridge::client::unwrap_sol(WarWorld::chest(&t.mint), 0)];
    let ix = war::claim_bounty(holder.pubkey(), t.mint, orders, RAID_SLOT, vec![], &inner);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::WrongWarOrders));
}
