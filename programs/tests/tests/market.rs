// Changed by Hookwars: new file, `hookwars_market` (docs/spec/10-expansion.md sections 1, 4, 5).
//! Listings and sales with exact splits, expiry, slippage, collections, leases and commissions.
//! Items are real armory items; the token's slots are filled through the armory (`launch_stub`
//! signs `equip_launch`) and changed by an ordinary holder vote.

use anchor_lang::prelude::Pubkey;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::*;
use bordrless_program_tests::expansion::*;
use bordrless_program_tests::slots::item_slot;
use hookwars_common::{template_id as t, EquipConfig};
use hookwars_market::error::MarketError;
use hookwars_market::state::{self as ms, Collection, Commission, Lease, Listing};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

struct Mw {
    hw: Hw,
    treasury: Pubkey,
}

impl Mw {
    fn new() -> Self {
        let mut hw = Hw::new();
        let treasury = Keypair::new().pubkey();
        hw.w.env.fund(treasury, SOL);
        load(&mut hw.w.env, treasury);
        Self { hw, treasury }
    }

    /// A Transfer Fee item held by its (funded) author.
    fn fee_item(&mut self) -> (Keypair, Pubkey, Pubkey) {
        self.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100)
    }

    fn fee_config(&self) -> EquipConfig {
        EquipConfig {
            targets: vec![Pubkey::new_unique()],
            role: 0,
        }
    }

    fn holds(&self, mint: &Pubkey, owner: &Pubkey) -> u64 {
        self.hw.w.env.holding(mint, owner)
    }
}

#[test]
fn a_sale_pays_fee_resale_and_seller_exactly_and_moves_the_item() {
    let mut m = Mw::new();
    let (author, item, item_mint) = m.fee_item();
    let seller = m.hw.w.env.funded(10 * SOL);
    m.hw.give_item(&author, &item_mint, &seller.pubkey());
    let price = 10 * SOL;
    send(&mut m.hw.w.env, &seller, &[list_ix(&seller.pubkey(), &item, &item_mint, price, 0)]).ok();
    let escrow = ms::escrow_address(&item_mint).0;
    assert_eq!(m.holds(&item_mint, &escrow), 1);
    assert_eq!(m.holds(&item_mint, &seller.pubkey()), 0);
    let listing_rent = m.hw.w.env.lamports(&Listing::address(&item_mint).0);

    let buyer = m.hw.w.env.funded(20 * SOL);
    let (t0, a0, s0) = (
        m.hw.w.env.lamports(&m.treasury),
        m.hw.w.env.lamports(&author.pubkey()),
        m.hw.w.env.lamports(&seller.pubkey()),
    );
    let ix = buy_ix(&buyer.pubkey(), &seller.pubkey(), &author.pubkey(), &m.treasury, &item, &item_mint, price);
    send(&mut m.hw.w.env, &buyer, &[ix]).ok();
    let fee = price * u64::from(TEST_MARKET.fee_bps) / 10_000;
    let resale = price * u64::from(TEST_MARKET.author_resale_bps) / 10_000;
    assert_eq!(m.hw.w.env.lamports(&m.treasury) - t0, fee);
    assert_eq!(m.hw.w.env.lamports(&author.pubkey()) - a0, resale);
    assert_eq!(m.hw.w.env.lamports(&seller.pubkey()) - s0, price - fee - resale + listing_rent);
    assert_eq!(m.holds(&item_mint, &buyer.pubkey()), 1);
    assert_eq!(m.holds(&item_mint, &escrow), 0);
    assert!(m.hw.w.env.account(&Listing::address(&item_mint).0).is_none());
}

