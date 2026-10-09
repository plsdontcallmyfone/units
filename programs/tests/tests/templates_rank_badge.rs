// Changed by Hookwars: new file (arsenal wave B), Rank Badge (id 35, 08 section 4.7).

use bordrless_program_tests::armory::{params, test_schema, Hw};
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::items::equip;
use hookwars_common::{combine, template_id as T};
use hookwars_items::templates::rank_badge::read;
use solana_keypair::Keypair;
use solana_signer::Signer;

/// A token with Rank Badge in slot 2 and a market priced at 2 quote per token for 600 s (or for
/// `age` seconds).
fn setup(age: i64, p: &[u32]) -> (Hw, Keypair, anchor_lang::prelude::Pubkey, anchor_lang::prelude::Pubkey, Keypair) {
    let mut hw = world(&[T::RANK_BADGE]);
    let (owner, mint) = tok_market(&mut hw);
    let rb = item(&mut hw, T::RANK_BADGE, p, 0);
    equip(&mut hw, &owner, &mint, 2, rb, vec![], 0).ok();
    let now = hw.w.env.now;
    let pool = put_market(&mut hw, &mint, 0, 2 * Q64, now - age, &flat(now - age));
    hw.mint_to(&owner, &mint, &pool, 10_000_000);
    let d = delegate(&mut hw, &mint, &pool);
    (hw, owner, mint, pool, d)
}

#[test]
fn buys_earn_units_by_their_quote_value_and_ranks_by_step() {
    let (mut hw, _owner, mint, pool, d) = setup(600, &[100, 10, 5]);
    let alice = hw.w.env.funded(SOL);
    let owners = [pool, alice.pubkey()];
    // 1,000 tokens at 2 quote each: 2,000 quote, 20 units, rank 2.
    send(&mut hw, &d, &mint, &pool, &alice.pubkey(), 1_000).ok();
    assert_eq!(read(&range(&hw, &mint, &alice.pubkey(), 2)), Some((20, 2)));
    send(&mut hw, &d, &mint, &pool, &alice.pubkey(), 1_550).ok();
    assert_eq!(read(&range(&hw, &mint, &alice.pubkey(), 2)), Some((51, 5)));
    // Abuse: wash buys raise the rank only to max_rank.
    send(&mut hw, &d, &mint, &pool, &alice.pubkey(), 100_000).ok();
    assert_eq!(read(&range(&hw, &mint, &alice.pubkey(), 2)), Some((2_051, 5)));
    check(&hw, &mint, &owners);
    // A send moves nothing; emptying clears the badge.
    let bob = hw.w.env.funded(SOL);
    let all = bal(&hw, &mint, &alice.pubkey());
    send(&mut hw, &alice, &mint, &alice.pubkey(), &bob.pubkey(), all).ok();
    assert_eq!(read(&range(&hw, &mint, &bob.pubkey(), 2)), None);
    assert_eq!(read(&range(&hw, &mint, &alice.pubkey(), 2)), None);
    check(&hw, &mint, &[pool, alice.pubkey(), bob.pubkey()]);
}

#[test]
fn without_a_full_window_of_prices_a_buy_earns_nothing() {
    let (mut hw, _owner, mint, pool, d) = setup(100, &[100, 10, 5]);
    let alice = hw.w.env.funded(SOL);
    send(&mut hw, &d, &mint, &pool, &alice.pubkey(), 1_000).ok();
    assert_eq!(read(&range(&hw, &mint, &alice.pubkey(), 2)), None);
    check(&hw, &mint, &[pool, alice.pubkey()]);
}

#[test]
fn rank_badge_forges_its_unit_toward_the_floor_and_keeps_the_steps() {
    let (min, max, _, _) = test_schema(T::RANK_BADGE);
    let out = combine(T::RANK_BADGE, &min, &max, 5_000, &params(&[100, 10, 5]), &params(&[51, 10, 5])).unwrap();
    assert_eq!((out[0], out[1], out[2]), (51 - 25, 10, 5));
    assert!(combine(T::RANK_BADGE, &min, &max, 5_000, &params(&[100, 10, 5]), &params(&[100, 11, 5])).is_err());
}
