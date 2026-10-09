// Changed by Hookwars: new file (arsenal wave C), Dump Brake (id 21, 08 section 4.2).

use bordrless_program_tests::armory::{params, test_schema, Hw};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::{equip, equip_state};
use hookwars_common::{combine, template_id as T};

/// Dump Brake [short 300, long 1,200, drop 1,000, sell cut 200] on a market at `p1` from 2,000 s
/// to 600 s ago and `p2` since.
fn setup(p1: u128, p2: u128) -> (Hw, anchor_lang::prelude::Pubkey, anchor_lang::prelude::Pubkey) {
    let mut hw = world(&[T::DUMP_BRAKE]);
    let (owner, mint) = tok_market(&mut hw);
    let db = item(&mut hw, T::DUMP_BRAKE, &[300, 1_200, 1_000, 200], 0);
    equip(&mut hw, &owner, &mint, 0, db, vec![], 0).ok();
    let now = hw.w.env.now;
    let pool = put_market(&mut hw, &mint, 0, p2, now - 600, &two_prices(now - 2_000, now - 600, p1, 0));
    (hw, mint, pool)
}

#[test]
fn sells_pay_while_the_price_is_well_under_its_average() {
    // Short 0.8 against long 0.94: under 0.94 x 90% = 0.846: braking; only a sell's output pays.
    let (mut hw, mint, pool) = setup(Q64, Q64 * 4 / 5);
    let side = 1_000_000;
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (0, 0, 0, fee(side, 200)));
    assert_eq!(equip_state(&hw, &mint, 0).pool_owed, fee(side, 200));
    // Short 0.9 against long 0.97: over 0.873: no brake.
    let (mut hw, mint, pool) = setup(Q64, Q64 * 9 / 10);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (0, 0, 0, 0));
    // A rise never brakes.
    let (mut hw, mint, pool) = setup(Q64, Q64 * 3 / 2);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (0, 0, 0, 0));
}

#[test]
fn a_dumper_cannot_lift_the_reads_with_a_one_second_spike() {
    // Abuse: after a fall to 0.8, a spike to 2.0 for the last second moves the short read to
    // (0.8 x 599 + 2.0) / 600 = 0.802 against a long read of 0.94: still braking.
    let mut hw = world(&[T::DUMP_BRAKE]);
    let (owner, mint) = tok_market(&mut hw);
    let db = item(&mut hw, T::DUMP_BRAKE, &[300, 1_200, 1_000, 200], 0);
    equip(&mut hw, &owner, &mint, 0, db, vec![], 0).ok();
    let now = hw.w.env.now;
    let p08 = Q64 * 4 / 5;
    let entries = vec![
        (now - 2_000, 0, 0, 0),
        (now - 600, Q64 * 1_400, 0, 0),
        (now - 1, Q64 * 1_400 + p08 * 599, 0, 0),
    ];
    let pool = put_market(&mut hw, &mint, 0, 2 * Q64, now - 1, &entries);
    let side = 1_000_000;
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side).3, fee(side, 200));
}

#[test]
fn dump_brake_forges_its_drop_down_and_its_cut_up() {
    let (min, max, _, _) = test_schema(T::DUMP_BRAKE);
    let out = combine(T::DUMP_BRAKE, &min, &max, 5_000, &params(&[300, 1_200, 1_000, 200]), &params(&[300, 1_200, 600, 100])).unwrap();
    assert_eq!((out[2], out[3]), (300, 250));
}
