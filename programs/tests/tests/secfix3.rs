// Changed by Hookwars: new file, security review 3 regression tests (~/ideas/hookwars/SECURITY-REVIEW-3.md,
// docs/spec/15-secfix3.md). Each test is the review's proof of concept turned around: the attack now
// fails, and the honest path still works. The war findings (M-3, L-7, L-3, M-6) are in war_expansion.rs.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::system_instruction;
use anchor_lang::{AccountDeserialize, AccountSerialize, ToAccountMetas};
use bordrless_program_tests::agents::*;
use bordrless_program_tests::armory::{params, Hw};
use bordrless_program_tests::economy::*;
use bordrless_program_tests::expansion::{self as ex, events, market_code};
use bordrless_token::client as token;
use bordrless_token::state::Holding;
use hookwars_agents::constants::{kind, pda as apda};
use hookwars_agents::error::AgentsError as E;
use hookwars_agents::state::{Policy, PolicyLimits, TrackedLimit};
use hookwars_book::error::BookError as B;
use hookwars_book::state::{self as bs, side, BookParams};
use hookwars_common::economy::counter;
use hookwars_common::{ids, pda, template_id as t, EquipConfig};
use hookwars_craft::state::{material_mint_address, source};
use hookwars_market::error::MarketError as M;
use hookwars_market::licence::{self as lic, per, License};
use solana_keypair::Keypair;
use solana_signer::Signer;

// ---- agents ------------------------------------------------------------------------------------------

/// An agent whose vault holds 1,000 units of a tracked mint, 10 per action and per day.
fn vault_world() -> (Aw, Agent, Pubkey) {
    let mut aw = Aw::new();
    let a = aw.agent("vaulted", kind::RAIDER);
    let vault = apda::vault(&a.passport).0;
    let funder = aw.hw.w.env.funded(100 * SOL);
    let mint = aw.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TRK");
    let ixs = [
        token::create_holding(funder.pubkey(), mint, vault),
        token::transfer(funder.pubkey(), token::holding_address(&mint, &funder.pubkey()), token::holding_address(&mint, &vault), mint, None, vec![], 1_000),
    ];
    aw.hw.w.env.send_paid_by(&ixs, &funder, &[]).ok();
    let limits = PolicyLimits {
        per_action_lamports: SOL,
        per_day_lamports: SOL,
        tracked: vec![TrackedLimit { mint, per_action: 10, per_day: 10 }],
        targets: vec![bordrless_token::ID],
    };
    let ix = aw.init_policy_ix(&a, limits);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    (aw, a, mint)
}

/// H-1: `approve` (and `close_holding`) through `spend` are refused; a transfer within the limits
/// still goes through; the operator can clear a delegate with `revoke_vault`.
#[test]
fn sf3_h1_spend_refuses_approve_and_close_and_the_operator_can_revoke() {
    let (mut aw, a, mint) = vault_world();
    let vault = apda::vault(&a.passport).0;
    let vh = token::holding_address(&mint, &vault);
    let inner = token::approve(vault, vh, a.key.pubkey(), u64::MAX);
    let ix = aw.spend_ix(&a, inner);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).expect_code(agents_code(E::InstructionNotAllowed));
    let inner = token::close_holding(vault, mint, vh, a.key.pubkey());
    let ix = aw.spend_ix(&a, inner);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).expect_code(agents_code(E::InstructionNotAllowed));
    let h: Holding = aw.hw.w.env.read(&vh);
    assert!(h.delegate.is_none());
    // The honest path: 10 units out, counted.
    let dest = aw.hw.w.env.funded(SOL);
    let ixs = [token::create_holding(dest.pubkey(), mint, dest.pubkey())];
    aw.hw.w.env.send_paid_by(&ixs, &dest, &[]).ok();
    let inner = token::transfer(vault, vh, token::holding_address(&mint, &dest.pubkey()), mint, None, vec![], 10);
    let ix = aw.spend_ix(&a, inner);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).ok();
    let pol: Policy = aw.hw.w.env.read(&apda::policy(&a.passport).0);
    assert_eq!(pol.tracked[0].spent_today, 10);
    // A delegate however it got there (written in place here): the operator revokes it.
    let mut acc = aw.hw.w.env.account(&vh).unwrap();
    let mut h = Holding::try_deserialize(&mut &acc.data[..]).unwrap();
    h.delegate = Some(a.key.pubkey());
    h.delegated_amount = u64::MAX;
    let mut data = vec![0u8; acc.data.len()];
    h.try_serialize(&mut &mut data[..]).unwrap();
    acc.data = data;
    aw.hw.w.env.put(vh, acc);
    let revoke = agents_ix(
        hookwars_agents::accounts::RevokeVault {
            operator: a.operator.pubkey(),
            passport: a.passport,
            policy: apda::policy(&a.passport).0,
            vault,
            holding: vh,
            token: token_accs(),
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::RevokeVault {},
    );
    // Not the agent key: only the operator.
    let mut by_agent = revoke.clone();
    by_agent.accounts[0] = AccountMeta::new_readonly(a.key.pubkey(), true);
    aw.hw.w.env.send_paid_by(&[by_agent], &a.key, &[]).expect_code(agents_code(E::NotOperator));
    aw.hw.w.env.send_paid_by(&[revoke], &a.operator, &[]).ok();
    let h: Holding = aw.hw.w.env.read(&vh);
    assert!(h.delegate.is_none() && h.delegated_amount == 0);
}

