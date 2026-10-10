// Changed by Hookwars: new file, `hookwars_book` (docs/spec/11-hook-economy.md section 6).
//! Price-time priority, partial fills, exact fees, cancel, eviction of the worst order when a side
//! is full, crank of expired orders with its bounty, post-only, and class bids that fill only
//! fitting items. After every step the escrow holds exactly what the resting orders are owed
//! (escrow conservation, R44).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::params;
use bordrless_program_tests::economy::*;
use bordrless_program_tests::expansion::list_ix;
use hookwars_book::error::BookError as E;
use hookwars_book::state::{side, ClassKey};
use hookwars_common::economy::counter;
use hookwars_common::{template_id as t, PARAM_FIELDS};
use hookwars_craft::state::{material_mint_address, source};
use solana_keypair::Keypair;
use solana_signer::Signer;

const MAT: u16 = 1;
const TICK: u64 = 1_000;

/// A world with one material book (tick 1,000 lamports, min size 1).
fn world() -> (Ew, Pubkey) {
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(source::QUEST_CLAIM, MAT, 1, 1);
    let creator = ew.funded(SOL);
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 1);
    ew.send(&creator, &[ix]).ok();
    (ew, material_mint_address(MAT).0)
}

/// A trader with `units` of the material and 100 SOL.
fn trader(ew: &mut Ew, units: u64) -> Keypair {
    let k = ew.funded(100 * SOL);
    if units > 0 {
        ew.stub_drop(source::QUEST_CLAIM, MAT, units, &k.pubkey()).ok();
    }
    k
}

fn place(ew: &mut Ew, who: &Keypair, base: &Pubkey, s: u8, price: u64, size: u64, makers: &[Pubkey]) -> bordrless_program_tests::Tx {
    let ix = ew.place_ix(&who.pubkey(), base, s, price, size, false, 0, makers);
    let tx = ew.send(who, &[ix]);
    if tx.result.is_ok() {
        ew.assert_escrow(base);
    }
    tx
}

#[test]
fn price_time_priority_partial_fills_and_exact_fees() {
    let (mut ew, base) = world();
    let (a, b, c) = (trader(&mut ew, 100), trader(&mut ew, 100), trader(&mut ew, 100));
    // Asks: a at 5,000 (first), b at 5,000 (second), c at 4,000 (best).
    place(&mut ew, &a, &base, side::ASK, 5 * TICK, 10, &[]).ok();
    place(&mut ew, &b, &base, side::ASK, 5 * TICK, 10, &[]).ok();
    place(&mut ew, &c, &base, side::ASK, 4 * TICK, 10, &[]).ok();
    let m = ew.book(&base);
    assert_eq!(m.asks.iter().map(|o| o.owner).collect::<Vec<_>>(), vec![c.pubkey(), a.pubkey(), b.pubkey()]);
    // A buyer takes 25 at up to 5,000: all of c, all of a, 5 of b.
    let buyer = trader(&mut ew, 0);
    ew.open_profile(&buyer);
    let (tr0, a0, b0, c0, u0) = (
        ew.lamports(&ew.book_treasury),
        ew.lamports(&a.pubkey()),
        ew.lamports(&b.pubkey()),
        ew.lamports(&c.pubkey()),
        ew.lamports(&buyer.pubkey()),
    );
    let tx = place(&mut ew, &buyer, &base, side::BID, 5 * TICK, 25, &[c.pubkey(), a.pubkey(), b.pubkey()]);
    tx.ok();
    assert_eq!(ew.holding(&base, &buyer.pubkey()), 25);
    let fees = |quote: u64| (quote * u64::from(TEST_BOOK.taker_bps) / 10_000, quote * u64::from(TEST_BOOK.maker_bps) / 10_000);
    let (qc, qa, qb) = (40 * TICK, 50 * TICK, 25 * TICK);
    let (tc, mc) = fees(qc);
    let (ta, ma) = fees(qa);
    let (tb, mb) = fees(qb);
    // Filled makers get their quote less the maker fee, plus their bounty back when done.
    let bounty = TEST_BOOK.order_bounty_lamports;
    assert_eq!(ew.lamports(&c.pubkey()) - c0, qc - mc + bounty);
    assert_eq!(ew.lamports(&a.pubkey()) - a0, qa - ma + bounty);
    assert_eq!(ew.lamports(&b.pubkey()) - b0, qb - mb);
    assert_eq!(ew.lamports(&ew.book_treasury) - tr0, tc + mc + ta + ma + tb + mb);
    // The buyer paid quote plus taker fees, plus the transaction fee and the holding rent.
    let paid = u0 - ew.lamports(&buyer.pubkey());
    assert!(paid >= qc + qa + qb + tc + ta + tb);
    let m = ew.book(&base);
    assert_eq!(m.asks.len(), 1);
    assert_eq!((m.asks[0].owner, m.asks[0].size), (b.pubkey(), 5));
    assert!(m.bids.is_empty());
    // The taker's fills counted.
    assert_eq!(ew.profile(&buyer.pubkey()).counters[usize::from(counter::BOOK_FILLS)], 3);
}

