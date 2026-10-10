// Changed by Hookwars: new file, regression tests for security review 1
// (~/ideas/hookwars/SECURITY-REVIEW-1.md): each test runs the attack the review describes and
// shows it now fails.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::armory::*;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::{self as wh, OrdersSpec, WarWorld};
use bordrless_token::state::Holding;
use hookwars_armory::error::ArmoryError as E;
use hookwars_armory::state::{proposal_status as S, Proposal};
use hookwars_common::{ids, pda, template_id as T, EquipConfig, PerformanceRule};
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use hookwars_war::state::WarState;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn war_params(threshold: u32) -> hookwars_common::Params {
    params(&[threshold, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10])
}

/// A token with War orders `a` in slot 0, owner holding 1,000 of a 1,000 supply, and a passed
/// proposal for War orders `b`. Answers the owner, the mint, `b` and the proposal.
fn passed_proposal(hw: &mut Hw) -> (Keypair, Pubkey, Pubkey, Pubkey) {
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, a, _) = hw.item(T::WAR_ORDERS, war_params(10), 0);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(a), EquipConfig::default()))
        .ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    let (_, b, _) = hw.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, proposal) = hw.propose(&owner, &mint, 0, Some(b), EquipConfig::default());
    tx.ok();
    hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::PASSED);
    (owner, mint, b, proposal)
}

/// `fail_stale` with the given optional accounts.
fn fail_stale_ix(
    proposal: Pubkey,
    p: &Proposal,
    item: Option<Pubkey>,
    template: Option<Pubkey>,
    program: Option<Pubkey>,
    programdata: Option<Pubkey>,
) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::FailStale {
            config: pda::config().0,
            proposal,
            slot_state: pda::slot_state(&p.mint, p.slot).0,
            token_mint: p.mint,
            item,
            template,
            template_program: program,
            template_programdata: programdata,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::FailStale {},
    )
}

// ---------------------------------------------------------------------------------------- H-1

#[test]
fn h1_fail_stale_without_the_proposals_accounts_is_an_error_not_staleness() {
    let mut hw = Hw::new();
    let (owner, _mint, _b, proposal) = passed_proposal(&mut hw);
    let p: Proposal = hw.w.env.read(&proposal);
    let stranger = hw.w.env.funded(1_000_000_000);
    // The review's proof of concept: every optional account left out.
    let ix = fail_stale_ix(proposal, &p, None, None, None, None);
    hw.w.env
        .send_paid_by(&[ix], &stranger, &[])
        .expect_code(armory_code(E::WrongAccount));
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::PASSED);
    // The holders' vote stands: it executes after its notice.
    hw.w.env.warp(600);
    hw.execute(&owner, &proposal).ok();
}

#[test]
fn h1_fail_stale_with_a_forged_programdata_is_refused() {
    let mut hw = Hw::new();
    let (_owner, _mint, b, proposal) = passed_proposal(&mut hw);
    let p: Proposal = hw.w.env.read(&proposal);
    let template = pda::template(hw.read_item(&b).template_id).0;
    let stranger = hw.w.env.funded(1_000_000_000);
    let ix = fail_stale_ix(
        proposal,
        &p,
        Some(b),
        Some(template),
        Some(ids::ITEMS_ID),
        Some(Pubkey::new_unique()),
    );
    let tx = hw.w.env.send_paid_by(&[ix], &stranger, &[]);
    tx.expect_fail();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::PASSED);
    // A wrong item is an error too.
    let ix = fail_stale_ix(
        proposal,
        &p,
        Some(Pubkey::new_unique()),
        Some(template),
        Some(ids::ITEMS_ID),
        Some(hookwars_common::programdata_address(&ids::ITEMS_ID)),
    );
    hw.w.env.send_paid_by(&[ix], &stranger, &[]).expect_fail();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::PASSED);
    // With the right accounts and nothing stale: NotStale.
    let ix = fail_stale_ix(
        proposal,
        &p,
        Some(b),
        Some(template),
        Some(ids::ITEMS_ID),
        Some(hookwars_common::programdata_address(&ids::ITEMS_ID)),
    );
    hw.w.env
        .send_paid_by(&[ix], &stranger, &[])
        .expect_code(armory_code(E::NotStale));
}