/// L-5: a link statement names the passport's link counter, so after `unlink` the old ed25519
/// instruction cannot restore the link; a fresh statement can.
#[test]
fn sf3_l5_a_link_statement_cannot_be_replayed_after_unlink() {
    let mut aw = Aw::new();
    let a = aw.agent("linker", 0);
    let payer = aw.hw.w.env.funded(SOL);
    let stmt = aw.statement(&a, 0, "linker_x");
    let ixs = [ed25519_ix(&a.key, &stmt), aw.link_ix(&payer.pubkey(), &a.passport, 0, "linker_x")];
    aw.hw.w.env.send_paid_by(&ixs, &payer, &[]).ok();
    assert_eq!(aw.passport(&a.passport).link_nonce, 1);
    let unlink = agents_ix(
        hookwars_agents::accounts::Unlink {
            authority: a.key.pubkey(),
            passport: a.passport,
            link: apda::link(&a.passport, 0).0,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::Unlink { platform: 0 },
    );
    aw.hw.w.env.send_paid_by(&[unlink], &a.key, &[]).ok();
    // The old statement, copied from chain by anyone.
    aw.hw.w.env.warp(1);
    let stranger = aw.hw.w.env.funded(SOL);
    let ixs = [ed25519_ix(&a.key, &stmt), aw.link_ix(&stranger.pubkey(), &a.passport, 0, "linker_x")];
    aw.hw.w.env.send_paid_by(&ixs, &stranger, &[]).expect_code(agents_code(E::BadLinkSignature));
    // A new statement (nonce 1) links again.
    let fresh = aw.statement(&a, 0, "linker_x");
    assert_ne!(fresh, stmt);
    let ixs = [ed25519_ix(&a.key, &fresh), aw.link_ix(&stranger.pubkey(), &a.passport, 0, "linker_x")];
    aw.hw.w.env.send_paid_by(&ixs, &stranger, &[]).ok();
}

// ---- book --------------------------------------------------------------------------------------------

const MAT: u16 = 1;
const TICK: u64 = 1_000;

fn book_world() -> (Ew, Pubkey) {
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(source::QUEST_CLAIM, MAT, 1, 1);
    let creator = ew.funded(SOL);
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 1);
    ew.send(&creator, &[ix]).ok();
    (ew, material_mint_address(MAT).0)
}

fn trader(ew: &mut Ew, units: u64) -> Keypair {
    let k = ew.funded(100 * SOL);
    if units > 0 {
        ew.stub_drop(source::QUEST_CLAIM, MAT, units, &k.pubkey()).ok();
    }
    k
}

#[allow(clippy::too_many_arguments)]
fn place(ew: &mut Ew, who: &Keypair, base: &Pubkey, s: u8, price: u64, size: u64, makers: &[Pubkey]) -> bordrless_program_tests::Tx {
    let ix = ew.place_ix(&who.pubkey(), base, s, price, size, false, 0, makers);
    ew.send(who, &[ix])
}

