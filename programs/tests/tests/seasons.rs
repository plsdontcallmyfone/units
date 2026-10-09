// Changed by Hookwars: new file; security review 2: raid volume capped by season funding (M-B).
//! Seasons (05 section 10): proposals behind the timelock, opening, king of the hill in O(1) per
//! call with a challenge window that never extends, finalizing, the prize split from protocol fees
//! (R14), and treaty time.

use bordrless_hook::slot_kind;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_swap::client as swap;
use hookwars_war::client as war;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use hookwars_war::instructions::SeasonArgs;
use hookwars_war::state::*;
use solana_signer::Signer;

fn by_raid_volume() -> ScoreWeights {
    ScoreWeights {
        raid_volume_won: 1,
        ..ScoreWeights::default()
    }
}

#[test]
fn a_season_waits_for_the_timelock_and_its_start() {
    let mut ww = WarWorld::new();
    let admin = ww.w.env.deployer.insecure_clone();
    let early = SeasonArgs {
        number: 1,
        starts_at: ww.now() + 10,
        weights: by_raid_volume(),
        penalize_besieged: false,
    };
    ww.w.env
        .send(&[war::propose_season(admin.pubkey(), early)], &[&admin])
        .expect_code(war_code(WarError::TimelockNotPassed));
    let stranger = ww.w.env.funded(SOL);
    let args = SeasonArgs {
        starts_at: ww.now() + TEST_PARAMS.admin_timelock_secs,
        ..early
    };
    ww.w.env
        .send(&[war::propose_season(stranger.pubkey(), args)], &[&stranger])
        .expect_code(war_code(WarError::NotAdmin));
    let tx = ww.w.env.send(&[war::propose_season(admin.pubkey(), args)], &[&admin]);
    let e = tx.event::<SeasonProposed>();
    assert_eq!(e.ends_at, args.starts_at + TEST_PARAMS.season_secs);
    // No loot table yet: cannot open.
    ww.w.env.send(&[war::open_season(0)], &[]).expect_fail();
    let templates = [
        hookwars_war::foreign::template_address(RAID_TEMPLATE),
        hookwars_war::foreign::template_address(WAR_ORDERS_TEMPLATE),
    ];
    ww.w.env
        .send(&[war::propose_loot_table(admin.pubkey(), 1, WarWorld::loot_entries(), &templates)], &[&admin])
        .ok();
    // Too early.
    ww.w.env
        .send(&[war::open_season(0)], &[])
        .expect_code(war_code(WarError::TimelockNotPassed));
    ww.w.env.warp(TEST_PARAMS.admin_timelock_secs);
    let e = ww.w.env.send(&[war::open_season(0)], &[]).event::<SeasonOpened>();
    assert_eq!(e.season, 1);
    assert_eq!(ww.config().current_season, 1);
}

#[test]
fn king_of_the_hill_takes_strictly_higher_scores_in_its_window() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("AAA", OrdersSpec::default());
    let b = ww.war_token("BBB", OrdersSpec::default());
    let s = ww.open_season(by_raid_volume());
    let n = s.number;
    ww.put_ledger(&a.mint, n, 500, &[]);
    ww.put_ledger(&b.mint, n, 800, &[]);
    // Security review 2, M-B: raid volume scores up to `raid_volume_per_funded` times the season's
    // chest funding; both chests receive enough for their whole volume to count.
    ww.fund_chest(&a.mint, SOL);
    ww.fund_chest(&b.mint, SOL);
    let me = ww.w.env.funded(SOL);
    let submit = |ww: &mut WarWorld, mint| {
        ww.w.env
            .send(&[war::submit_candidate(me.pubkey(), n, mint, true)], &[&me])
    };
    // Not before the season ends.
    submit(&mut ww, a.mint).expect_code(war_code(WarError::SeasonNotEnded));
    let to_end = s.ends_at - ww.now();
    ww.w.env.warp(to_end);
    let e = submit(&mut ww, a.mint).event::<CandidateSubmitted>();
    assert_eq!((e.mint, e.score), (a.mint, 500));
    submit(&mut ww, a.mint).expect_code(war_code(WarError::NotHigherScore));
    let e = submit(&mut ww, b.mint).event::<CandidateChallenged>();
    assert_eq!((e.mint, e.score, e.beaten), (b.mint, 800, a.mint));
    submit(&mut ww, a.mint).expect_code(war_code(WarError::NotHigherScore));
    // Finalizing waits for the window, which never extends.
    ww.w.env
        .send(&[war::finalize_season(n)], &[])
        .expect_code(war_code(WarError::SeasonNotEnded));
    ww.w.env.warp(TEST_PARAMS.challenge_secs);
    submit(&mut ww, a.mint).expect_code(war_code(WarError::ChallengeClosed));
    let e = ww.w.env.send(&[war::finalize_season(n)], &[]).event::<SeasonFinalized>();
    assert_eq!((e.winner, e.score), (Some(b.mint), 800));
    let c = ww.config();
    assert_eq!((c.last_winner, c.last_winner_season), (Some(b.mint), n));
    // The next season can open only now.
    let s2 = ww.open_season(by_raid_volume());
    assert_eq!(s2.number, n + 1);
}

