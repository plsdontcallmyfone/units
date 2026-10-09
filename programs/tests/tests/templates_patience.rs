// Changed by Hookwars: new file (arsenal wave D), Patience (08 section 4.7).

use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, init_ledger, transfer};
use hookwars_common::arsenal2 as a2;
use solana_signer::Signer;

#[test]
fn patience_discounts_old_sellers_once_and_not_young_ones() {
    let mut hw = world();
    let t = tok(&mut hw);
    let pa = item(&mut hw, a2::PATIENCE, &[3_600, 2_000, 1], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, pa, vec![], 0).ok();
    let payer = t.owner.insecure_clone();
    init_ledger(&mut hw, &payer, &t.mint).ok();
    let p = t.pool.pubkey();
    let old = hw.w.env.funded(SOL);
    let young = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &t.pool.pubkey(), 1_000_000);

    // Buys stamp the age.
    let pool = t.pool.insecure_clone();
    transfer(&mut hw, &pool, &t.mint, &old.pubkey(), 100_000).ok();
    assert_eq!(range(&hw, &t.mint, &old.pubkey(), 1)[0], 0x28);
    hw.w.env.warp(3_600);
    transfer(&mut hw, &pool, &t.mint, &young.pubkey(), 100_000).ok();

    // The old holder sells: marked, then discounted once.
    transfer(&mut hw, &old, &t.mint, &p, 10_000).ok();
    let mut s = sell(500_000, &t.mint, &p, false);
    s.actor = old.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &s, &[]);
    assert_eq!(a.discount_bps, 2_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &s, &[]);
    assert_eq!(a.discount_bps, 0, "a mark discounts one sell");

    // The young holder sells: no mark, no discount; someone else's sell never borrows a mark.
    transfer(&mut hw, &young, &t.mint, &p, 10_000).ok();
    s.actor = young.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &s, &[]);
    assert_eq!(a.discount_bps, 0);
}

#[test]
fn patience_age_travels_and_blends() {
    let mut hw = world();
    let t = tok(&mut hw);
    let pa = item(&mut hw, a2::PATIENCE, &[3_600, 2_000, 1], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, pa, vec![], 0).ok();
    let pool = t.pool.insecure_clone();
    let a = hw.w.env.funded(SOL);
    let fresh = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &pool.pubkey(), 1_000_000);
    transfer(&mut hw, &pool, &t.mint, &a.pubkey(), 100_000).ok();
    let since = |b: Vec<u8>| u32::from_le_bytes(b[1..5].try_into().unwrap());
    let a_since = since(range(&hw, &t.mint, &a.pubkey(), 1));
    hw.w.env.warp(7_200);
    // Sent to a fresh wallet, the tokens keep their age.
    transfer(&mut hw, &a, &t.mint, &fresh.pubkey(), 50_000).ok();
    assert_eq!(since(range(&hw, &t.mint, &fresh.pubkey(), 1)), a_since);
    // A buy blends by weight: half new tokens move the stamp halfway.
    transfer(&mut hw, &pool, &t.mint, &fresh.pubkey(), 50_000).ok();
    let now = u32::try_from(hw.w.env.now).unwrap();
    assert_eq!(since(range(&hw, &t.mint, &fresh.pubkey(), 1)), (a_since + now) / 2);
}

#[test]
fn patience_only_accepts_its_own_age_source_and_forges() {
    let mut hw = world();
    let author = hw.w.env.funded(10 * SOL);
    let (tx, _, _) = hw.create_item(&author, a2::PATIENCE, bordrless_program_tests::armory::params(&[3_600, 2_000, 2]), 0);
    tx.expect_fail();
    let f = forge(a2::PATIENCE, 5_000, &[3_600, 1_000, 1], &[1_800, 2_000, 1]).unwrap();
    assert_eq!(f[..3], [900, 3_500, 1]);
}
