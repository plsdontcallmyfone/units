// Changed by Hookwars: new file (arsenal wave D), Referral (08 section 4.7).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{create_holding, equip, equip_state, give_sol, settle_ix};
use hookwars_common::arsenal2 as a2;
use hookwars_common::{ids, pda};
use hookwars_items::ArsenalError;
use solana_signer::Signer;

#[test]
fn referral_records_owed_cuts_and_pays_the_referrer_net() {
    let mut hw = world();
    let t = tok(&mut hw);
    let rf = item(&mut hw, a2::REFERRAL, &[200], 1_000);
    equip(&mut hw, &t.owner, &t.mint, 1, rf, vec![], 0).ok();
    let p = t.pool.pubkey();
    let buyer = hw.w.env.funded(SOL);
    let referrer = hw.w.env.funded(SOL);
    let stranger = hw.w.env.funded(SOL);

    // Self-referral is refused; a referrer is set once.
    let ix = set_referrer_ix(&buyer.pubkey(), &t.mint, &buyer.pubkey());
    hw.w.env.send_paid_by(&[ix], &buyer, &[]).expect_code(code(ArsenalError::SelfReferral));
    let ix = set_referrer_ix(&buyer.pubkey(), &t.mint, &referrer.pubkey());
    hw.w.env.send_paid_by(&[ix], &buyer, &[]).ok();
    let ix = set_referrer_ix(&buyer.pubkey(), &t.mint, &stranger.pubkey());
    hw.w.env.send_paid_by(&[ix], &buyer, &[]).expect_fail();

    // A referred buy pays 2% into owed; a buyer without a record pays nothing.
    let referred = a2::pda::referred(&t.mint, &buyer.pubkey()).0;
    let mut c = buy(1_000_000, &t.mint, &p, true);
    c.actor = buyer.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(referred, true)]);
    assert_eq!(a.cut, 20_000);
    let r: hookwars_items::Referred = hw.w.env.read(&referred);
    assert_eq!((r.owed, r.referrer), (20_000, referrer.pubkey()));
    c.actor = stranger.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(a2::pda::referred(&t.mint, &stranger.pubkey()).0, true)]);
    assert_eq!(a.cut, 0);
    // Someone passing another buyer's record gets nothing and moves nothing.
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[(referred, true)]);
    assert_eq!(a.cut, 0);
    assert_eq!(hw.w.env.read::<hookwars_items::Referred>(&referred).owed, 20_000);

    // settle_equip moves the module's share (royalty and bounty out) into the Referral vault.
    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&t.mint).0, 20_000);
    let sol = hw.w.sol;
    let owner = a2::pda::referral_owner(&t.mint).0;
    create_holding(&mut hw, &funder, &sol, &owner);
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 1, &[(ids::ITEMS_ID, pda::holding(&sol, &owner))]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let bps = u64::from(hw.config().params.settle_bounty_bps);
    let royalty = 2_000u64;
    let in_vault = 20_000 - royalty - (20_000 - royalty) * bps / 10_000;
    assert_eq!(bal(&hw, &sol, &owner), in_vault);
    assert_eq!(equip_state(&hw, &t.mint, 1).pool_unsettled[0], 0);

    // settle_referral pays the referrer the same net, less this sender's bounty.
    create_holding(&mut hw, &funder, &sol, &referrer.pubkey());
    let c2 = hw.w.env.funded(SOL);
    create_holding(&mut hw, &funder, &sol, &c2.pubkey());
    let ix = settle_referral_ix(&hw, &c2.pubkey(), &t.mint, 1, &buyer.pubkey());
    hw.w.env.send_paid_by(&[ix], &c2, &[]).ok();
    let bounty = in_vault * bps / 10_000;
    assert_eq!(bal(&hw, &sol, &referrer.pubkey()), in_vault - bounty);
    assert_eq!(bal(&hw, &sol, &c2.pubkey()), bounty);
    assert_eq!(bal(&hw, &sol, &owner), 0);
    let r: hookwars_items::Referred = hw.w.env.read(&referred);
    assert_eq!((r.owed, r.paid), (0, in_vault - bounty));
    // Nothing left: a second settle is refused.
    let ix = settle_referral_ix(&hw, &c2.pubkey(), &t.mint, 1, &buyer.pubkey());
    hw.w.env.send_paid_by(&[ix], &c2, &[]).expect_code(code(ArsenalError::NothingToPay));
    let _ = Pubkey::default();
}

#[test]
fn referral_forges_up() {
    assert_eq!(forge(a2::REFERRAL, 10_000, &[100], &[50]).unwrap()[0], 300);
}
