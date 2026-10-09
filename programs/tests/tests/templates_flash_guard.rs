// Changed by Hookwars: new file (arsenal wave B), Flash Guard (id 20, 08 section 4.2).

use bordrless_program_tests::armory::{items_code, params, test_schema};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};
use hookwars_items::templates::flash_guard::buy_slot;
use hookwars_items::ItemsError as E;
use solana_signer::Signer;

#[test]
fn a_same_slot_round_trip_is_refused_and_a_later_sell_passes() {
    let mut hw = world(&[T::FLASH_GUARD]);
    let t = tok(&mut hw);
    let fg = item(&mut hw, T::FLASH_GUARD, &[2], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, fg, vec![], 0).ok();
    let pool = t.pool.pubkey();
    hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
    let bot = hw.w.env.funded(SOL);
    let owners = [pool, bot.pubkey()];

    let s0 = hw.w.env.slot;
    send(&mut hw, &t.pool, &t.mint, &pool, &bot.pubkey(), 10_000).ok();
    assert_eq!(buy_slot(&range(&hw, &t.mint, &bot.pubkey(), 1)), Some(s0 as u32));
    // Abuse: the back leg of a sandwich in the same slot fails, and one slot later still fails.
    send(&mut hw, &bot, &t.mint, &bot.pubkey(), &pool, 10_000).expect_code(items_code(E::FlashSellTooSoon));
    hw.w.env.warp(1);
    send(&mut hw, &bot, &t.mint, &bot.pubkey(), &pool, 10_000).expect_code(items_code(E::FlashSellTooSoon));
    check(&hw, &t.mint, &owners);
    // Sends are not refused (Cooldown covers them).
    let friend = hw.w.env.funded(SOL);
    send(&mut hw, &bot, &t.mint, &bot.pubkey(), &friend.pubkey(), 1_000).ok();
    // Two slots after the buy the sell goes through; emptying clears the stamp.
    hw.w.env.warp(1);
    send(&mut hw, &bot, &t.mint, &bot.pubkey(), &pool, 9_000).ok();
    assert_eq!(bal(&hw, &t.mint, &bot.pubkey()), 0);
    check(&hw, &t.mint, &[pool, bot.pubkey(), friend.pubkey()]);
}

#[test]
fn flash_guard_forges_toward_its_ceiling() {
    let (min, max, _, _) = test_schema(T::FLASH_GUARD);
    let out = combine(T::FLASH_GUARD, &min, &max, 5_000, &params(&[2]), &params(&[10])).unwrap();
    assert_eq!(out[0], 10 + (1_000 - 10) / 2);
}
