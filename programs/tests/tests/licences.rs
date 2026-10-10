// Changed by Hookwars: new file, market licences (docs/spec/11-hook-economy.md sections 1.4, 2.3).
//! The licence split pays the protocol, the template author and the item holder exactly; income
//! follows the item (R45); `max_live` and exclusive terms; renew extends; per-period revocation
//! refunds the unused fraction exactly from the holder; expiry frees the live slot; parameters
//! wait for the timelock.

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::params;
use bordrless_program_tests::economy::*;
use bordrless_program_tests::expansion::market_code;
use hookwars_common::economy::counter;
use hookwars_common::template_id as t;
use hookwars_market::error::MarketError as M;
use hookwars_market::licence::{self as lic, per, License, LicenceConfig, LicenceError as L, LicenceOffer};
use solana_keypair::Keypair;
use solana_signer::Signer;

const TERM: u32 = 86_400;

fn lcode(e: L) -> u32 {
    7_300 + e as u32
}

struct Lw {
    ew: Ew,
    holder: Keypair,
    item: Pubkey,
    item_mint: Pubkey,
    author: Pubkey,
}

/// An item held by its author, offered at `price` per `TERM`.
fn world(price: u64, per_: u8, max_live: u16, exclusive: bool) -> Lw {
    let mut ew = Ew::new();
    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let author = ew.template_author(&item);
    let ix = ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, price, TERM, per_, max_live, exclusive, true);
    ew.send(&holder, &[ix]).ok();
    Lw { ew, holder, item, item_mint, author }
}

fn token(lw: &mut Lw) -> Pubkey {
    let funder = lw.ew.funded(10 * SOL);
    lw.ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK")
}

fn buy(lw: &mut Lw, payer: &Keypair, token_mint: &Pubkey, holder: &Pubkey, max: u64, renew: bool) -> bordrless_program_tests::Tx {
    let ix = lw.ew.buy_license_ix(&payer.pubkey(), &lw.item, &lw.item_mint, token_mint, holder, &lw.author, max, renew);
    lw.ew.send(payer, &[ix])
}

#[test]
fn a_licence_pays_protocol_author_and_holder_exactly() {
    let price = 1_000_000_007;
    let mut lw = world(price, per::PER_TOKEN, 3, false);
    let tok = token(&mut lw);
    let payer = lw.ew.funded(10 * SOL);
    let holder = lw.holder.pubkey();
    lw.ew.open_profile(&lw.holder.insecure_clone());
    let (t0, a0, h0) = (lw.ew.lamports(&lw.ew.market_treasury), lw.ew.lamports(&lw.author), lw.ew.lamports(&holder));
    buy(&mut lw, &payer, &tok, &holder, price, false).ok();
    let protocol = price * u64::from(TEST_LICENCE.protocol_bps) / 10_000;
    let author = (price - protocol) * u64::from(TEST_LICENCE.author_bps) / 10_000;
    assert_eq!(lw.ew.lamports(&lw.ew.market_treasury) - t0, protocol);
    assert_eq!(lw.ew.lamports(&lw.author) - a0, author);
    assert_eq!(lw.ew.lamports(&holder) - h0, price - protocol - author);
    let l: License = lw.ew.hw.w.env.read(&lic::license_address(&lw.item, &tok).0);
    assert_eq!((l.payer, l.price_paid, l.ends_at - l.starts_at), (payer.pubkey(), price, i64::from(TERM)));
    assert!(l.live_at(lw.ew.now()) && l.counted);
    let c: LicenceConfig = lw.ew.hw.w.env.read(&lic::licence_config_address().0);
    assert_eq!((c.protocol_fees_total, c.licences_sold), (u128::from(protocol), 1));
    // The holder's builder counters moved (market is a registered caller).
    let p = lw.ew.profile(&holder);
    assert_eq!(p.counters[usize::from(counter::LICENCES_SOLD)], 1);
    assert_eq!(p.counters[usize::from(counter::LICENCE_REVENUE_LAMPORTS)], price - protocol - author);
    // The equip gate's reader sees it live, then not after the term.
    let acc = lw.ew.hw.w.env.account(&lic::license_address(&lw.item, &tok).0).unwrap();
    assert!(l.live_at(lw.ew.now()) && acc.owner == hookwars_market::ID);
    lw.ew.warp(i64::from(TERM));
    let l: License = lw.ew.hw.w.env.read(&lic::license_address(&lw.item, &tok).0);
    assert!(!l.live_at(lw.ew.now()));
}