/// Empties `k`'s wallet (the fee paid by someone else): the account stops existing.
fn drain(ew: &mut Ew, k: &Keypair) {
    let all = ew.lamports(&k.pubkey());
    let payer = ew.funded(SOL);
    let ix = system_instruction::transfer(&k.pubkey(), &Pubkey::new_unique(), all);
    ew.hw.w.env.send_paid_by(&[ix], &payer, &[k]).ok();
    assert_eq!(ew.lamports(&k.pubkey()), 0);
}

fn set_book_params(ew: &mut Ew, p: BookParams) {
    let d = ew.deployer();
    let treasury = ew.book_treasury;
    let ix = book_ix(
        hookwars_book::accounts::ProposeParams {
            admin: d.pubkey(),
            config: bs::config_address().0,
            pending: bs::pending_address().0,
            system_program: anchor_lang::system_program::ID,
            event_authority: events(&hookwars_book::ID),
            program: hookwars_book::ID,
        },
        hookwars_book::instruction::ProposeParams { params: p, treasury },
    );
    ew.send(&d, &[ix]).ok();
    ew.warp(i64::from(TEST_BOOK.admin_timelock_secs));
    let ix = book_ix(
        hookwars_book::accounts::ApplyParams { config: bs::config_address().0, pending: bs::pending_address().0 },
        hookwars_book::instruction::ApplyParams {},
    );
    ew.send(&d, &[ix]).ok();
}

/// M-1: a maker who emptied its wallet no longer freezes the book. What it is owed below the rent
/// minimum goes to the instruction's payer; the best ask fills, a full side evicts.
#[test]
fn sf3_m1_a_drained_maker_no_longer_freezes_the_book() {
    let (mut ew, base) = book_world();
    let griefer = trader(&mut ew, 10);
    place(&mut ew, &griefer, &base, side::ASK, TICK, 1, &[]).ok();
    drain(&mut ew, &griefer);
    let honest = trader(&mut ew, 100);
    place(&mut ew, &honest, &base, side::ASK, 2 * TICK, 50, &[]).ok();
    let buyer = trader(&mut ew, 0);
    place(&mut ew, &buyer, &base, side::BID, 2 * TICK, 10, &[griefer.pubkey(), honest.pubkey()]).ok();
    assert_eq!(ew.holding(&base, &buyer.pubkey()), 10);
    assert_eq!(ew.lamports(&griefer.pubkey()), 0, "nothing landed in the emptied wallet");
    ew.assert_escrow(&base);

    // A full side of drained owners evicts.
    let (mut ew, base) = book_world();
    for i in 0..TEST_BOOK.slots {
        let g = trader(&mut ew, 10);
        place(&mut ew, &g, &base, side::ASK, (100 + u64::from(i)) * TICK, 1, &[]).ok();
        drain(&mut ew, &g);
    }
    let worst = ew.book(&base).asks.last().unwrap().owner;
    let honest = trader(&mut ew, 100);
    place(&mut ew, &honest, &base, side::ASK, 50 * TICK, 10, &[worst]).ok();
    assert_eq!(ew.book(&base).asks[0].owner, honest.pubkey());
    ew.assert_escrow(&base);
    // A drained bidder's reserve on eviction also goes to the placer, not into the void.
    let (mut ew, base) = book_world();
    for i in 0..TEST_BOOK.slots {
        let g = trader(&mut ew, 0);
        place(&mut ew, &g, &base, side::BID, (10 + u64::from(i)) * TICK, 1, &[]).ok();
        drain(&mut ew, &g);
    }
    let worst = ew.book(&base).bids.last().unwrap().owner;
    let better = trader(&mut ew, 0);
    place(&mut ew, &better, &base, side::BID, 20 * TICK, 1, &[worst]).ok();
    ew.assert_escrow(&base);
}

