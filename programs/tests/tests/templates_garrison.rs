// Changed by Hookwars: new file (arsenal wave E), Garrison (08 section 4.5).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, put_war_state};
use hookwars_common::arsenal2 as a2;

#[test]
fn garrison_discounts_buys_only_under_siege() {
    let mut hw = world();
    let t = tok(&mut hw);
    let g = item(&mut hw, a2::GARRISON, &[1_500], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, g, vec![], 0).ok();
    let p = t.pool.pubkey();
    let now = hw.w.env.now;
    put_war_state(&mut hw, &t.mint, 0, Pubkey::default());
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0, "not under siege");
    put_war_state(&mut hw, &t.mint, now + 600, Pubkey::new_unique());
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 1_500);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0, "sells");
    hw.w.env.warp(601);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.discount_bps, 0, "siege over");
}

#[test]
fn garrison_forges_up() {
    assert_eq!(forge(a2::GARRISON, 10_000, &[100], &[200]).unwrap()[0], 5_000);
}
