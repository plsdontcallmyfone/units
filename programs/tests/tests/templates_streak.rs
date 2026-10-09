// Changed by Hookwars: new file (arsenal wave B), Streak (id 26, 08 section 4.3).

use bordrless_program_tests::armory::{params, test_schema};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};
use hookwars_items::templates::daily_sell_cap::day;
use hookwars_items::templates::streak::read;
use solana_signer::Signer;

#[test]
fn consecutive_daily_buys_build_a_capped_streak_and_any_sell_resets_it() {
    let mut hw = world(&[T::STREAK]);
    let t = tok(&mut hw);
    let st = item(&mut hw, T::STREAK, &[3], 0);
    equip(&mut hw, &t.owner, &t.mint, 2, st, vec![], 0).ok();
    let pool = t.pool.pubkey();
    hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
    let alice = hw.w.env.funded(SOL);
    let owners = [pool, alice.pubkey()];
    let streak = |hw: &bordrless_program_tests::armory::Hw| read(&range(hw, &t.mint, &alice.pubkey(), 2));

    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 100).ok();
    let d0 = day(hw.w.env.now);
    assert_eq!(streak(&hw), Some((d0, 1)));
    // A second buy the same day keeps the streak.
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 100).ok();
    assert_eq!(streak(&hw), Some((d0, 1)));
    for n in 2..=4u16 {
        hw.w.env.warp(86_400);
        send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 100).ok();
        assert_eq!(streak(&hw), Some((day(hw.w.env.now), n.min(3))), "day {n}");
        check(&hw, &t.mint, &owners);
    }
    // A sell resets it to 0, keeping the day.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 50).ok();
    assert_eq!(streak(&hw), Some((day(hw.w.env.now), 0)));
    // Missing a day starts over at 1.
    hw.w.env.warp(2 * 86_400);
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 100).ok();
    assert_eq!(streak(&hw), Some((day(hw.w.env.now), 1)));
    check(&hw, &t.mint, &owners);
}

#[test]
fn a_send_resets_the_sender_and_gives_the_receiver_nothing() {
    let mut hw = world(&[T::STREAK]);
    let t = tok(&mut hw);
    let st = item(&mut hw, T::STREAK, &[30], 0);
    equip(&mut hw, &t.owner, &t.mint, 2, st, vec![], 0).ok();
    let pool = t.pool.pubkey();
    hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 100).ok();
    hw.w.env.warp(86_400);
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 100).ok();
    assert_eq!(read(&range(&hw, &t.mint, &alice.pubkey(), 2)).unwrap().1, 2);
    // Abuse: a streak cannot be handed to another wallet.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &bob.pubkey(), 200).ok();
    assert_eq!(read(&range(&hw, &t.mint, &bob.pubkey(), 2)), None);
    assert_eq!(bal(&hw, &t.mint, &alice.pubkey()), 0);
    check(&hw, &t.mint, &[pool, alice.pubkey(), bob.pubkey()]);
}

#[test]
fn streak_forges_toward_its_ceiling() {
    let (min, max, _, _) = test_schema(T::STREAK);
    let out = combine(T::STREAK, &min, &max, 5_000, &params(&[7]), &params(&[30])).unwrap();
    assert_eq!(out[0], 30 + (365 - 30) / 2);
}
