// Changed by Hookwars: new file (arsenal wave E), Loyalty Pot (08 section 4.3).

use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{create_holding, equip, give_sol, settle_ix, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::{ids, pda};
use hookwars_items::ArsenalError;
use solana_signer::Signer;

#[test]
fn loyalty_pot_pays_full_epoch_holders_by_balance() {
    let mut hw = world();
    let t = tok(&mut hw);
    let lp = item(&mut hw, a2::LOYALTY_POT, &[100, 86_400], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, lp, vec![], 0).ok();
    let payer = t.owner.insecure_clone();
    hw.w.env.send_paid_by(&[init_loyalty_ix(&payer.pubkey(), &t.mint, 1)], &payer, &[]).ok();
    let p = t.pool.pubkey();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 600_000);
    hw.mint_to(&t.owner, &t.mint, &bob.pubkey(), 400_000);

    // Sells pay 1% of the quote out into the pot (through settlement).
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 10_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.cut, 0, "buys pay nothing");
    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&t.mint).0, 10_000);
    let sol = hw.w.sol;
    let pot = a2::pda::loyalty(&t.mint).0;
    create_holding(&mut hw, &funder, &sol, &pot);
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 1, &[(ids::ITEMS_ID, pda::holding(&sol, &pot))]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let bps = u64::from(hw.config().params.settle_bounty_bps);
    let in_pot = 10_000 - 10_000 * bps / 10_000;
    assert_eq!(bal(&hw, &sol, &pot), in_pot);
    for w in [&alice, &bob] {
        create_holding(&mut hw, &funder, &sol, &w.pubkey());
    }

    // The next epoch: bob receives tokens in it, so only alice claims it.
    hw.w.env.warp(86_400);
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 100_000).ok();
    let ix = claim_loyalty_ix(&hw, &alice.pubkey(), &t.mint, &p);
    hw.w.env.send_paid_by(&[ix], &alice, &[]).ok();
    let alice_paid = in_pot * 500_000 / 1_000_000;
    assert_eq!(bal(&hw, &sol, &alice.pubkey()), alice_paid);
    let ix = claim_loyalty_ix(&hw, &alice.pubkey(), &t.mint, &p);
    hw.w.env.send_paid_by(&[ix], &alice, &[]).expect_code(code(ArsenalError::NotEligible));
    let ix = claim_loyalty_ix(&hw, &bob.pubkey(), &t.mint, &p);
    hw.w.env.send_paid_by(&[ix], &bob, &[]).expect_code(code(ArsenalError::NotEligible));

    // The epoch after: what was not claimed rolls in; bob has held a full epoch now.
    hw.w.env.warp(86_400);
    let left = in_pot - alice_paid;
    let ix = claim_loyalty_ix(&hw, &bob.pubkey(), &t.mint, &p);
    hw.w.env.send_paid_by(&[ix], &bob, &[]).ok();
    assert_eq!(bal(&hw, &sol, &bob.pubkey()), left * 500_000 / 1_000_000);
    let ix = claim_loyalty_ix(&hw, &alice.pubkey(), &t.mint, &p);
    hw.w.env.send_paid_by(&[ix], &alice, &[]).ok();
    // Claims never exceed what the pot held.
    let paid = bal(&hw, &sol, &alice.pubkey()) + bal(&hw, &sol, &bob.pubkey());
    assert!(paid <= in_pot);
    assert_eq!(bal(&hw, &sol, &pot), in_pot - paid);
}

#[test]
fn loyalty_pot_refuses_a_zero_epoch_and_forges() {
    let mut hw = world();
    let author = hw.w.env.funded(10 * SOL);
    let (tx, _, _) = hw.create_item(&author, a2::LOYALTY_POT, bordrless_program_tests::armory::params(&[100, 0]), 0);
    tx.expect_fail();
    let f = forge(a2::LOYALTY_POT, 5_000, &[100, 86_400], &[300, 86_400]).unwrap();
    assert_eq!(f[..2], [650, 86_400]);
}
