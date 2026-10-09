// Changed by Hookwars: new file (arsenal wave E), Sell Ladder (08 section 4.7).

use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use solana_signer::Signer;

#[test]
fn sell_ladder_charges_by_share_of_holding_sold() {
    let mut hw = world();
    let t = tok(&mut hw);
    let sl = item(&mut hw, a2::SELL_LADDER, &[1_000, 50, 300], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, sl, vec![], 0).ok();
    let pool = t.pool.insecure_clone();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 1_000_000);
    hw.mint_to(&t.owner, &t.mint, &bob.pubkey(), 1_000_000);
    let vault = pda::equip_state(&t.mint, 0).0;

    // 5% of the holding: under one step, free.
    transfer(&mut hw, &alice, &t.mint, &pool.pubkey(), 50_000).ok();
    assert_eq!(bal(&hw, &t.mint, &vault), 0);
    // 25%: two steps of 50 bps = 1%.
    transfer(&mut hw, &bob, &t.mint, &pool.pubkey(), 250_000).ok();
    assert_eq!(bal(&hw, &t.mint, &vault), 2_500);
    // Everything left: capped at 3%.
    transfer(&mut hw, &bob, &t.mint, &pool.pubkey(), 750_000).ok();
    assert_eq!(bal(&hw, &t.mint, &vault), 2_500 + 22_500);
    // Sends between wallets and buys pay nothing.
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 900_000).ok();
    transfer(&mut hw, &pool, &t.mint, &alice.pubkey(), 10_000).ok();
    assert_eq!(bal(&hw, &t.mint, &vault), 25_000);
}

#[test]
fn sell_ladder_refuses_a_zero_step_and_forges() {
    let mut hw = world();
    let author = hw.w.env.funded(10 * SOL);
    let (tx, _, _) = hw.create_item(&author, a2::SELL_LADDER, bordrless_program_tests::armory::params(&[1, 100, 50]), 0);
    tx.expect_fail();
    let f = forge(a2::SELL_LADDER, 5_000, &[1_000, 50, 300], &[1_000, 100, 200]).unwrap();
    assert_eq!(f[..3], [1_000, 550, 1_650]);
}