#[test]
fn listing_refusals_and_slippage() {
    let mut m = Mw::new();
    let (author, item, item_mint) = m.fee_item();
    let stranger = m.hw.w.env.funded(10 * SOL);
    // A non-holder cannot list.
    send(&mut m.hw.w.env, &stranger, &[list_ix(&stranger.pubkey(), &item, &item_mint, SOL, 0)])
        .expect_fail();
    // Zero price refused.
    send(&mut m.hw.w.env, &author, &[list_ix(&author.pubkey(), &item, &item_mint, 0, 0)])
        .expect_code(market_code(MarketError::ZeroPrice));
    send(&mut m.hw.w.env, &author, &[list_ix(&author.pubkey(), &item, &item_mint, 5 * SOL, 0)]).ok();
    // A second listing of the same item is refused (the seed is the item mint).
    send(&mut m.hw.w.env, &author, &[list_ix(&author.pubkey(), &item, &item_mint, 6 * SOL, 0)])
        .expect_fail();
    // The buyer's maximum below the price.
    let buyer = m.hw.w.env.funded(20 * SOL);
    let ix = buy_ix(&buyer.pubkey(), &author.pubkey(), &author.pubkey(), &m.treasury, &item, &item_mint, 4 * SOL);
    send(&mut m.hw.w.env, &buyer, &[ix]).expect_code(market_code(MarketError::PriceMoved));
    // Only the seller delists; the item returns.
    send(&mut m.hw.w.env, &stranger, &[unlist_ix(&stranger.pubkey(), &author.pubkey(), &item_mint, false)])
        .expect_code(market_code(MarketError::NotSeller));
    send(&mut m.hw.w.env, &author, &[unlist_ix(&author.pubkey(), &author.pubkey(), &item_mint, false)]).ok();
    assert_eq!(m.holds(&item_mint, &author.pubkey()), 1);
}

#[test]
fn an_expired_listing_returns_to_its_seller_by_anyone() {
    let mut m = Mw::new();
    let (author, item, item_mint) = m.fee_item();
    let expires = m.hw.w.env.now + 3_600;
    send(&mut m.hw.w.env, &author, &[list_ix(&author.pubkey(), &item, &item_mint, SOL, expires)]).ok();
    let cranker = m.hw.w.env.funded(SOL);
    send(&mut m.hw.w.env, &cranker, &[unlist_ix(&cranker.pubkey(), &author.pubkey(), &item_mint, true)])
        .expect_code(market_code(MarketError::NotExpired));
    m.hw.w.env.warp(3_601);
    let buyer = m.hw.w.env.funded(5 * SOL);
    let ix = buy_ix(&buyer.pubkey(), &author.pubkey(), &author.pubkey(), &m.treasury, &item, &item_mint, SOL);
    send(&mut m.hw.w.env, &buyer, &[ix]).expect_code(market_code(MarketError::ListingExpired));
    send(&mut m.hw.w.env, &cranker, &[unlist_ix(&cranker.pubkey(), &author.pubkey(), &item_mint, true)]).ok();
    assert_eq!(m.holds(&item_mint, &author.pubkey()), 1);
}

#[test]
fn an_equipped_item_sells_and_stays_equipped() {
    let mut m = Mw::new();
    let owner = m.hw.w.env.funded(100 * SOL);
    let mint = m.hw.slot_mint(&owner, test_slots());
    let (author, item, item_mint) = m.fee_item();
    let cfg = m.fee_config();
    m.hw.equip_launch(&owner, &mint, Hw::entry(2, Some(item), cfg)).ok();
    assert_eq!(m.hw.slot_item(&mint, 2), item);
    send(&mut m.hw.w.env, &author, &[list_ix(&author.pubkey(), &item, &item_mint, SOL, 0)]).ok();
    let buyer = m.hw.w.env.funded(5 * SOL);
    let ix = buy_ix(&buyer.pubkey(), &author.pubkey(), &author.pubkey(), &m.treasury, &item, &item_mint, SOL);
    send(&mut m.hw.w.env, &buyer, &[ix]).ok();
    assert_eq!(m.holds(&item_mint, &buyer.pubkey()), 1);
    assert_eq!(m.hw.slot_item(&mint, 2), item, "the equip is bound to the item, not its holder");
}

