// Changed by Hookwars: new file (arsenal wave E), First Blood (08 section 4.7).

use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::equip;
use hookwars_common::arsenal2 as a2;
use solana_signer::Signer;

#[test]
fn first_blood_gives_one_prize_a_day() {
    let mut hw = world();
    let t = tok(&mut hw);
    let fb = item(&mut hw, a2::FIRST_BLOOD, &[3_000, 2_000_000], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, fb, vec![], 0).ok();
    let p = t.pool.pubkey();

    // Without its state: nothing.
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(5_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0);
    let payer = t.owner.insecure_clone();
    hw.w.env.send_paid_by(&[init_first_blood_ix(&payer.pubkey(), &t.mint)], &payer, &[]).ok();

    // Under the minimum: nothing; the first big buy wins; the second does not.
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(5_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 3_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(5_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0);
    // Sells never win.
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(5_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0);
    // Next day, a new prize.
    hw.w.env.warp(86_400);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(5_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 3_000);
}

#[test]
fn first_blood_forges_the_discount_and_keeps_the_minimum() {
    let f = forge(a2::FIRST_BLOOD, 5_000, &[1_000, 7], &[3_000, 7]).unwrap();
    assert_eq!(f[..2], [4_000, 7]);
}