/// M-8: a market's tick and minimum size stay inside the config's bounds, and the admin can change
/// them after the timelock.
#[test]
fn sf3_m8_market_terms_are_bounded_and_settable_by_the_admin() {
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000_000);
    let creator = ew.funded(SOL);
    for (tick, min) in [(0, 1), (TEST_BOOK.tick_max_lamports + 1, 1), (u64::MAX, 1), (TICK, 0), (TICK, TEST_BOOK.min_size_max + 1)] {
        let ix = ew.create_market_ix(&creator.pubkey(), MAT, tick, min);
        ew.send(&creator, &[ix]).expect_code(book_code(B::BadTerms));
    }
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 1);
    ew.send(&creator, &[ix]).ok();
    let base = material_mint_address(MAT).0;
    let market = bs::market_address(&base).0;
    let propose = |admin: &Pubkey, tick: u64, min: u64| {
        book_ix(
            hookwars_book::accounts::ProposeMarketTerms {
                admin: *admin,
                config: bs::config_address().0,
                market,
                pending: bs::pending_terms_address(&market).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: events(&hookwars_book::ID),
                program: hookwars_book::ID,
            },
            hookwars_book::instruction::ProposeMarketTerms { tick_lamports: tick, min_size: min },
        )
    };
    let apply = book_ix(
        hookwars_book::accounts::ApplyMarketTerms {
            config: bs::config_address().0,
            market,
            pending: bs::pending_terms_address(&market).0,
            event_authority: events(&hookwars_book::ID),
            program: hookwars_book::ID,
        },
        hookwars_book::instruction::ApplyMarketTerms {},
    );
    ew.send(&creator, &[propose(&creator.pubkey(), 2 * TICK, 5)]).expect_code(book_code(B::NotAdmin));
    let d = ew.deployer();
    ew.send(&d, &[propose(&d.pubkey(), 0, 5)]).expect_code(book_code(B::BadTerms));
    ew.send(&d, &[propose(&d.pubkey(), 2 * TICK, 5)]).ok();
    ew.send(&d, std::slice::from_ref(&apply)).expect_code(book_code(B::NotReady));
    ew.warp(i64::from(TEST_BOOK.admin_timelock_secs));
    ew.send(&d, &[apply]).ok();
    let m = ew.book(&base);
    assert_eq!((m.tick_lamports, m.min_size), (2 * TICK, 5));
}

/// L-1 (book): fills whose fees are under `skill_min_fee_lamports` do not count toward Trader.
#[test]
fn sf3_l1_cheap_fills_do_not_count_for_skills() {
    let (mut ew, base) = book_world();
    set_book_params(&mut ew, BookParams { skill_min_fee_lamports: 1_000, ..TEST_BOOK });
    let a = trader(&mut ew, 100);
    let b = trader(&mut ew, 0);
    ew.open_profile(&b);
    for _ in 0..TEST_BOOK.slots {
        place(&mut ew, &a, &base, side::ASK, TICK, 1, &[]).ok();
    }
    let makers = vec![a.pubkey(); usize::from(TEST_BOOK.slots)];
    place(&mut ew, &b, &base, side::BID, TICK, u64::from(TEST_BOOK.slots), &makers).ok();
    assert_eq!(ew.holding(&base, &b.pubkey()), u64::from(TEST_BOOK.slots));
    assert_eq!(ew.profile(&b.pubkey()).counters[usize::from(counter::BOOK_FILLS)], 0);
}

/// L-2: a bid resting under the old maker rate still fills after `maker_bps` rises.
#[test]
fn sf3_l2_a_higher_maker_rate_does_not_strand_resting_bids() {
    let (mut ew, base) = book_world();
    let bidder = trader(&mut ew, 0);
    place(&mut ew, &bidder, &base, side::BID, 10 * TICK, 10, &[]).ok();
    set_book_params(&mut ew, BookParams { maker_bps: 500, ..TEST_BOOK });
    let seller = trader(&mut ew, 100);
    // Partly, then the rest: every fill within the reserve.
    place(&mut ew, &seller, &base, side::ASK, 10 * TICK, 4, &[bidder.pubkey()]).ok();
    ew.assert_escrow(&base);
    place(&mut ew, &seller, &base, side::ASK, 10 * TICK, 6, &[bidder.pubkey()]).ok();
    assert_eq!(ew.holding(&base, &bidder.pubkey()), 10);
    assert!(ew.book(&base).bids.is_empty());
    ew.assert_escrow(&base);
}