#[test]
fn licence_income_follows_the_item() {
    let price = SOL;
    let mut lw = world(price, per::PER_TOKEN, 3, false);
    let new_holder = lw.ew.funded(SOL);
    let h = lw.holder.insecure_clone();
    lw.ew.hw.give_item(&h, &lw.item_mint, &new_holder.pubkey());
    let tok = token(&mut lw);
    let payer = lw.ew.funded(10 * SOL);
    // Naming the old holder is refused; the new holder is paid.
    let old = lw.holder.pubkey();
    buy(&mut lw, &payer, &tok, &old, price, false).expect_code(market_code(M::NotItemHolder));
    let h0 = lw.ew.lamports(&new_holder.pubkey());
    buy(&mut lw, &payer, &tok, &new_holder.pubkey(), price, false).ok();
    let protocol = price * u64::from(TEST_LICENCE.protocol_bps) / 10_000;
    let author = (price - protocol) * u64::from(TEST_LICENCE.author_bps) / 10_000;
    assert_eq!(lw.ew.lamports(&new_holder.pubkey()) - h0, price - protocol - author);
}

#[test]
fn live_slots_exclusive_terms_slippage_and_expiry() {
    let price = SOL;
    let mut lw = world(price, per::PER_TOKEN, 2, false);
    let holder = lw.holder.pubkey();
    let payer = lw.ew.funded(100 * SOL);
    let (t1, t2, t3) = (token(&mut lw), token(&mut lw), token(&mut lw));
    buy(&mut lw, &payer, &t1, &holder, price - 1, false).expect_code(market_code(M::PriceMoved));
    buy(&mut lw, &payer, &t1, &holder, price, false).ok();
    buy(&mut lw, &payer, &t2, &holder, price, false).ok();
    buy(&mut lw, &payer, &t3, &holder, price, false).expect_code(lcode(L::LicenceSoldOut));
    // Buying again while live extends instead of taking a slot.
    buy(&mut lw, &payer, &t1, &holder, price, false).ok();
    let l: License = lw.ew.hw.w.env.read(&lic::license_address(&lw.item, &t1).0);
    assert_eq!(l.ends_at - l.starts_at, 2 * i64::from(TERM));
    // t2 ends; until someone expires it, its slot stays taken.
    lw.ew.warp(i64::from(TERM));
    buy(&mut lw, &payer, &t3, &holder, price, false).expect_code(lcode(L::LicenceSoldOut));
    let ix = lw.ew.expire_license_ix(&lw.item, &t1);
    lw.ew.send(&payer, &[ix]).expect_code(lcode(L::NotEnded));
    let ix = lw.ew.expire_license_ix(&lw.item, &t2);
    lw.ew.send(&payer, &[ix]).ok();
    let ix = lw.ew.expire_license_ix(&lw.item, &t2);
    lw.ew.send(&payer, &[ix]).expect_code(lcode(L::NotEnded));
    buy(&mut lw, &payer, &t3, &holder, price, false).ok();
    let o: LicenceOffer = lw.ew.hw.w.env.read(&lic::licence_offer_address(&lw.item).0);
    assert_eq!(o.live, 2);
    // Renewing needs an existing licence.
    let t4 = token(&mut lw);
    buy(&mut lw, &payer, &t4, &holder, price, true).expect_code(lcode(L::NotLive));
    // An exclusive offer allows one live licence; exclusive with max_live 2 is refused.
    let h = lw.holder.insecure_clone();
    let ix = lw.ew.set_offer_ix(&h.pubkey(), &lw.item, &lw.item_mint, price, TERM, per::PER_TOKEN, 2, true, true);
    lw.ew.send(&h, &[ix]).expect_code(lcode(L::BadLicenceTerms));
    let ix = lw.ew.set_offer_ix(&h.pubkey(), &lw.item, &lw.item_mint, price, 60, per::PER_TOKEN, 1, false, true);
    lw.ew.send(&h, &[ix]).expect_code(lcode(L::BadLicenceTerms));
    // A closed offer sells nothing.
    let ix = lw.ew.set_offer_ix(&h.pubkey(), &lw.item, &lw.item_mint, price, TERM, per::PER_TOKEN, 1, true, false);
    lw.ew.send(&h, &[ix]).ok();
    buy(&mut lw, &payer, &t4, &holder, price, false).expect_code(lcode(L::NotLicensable));
    // Only the holder sets terms.
    let stranger = lw.ew.funded(SOL);
    let ix = lw.ew.set_offer_ix(&stranger.pubkey(), &lw.item, &lw.item_mint, price, TERM, per::PER_TOKEN, 1, false, true);
    lw.ew.send(&stranger, &[ix]).expect_fail();
}