#[test]
fn a_seller_crosses_resting_bids_and_partial_bids_keep_their_reserve() {
    let (mut ew, base) = world();
    let (a, b) = (trader(&mut ew, 0), trader(&mut ew, 0));
    place(&mut ew, &a, &base, side::BID, 3 * TICK, 10, &[]).ok();
    place(&mut ew, &b, &base, side::BID, 2 * TICK, 10, &[]).ok();
    let seller = trader(&mut ew, 100);
    let (s0, tr0) = (ew.lamports(&seller.pubkey()), ew.lamports(&ew.book_treasury));
    let hold = |ew: &Ew, o: &Pubkey| ew.lamports(&bordrless_token::client::holding_address(&base, o));
    // Sells 13 at down to 2,000: 10 to a at 3,000, 3 to b at 2,000.
    place(&mut ew, &seller, &base, side::ASK, 2 * TICK, 13, &[a.pubkey(), b.pubkey()]).ok();
    assert_eq!(ew.holding(&base, &a.pubkey()), 10);
    assert_eq!(ew.holding(&base, &b.pubkey()), 3);
    let (qa, qb) = (30 * TICK, 6 * TICK);
    let tf = |q: u64| q * u64::from(TEST_BOOK.taker_bps) / 10_000;
    let mf = |q: u64| q * u64::from(TEST_BOOK.maker_bps) / 10_000;
    assert_eq!(ew.lamports(&ew.book_treasury) - tr0, tf(qa) + mf(qa) + tf(qb) + mf(qb));
    // The seller receives quote less taker fees (the fee payer paid the transaction fee).
    // The seller paid the makers' new holdings.
    let rents = hold(&ew, &a.pubkey()) + hold(&ew, &b.pubkey());
    assert_eq!(ew.lamports(&seller.pubkey()) + rents + 5_000 - s0, qa - tf(qa) + qb - tf(qb));
    let m = ew.book(&base);
    assert_eq!((m.bids.len(), m.bids[0].size), (1, 7));
    // Cancelling b's remainder returns its whole remaining lock and bounty.
    let b0 = ew.lamports(&b.pubkey());
    let lock = m.bids[0].quote_locked + m.bids[0].bounty;
    let id = m.bids[0].id;
    let ix = ew.cancel_ix(&b.pubkey(), &base, id);
    ew.send(&b, &[ix]).ok();
    assert_eq!(ew.lamports(&b.pubkey()) - b0, lock - 5_000);
    ew.assert_escrow(&base);
    assert!(ew.book(&base).bids.is_empty());
}

