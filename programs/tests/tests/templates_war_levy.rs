// Changed by Hookwars: new file (arsenal wave E), War Levy (08 section 4.5).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, init_ledger, put_war_config};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use hookwars_common::raid::RaidLedger;

fn set_inbound(hw: &mut bordrless_program_tests::armory::Hw, mint: &Pubkey, volume: u64) {
    let key = pda::raid_ledger(mint).0;
    let mut acc = hw.w.env.account(&key).unwrap();
    let mut l = RaidLedger::decode(&acc.data).unwrap();
    l.inbound[0].rival_mint = Pubkey::new_unique();
    l.inbound[0].window_start = hw.w.env.now;
    l.inbound[0].volume = volume;
    l.encode(&mut acc.data).unwrap();
    hw.w.env.put(key, acc);
}

#[test]
fn war_levy_charges_sellers_while_a_raid_is_under_way() {
    let mut hw = world();
    let t = tok(&mut hw);
    put_war_config(&mut hw, 1);
    let wl = item(&mut hw, a2::WAR_LEVY, &[5_000_000, 300], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, wl, vec![], 0).ok();
    let payer = t.owner.insecure_clone();
    init_ledger(&mut hw, &payer, &t.mint).ok();
    let p = t.pool.pubkey();

    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 0, "no raid");
    set_inbound(&mut hw, &t.mint, 4_999_999);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 0, "under the trigger");
    set_inbound(&mut hw, &t.mint, 5_000_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 30_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.cut, 0, "buys pay nothing");
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.cut, 0, "a sell's before side is base");
    // Two windows later the raid has passed.
    let w = bordrless_program_tests::war::TEST_PARAMS.raid_window_secs;
    hw.w.env.warp(2 * w);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 0, "the raid passed");
}

#[test]
fn war_levy_forges_trigger_down_and_cut_up() {
    let f = forge(a2::WAR_LEVY, 10_000, &[1_000, 100], &[2_000, 200]).unwrap();
    assert_eq!(f[..2], [0, 1_000]);
}