/// Security review 2, M-B: a community that washes raid volume with self-raid loops ("sell rival X on
/// X's pool, buy ours, sell ours, buy back X": every loop adds its whole buy to our raid volume for
/// only the fees) gains score only up to `raid_volume_per_funded` times what its chest received in
/// the season. The ledger below stands for those loops: one honest raid of 800 for `b`, and 200
/// loops of 1,000 for `a`.
#[test]
fn washed_raid_volume_scores_only_up_to_the_seasons_funding() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("WASH", OrdersSpec::default());
    let b = ww.war_token("FAIR", OrdersSpec::default());
    let s = ww.open_season(by_raid_volume());
    let n = s.number;
    let loops: u64 = 200;
    ww.put_ledger(&a.mint, n, loops * 1_000, &[]);
    ww.put_ledger(&b.mint, n, 800, &[]);
    // The washer's chest received 1 lamport this season, the honest token's enough for its raids.
    ww.fund_chest(&a.mint, 1);
    ww.fund_chest(&b.mint, SOL);
    let me = ww.w.env.funded(SOL);
    ww.w.env.warp(s.ends_at - ww.now());
    let submit = |ww: &mut WarWorld, mint| {
        ww.w.env
            .send(&[war::submit_candidate(me.pubkey(), n, mint, true)], &[&me])
    };
    let e = submit(&mut ww, a.mint).event::<CandidateSubmitted>();
    // 200,000 of raid volume, scored at 1 lamport of funding times the TEST 1,000.
    assert_eq!(e.score, i128::from(TEST_PARAMS.raid_volume_per_funded));
    let e = submit(&mut ww, b.mint).event::<CandidateChallenged>();
    assert_eq!((e.score, e.beaten), (800, a.mint));
    // Funding recorded before the season opened does not count for it.
    let mut ww = WarWorld::new();
    let c = ww.war_token("EARLY", OrdersSpec::default());
    ww.fund_chest(&c.mint, SOL);
    let s = ww.open_season(by_raid_volume());
    ww.put_ledger(&c.mint, s.number, 5_000, &[]);
    ww.w.env.warp(s.ends_at - ww.now());
    let me = ww.w.env.funded(SOL);
    let e = ww
        .w
        .env
        .send(&[war::submit_candidate(me.pubkey(), s.number, c.mint, true)], &[&me])
        .event::<CandidateSubmitted>();
    assert_eq!(e.score, 0);
}

#[test]
fn a_season_cannot_open_while_the_last_is_unfinalized() {
    let mut ww = WarWorld::new();
    ww.open_season(by_raid_volume());
    let admin = ww.w.env.deployer.insecure_clone();
    let args = SeasonArgs {
        number: 2,
        starts_at: ww.now() + TEST_PARAMS.admin_timelock_secs,
        weights: by_raid_volume(),
        penalize_besieged: false,
    };
    let templates = [
        hookwars_war::foreign::template_address(RAID_TEMPLATE),
        hookwars_war::foreign::template_address(WAR_ORDERS_TEMPLATE),
    ];
    let ixs = [
        war::propose_season(admin.pubkey(), args),
        war::propose_loot_table(admin.pubkey(), 2, WarWorld::loot_entries(), &templates),
    ];
    ww.w.env.send(&ixs, &[&admin]).ok();
    ww.w.env.warp(TEST_PARAMS.admin_timelock_secs);
    ww.w.env
        .send(&[war::open_season(1)], &[])
        .expect_code(war_code(WarError::SeasonNotFinalized));
}