#[test]
fn collections_group_active_templates_only() {
    let mut m = Mw::new();
    let curator = m.hw.w.env.funded(SOL);
    send(&mut m.hw.w.env, &curator, &[create_collection_ix(&curator.pubkey(), 0, "Defense", vec![t::WALL, t::SHIELD])]).ok();
    let c: Collection = m.hw.w.env.read(&ms::collection_address(0).0);
    assert_eq!((c.id, c.name.as_str(), c.template_ids.clone()), (0, "Defense", vec![t::WALL, t::SHIELD]));
    send(&mut m.hw.w.env, &curator, &[create_collection_ix(&curator.pubkey(), 1, "Twice", vec![t::WALL, t::WALL])])
        .expect_code(market_code(MarketError::DuplicateTemplate));
    send(&mut m.hw.w.env, &curator, &[create_collection_ix(&curator.pubkey(), 1, "Unknown", vec![40])])
        .expect_fail();
    send(&mut m.hw.w.env, &curator, &[create_collection_ix(&curator.pubkey(), 1, "Big", vec![1, 2, 3, 4, 5])])
        .expect_code(market_code(MarketError::TooManyTemplates));
}

#[test]
fn a_lease_escrows_the_item_pays_its_fee_and_returns_after_the_term() {
    let mut m = Mw::new();
    let owner = m.hw.w.env.funded(100 * SOL);
    let mint = m.hw.slot_mint(&owner, test_slots());
    let (lessor, item, item_mint) = m.fee_item();
    let term = TEST_MARKET.lease_min_secs;
    // Out-of-range term and rent are refused; a slot of another kind does not fit.
    send(&mut m.hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 100, SOL, term - 1)])
        .expect_code(market_code(MarketError::BadTerm));
    send(&mut m.hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, TEST_MARKET.max_rent_bps + 1, SOL, term)])
        .expect_code(market_code(MarketError::RentTooHigh));
    send(&mut m.hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 1, 100, SOL, term)])
        .expect_code(market_code(MarketError::DoesNotFit));
    send(&mut m.hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 1_000, SOL, term)]).ok();
    let le = ms::lease_escrow_address(&item_mint).0;
    assert_eq!(m.holds(&item_mint, &le), 1);
    // While leased the lessor cannot list it (they no longer hold it).
    send(&mut m.hw.w.env, &lessor, &[list_ix(&lessor.pubkey(), &item, &item_mint, SOL, 0)]).expect_fail();
    let payer = m.hw.w.env.funded(5 * SOL);
    let l0 = m.hw.w.env.lamports(&lessor.pubkey());
    send(&mut m.hw.w.env, &payer, &[accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey())]).ok();
    assert_eq!(m.hw.w.env.lamports(&lessor.pubkey()) - l0, SOL);
    let l: Lease = m.hw.w.env.read(&ms::lease_address(&item).0);
    assert_eq!((l.state, l.ends_at - l.starts_at), (ms::lease_state::ACTIVE, i64::from(term)));
    // A second accept and an early end are refused.
    send(&mut m.hw.w.env, &payer, &[accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey())])
        .expect_code(market_code(MarketError::WrongLeaseState));
    let cranker = m.hw.w.env.funded(SOL);
    send(&mut m.hw.w.env, &cranker, &[close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true)])
        .expect_code(market_code(MarketError::LeaseNotOver));
    m.hw.w.env.warp(i64::from(term));
    // Review 3 M-5: the revert accounts (or, with the item not in the slot, the token mint) are required.
    send(&mut m.hw.w.env, &cranker, &[close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true)])
        .expect_code(market_code(MarketError::RevertAccountsMissing));
    let mut ix = close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(mint, false));
    send(&mut m.hw.w.env, &cranker, &[ix]).ok();
    assert_eq!(m.holds(&item_mint, &lessor.pubkey()), 1);
    assert!(m.hw.w.env.account(&ms::lease_address(&item).0).is_none());
}

