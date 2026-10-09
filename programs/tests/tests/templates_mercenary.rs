// Changed by Hookwars: new file (arsenal wave E), Mercenary (08 section 4.5).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, init_ledger, put_war_config, raid_route, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use hookwars_common::raid::{RaidLedger, RaidRange};
use solana_signer::Signer;

#[test]
fn mercenary_marks_any_inflow_and_stamps_points_without_counting_siege_volume() {
    let mut hw = world();
    let t = tok(&mut hw);
    put_war_config(&mut hw, 1);
    let me = item(&mut hw, a2::MERCENARY, &[2], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, me, vec![], 0).ok();
    let payer = t.owner.insecure_clone();
    init_ledger(&mut hw, &payer, &t.mint).ok();
    let p = t.pool.pubkey();
    let buyer = hw.w.env.funded(SOL);
    let amount = 30 * 1_000_000u64;

    // A plain buy: no mark.
    let mut c = buy(amount, &t.mint, &p, false);
    c.recipient = buyer.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[]);
    assert_eq!((a.cut, a.discount_bps), (0, 0));
    let ledger = |hw: &bordrless_program_tests::armory::Hw| {
        RaidLedger::decode(&hw.w.env.account(&pda::raid_ledger(&t.mint).0).unwrap().data).unwrap()
    };
    assert_eq!(ledger(&hw).mark.clock_slot, 0);

    // Arriving from any other token marks, with no discount and no siege volume.
    c.route = raid_route(&Pubkey::new_unique(), &Pubkey::new_unique(), &t.mint, amount);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &c, &[]);
    assert_eq!((a.cut, a.discount_bps), (0, 0));
    let l = ledger(&hw);
    assert_eq!((l.mark.recipient, l.mark.quote_volume), (buyer.pubkey(), amount));
    assert!(l.inbound.iter().all(|e| e.volume == 0), "no inbound volume");
    assert_eq!(l.outbound_volume_season, 0);

    // The delivery stamps points: 30 units x 2 points (TEST point unit 1,000,000 lamports).
    let pool = t.pool.insecure_clone();
    hw.mint_to(&t.owner, &t.mint, &p, 1_000_000);
    transfer(&mut hw, &pool, &t.mint, &buyer.pubkey(), 100_000).ok();
    let r = RaidRange::read(&range(&hw, &t.mint, &buyer.pubkey(), 1), 1);
    let unit = bordrless_program_tests::war::TEST_PARAMS.point_unit_lamports;
    assert_eq!(u64::from(r.raid_points), amount / unit * 2);
}

#[test]
fn mercenary_forges_up() {
    assert_eq!(forge(a2::MERCENARY, 0, &[3], &[7]).unwrap()[0], 7);
}
