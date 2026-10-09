// Changed by Hookwars: new file (arsenal wave D), Embargo (08 section 4.4).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use hookwars_items::ArsenalError;
use solana_signer::Signer;

use bordrless_program_tests::items::{create_holding, give_sol, raid_route, settle_ix};
use hookwars_common::ids;

#[test]
fn embargo_charges_arrivals_from_its_targets_and_settles_to_the_war_chest() {
    let mut hw = world();
    let t = tok(&mut hw);
    let rival = Pubkey::new_unique();
    let em = item(&mut hw, a2::EMBARGO, &[500], 1_000);
    equip(&mut hw, &t.owner, &t.mint, 1, em, vec![rival], 0).ok();
    let p = t.pool.pubkey();

    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.cut, 0, "a plain buy");
    let mut c = buy(1_000_000, &t.mint, &p, true);
    c.route = raid_route(&Pubkey::new_unique(), &Pubkey::new_unique(), &t.mint, 1_000_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[]);
    assert_eq!(a.cut, 0, "from another token");
    c.route = raid_route(&rival, &Pubkey::new_unique(), &t.mint, 1_000_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[]);
    assert_eq!(a.cut, 50_000);
    let mut s = sell(1_000_000, &t.mint, &p, true);
    s.route = c.route.clone();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &s, &[]);
    assert_eq!(a.cut, 0, "sells are not embargoed");

    // Settlement: royalty, bounty, the rest to the war chest.
    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&t.mint).0, 50_000);
    let chest = Pubkey::find_program_address(&[b"war-chest", t.mint.as_ref()], &ids::WAR_ID).0;
    let sol = hw.w.sol;
    create_holding(&mut hw, &funder, &sol, &chest);
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 1, &[(ids::ITEMS_ID, pda::holding(&sol, &chest))]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let bounty_bps = u64::from(hw.config().params.settle_bounty_bps);
    let royalty = 5_000u64;
    let bounty = (50_000 - royalty) * bounty_bps / 10_000;
    assert_eq!(bal(&hw, &sol, &chest), 50_000 - royalty - bounty);
}

#[test]
fn embargo_forges_up() {
    assert_eq!(forge(a2::EMBARGO, 5_000, &[100], &[300]).unwrap()[0], 650);
}