// ---- market: licences --------------------------------------------------------------------------------

const TERM: u32 = 86_400;

/// M-2: only the licence's payer renews it while live; a revocation refunds that payer.
#[test]
fn sf3_m2_a_renewal_cannot_redirect_the_revocation_refund() {
    let price = SOL;
    let mut ew = Ew::new();
    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    ew.hw.w.env.fund(holder.pubkey(), 20 * SOL);
    let author = ew.template_author(&item);
    let ix = ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, price, TERM, per::PER_PERIOD, 1, true, true);
    ew.send(&holder, &[ix]).ok();
    let funder = ew.funded(10 * SOL);
    let tok = ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK");
    let payer = ew.funded(10 * SOL);
    for renew in [false, true, true, true] {
        let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, price, renew);
        ew.send(&payer, &[ix]).ok();
    }
    // The holder (or anyone) renewing someone else's live licence is refused.
    let ix = ew.buy_license_ix(&holder.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, price, true);
    ew.send(&holder, &[ix]).expect_code(market_code(M::NotLicencePayer));
    let l: License = ew.hw.w.env.read(&lic::license_address(&item, &tok).0);
    assert_eq!(l.payer, payer.pubkey());
    // Revoking at once refunds the payer (almost) everything it prepaid.
    let p0 = ew.lamports(&payer.pubkey());
    let ix = ew.revoke_license_ix(&holder.pubkey(), &item, &item_mint, &tok, &payer.pubkey());
    ew.send(&holder, &[ix]).ok();
    assert!(ew.lamports(&payer.pubkey()) - p0 >= 4 * price - 4 * price / 86_400);
}

/// L-4: a licence on a listed item (held by the listing escrow) is refused.
#[test]
fn sf3_l4_a_licence_never_pays_a_market_escrow() {
    let mut ew = Ew::new();
    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let author = ew.template_author(&item);
    let ix = ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, SOL, TERM, per::PER_TOKEN, 1, false, true);
    ew.send(&holder, &[ix]).ok();
    ew.send(&holder, &[ex::list_ix(&holder.pubkey(), &item, &item_mint, SOL, 0)]).ok();
    let escrow = hookwars_market::state::escrow_address(&item_mint).0;
    let funder = ew.funded(10 * SOL);
    let tok = ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK");
    let payer = ew.funded(10 * SOL);
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &escrow, &author, SOL, false);
    ew.send(&payer, &[ix]).expect_code(market_code(M::HolderIsEscrow));
}

/// L-1 (licences): a licence whose protocol fee is under the floor does not count as sold.
#[test]
fn sf3_l1_cheap_licences_do_not_count_for_skills() {
    let mut ew = Ew::new();
    let d = ew.deployer();
    let p = hookwars_market::licence::LicenceParams { skill_min_fee_lamports: 1_000, ..TEST_LICENCE };
    let ix = ex::market_ix(
        hookwars_market::accounts::ProposeLicenceParams {
            admin: d.pubkey(),
            config: hookwars_market::state::config_address().0,
            licence_config: lic::licence_config_address().0,
            event_authority: events(&hookwars_market::ID),
            program: hookwars_market::ID,
        },
        hookwars_market::instruction::ProposeLicenceParams { params: p },
    );
    ew.send(&d, &[ix]).ok();
    ew.warp(600);
    let ix = ex::market_ix(
        hookwars_market::accounts::ApplyLicenceParams { licence_config: lic::licence_config_address().0 },
        hookwars_market::instruction::ApplyLicenceParams {},
    );
    ew.send(&d, &[ix]).ok();
    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    ew.open_profile(&holder);
    let author = ew.template_author(&item);
    // A 1-lamport licence: no protocol fee, no count.
    let ix = ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, 1, TERM, per::PER_TOKEN, 2, false, true);
    ew.send(&holder, &[ix]).ok();
    let funder = ew.funded(10 * SOL);
    let tok = ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK");
    let payer = ew.funded(10 * SOL);
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, 1, false);
    ew.send(&payer, &[ix]).ok();
    assert_eq!(ew.profile(&holder.pubkey()).counters[usize::from(counter::LICENCES_SOLD)], 0);
    // At a real price it counts.
    let ix = ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, SOL, TERM, per::PER_TOKEN, 2, false, true);
    ew.send(&holder, &[ix]).ok();
    let tok2 = ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK2");
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok2, &holder.pubkey(), &author, SOL, false);
    ew.send(&payer, &[ix]).ok();
    assert_eq!(ew.profile(&holder.pubkey()).counters[usize::from(counter::LICENCES_SOLD)], 1);
}