#[test]
fn cancel_is_owner_only_and_returns_the_base() {
    let (mut ew, base) = world();
    let a = trader(&mut ew, 50);
    place(&mut ew, &a, &base, side::ASK, 5 * TICK, 20, &[]).ok();
    assert_eq!(ew.holding(&base, &a.pubkey()), 30);
    let id = ew.book(&base).asks[0].id;
    let stranger = trader(&mut ew, 0);
    let ix = ew.cancel_ix(&stranger.pubkey(), &base, id);
    ew.send(&stranger, &[ix]).expect_code(book_code(E::NotOwner));
    let ix = ew.cancel_ix(&a.pubkey(), &base, 999);
    ew.send(&a, &[ix]).expect_code(book_code(E::NoSuchOrder));
    let ix = ew.cancel_ix(&a.pubkey(), &base, id);
    ew.send(&a, &[ix]).ok();
    assert_eq!(ew.holding(&base, &a.pubkey()), 50);
    ew.assert_escrow(&base);
}

#[test]
fn a_full_side_evicts_its_worst_order_only_for_a_better_one() {
    let (mut ew, base) = world();
    let mut owners = Vec::new();
    for i in 0..TEST_BOOK.slots {
        let k = trader(&mut ew, 0);
        place(&mut ew, &k, &base, side::BID, (10 + u64::from(i)) * TICK, 1, &[]).ok();
        owners.push(k);
    }
    // Not better than the worst (10,000): refused.
    let late = trader(&mut ew, 0);
    place(&mut ew, &late, &base, side::BID, 10 * TICK, 1, &[]).expect_code(book_code(E::BookFull));
    // Better: the worst (owners[0]) is refunded in full.
    let worst = owners[0].pubkey();
    let w0 = ew.lamports(&worst);
    let m = ew.book(&base);
    let refund = m.bids.last().unwrap().quote_locked + m.bids.last().unwrap().bounty;
    place(&mut ew, &late, &base, side::BID, 11 * TICK, 1, &[worst]).ok();
    assert_eq!(ew.lamports(&worst) - w0, refund);
    let m = ew.book(&base);
    assert_eq!(m.bids.len(), usize::from(TEST_BOOK.slots));
    assert!(m.bids.iter().all(|o| o.owner != worst));
    // Time priority at equal price: the earlier 11,000 bid stays ahead of the later one.
    let elevens: Vec<_> = m.bids.iter().filter(|o| o.price == 11 * TICK).map(|o| o.owner).collect();
    assert_eq!(elevens, vec![owners[1].pubkey(), late.pubkey()]);
}

#[test]
fn crank_returns_expired_orders_and_pays_the_bounty() {
    let (mut ew, base) = world();
    let (a, b) = (trader(&mut ew, 10), trader(&mut ew, 0));
    let exp = ew.now() + 60;
    let ix = ew.place_ix(&a.pubkey(), &base, side::ASK, 5 * TICK, 10, false, exp, &[]);
    ew.send(&a, &[ix]).ok();
    let ix = ew.place_ix(&b.pubkey(), &base, side::BID, 4 * TICK, 10, false, exp, &[]);
    ew.send(&b, &[ix]).ok();
    ew.assert_escrow(&base);
    let cranker = trader(&mut ew, 0);
    // Nothing expired yet: nothing removed.
    let ix = ew.crank_ix(&cranker.pubkey(), &base, 8, &[]);
    ew.send(&cranker, &[ix]).ok();
    assert_eq!(ew.book(&base).asks.len(), 1);
    ew.warp(61);
    // An expired ask no longer fills.
    let buyer = trader(&mut ew, 0);
    let ix = ew.place_ix(&buyer.pubkey(), &base, side::BID, 5 * TICK, 1, true, 0, &[]);
    ew.send(&buyer, &[ix]).ok();
    assert_eq!(ew.holding(&base, &buyer.pubkey()), 0);
    let c0 = ew.lamports(&cranker.pubkey());
    let b0 = ew.lamports(&b.pubkey());
    let lock = ew.book(&base).bids.iter().find(|o| o.owner == b.pubkey()).unwrap().quote_locked;
    let ix = ew.crank_ix(&cranker.pubkey(), &base, 8, &[b.pubkey(), a.pubkey()]);
    ew.send(&cranker, &[ix]).ok();
    assert_eq!(ew.lamports(&cranker.pubkey()) - c0, 2 * TEST_BOOK.order_bounty_lamports - 5_000);
    assert_eq!(ew.lamports(&b.pubkey()) - b0, lock);
    assert_eq!(ew.holding(&base, &a.pubkey()), 10);
    let m = ew.book(&base);
    assert!(m.asks.is_empty());
    assert_eq!(m.bids.len(), 1); // the buyer's live bid
    ew.assert_escrow(&base);
}