#[test]
fn h1_fail_stale_still_fails_a_proposal_that_no_longer_fits() {
    let mut hw = Hw::new();
    let (owner, _mint, b, proposal) = passed_proposal(&mut hw);
    let p: Proposal = hw.w.env.read(&proposal);
    // The template is retired after the vote: the proposal is stale.
    let admin = hw.admin.insecure_clone();
    let ix = armory_ix(
        hookwars_armory::accounts::RetireTemplate {
            admin: admin.pubkey(),
            config: pda::config().0,
            template: pda::template(T::WAR_ORDERS).0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::RetireTemplate {
            template_id: T::WAR_ORDERS,
        },
    );
    hw.send_gated(&admin, ix, &[]).ok();
    let stranger = hw.w.env.funded(1_000_000_000);
    let ix = fail_stale_ix(
        proposal,
        &p,
        Some(b),
        Some(pda::template(T::WAR_ORDERS).0),
        Some(ids::ITEMS_ID),
        Some(hookwars_common::programdata_address(&ids::ITEMS_ID)),
    );
    hw.w.env.send_paid_by(&[ix], &stranger, &[]).ok();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::FAILED);
    hw.w.env.warp(600);
    hw.execute(&owner, &proposal).expect_fail();
}

// ---------------------------------------------------------------------------------------- M-1

#[test]
fn m1_finalize_needs_the_launch_address() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, a, _) = hw.item(T::WAR_ORDERS, war_params(10), 0);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(a), EquipConfig::default()))
        .ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    let (_, b, _) = hw.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, proposal) = hw.propose(&owner, &mint, 0, Some(b), EquipConfig::default());
    tx.ok();
    hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    let p: Proposal = hw.w.env.read(&proposal);
    let finalize = |launch: Option<Pubkey>| {
        armory_ix(
            hookwars_armory::accounts::Finalize {
                config: pda::config().0,
                proposal,
                slot_state: pda::slot_state(&p.mint, p.slot).0,
                token_mint: p.mint,
                launch,
                pool_base_vault: None,
                launch_holding: None,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Finalize {},
        )
    };
    let griefer = hw.w.env.funded(1_000_000_000);
    // The launch left out, or the wrong address: refused, the proposal stays open.
    for launch in [None, Some(Pubkey::new_unique())] {
        hw.w.env
            .send_paid_by(&[finalize(launch)], &griefer, &[])
            .expect_code(armory_code(E::WrongAccount));
        assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::OPEN);
    }
    // The right address of a token that never launched: eligible is the supply, it passes.
    hw.w.env
        .send_paid_by(&[finalize(Some(pda::launch(&mint).0))], &griefer, &[])
        .ok();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::PASSED);
}

// ---------------------------------------------------------------------------------------- M-2