#[test]
fn an_unaccepted_offer_is_withdrawn_by_the_lessor_only() {
    let mut m = Mw::new();
    let owner = m.hw.w.env.funded(100 * SOL);
    let mint = m.hw.slot_mint(&owner, test_slots());
    let (lessor, item, item_mint) = m.fee_item();
    let term = TEST_MARKET.lease_min_secs;
    send(&mut m.hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 0, 0, term)]).ok();
    let stranger = m.hw.w.env.funded(SOL);
    send(&mut m.hw.w.env, &stranger, &[close_lease_ix(&stranger.pubkey(), &item, &item_mint, &lessor.pubkey(), false)])
        .expect_code(market_code(MarketError::NotLessor));
    send(&mut m.hw.w.env, &lessor, &[close_lease_ix(&lessor.pubkey(), &item, &item_mint, &lessor.pubkey(), false)]).ok();
    assert_eq!(m.holds(&item_mint, &lessor.pubkey()), 1);
}

/// The full commission path: open, submit, the item wins a holder vote and is equipped, the
/// bounty is paid exactly; paying before the equip is refused.
#[test]
fn a_commission_pays_its_bounty_once_the_submission_is_equipped() {
    let mut m = Mw::new();
    let owner = m.hw.w.env.funded(100 * SOL);
    let mint = m.hw.slot_mint(&owner, test_slots());
    let (_, incumbent, _) = m.fee_item();
    let cfg = m.fee_config();
    m.hw.equip_launch(&owner, &mint, Hw::entry(2, Some(incumbent), cfg.clone())).ok();
    let o = owner.pubkey();
    m.hw.mint_to(&owner, &mint, &o, 1_000);

    let creator = m.hw.w.env.funded(10 * SOL);
    let bounty = TEST_MARKET.commission_min_lamports;
    send(&mut m.hw.w.env, &creator, &[open_commission_ix(&creator.pubkey(), &mint, 0, 2, bounty - 1, 3_600)])
        .expect_code(market_code(MarketError::BountyTooSmall));
    send(&mut m.hw.w.env, &creator, &[open_commission_ix(&creator.pubkey(), &mint, 0, 2, bounty, 3_600)]).ok();
    let commission = ms::commission_address(&mint, 0).0;
    let c: Commission = m.hw.w.env.read(&commission);
    assert_eq!((c.incumbent, c.bounty_lamports), (incumbent, bounty));

    // A War orders item does not fit a Fee slot.
    let (war_author, war_item, war_mint) =
        m.hw.item(t::WAR_ORDERS, params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]), 100);
    send(&mut m.hw.w.env, &war_author, &[submit_ix(&war_author.pubkey(), &commission, &mint, &war_item, &war_mint)])
        .expect_code(market_code(MarketError::DoesNotFit));
    let (submitter, item, item_mint) = m.fee_item();
    send(&mut m.hw.w.env, &submitter, &[submit_ix(&submitter.pubkey(), &commission, &mint, &item, &item_mint)]).ok();

    // The bounty cannot be paid before the window closes, nor before the item is equipped.
    let cranker = m.hw.w.env.funded(SOL);
    send(&mut m.hw.w.env, &cranker, &[pay_commission_ix(&commission, &mint, &item, &submitter.pubkey())])
        .expect_code(market_code(MarketError::CommissionNotOpen));
    m.hw.w.env.warp(3_600);
    let (late, late_item, late_mint) = m.fee_item();
    send(&mut m.hw.w.env, &late, &[submit_ix(&late.pubkey(), &commission, &mint, &late_item, &late_mint)])
        .expect_code(market_code(MarketError::CommissionClosed));
    send(&mut m.hw.w.env, &cranker, &[pay_commission_ix(&commission, &mint, &item, &submitter.pubkey())])
        .expect_code(market_code(MarketError::NotEquipped));

    // Holders vote the submission in.
    let (tx, proposal) = m.hw.propose(&owner, &mint, 2, Some(item), cfg);
    tx.ok();
    m.hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
    m.hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    m.hw.finalize(&proposal).ok();
    m.hw.w.env.warp(600);
    let payer = owner.insecure_clone();
    m.hw.execute(&payer, &proposal).ok();
    assert_eq!(m.hw.slot_item(&mint, 2), item);

    let vault = ms::commission_vault_address(&commission).0;
    let v = m.hw.w.env.lamports(&vault);
    assert_eq!(v, bounty);
    let s0 = m.hw.w.env.lamports(&submitter.pubkey());
    send(&mut m.hw.w.env, &cranker, &[pay_commission_ix(&commission, &mint, &item, &submitter.pubkey())]).ok();
    assert_eq!(m.hw.w.env.lamports(&submitter.pubkey()) - s0, bounty);
    assert_eq!(m.hw.w.env.lamports(&vault), 0);
    let c: Commission = m.hw.w.env.read(&commission);
    assert_eq!((c.state, c.winner), (ms::commission_state::PAID, Some(item)));
    // Neither a second payment nor a refund afterwards.
    send(&mut m.hw.w.env, &cranker, &[pay_commission_ix(&commission, &mint, &item, &submitter.pubkey())])
        .expect_code(market_code(MarketError::CommissionNotOpen));
    m.hw.w.env.warp(i64::from(TEST_MARKET.commission_vote_secs));
    send(&mut m.hw.w.env, &cranker, &[refund_commission_ix(&commission, &creator.pubkey())])
        .expect_code(market_code(MarketError::CommissionNotOpen));
}