// ---- market: leases ----------------------------------------------------------------------------------

fn fee_config() -> EquipConfig {
    EquipConfig { targets: vec![Pubkey::new_unique()], role: 0 }
}

fn revert_tail(hw: &Hw, cranker: &Pubkey, mint: &Pubkey, slot: u8, item: Pubkey, launch_item: Option<Pubkey>) -> Vec<AccountMeta> {
    let mut metas = hookwars_armory::accounts::RevertForLeaseEnd {
        market_caller: hookwars_common::market::caller().0,
        config: pda::config().0,
        slot_state: pda::slot_state(mint, slot).0,
        equip: hw.equip_accounts(cranker, mint, slot, Some(item), launch_item),
        event_authority: bordrless_program_tests::armory::armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    metas[0].is_signer = false;
    metas.extend(hw.refresh_tail(mint, slot));
    metas
}

fn vote_in(hw: &mut Hw, owner: &Keypair, mint: &Pubkey, item: Pubkey) {
    let (tx, proposal) = hw.propose(owner, mint, 2, Some(item), fee_config());
    tx.ok();
    hw.vote(owner, mint, &proposal, true, 1_000).ok();
    hw.w.env.warp(i64::from(bordrless_program_tests::armory::TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    hw.w.env.warp(600);
    hw.execute(owner, &proposal).ok();
    assert_eq!(hw.slot_item(mint, 2), item);
}

fn lease_world() -> (Hw, Keypair, Pubkey, Pubkey, Pubkey) {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    ex::load(&mut hw.w.env, treasury);
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, bordrless_program_tests::armory::test_slots());
    let other = hw.slot_mint(&owner, bordrless_program_tests::armory::test_slots());
    let (_, incumbent, _) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    hw.equip_launch(&owner, &mint, Hw::entry(2, Some(incumbent), fee_config())).ok();
    hw.equip_launch(&owner, &other, Hw::entry(2, Some(incumbent), fee_config())).ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    hw.mint_to(&owner, &other, &o, 1_000);
    (hw, owner, mint, other, incumbent)
}

/// M-4: `end_lease` refuses revert accounts for a token the lease does not name.
#[test]
fn sf3_m4_end_lease_reverts_only_the_leases_own_token() {
    let (mut hw, owner, mint, other, incumbent) = lease_world();
    let (lessor, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    vote_in(&mut hw, &owner, &other, item);
    let term = ex::TEST_MARKET.lease_min_secs;
    ex::send(&mut hw.w.env, &lessor, &[ex::offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 1_000, 0, term)]).ok();
    let payer = hw.w.env.funded(SOL);
    ex::send(&mut hw.w.env, &payer, &[ex::accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey())]).ok();
    hw.w.env.warp(i64::from(term));
    let cranker = hw.w.env.funded(SOL);
    let mut ix = ex::close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.extend(revert_tail(&hw, &cranker.pubkey(), &other, 2, item, Some(incumbent)));
    ex::send(&mut hw.w.env, &cranker, &[ix]).expect_code(market_code(M::WrongAccount));
    assert_eq!(hw.slot_item(&other, 2), item, "the other token's vote stands");
    // The other token's mint as the lone account is not the lease's token either.
    let mut ix = ex::close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.push(AccountMeta::new_readonly(other, false));
    ex::send(&mut hw.w.env, &cranker, &[ix]).expect_code(market_code(M::WrongAccount));
    // The lease's own token (the item is not in its slot): the mint alone ends it.
    let mut ix = ex::close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.push(AccountMeta::new_readonly(mint, false));
    ex::send(&mut hw.w.env, &cranker, &[ix]).ok();
    assert_eq!(hw.w.env.holding(&item_mint, &lessor.pubkey()), 1);
}

/// M-5: `end_lease` cannot leave the leased item equipped: without the revert, and with the mint
/// alone while the slot still holds the item, it is refused; with the revert the slot goes back.
#[test]
fn sf3_m5_end_lease_cannot_leave_the_item_equipped() {
    let (mut hw, owner, mint, _other, incumbent) = lease_world();
    let (lessor, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let term = ex::TEST_MARKET.lease_min_secs;
    ex::send(&mut hw.w.env, &lessor, &[ex::offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 1_000, 0, term)]).ok();
    let payer = hw.w.env.funded(SOL);
    ex::send(&mut hw.w.env, &payer, &[ex::accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey())]).ok();
    vote_in(&mut hw, &owner, &mint, item);
    hw.w.env.warp(i64::from(term));
    let cranker = hw.w.env.funded(SOL);
    let ix = ex::close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ex::send(&mut hw.w.env, &cranker, &[ix]).expect_code(market_code(M::RevertAccountsMissing));
    let mut ix = ex::close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.push(AccountMeta::new_readonly(mint, false));
    ex::send(&mut hw.w.env, &cranker, &[ix]).expect_code(market_code(M::RevertAccountsMissing));
    assert_eq!(hw.slot_item(&mint, 2), item);
    let mut ix = ex::close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.extend(revert_tail(&hw, &cranker.pubkey(), &mint, 2, item, Some(incumbent)));
    ex::send(&mut hw.w.env, &cranker, &[ix]).ok();
    assert_eq!(hw.slot_item(&mint, 2), incumbent);
    assert_eq!(hw.w.env.holding(&item_mint, &lessor.pubkey()), 1);
}