#[test]
fn a_pending_season_can_be_cancelled_but_not_an_open_one() {
    let mut ww = WarWorld::new();
    let admin = ww.w.env.deployer.insecure_clone();
    ww.open_season(by_raid_volume());
    ww.w.env
        .send(&[war::cancel_pending(admin.pubkey(), 1, 1)], &[&admin])
        .expect_code(war_code(WarError::SeasonClosed));
    let args = SeasonArgs {
        number: 2,
        starts_at: ww.now() + TEST_PARAMS.admin_timelock_secs,
        weights: by_raid_volume(),
        penalize_besieged: false,
    };
    ww.w.env.send(&[war::propose_season(admin.pubkey(), args)], &[&admin]).ok();
    let tx = ww.w.env.send(&[war::cancel_pending(admin.pubkey(), 1, 2)], &[&admin]);
    assert_eq!(tx.event::<PendingCancelled>().season, 2);
    assert!(ww.w.env.account(&Season::address(2).0).is_none_or(|a| a.lamports == 0));
}

#[test]
fn loot_tables_stay_inside_their_templates() {
    let mut ww = WarWorld::new();
    let admin = ww.w.env.deployer.insecure_clone();
    let mut entries = WarWorld::loot_entries();
    entries[0].ranges[0].max = 10_001; // above the Raid template's ceiling
    let templates = [
        hookwars_war::foreign::template_address(RAID_TEMPLATE),
        hookwars_war::foreign::template_address(WAR_ORDERS_TEMPLATE),
    ];
    ww.w.env
        .send(&[war::propose_loot_table(admin.pubkey(), 1, entries, &templates)], &[&admin])
        .expect_code(war_code(WarError::InvalidLootEntry));
    // A template that is not loot-enabled (the Treaty) is refused.
    let mut entries = WarWorld::loot_entries();
    entries[0].template_id = TREATY_TEMPLATE;
    entries[0].ranges = [ParamRange::default(); hookwars_war::foreign::PARAM_FIELDS];
    let templates = [
        hookwars_war::foreign::template_address(TREATY_TEMPLATE),
        hookwars_war::foreign::template_address(WAR_ORDERS_TEMPLATE),
    ];
    ww.w.env
        .send(&[war::propose_loot_table(admin.pubkey(), 1, entries, &templates)], &[&admin])
        .expect_code(war_code(WarError::InvalidLootEntry));
}

/// R14: protocol fees reach the prize vault through the DEX's own `collect_protocol_fees_sol`, and
/// `split_protocol_fees` pays the last winner's chest its share and the treasury the rest.
#[test]
fn the_prize_is_a_share_of_protocol_fees() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("WIN", OrdersSpec::default());
    let other = ww.launch("FEES", LaunchRules::NONE);
    // The DEX's fee collector becomes the prize vault.
    let admin = ww.w.env.deployer.insecure_clone();
    let vault = prize_vault_address().0;
    let args = bordrless_swap::instructions::ConfigArgs {
        admin: admin.pubkey(),
        protocol_fee_bps: bordrless_core::policy::PROTOCOL_FEE_BPS,
        launch_protocol_share_bps: bordrless_core::policy::LAUNCH_PROTOCOL_SHARE_BPS,
        fee_collector: vault,
        treasury: ww.w.env.treasury.pubkey(),
        pool_creation_fee_lamports: 0,
        paused: false,
    };
    ww.w.env.send(&[swap::set_config(admin.pubkey(), args)], &[&admin]).ok();
    ww.w.env.fund(vault, ww.w.env.rent(0));

    // Season 1, won by the war token.
    let s = ww.open_season(by_raid_volume());
    ww.put_ledger(&a.mint, s.number, 1_000, &[]);
    ww.w.env.warp(s.ends_at - ww.now());
    let me = ww.w.env.funded(SOL);
    ww.w.env
        .send(&[war::submit_candidate(me.pubkey(), s.number, a.mint, true)], &[&me])
        .ok();
    ww.w.env.warp(TEST_PARAMS.challenge_secs);
    ww.w.env.send(&[war::finalize_season(s.number)], &[]).ok();

    // Trading on a launch pool, then the DEX's own SOL collection into the prize vault.
    let trader = ww.w.wallet_with_sol(20 * SOL);
    ww.w.buy(&trader, &other, 10 * SOL).ok();
    let pool = ww.w.launch_pool_key(&other);
    let cranker = ww.w.env.funded(SOL);
    ww.w.env
        .send_paid_by(&[swap::collect_protocol_fees_sol(cranker.pubkey(), pool, vault)], &cranker, &[])
        .ok();
    let collected = ww.w.env.lamports(&vault) - ww.w.env.rent(0);
    assert!(collected > 0);

    let treasury = ww.w.env.treasury.pubkey();
    let before_treasury = ww.w.env.lamports(&treasury);
    let chest = WarWorld::chest(&a.mint);
    let inner = [
        bordrless_bridge::client::unwrap_sol(vault, 0),
        bordrless_bridge::client::wrap_sol(chest, 0),
    ];
    let ix = war::split_protocol_fees(cranker.pubkey(), treasury, Some((a.mint, s.number)), &inner);
    let tx = ww.w.env.send_paid_by(&[ix], &cranker, &[]);
    let e = tx.event::<PrizePaid>();
    assert_eq!(e.to_winner, collected * u64::from(TEST_PARAMS.season_prize_share_bps) / 10_000);
    assert_eq!(e.to_winner + e.to_treasury + e.bounty, collected);
    assert_eq!(ww.w.env.lamports(&treasury), before_treasury + e.to_treasury);
    assert_eq!(ww.chest_balance(&a.mint), e.to_winner);
    assert_eq!(ww.w.env.read::<Season>(&Season::address(s.number).0).prize_paid, e.to_winner);
    // The chest counts it as funding.
    ww.w.env.send(&[war::record_funding(a.mint)], &[]).ok();
    ww.assert_solvent(&a.mint);
    assert_eq!(ww.state(&a.mint).funded_total, e.to_winner);
    // Nothing left to split.
    ww.w.env
        .send_paid_by(
            &[war::split_protocol_fees(cranker.pubkey(), treasury, Some((a.mint, s.number)), &inner)],
            &cranker,
            &[],
        )
        .expect_code(war_code(WarError::NothingToDo));
}