#[test]
fn m2_a_proposer_below_the_threshold_cannot_hold_the_seat() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, a, _) = hw.item(T::WAR_ORDERS, war_params(10), 0);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(a), EquipConfig::default()))
        .ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 9_900);
    // The review's proof of concept: a wallet with no tokens.
    let griefer = hw.w.env.funded(1_000_000_000);
    let (_, junk, _) = hw.item(T::WAR_ORDERS, war_params(11), 0);
    let (tx, _) = hw.propose(&griefer, &mint, 0, Some(junk), EquipConfig::default());
    tx.expect_code(armory_code(E::BelowProposalThreshold));
    // Below the TEST threshold (1% of the supply; 100 once the supply is 10,000): refused.
    hw.mint_to(&owner, &mint, &griefer.pubkey(), 99);
    let (tx, _) = hw.propose(&griefer, &mint, 0, Some(junk), EquipConfig::default());
    tx.expect_code(armory_code(E::BelowProposalThreshold));
    // At the threshold: accepted, and the threshold is locked in place for the vote period.
    hw.mint_to(&owner, &mint, &griefer.pubkey(), 1);
    let (tx, proposal) = hw.propose(&griefer, &mint, 0, Some(junk), EquipConfig::default());
    tx.ok();
    let p: Proposal = hw.w.env.read(&proposal);
    let h: Holding = hw.w.env.read(&pda::holding(&mint, &griefer.pubkey()));
    assert_eq!((h.vote_locked, h.vote_lock_until), (100, p.vote_end));
    // The locked tokens cannot move to fund the next spam proposal elsewhere.
    let send = hw.w.slot_transfer_ix(&griefer.pubkey(), &mint, &owner.pubkey(), 1);
    hw.w.env.send_paid_by(&[send], &griefer, &[]).expect_code(
        anchor_lang::error::ERROR_CODE_OFFSET
            + bordrless_token::error::TokenError::VoteLocked as u32,
    );
}

// ---------------------------------------------------------------------------------------- I-3

#[test]
fn i3_an_emptying_proposal_names_no_targets() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, a, _) = hw.item(T::WAR_ORDERS, war_params(10), 0);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(a), EquipConfig::default()))
        .ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    let config = EquipConfig {
        targets: vec![Pubkey::new_unique(); 8],
        role: 0,
    };
    let (tx, _) = hw.propose(&owner, &mint, 0, None, config);
    tx.expect_code(armory_code(E::OverBounds));
}

// ---------------------------------------------------------------------------------------- M-3

fn threshold() -> u64 {
    u64::from(OrdersSpec::default().siege_threshold) * wh::TEST_PARAMS.siege_unit_lamports
}

/// A war token that besieged a rival and holds captured tokens.
fn besieged() -> (WarWorld, wh::WarToken, Pubkey) {
    let mut ww = WarWorld::new();
    let t = ww.war_token("ATK", OrdersSpec::default());
    let rival = ww.launch("RIV", LaunchRules::NONE);
    ww.buyer(&rival, SOL);
    ww.w.env.warp(120);
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, threshold(), &[(rival, now, threshold(), 0)]);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    ww.fund_chest(&t.mint, 10 * SOL);
    ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None)).0.ok();
    (ww, t, rival)
}

#[test]
fn m3_a_raze_waits_while_the_rival_trades_below_its_twap_floor() {
    let (mut ww, t, rival) = besieged();
    let captured = ww.state(&t.mint).captured[0].amount;
    let pool = ww.w.launch_pool_key(&rival);
    // A searcher depressed the pool: the spot is half the TWAP, far below the TEST 10% floor.
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot * 2, 3_600);
    let (tx, _) = ww.crank(|c, ww| ww.raze_ix(c, &t, &rival));
    let e = tx.event::<RazeWaited>();
    assert_eq!(e.rival_twap, spot * 2);
    assert!(tx.events::<Razed>().is_empty());
    assert_eq!(ww.state(&t.mint).captured[0].amount, captured);
    ww.assert_solvent(&t.mint);
    // At the TWAP again: it sells.
    ww.flat_observations(&pool, spot, 3_600);
    let (tx, _) = ww.crank(|c, ww| ww.raze_ix(c, &t, &rival));
    assert!(tx.event::<Razed>().sold > 0);
    ww.assert_solvent(&t.mint);
}

// ---------------------------------------------------------------------------------------- M-4

