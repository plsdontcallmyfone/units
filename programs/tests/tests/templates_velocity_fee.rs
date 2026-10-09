// Changed by Hookwars: new file (arsenal wave C), Velocity Fee (id 13, 08 section 4.1).

use bordrless_program_tests::armory::{params, test_schema, Hw};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::{equip, equip_state};
use hookwars_common::{combine, template_id as T};

/// A token with Velocity Fee [300 s window, threshold 2, 50 bps per extra trade, max 300] in slot
/// 0 and a market whose last 400 s saw `swaps` trades.
fn setup(swaps: u64, ring_age: i64) -> (Hw, anchor_lang::prelude::Pubkey, anchor_lang::prelude::Pubkey) {
    let mut hw = world(&[T::VELOCITY_FEE]);
    let (owner, mint) = tok_market(&mut hw);
    let vf = item(&mut hw, T::VELOCITY_FEE, &[300, 2, 50, 300], 0);
    equip(&mut hw, &owner, &mint, 0, vf, vec![], 0).ok();
    let now = hw.w.env.now;
    let t1 = now - 400.min(ring_age);
    let entries = if ring_age > 400 {
        vec![(now - ring_age, 0, 0, 0), (t1, Q64 * (ring_age - 400) as u128, 0, 100)]
    } else {
        vec![(t1, 0, 0, 100)]
    };
    let pool = put_market(&mut hw, &mint, 100 + swaps, Q64, t1, &entries);
    (hw, mint, pool)
}

#[test]
fn each_trade_over_the_threshold_adds_its_rate_on_the_quote_side() {
    let (mut hw, mint, pool) = setup(6, 1_000);
    // 6 trades, 4 over the threshold: 200 bps, on a buy's input and a sell's output only.
    let side = 1_000_000;
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (fee(side, 200), 0, 0, fee(side, 200)));
    let st = equip_state(&hw, &mint, 0);
    assert_eq!(st.pool_owed, 2 * fee(side, 200));

    let (mut hw, mint, pool) = setup(2, 1_000);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side), (0, 0, 0, 0));

    // Abuse: an attacker trading often raises the fee only to max_cut_bps.
    let (mut hw, mint, pool) = setup(1_000, 1_000);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, side).0, fee(side, 300));
}

#[test]
fn a_ring_younger_than_the_window_is_no_signal() {
    let (mut hw, mint, pool) = setup(50, 200);
    assert_eq!(four_cuts(&mut hw, &mint, &pool, 0, 1_000_000), (0, 0, 0, 0));
    assert_eq!(equip_state(&hw, &mint, 0).pool_owed, 0);
}

#[test]
fn velocity_fee_forges_by_its_rules() {
    let (min, max, _, _) = test_schema(T::VELOCITY_FEE);
    let out = combine(T::VELOCITY_FEE, &min, &max, 5_000, &params(&[300, 2, 50, 100]), &params(&[300, 4, 20, 200])).unwrap();
    assert_eq!((out[0], out[1], out[2], out[3]), (300, 4 + (1_000 - 4) / 2, 50 + (300 - 50) / 2, 200 + (300 - 200) / 2));
    assert!(combine(T::VELOCITY_FEE, &min, &max, 5_000, &params(&[300, 2, 50, 100]), &params(&[600, 2, 50, 100])).is_err());
}
