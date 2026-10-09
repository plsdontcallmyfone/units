// Changed by Hookwars: new file (arsenal wave C), Rush Hour (id 16, 08 section 4.1).

use bordrless_program_tests::armory::{params, test_schema};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};
use solana_signer::Signer;

#[test]
fn the_rate_follows_the_utc_hour_and_wraps_past_midnight() {
    let mut hw = world(&[T::RUSH_HOUR]);
    let t = tok(&mut hw);
    // 22:00 for 4 hours (to 02:00): 200 bps inside, 20 bps outside.
    let rh = item(&mut hw, T::RUSH_HOUR, &[22, 4, 200, 20], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, rh, vec![], 0).ok();
    let pool = t.pool.pubkey();
    let side = 1_000_000;
    for (hour, bps) in [(22, 200), (23, 200), (1, 200), (2, 20), (12, 20), (21, 20)] {
        warp_to_hour(&mut hw, hour, 30);
        assert_eq!(four_cuts(&mut hw, &t.mint, &pool, 0, side), (fee(side, bps), 0, 0, fee(side, bps)), "hour {hour}");
    }
}

#[test]
fn a_full_day_window_is_always_inside() {
    let mut hw = world(&[T::RUSH_HOUR]);
    let t = tok(&mut hw);
    let rh = item(&mut hw, T::RUSH_HOUR, &[5, 24, 50, 300], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, rh, vec![], 0).ok();
    let pool = t.pool.pubkey();
    // Abuse: traders can only time trades into the cheaper hours, which is the intent; a full-day
    // window has none.
    for hour in [0, 4, 5, 13, 23] {
        warp_to_hour(&mut hw, hour, 0);
        assert_eq!(four_cuts(&mut hw, &t.mint, &pool, 0, 1_000_000).0, fee(1_000_000, 50));
    }
}

#[test]
fn rush_hour_keeps_its_window_and_forges_its_cuts_up() {
    let (min, max, _, _) = test_schema(T::RUSH_HOUR);
    let out = combine(T::RUSH_HOUR, &min, &max, 5_000, &params(&[22, 4, 200, 20]), &params(&[22, 4, 100, 60])).unwrap();
    assert_eq!((out[0], out[1], out[2], out[3]), (22, 4, 250, 180));
    assert!(combine(T::RUSH_HOUR, &min, &max, 5_000, &params(&[22, 4, 200, 20]), &params(&[21, 4, 200, 20])).is_err());
}