#[test]
fn per_period_revocation_refunds_the_unused_fraction_exactly() {
    let price = 1_000_000_000;
    let mut lw = world(price, per::PER_PERIOD, 1, true);
    let holder = lw.holder.insecure_clone();
    let tok = token(&mut lw);
    let payer = lw.ew.funded(10 * SOL);
    buy(&mut lw, &payer, &tok, &holder.pubkey(), price, false).ok();
    lw.ew.warp(i64::from(TERM) / 4);
    let p0 = lw.ew.lamports(&payer.pubkey());
    let ix = lw.ew.revoke_license_ix(&holder.pubkey(), &lw.item, &lw.item_mint, &tok, &payer.pubkey());
    lw.ew.send(&holder, &[ix]).ok();
    let refund = (u128::from(price) * 3 * u128::from(TERM / 4) / u128::from(TERM)) as u64;
    assert_eq!(lw.ew.lamports(&payer.pubkey()) - p0, refund);
    let l: License = lw.ew.hw.w.env.read(&lic::license_address(&lw.item, &tok).0);
    assert!(!l.live_at(lw.ew.now()) && l.revoked_at != 0 && !l.counted);
    // Revoked twice is refused; a fresh purchase takes the freed slot.
    let ix = lw.ew.revoke_license_ix(&holder.pubkey(), &lw.item, &lw.item_mint, &tok, &payer.pubkey());
    lw.ew.send(&holder, &[ix]).expect_code(lcode(L::NotLive));
    buy(&mut lw, &payer, &tok, &holder.pubkey(), price, false).ok();
}

#[test]
fn per_token_licences_cannot_be_revoked() {
    let mut lw = world(SOL, per::PER_TOKEN, 1, false);
    let holder = lw.holder.insecure_clone();
    let tok = token(&mut lw);
    let payer = lw.ew.funded(10 * SOL);
    buy(&mut lw, &payer, &tok, &holder.pubkey(), SOL, false).ok();
    let ix = lw.ew.revoke_license_ix(&holder.pubkey(), &lw.item, &lw.item_mint, &tok, &payer.pubkey());
    lw.ew.send(&holder, &[ix]).expect_code(lcode(L::NotRevocable));
}

#[test]
fn licence_parameters_wait_for_the_timelock() {
    let mut ew = Ew::new();
    let d = ew.deployer();
    let mut p = TEST_LICENCE;
    p.protocol_bps = 700;
    let ix = market_ix(
        hookwars_market::accounts::ProposeLicenceParams {
            admin: d.pubkey(),
            config: hookwars_market::state::config_address().0,
            licence_config: lic::licence_config_address().0,
            event_authority: bordrless_program_tests::expansion::events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::ProposeLicenceParams { params: p },
    );
    ew.send(&d, &[ix]).ok();
    let apply = market_ix(
        hookwars_market::accounts::ApplyLicenceParams { licence_config: lic::licence_config_address().0 },
        hookwars_market::instruction::ApplyLicenceParams {},
    );
    ew.send(&d, &[apply.clone()]).expect_code(market_code(M::NotReady));
    ew.warp(600);
    ew.send(&d, &[apply]).ok();
    let c: LicenceConfig = ew.hw.w.env.read(&lic::licence_config_address().0);
    assert_eq!(c.params.protocol_bps, 700);
}
