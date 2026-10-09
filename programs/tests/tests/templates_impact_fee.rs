// Changed by Hookwars: new file (arsenal wave C), Impact Fee (id 14, 08 section 4.1).

use bordrless_program_tests::armory::{params, test_schema};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::{equip, equip_state};
use hookwars_common::{combine, template_id as T};
use solana_signer::Signer;

#[test]
fn the_cut_scales_with_the_trades_own_price_move() {
    let mut hw = world(&[T::IMPACT_FEE]);
    let t = tok(&mut hw);
    let imp = item(&mut hw, T::IMPACT_FEE, &[100, 300], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, imp, vec![], 0).ok();
    let pool = t.pool.pubkey();
    let deep = Book {
        base_reserve: 1_000_000_000,
        quote_reserve: 1_000_000_000,
        ..Default::default()
    };

    // A buy of 1% of the quote side moves the price (1.01^2 - 1) = 201 bps: a 201 bps cut.
    let q = 10_000_000;
    let (_, a) = pool_call_book(&mut hw, &t.mint, &pool, 0, &buy(q, &t.mint, &pool, true), deep);
    assert_eq!(a.cut, fee(q, 201));
    // The same buy against ten times the depth (virtual reserves count) moves it 20 bps.
    let deeper = Book {
        virtual_base: 9_000_000_000,
        virtual_quote: 9_000_000_000,
        ..deep
    };
    let (_, a) = pool_call_book(&mut hw, &t.mint, &pool, 0, &buy(q, &t.mint, &pool, true), deeper);
    assert_eq!(a.cut, fee(q, 20));

    // A sell measured after the swap: 1e7 base in moved the pool from (1e9, 1e9) to
    // (1.01e9, 990,099,010): the price fell 197 bps.
    let after = Book {
        base_reserve: 1_010_000_000,
        quote_reserve: 990_099_010,
        amount_out: 9_900_990,
        ..Default::default()
    };
    let mut call = sell(9_900_990, &t.mint, &pool, false);
    call.amount_in = 10_000_000;
    let (_, a) = pool_call_book(&mut hw, &t.mint, &pool, 0, &call, after);
    assert_eq!(a.cut, fee(9_900_990, 197));
    // The base sides answer nothing.
    let (_, a) = pool_call_book(&mut hw, &t.mint, &pool, 0, &sell(q, &t.mint, &pool, true), deep);
    assert_eq!(a.cut, 0);
    assert_eq!(equip_state(&hw, &t.mint, 0).pool_owed, fee(q, 201) + fee(q, 20) + fee(9_900_990, 197));
}

#[test]
fn a_whale_trade_is_capped_and_splitting_pays_the_launch_fees_each_time() {
    let mut hw = world(&[T::IMPACT_FEE]);
    let t = tok(&mut hw);
    let imp = item(&mut hw, T::IMPACT_FEE, &[100, 300], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, imp, vec![], 0).ok();
    let pool = t.pool.pubkey();
    let book = Book {
        base_reserve: 1_000_000_000,
        quote_reserve: 1_000_000_000,
        ..Default::default()
    };
    // A buy of half the quote side would move the price 12,500 bps: capped at 300.
    let q = 500_000_000;
    let (_, a) = pool_call_book(&mut hw, &t.mint, &pool, 0, &buy(q, &t.mint, &pool, true), book);
    assert_eq!(a.cut, fee(q, 300));
    // Abuse: a hundredth of it moves the price 100 bps, so the item takes 100 bps of it; the
    // launch's own fees are still paid per trade (the item never waives them).
    let (_, a) = pool_call_book(&mut hw, &t.mint, &pool, 0, &buy(q / 100, &t.mint, &pool, true), book);
    assert_eq!(a.cut, fee(q / 100, 100));
    assert_eq!(a.discount_bps, 0);
}

#[test]
fn impact_fee_forges_toward_its_ceilings() {
    let (min, max, _, _) = test_schema(T::IMPACT_FEE);
    let out = combine(T::IMPACT_FEE, &min, &max, 5_000, &params(&[100, 100]), &params(&[200, 50])).unwrap();
    assert_eq!((out[0], out[1]), (200 + (1_000 - 200) / 2, 100 + (300 - 100) / 2));
}
