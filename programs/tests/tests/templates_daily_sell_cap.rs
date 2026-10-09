// Changed by Hookwars: new file (arsenal wave B), Daily Sell Cap (id 18, 08 section 4.2).

use bordrless_program_tests::armory::{items_code, params, test_schema};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};
use hookwars_items::templates::daily_sell_cap::{day, read};
use hookwars_items::ItemsError as E;
use solana_signer::Signer;

#[test]
fn a_wallet_sells_at_most_its_cap_of_the_days_starting_balance() {
    let mut hw = world(&[T::DAILY_SELL_CAP]);
    let t = tok(&mut hw);
    let cap = item(&mut hw, T::DAILY_SELL_CAP, &[2_500], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, cap, vec![], 0).ok();
    let pool = t.pool.pubkey();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 100_000);
    hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
    let owners = [pool, alice.pubkey(), bob.pubkey()];

    // 25% of 100,000 a day: 20,000 then 5,000 pass, one more unit does not.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 20_000).ok();
    let today = day(hw.w.env.now);
    assert_eq!(read(&range(&hw, &t.mint, &alice.pubkey(), 1)), Some((today, 100_000)));
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &bob.pubkey(), 5_000).ok();
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 1).expect_code(items_code(E::DailyCapExceeded));
    check(&hw, &t.mint, &owners);

    // Buying the same day raises the base by what arrived.
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 40_000).ok();
    assert_eq!(read(&range(&hw, &t.mint, &alice.pubkey(), 1)), Some((today, 140_000)));
    // spent so far 25,000 of a 35,000 cap: 10,000 more passes, 10,001 would not.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 10_001).expect_code(items_code(E::DailyCapExceeded));
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 10_000).ok();

    // The next day the base resets to the balance then.
    hw.w.env.warp(86_400);
    let left = bal(&hw, &t.mint, &alice.pubkey());
    assert_eq!(left, 105_000);
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, left / 4).ok();
    assert_eq!(read(&range(&hw, &t.mint, &alice.pubkey(), 1)), Some((day(hw.w.env.now), left)));
    check(&hw, &t.mint, &owners);
}

#[test]
fn splitting_across_wallets_binds_each_wallets_own_balance() {
    let mut hw = world(&[T::DAILY_SELL_CAP]);
    let t = tok(&mut hw);
    let cap = item(&mut hw, T::DAILY_SELL_CAP, &[1_000], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, cap, vec![], 0).ok();
    let pool = t.pool.pubkey();
    let alice = hw.w.env.funded(SOL);
    let mule = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 100_000);
    hw.mint_to(&t.owner, &t.mint, &pool, 1);
    // Abuse: moving 10% to a mule spends the day's cap; the mule's cap is 10% of what it holds.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &mule.pubkey(), 10_000).ok();
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 1).expect_code(items_code(E::DailyCapExceeded));
    send(&mut hw, &mule, &t.mint, &mule.pubkey(), &pool, 1_001).expect_code(items_code(E::DailyCapExceeded));
    send(&mut hw, &mule, &t.mint, &mule.pubkey(), &pool, 1_000).ok();
    check(&hw, &t.mint, &[pool, alice.pubkey(), mule.pubkey()]);
}

#[test]
fn daily_sell_cap_forges_toward_its_floor() {
    let (min, max, _, _) = test_schema(T::DAILY_SELL_CAP);
    let out = combine(T::DAILY_SELL_CAP, &min, &max, 5_000, &params(&[2_000]), &params(&[1_001])).unwrap();
    assert_eq!(out[0], 1_001 - (1_001 - 1) / 2);
}
