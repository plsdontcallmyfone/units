// Changed by Hookwars: new file (arsenal wave C), Volatility Fee (id 15, 08 section 4.1).

use bordrless_program_tests::armory::{params, test_schema, Hw};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};

/// Volatility Fee [short 300, long 1,200, trigger 500, cut 100] on a market at price `p1` from
/// 2,000 s ago to 600 s ago and `p2` since.
fn setup(p1: u128, p2: u128) -> (Hw, anchor_lang::prelude::Pubkey, anchor_lang::prelude::Pubkey) {
    let mut hw = world(&[T::VOLATILITY_FEE]);
    let (owner, mint) = tok_market(&mut hw);
    let vf = item(&mut hw, T::VOLATILITY_FEE, &[300, 1_200, 500, 100], 0);
    equip(&mut hw, &owner, &mint, 0, vf, vec![], 0).ok();
    let now = hw.w.env.now;
    let pool = put_market(&mut hw, &mint, 0, p2, now - 600, &two_prices(now - 2_000, now - 600, p1, 0));
    (hw, mint, pool)
}

#[test]
fn a_move_from_the_long_average_beyond_the_trigger_charges_both_sides() {
    // Long read: (1.0 x 1,400 + 1.2 x 600) / 2,000 = 1.06; short: 1.2; 1,320 bps apart.
    let (mut hw, mint, pool) = setup(Q64, Q64 * 6 / 5);
    let side = 1_000_000;
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (fee(side, 100), 0, 0, fee(side, 100)));
    // A fall counts the same as a rise.
    let (mut hw, mint, pool) = setup(Q64, Q64 * 4 / 5);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side).0, fee(side, 100));
    // Calm: no cut.
    let (mut hw, mint, pool) = setup(Q64, Q64);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (0, 0, 0, 0));
    // Under the trigger: 1.03 against 1.009, 208 bps apart.
    let (mut hw, mint, pool) = setup(Q64, Q64 * 103 / 100);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (0, 0, 0, 0));
}

#[test]
fn a_one_second_spike_does_not_reach_the_short_read() {
    // Abuse: the ring averages; the template never reads the spot price, so a spike held for one
    // second after a calm stretch leaves both reads calm.
    let mut hw = world(&[T::VOLATILITY_FEE]);
    let (owner, mint) = tok_market(&mut hw);
    let vf = item(&mut hw, T::VOLATILITY_FEE, &[300, 1_200, 500, 100], 0);
    equip(&mut hw, &owner, &mint, 0, vf, vec![], 0).ok();
    let now = hw.w.env.now;
    // Price 1 until 1 s ago, then 3 for the last second.
    let entries = vec![(now - 2_000, 0, 0, 0), (now - 600, Q64 * 1_400, 0, 0), (now - 1, Q64 * 1_999, 0, 0)];
    let pool = put_market(&mut hw, &mint, 0, 3 * Q64, now - 1, &entries);
    // Short read: (1 x 599 + 3 x 1) / 600 = 1.0033; long: 1.001; 23 bps apart: under 500.
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, 1_000_000), (0, 0, 0, 0));
}

#[test]
fn volatility_fee_forges_its_trigger_down_and_its_cut_up() {
    let (min, max, _, _) = test_schema(T::VOLATILITY_FEE);
    let out = combine(T::VOLATILITY_FEE, &min, &max, 5_000, &params(&[300, 1_200, 500, 100]), &params(&[300, 1_200, 800, 50])).unwrap();
    assert_eq!((out[0], out[1], out[2], out[3]), (300, 1_200, 250, 200));
}