#[test]
fn without_a_winner_the_prize_vault_pays_the_treasury() {
    let mut ww = WarWorld::new();
    let vault = prize_vault_address().0;
    ww.w.env.fund(vault, ww.w.env.rent(0) + SOL);
    let cranker = ww.w.env.funded(SOL);
    let treasury = ww.w.env.treasury.pubkey();
    let before = ww.w.env.lamports(&treasury);
    let inner = [bordrless_bridge::client::unwrap_sol(vault, 0)];
    let tx = ww
        .w
        .env
        .send_paid_by(&[war::split_protocol_fees(cranker.pubkey(), treasury, None, &inner)], &cranker, &[]);
    let e = tx.event::<PrizePaid>();
    assert_eq!(e.to_winner, 0);
    assert_eq!(e.to_treasury + e.bounty, SOL);
    assert_eq!(e.bounty, SOL * u64::from(TEST_PARAMS.max_crank_bounty_bps) / 10_000);
    assert_eq!(ww.w.env.lamports(&treasury), before + e.to_treasury);
}

#[test]
fn treaty_time_counts_only_while_both_sides_equip_it() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("AAA", OrdersSpec::default());
    let b = ww.war_token("BBB", OrdersSpec::default());
    let treaty = ww.put_item(TREATY_TEMPLATE, [0; hookwars_war::foreign::PARAM_FIELDS]);
    ww.add_slot(&a.mint, WarWorld::named_slot(slot_kind::RELATION, treaty));
    let s = ww.open_season(ScoreWeights {
        treaty_secs: 1,
        ..ScoreWeights::default()
    });
    // One side only: no treaty.
    ww.w.env
        .send(&[war::accrue_treaty_time(a.mint, s.number, &[(treaty, b.mint)])], &[])
        .expect_code(war_code(WarError::NoTreaty));
    ww.add_slot(&b.mint, WarWorld::named_slot(slot_kind::RELATION, treaty));
    ww.w.env
        .send(&[war::accrue_treaty_time(a.mint, s.number, &[(treaty, b.mint)])], &[])
        .ok();
    let start = ww.state(&a.mint).season.treaty_secs;
    ww.w.env.warp(1_000);
    let tx = ww
        .w
        .env
        .send(&[war::accrue_treaty_time(a.mint, s.number, &[(treaty, b.mint)])], &[]);
    assert_eq!(tx.event::<TreatyTimeAccrued>().secs, 1_000);
    assert_eq!(ww.state(&a.mint).season.treaty_secs, start + 1_000);
}