// ---- market: licence terms from the armory (11 E-1, spec 14) -----------------------------------------

/// A `Licensed` `AccessPolicy` with terms is where `buy_license` reads price, term, `per` and
/// `max_live`; no `LicenceOffer` is needed (it opens one for the live count).
#[test]
fn sf3_buy_license_reads_the_armory_access_policy() {
    let mut ew = Ew::new();
    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let author = ew.template_author(&item);
    let terms = hookwars_armory::state::LicenceTerms { price_lamports: 2 * SOL, term_secs: TERM, per: per::PER_PERIOD, max_live: 1 };
    let (policy_key, bump) = hookwars_common::access::policy_address(&item);
    let policy = hookwars_armory::state::AccessPolicy {
        item,
        mode: hookwars_common::access::LICENSED,
        exclusive: true,
        licence_terms: Some(terms),
        holder_at_set: holder.pubkey(),
        updated_at: ew.now(),
        bump,
    };
    let mut data = Vec::new();
    policy.try_serialize(&mut data).unwrap();
    let lamports = ew.hw.w.env.rent(data.len());
    ew.hw.w.env.put(policy_key, solana_account::Account { lamports, data, owner: ids::ARMORY_ID, executable: false, rent_epoch: 0 });
    let funder = ew.funded(10 * SOL);
    let tok = ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK");
    let tok2 = ew.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TOK2");
    let payer = ew.funded(10 * SOL);
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, SOL, false);
    ew.send(&payer, &[ix]).expect_code(market_code(M::PriceMoved));
    let h0 = ew.lamports(&holder.pubkey());
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, 2 * SOL, false);
    ew.send(&payer, &[ix]).ok();
    assert!(ew.lamports(&holder.pubkey()) > h0);
    let l: License = ew.hw.w.env.read(&lic::license_address(&item, &tok).0);
    assert_eq!((l.per, l.ends_at - l.starts_at), (per::PER_PERIOD, i64::from(TERM)));
    let o: hookwars_market::licence::LicenceOffer = ew.hw.w.env.read(&lic::licence_offer_address(&item).0);
    assert_eq!(o.live, 1);
    // `max_live` 1 from the policy: a second token is refused.
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok2, &holder.pubkey(), &author, 2 * SOL, false);
    ew.send(&payer, &[ix]).expect_code(market_code(M::LicenceSoldOut));
}