#[test]
fn post_only_tick_and_size_rules() {
    let (mut ew, base) = world();
    let a = trader(&mut ew, 10);
    place(&mut ew, &a, &base, side::ASK, 5 * TICK, 5, &[]).ok();
    let b = trader(&mut ew, 0);
    let ix = ew.place_ix(&b.pubkey(), &base, side::BID, 5 * TICK, 1, true, 0, &[a.pubkey()]);
    ew.send(&b, &[ix]).expect_code(book_code(E::WouldCross));
    place(&mut ew, &b, &base, side::BID, 5 * TICK + 1, 1, &[]).expect_code(book_code(E::BadPrice));
    place(&mut ew, &b, &base, side::BID, 0, 1, &[]).expect_code(book_code(E::BadPrice));
    place(&mut ew, &b, &base, side::BID, TICK, 0, &[]).expect_code(book_code(E::BadSize));
    // A maker pair that is not the order's owner is refused.
    place(&mut ew, &b, &base, side::BID, 5 * TICK, 1, &[b.pubkey()]).expect_code(book_code(E::WrongAccount));
}

#[test]
fn markets_exist_only_for_materials() {
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000);
    let creator = ew.funded(SOL);
    let mut ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 1);
    // A material account owned by something else is refused.
    let fake = Pubkey::new_unique();
    ew.hw.w.env.fund(fake, SOL);
    ix.accounts[2].pubkey = fake;
    ew.send(&creator, &[ix]).expect_code(book_code(E::BadBase));
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, 0, 1);
    ew.send(&creator, &[ix]).expect_code(book_code(E::BadParams));
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 1);
    ew.send(&creator, &[ix]).ok();
    // One book per material.
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 2);
    ew.send(&creator, &[ix]).expect_fail();
}

fn class(template_id: u16, min_level: u8, lo: u32, hi: u32) -> ClassKey {
    let mut param_min = [0u32; PARAM_FIELDS];
    let mut param_max = [u32::MAX; PARAM_FIELDS];
    param_min[0] = lo;
    param_max[0] = hi;
    ClassKey { template_id, min_level, param_min, param_max }
}

