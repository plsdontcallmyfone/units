// Changed by Hookwars: new file (arsenal wave D), Ally Pass (08 section 4.4).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use hookwars_items::ArsenalError;
use solana_signer::Signer;

#[test]
fn ally_pass_discounts_buyers_holding_the_ally() {
    let mut hw = world();
    let t = tok(&mut hw);
    let friend = hw.w.env.funded(SOL);
    let other = hw.w.env.funded(SOL);
    let ally = other_token(&mut hw, &friend.pubkey(), 2_000);
    let ap = item(&mut hw, a2::ALLY_PASS, &[1_000, 2_500], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, ap, vec![ally], 0).ok();
    let p = t.pool.pubkey();
    let mut c = buy(1_000_000, &t.mint, &p, true);
    c.actor = friend.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&ally, &friend.pubkey()), false)]);
    assert_eq!((a.discount_bps, a.cut), (2_500, 0));

    // No ally holding, or someone else's holding: no discount.
    c.actor = other.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&ally, &other.pubkey()), false)]);
    assert_eq!(a.discount_bps, 0);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(pda::holding(&ally, &friend.pubkey()), false)]);
    assert_eq!(a.discount_bps, 0, "a borrowed holding gives nothing");

    // Sells: nothing.
    let mut s = sell(1_000_000, &t.mint, &p, true);
    s.actor = friend.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &s, &[(pda::holding(&ally, &friend.pubkey()), false)]);
    assert_eq!(a.discount_bps, 0);
}

#[test]
fn ally_pass_forges_min_down_and_discount_up() {
    let f = forge(a2::ALLY_PASS, 10_000, &[1_000, 2_000], &[500, 3_000]).unwrap();
    assert_eq!(f[..2], [0, 5_000]);
}
