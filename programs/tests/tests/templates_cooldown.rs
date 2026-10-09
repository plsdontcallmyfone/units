// Changed by Hookwars: new file (arsenal wave B), Cooldown (id 17, 08 section 4.2).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::{items_code, params, test_schema};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};
use hookwars_items::templates::cooldown::{last_buy, TAG};
use hookwars_items::ItemsError as E;
use solana_signer::Signer;

#[test]
fn a_buyer_waits_out_the_cooldown_before_selling_or_sending() {
    let mut hw = world(&[T::COOLDOWN]);
    let t = tok(&mut hw);
    let cd = item(&mut hw, T::COOLDOWN, &[600], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, cd, vec![], 0).ok();
    let pool = t.pool.pubkey();
    hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    let owners = [pool, alice.pubkey(), bob.pubkey()];

    // A buy stamps the buyer with the buy time.
    let t0 = hw.w.env.now;
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 10_000).ok();
    let r = range(&hw, &t.mint, &alice.pubkey(), 1);
    assert_eq!((r[0], last_buy(&r)), (TAG, Some(t0 as u32)));
    check(&hw, &t.mint, &owners);

    // Inside the cooldown: no sell, no send.
    hw.w.env.warp(599);
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 1_000).expect_code(items_code(E::CooldownActive));
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &bob.pubkey(), 1_000).expect_code(items_code(E::CooldownActive));
    check(&hw, &t.mint, &owners);

    // At the end of the cooldown both go through; a send does not stamp the receiver.
    hw.w.env.warp(1);
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &bob.pubkey(), 4_000).ok();
    assert_eq!(last_buy(&range(&hw, &t.mint, &bob.pubkey(), 1)), None);
    send(&mut hw, &bob, &t.mint, &bob.pubkey(), &pool, 1_000).ok();
    // Emptying the holding clears the stamp.
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 6_000).ok();
    assert_eq!(bal(&hw, &t.mint, &alice.pubkey()), 0);
    check(&hw, &t.mint, &owners);
}

#[test]
fn a_new_buy_extends_the_cooldown_and_moving_tokens_cannot_reset_it() {
    let mut hw = world(&[T::COOLDOWN]);
    let t = tok(&mut hw);
    let cd = item(&mut hw, T::COOLDOWN, &[600], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, cd, vec![], 0).ok();
    let pool = t.pool.pubkey();
    hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
    let alice = hw.w.env.funded(SOL);
    let fresh = hw.w.env.funded(SOL);
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 10_000).ok();
    hw.w.env.warp(500);
    // Abuse: the holder cannot move the tokens to a fresh wallet to escape (the send is refused).
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &fresh.pubkey(), 10_000).expect_code(items_code(E::CooldownActive));
    // A second buy restarts the clock from the later buy.
    send(&mut hw, &t.pool, &t.mint, &pool, &alice.pubkey(), 1_000).ok();
    hw.w.env.warp(200);
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 1_000).expect_code(items_code(E::CooldownActive));
    hw.w.env.warp(400);
    send(&mut hw, &alice, &t.mint, &alice.pubkey(), &pool, 1_000).ok();
    check(&hw, &t.mint, &[pool, alice.pubkey(), fresh.pubkey()]);
}

#[test]
fn cooldown_forges_toward_its_ceiling() {
    let (min, max, _, _) = test_schema(T::COOLDOWN);
    let out = combine(T::COOLDOWN, &min, &max, 5_000, &params(&[600]), &params(&[1_200])).unwrap();
    assert_eq!(out[0], 1_200 + (86_400 - 1_200) / 2);
    let top = combine(T::COOLDOWN, &min, &max, 10_000, &params(&[86_400]), &params(&[86_400])).unwrap();
    assert_eq!(top[0], 86_400);
    let _ = Pubkey::default();
}
