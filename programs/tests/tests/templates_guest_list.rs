// Changed by Hookwars: new file (arsenal wave D), Guest List (08 section 4.2).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use hookwars_items::ArsenalError;
use solana_signer::Signer;

#[test]
fn guest_list_gates_the_opening_to_target_holders() {
    let mut hw = world();
    let t = tok(&mut hw);
    let holder = hw.w.env.funded(SOL);
    let stranger = hw.w.env.funded(SOL);
    let target = other_token(&mut hw, &holder.pubkey(), 5_000);
    let gl = item(&mut hw, a2::GUEST_LIST, &[1_000, 3_600], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, gl, vec![target], 0).ok();
    let p = t.pool.pubkey();

    // A holder of at least min_hold buys during the opening.
    let mut c = buy(1_000_000, &t.mint, &p, true);
    c.actor = holder.pubkey();
    let (tx, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&target, &holder.pubkey()), false)]);
    tx.ok();
    assert_eq!((a.cut, a.discount_bps, a.burn), (0, 0, 0));

    // A stranger is refused; so is a stranger passing the holder's holding (fail closed).
    c.actor = stranger.pubkey();
    let (tx, _) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&target, &stranger.pubkey()), false)]);
    tx.expect_code(code(ArsenalError::GuestListClosed));
    let (tx, _) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&target, &holder.pubkey()), false)]);
    tx.expect_code(code(ArsenalError::GuestListClosed));

    // Sells are never gated.
    let mut s = sell(1_000_000, &t.mint, &p, true);
    s.actor = stranger.pubkey();
    pool_call_with(&mut hw, &t.mint, &p, 1, &s, &[(pda::holding(&target, &stranger.pubkey()), false)]).0.ok();

    // After the opening anyone buys.
    hw.w.env.warp(3_600);
    let (tx, _) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&target, &stranger.pubkey()), false)]);
    tx.ok();
}

#[test]
fn guest_list_forges_min_hold_down_and_keeps_the_window() {
    let f = forge(a2::GUEST_LIST, 5_000, &[1_000, 3_600], &[400, 3_600]).unwrap();
    assert_eq!(f[..2], [200, 3_600]);
    assert!(forge(a2::GUEST_LIST, 5_000, &[1_000, 3_600], &[1_000, 60]).is_err());
}
