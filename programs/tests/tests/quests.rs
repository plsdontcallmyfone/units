// Changed by Hookwars: new file.
//! Quests (05 section 9): only conditions the program checks, one loot ticket each, once per period
//! per owner in a `QuestMark` PDA (never in hook data).

use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use hookwars_war::client as war;
use hookwars_war::constants::quest;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use hookwars_war::instructions::quests::period_at;
use hookwars_war::state::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn world() -> (WarWorld, WarToken, Keypair, Season) {
    let mut ww = WarWorld::new();
    let t = ww.war_token("QST", OrdersSpec::default());
    let s = ww.open_season(ScoreWeights::default());
    let holder = ww.buyer(&t.mint, SOL);
    (ww, t, holder, s)
}

fn period(ww: &WarWorld, s: &Season) -> u32 {
    period_at(s, ww.now(), TEST_PARAMS.quest_period_secs)
}

#[test]
fn the_raid_quest_spends_points_for_a_ticket_once_a_period() {
    let (mut ww, t, holder, s) = world();
    let need = TEST_PARAMS.quest_raid_points;
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, 3 * need, 0);
    let p = period(&ww, &s);
    let ix = war::claim_quest(holder.pubkey(), t.mint, s.number, quest::RAID, p, RAID_SLOT, vec![]);
    let e = ww.w.env.send(&[ix.clone()], &[&holder]).event::<QuestClaimed>();
    assert_eq!((e.quest_id, e.period), (quest::RAID, p));
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()), (s.number, 2 * need, 1));
    // Once a period.
    ww.w.env.warp(1);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::QuestAlreadyClaimed));
    // The next period: again.
    ww.w.env.warp(TEST_PARAMS.quest_period_secs);
    let p2 = period(&ww, &s);
    assert_eq!(p2, p + 1);
    let ix = war::claim_quest(holder.pubkey(), t.mint, s.number, quest::RAID, p2, RAID_SLOT, vec![]);
    ww.w.env.send(&[ix], &[&holder]).ok();
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()), (s.number, need, 2));
}

#[test]
fn the_raid_quest_needs_its_points_and_the_current_period() {
    let (mut ww, t, holder, s) = world();
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, TEST_PARAMS.quest_raid_points - 1, 0);
    let p = period(&ww, &s);
    let ix = war::claim_quest(holder.pubkey(), t.mint, s.number, quest::RAID, p, RAID_SLOT, vec![]);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::QuestConditionUnmet));
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, TEST_PARAMS.quest_raid_points, 0);
    let ix = war::claim_quest(holder.pubkey(), t.mint, s.number, quest::RAID, p + 1, RAID_SLOT, vec![]);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::QuestConditionUnmet));
}

#[test]
fn the_forge_quest_needs_a_new_forge() {
    let (mut ww, t, holder, s) = world();
    let p = period(&ww, &s);
    let ix = |p| war::claim_quest(holder.pubkey(), t.mint, s.number, quest::FORGE, p, RAID_SLOT, vec![]);
    // No forge counter yet.
    ww.w.env.send(&[ix(p)], &[&holder]).expect_fail();
    ww.put_forge_counter(holder.pubkey(), 1);
    ww.w.env.send(&[ix(p)], &[&holder]).ok();
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 1);
    ww.w.env.warp(TEST_PARAMS.quest_period_secs);
    let p2 = period(&ww, &s);
    ww.w.env
        .send(&[ix(p2)], &[&holder])
        .expect_code(war_code(WarError::QuestConditionUnmet));
    ww.put_forge_counter(holder.pubkey(), 2);
    ww.w.env.send(&[ix(p2)], &[&holder]).ok();
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 2);
}

#[test]
fn a_quest_mark_is_per_owner_and_closes_after_the_season() {
    let (mut ww, t, holder, s) = world();
    let need = TEST_PARAMS.quest_raid_points;
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, need, 0);
    let p = period(&ww, &s);
    ww.w.env
        .send(
            &[war::claim_quest(holder.pubkey(), t.mint, s.number, quest::RAID, p, RAID_SLOT, vec![])],
            &[&holder],
        )
        .ok();
    // Another holder has its own mark.
    let other = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &other.pubkey(), s.number, need, 0);
    ww.w.env
        .send(
            &[war::claim_quest(other.pubkey(), t.mint, s.number, quest::RAID, p, RAID_SLOT, vec![])],
            &[&other],
        )
        .ok();
    let mark = QuestMark::address(s.number, &t.mint, &holder.pubkey()).0;
    // Not before the season is finalized.
    ww.w.env
        .send(&[war::close_quest_mark(holder.pubkey(), s.number, mark)], &[&holder])
        .expect_fail();
    ww.w.env.warp(s.ends_at - ww.now() + TEST_PARAMS.challenge_secs);
    ww.w.env.send(&[war::finalize_season(s.number)], &[]).ok();
    ww.w.env
        .send(&[war::close_quest_mark(holder.pubkey(), s.number, mark)], &[&holder])
        .ok();
    assert!(ww.w.env.account(&mark).is_none_or(|a| a.lamports == 0));
}