#[test]
fn class_bids_fill_only_fitting_free_items() {
    let mut ew = Ew::new();
    let bidder = ew.funded(100 * SOL);
    let price = 3 * SOL;
    let ix = ew.place_class_bid_ix(&bidder.pubkey(), 1, class(t::TRANSFER_FEE, 0, 50, 150), price, 0);
    ew.send(&bidder, &[ix]).ok();
    let bid = hookwars_book::state::class_bid_address(&bidder.pubkey(), 1).0;
    let maker_fee = price * u64::from(TEST_BOOK.maker_bps) / 10_000;
    assert_eq!(ew.lamports(&bid) - ew.hw.w.env.rent(ew.hw.w.env.account(&bid).unwrap().data.len()), price + maker_fee);
    // Out of range: param 200.
    let (s1, item1, mint1) = ew.hw.item(t::TRANSFER_FEE, params(&[200, 0]), 100);
    let ix = ew.match_class_ix(&s1.pubkey(), &bidder.pubkey(), 1, &item1, &mint1);
    ew.send(&s1, &[ix]).expect_code(book_code(E::ClassMismatch));
    // Wrong template: a fee item sold into a bid for Shields.
    let ix = ew.place_class_bid_ix(&bidder.pubkey(), 2, class(t::SHIELD, 0, 0, u32::MAX), price, 0);
    ew.send(&bidder, &[ix]).ok();
    let (s2, item2, mint2) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let ix = ew.match_class_ix(&s2.pubkey(), &bidder.pubkey(), 2, &item2, &mint2);
    ew.send(&s2, &[ix]).expect_code(book_code(E::ClassMismatch));
    // A listed item is refused.
    let (s3, item3, mint3) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let l = list_ix(&s3.pubkey(), &item3, &mint3, SOL, 0);
    ew.send(&s3, &[l]).ok();
    let ix = ew.match_class_ix(&s3.pubkey(), &bidder.pubkey(), 1, &item3, &mint3);
    ew.send(&s3, &[ix]).expect_code(book_code(E::ClassMismatch));
    // An equipped item is refused (equipped_count written on the item).
    let (s4, item4, mint4) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let mut it: hookwars_armory::state::Item = ew.hw.w.env.read(&item4);
    it.equipped_count = 1;
    bordrless_program_tests::expansion::put_anchor(&mut ew.hw.w.env, item4, hookwars_common::ids::ARMORY_ID, &it, 0);
    let ix = ew.match_class_ix(&s4.pubkey(), &bidder.pubkey(), 1, &item4, &mint4);
    ew.send(&s4, &[ix]).expect_code(book_code(E::ClassMismatch));
    // A fitting free item fills: item to the bidder, price less taker fee to the seller.
    let (s5, item5, mint5) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let (s0, tr0, b0) = (ew.lamports(&s5.pubkey()), ew.lamports(&ew.book_treasury), ew.lamports(&bidder.pubkey()));
    let bid_lamports = ew.lamports(&bid);
    let ix = ew.match_class_ix(&s5.pubkey(), &bidder.pubkey(), 1, &item5, &mint5);
    ew.send(&s5, &[ix]).ok();
    let taker_fee = price * u64::from(TEST_BOOK.taker_bps) / 10_000;
    assert_eq!(ew.holding(&mint5, &bidder.pubkey()), 1);
    assert_eq!(ew.holding(&mint5, &s5.pubkey()), 0);
    assert_eq!(ew.lamports(&ew.book_treasury) - tr0, taker_fee + maker_fee);
    let holding_rent = ew.lamports(&bordrless_token::client::holding_address(&mint5, &bidder.pubkey()));
    assert_eq!(ew.lamports(&s5.pubkey()) + holding_rent + 5_000 - s0, price - taker_fee);
    assert_eq!(ew.lamports(&bidder.pubkey()) - b0, bid_lamports - price - maker_fee);
    assert!(ew.hw.w.env.account(&bid).is_none());
    let _ = (item1, item2);
}

#[test]
fn class_bids_cancel_in_full_and_expire() {
    let mut ew = Ew::new();
    let bidder = ew.funded(100 * SOL);
    let exp = ew.now() + 100;
    let ix = ew.place_class_bid_ix(&bidder.pubkey(), 7, class(t::TRANSFER_FEE, 0, 0, u32::MAX), SOL, exp);
    ew.send(&bidder, &[ix]).ok();
    ew.warp(101);
    let (s, item, mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let ix = ew.match_class_ix(&s.pubkey(), &bidder.pubkey(), 7, &item, &mint);
    ew.send(&s, &[ix]).expect_code(book_code(E::BidExpired));
    let b0 = ew.lamports(&bidder.pubkey());
    let held = ew.lamports(&hookwars_book::state::class_bid_address(&bidder.pubkey(), 7).0);
    let ix = ew.cancel_class_bid_ix(&bidder.pubkey(), 7);
    ew.send(&bidder, &[ix]).ok();
    assert_eq!(ew.lamports(&bidder.pubkey()) - b0, held - 5_000);
    // A stranger cannot cancel someone's bid.
    let ix = ew.place_class_bid_ix(&bidder.pubkey(), 8, class(t::TRANSFER_FEE, 0, 0, u32::MAX), SOL, 0);
    ew.send(&bidder, &[ix]).ok();
    let stranger = ew.funded(SOL);
    let mut ix = ew.cancel_class_bid_ix(&bidder.pubkey(), 8);
    ix.accounts[0].pubkey = stranger.pubkey();
    ew.send(&stranger, &[ix]).expect_fail();
}