#[test]
fn m4_a_siege_always_marks_a_rival_with_a_war_state() {
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
    // A wrong account where the rival's war state goes: refused.
    let (tx, _) = ww.crank(|c, ww| {
        let mut ix = ww.siege_ix(c, &t, &rival, true, None);
        let state = WarState::address(&rival).0;
        let m = ix.accounts.iter_mut().find(|m| m.pubkey == state).unwrap();
        m.pubkey = Pubkey::new_unique();
        ix
    });
    tx.expect_code(wh::war_code(WarError::WrongAccount));
    // The review's attack: the cranker says the rival has no war state. The rival is marked anyway.
    ww.crank(|c, ww| ww.siege_ix(c, &t, &rival, false, None)).0.ok();
    let d = ww.state(&r.mint);
    assert_eq!(d.siege_by_chest, WarWorld::chest(&t.mint));
    assert_eq!(d.under_siege_until, ww.now() + wh::TEST_PARAMS.siege_interval_secs);
    assert_eq!(d.season.times_besieged, 1);
    ww.assert_solvent(&t.mint);
    ww.assert_solvent(&r.mint);
}

// ---------------------------------------------------------------------------------------- M-6

#[test]
fn m6_a_bounty_point_never_pays_more_than_its_cap() {
    // TEST cap: a point pays at most 1% of the quote volume it stands for.
    let params = hookwars_war::state::WarParams {
        bounty_max_point_bps: 100,
        ..wh::TEST_PARAMS
    };
    let mut ww = WarWorld::with_params(params);
    // War orders voted a rate of a whole point unit per point (100% of the volume).
    let orders = OrdersSpec {
        bounty_rate: 1_000_000,
        ..OrdersSpec::default()
    };
    let t = ww.war_token("CAPD", orders);
    ww.fund_chest(&t.mint, 5 * SOL);
    let holder = ww.buyer(&t.mint, SOL);
    let season = ww.config().current_season;
    ww.set_raid(&t.mint, &holder.pubkey(), season, 300, 0);
    let ix = ww.bounty_ix(&holder.pubkey(), &t);
    let e = ww.w.env.send(&[ix], &[&holder]).event::<BountyClaimed>();
    let cap = params.point_unit_lamports * u64::from(params.bounty_max_point_bps) / 10_000;
    assert_eq!((e.points, e.paid), (300, 300 * cap));
    ww.assert_solvent(&t.mint);
}

// ------------------------------------------------------------------------- review 2, L-D

/// An equip of a `Relation` (or `Pool`) slot names the launch so the armory can refresh the slot
/// launch's pool registry in the same instruction; without those accounts the equip is refused,
/// with them it runs (here the token never launched: nothing to refresh).
#[test]
fn l_d_an_equip_of_a_pool_kind_slot_names_the_launch_for_the_registry_refresh() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let cfg = EquipConfig {
        targets: vec![Pubkey::new_unique()],
        role: 0,
    };
    let (_, t1, _) = hw.item(T::TREATY, params(&[50, 50, 0]), 0);
    let (_, t2, _) = hw.item(T::TREATY, params(&[100, 100, 1]), 0);
    let mut e = Hw::entry(3, Some(t1), cfg.clone());
    e.rule = Some(PerformanceRule {
        metric: 2,
        window_secs: 60,
        base_window_secs: 600,
        op: 0,
        ratio_bps: 10_000,
        hold_secs: 120,
    });
    hw.equip_launch(&owner, &mint, e).ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    let (tx, proposal) = hw.propose(&owner, &mint, 3, Some(t2), cfg);
    tx.ok();
    hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    hw.w.env.warp(600);
    let full = hw.execute_ix(&o, &proposal);
    let tail = hw.refresh_tail(&mint, 3).len();
    assert_eq!(tail, 3);
    let mut bare = full.clone();
    bare.accounts.truncate(bare.accounts.len() - tail);
    hw.w.env
        .send_paid_by(&[bare], &owner, &[])
        .expect_code(armory_code(E::WrongAccount));
    // A wrong launch address: refused too.
    let mut wrong = full.clone();
    let n = wrong.accounts.len();
    wrong.accounts[n - 2].pubkey = Pubkey::new_unique();
    hw.w.env
        .send_paid_by(&[wrong], &owner, &[])
        .expect_code(armory_code(E::WrongAccount));
    hw.w.env.send_paid_by(&[full], &owner, &[]).ok();
    assert_eq!(hw.slot_item(&mint, 3), t2);
}