#[test]
fn an_unfilled_commission_refunds_its_creator_after_the_vote_window() {
    let mut m = Mw::new();
    let owner = m.hw.w.env.funded(100 * SOL);
    let mint = m.hw.slot_mint(&owner, test_slots());
    let creator = m.hw.w.env.funded(10 * SOL);
    let bounty = TEST_MARKET.commission_min_lamports;
    send(&mut m.hw.w.env, &creator, &[open_commission_ix(&creator.pubkey(), &mint, 7, 2, bounty, 3_600)]).ok();
    let commission = ms::commission_address(&mint, 7).0;
    let cranker = m.hw.w.env.funded(SOL);
    m.hw.w.env.warp(3_600);
    send(&mut m.hw.w.env, &cranker, &[refund_commission_ix(&commission, &creator.pubkey())])
        .expect_code(market_code(MarketError::RefundTooEarly));
    m.hw.w.env.warp(i64::from(TEST_MARKET.commission_vote_secs));
    let c0 = m.hw.w.env.lamports(&creator.pubkey());
    send(&mut m.hw.w.env, &cranker, &[refund_commission_ix(&commission, &creator.pubkey())]).ok();
    assert_eq!(m.hw.w.env.lamports(&creator.pubkey()) - c0, bounty);
}

#[test]
fn a_commission_needs_a_vote_chosen_slot() {
    let mut m = Mw::new();
    let owner = m.hw.w.env.funded(100 * SOL);
    let slots = vec![
        item_slot(slot_kind::WAR, equip_rule::VOTE, 0, 0, false),
        item_slot(slot_kind::FEE, equip_rule::LOCKED, 300, 0, false),
    ];
    let mint = m.hw.slot_mint(&owner, slots);
    let creator = m.hw.w.env.funded(10 * SOL);
    let bounty = TEST_MARKET.commission_min_lamports;
    send(&mut m.hw.w.env, &creator, &[open_commission_ix(&creator.pubkey(), &mint, 0, 1, bounty, 3_600)])
        .expect_code(market_code(MarketError::SlotNotVotable));
    send(&mut m.hw.w.env, &creator, &[open_commission_ix(&creator.pubkey(), &mint, 0, 9, bounty, 3_600)])
        .expect_code(market_code(MarketError::NoSuchSlot));
}
