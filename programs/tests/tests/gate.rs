// Changed by Hookwars: new file (gating, docs/spec/18-gating.md part C): the external gate.
//! A Token-2022 mint of another protocol registers with the gate and runs units items as its
//! transfer hook: refusals and holder stamps work through the real Token-2022 program; a lapsed
//! licence makes its binding inert and anyone may then unbind it; items that cut, external
//! templates and exclusive items are refused at bind; `Execute` outside a transfer is refused; a
//! strict mint needs holder states; plain Token-2022 mints are untouched and cannot buy licences.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::InstructionData;
use bordrless_program_tests::armory::{items_code, params};
use bordrless_program_tests::economy::SOL;
use bordrless_program_tests::expansion::market_code;
use bordrless_program_tests::gate::*;
use hookwars_common::gate::{self as g, GATE_ID};
use hookwars_common::template_id as t;
use hookwars_gate::{kind, GateError as G, HolderState};
use hookwars_items::ItemsError as I;
use hookwars_market::error::MarketError as M;
use hookwars_market::licence::{self as lic, per};
use solana_keypair::Keypair;
use solana_signer::Signer;

const TERM: u32 = 86_400;

struct Setup {
    gw: Gw,
    auth: Keypair,
    mint: Pubkey,
    alice: Keypair,
    bob: Keypair,
    a: Pubkey,
    b: Pubkey,
}

/// A registered mint (venue `venue`, `strict`), alice holding 1,000,000 and bob 0.
fn setup(venue: Pubkey, strict: bool) -> Setup {
    let mut gw = Gw::new();
    let auth = gw.funded();
    let mint = gw.hooked_mint(&auth, &GATE_ID);
    gw.register(&auth, &mint, venue, Some(strict)).ok();
    let alice = gw.funded();
    let bob = gw.funded();
    let a = gw.account(&mint, &alice.pubkey(), &auth, 1_000_000, true);
    let b = gw.account(&mint, &bob.pubkey(), &auth, 0, true);
    Setup { gw, auth, mint, alice, bob, a, b }
}

/// Binds an owned item (deposited into the gate vault) to slot `slot`.
fn bind_owned(s: &mut Setup, slot: u8, template: u16, p: &[u32]) -> (Pubkey, Pubkey) {
    let (author, item, item_mint) = s.gw.item(template, params(p));
    let proof = s.gw.deposit_item(&author, &s.mint, &item_mint);
    let ix = s.gw.bind_ix(&s.auth.pubkey(), &s.mint, slot, kind::OWNED, &item, &proof, vec![], &[g::mint_gate(&s.mint).0]);
    let auth = s.auth.insecure_clone();
    s.gw.send(&auth, &[ix]).ok();
    (item, proof)
}

#[test]
fn an_owned_dust_guard_refuses_dust_on_an_external_token() {
    let mut s = setup(Pubkey::default(), false);
    let alice = s.alice.insecure_clone();
    // Registered, nothing bound: transfers go through.
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 5).ok();
    assert_eq!(s.gw.balance(&s.b), 5);
    bind_owned(&mut s, 0, t::DUST_GUARD, &[100]);
    let gate = s.gw.mint_gate(&s.mint);
    assert_eq!(gate.bindings.len(), 1);
    // Dust between wallets is refused by the item, through Token-2022 and the gate.
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 5).expect_code(items_code(I::DustRefused));
    let tx = s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 200);
    tx.ok();
    assert_eq!(s.gw.balance(&s.b), 205);
    println!("gated transfer, one binding: {} CU", tx.cu());
    assert!(tx.cu() < 200_000);
}

#[test]
fn a_licensed_binding_lapses_to_inert_and_anyone_unbinds_it() {
    let mut s = setup(Pubkey::default(), false);
    let (holder, item, item_mint) = s.gw.item(t::DUST_GUARD, params(&[100]));
    let author = s.gw.ew.template_author(&item);
    let price = SOL / 10;
    let ix = s.gw.ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, price, TERM, per::PER_TOKEN, 3, false, true);
    s.gw.send(&holder, &[ix]).ok();
    let proof = lic::license_address(&item, &s.mint).0;
    // Binding without a licence is refused.
    let ix = s.gw.bind_ix(&s.auth.pubkey(), &s.mint, 1, kind::LICENCE, &item, &proof, vec![], &[g::mint_gate(&s.mint).0]);
    let auth = s.auth.insecure_clone();
    s.gw.send(&auth, &[ix.clone()]).expect_code(gate_code(G::ProofNotLive));
    // The project buys a licence for its Token-2022 mint (the market accepts a mint hooked by the gate).
    let payer = s.gw.funded();
    let buy = s.gw.ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &s.mint, &holder.pubkey(), &author, price, false);
    s.gw.send(&payer, &[buy]).ok();
    s.gw.send(&auth, &[ix]).ok();
    let alice = s.alice.insecure_clone();
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 5).expect_code(items_code(I::DustRefused));
    // Nobody but the authority may unbind a live binding.
    let stranger = s.gw.funded();
    let ix = s.gw.unbind_ix(&stranger.pubkey(), &s.mint, 1, &proof);
    s.gw.send(&stranger, &[ix.clone()]).expect_code(gate_code(G::StillLive));
    // The term ends: the binding is inert, transfers pass, and anyone may unbind it.
    s.gw.ew.warp(i64::from(TERM));
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 5).ok();
    s.gw.send(&stranger, &[ix]).ok();
    assert!(s.gw.mint_gate(&s.mint).bindings.is_empty());
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 5).ok();
    assert_eq!(s.gw.balance(&s.b), 10);
}

#[test]
fn a_cooldown_stamps_buys_from_the_venue_and_strict_needs_holder_states() {
    let venue = Keypair::new();
    let mut s = setup(venue.pubkey(), true);
    s.gw.ew.hw.w.env.fund(venue.pubkey(), SOL);
    let auth = s.auth.insecure_clone();
    let v = s.gw.account(&s.mint, &venue.pubkey(), &auth, 1_000_000, true);
    bind_owned(&mut s, 0, t::COOLDOWN, &[60]);
    // Strict: alice has no holder state yet.
    s.gw.transfer(&venue, &s.mint, &v, &s.a, 1_000).expect_code(gate_code(G::HolderStateMissing));
    let (ak, bk) = (s.alice.pubkey(), s.bob.pubkey());
    s.gw.open_holder(&auth, &s.mint, &ak).ok();
    s.gw.open_holder(&auth, &s.mint, &bk).ok();
    // A buy from the venue stamps alice.
    s.gw.transfer(&venue, &s.mint, &v, &s.a, 1_000).ok();
    let h: HolderState = s.gw.ew.hw.w.env.read(&g::holder(&s.mint, &ak).0);
    assert_eq!(h.data[0], 0x11, "cooldown tag");
    assert_eq!(h.generations[0], s.gw.mint_gate(&s.mint).bindings[0].generation);
    // Alice may not send before the cooldown passes.
    let alice = s.alice.insecure_clone();
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 10).expect_code(items_code(I::CooldownActive));
    s.gw.ew.warp(61);
    s.gw.transfer(&alice, &s.mint, &s.a, &s.b, 10).ok();
}

#[test]
fn cut_items_unlive_proofs_and_wrong_extras_are_refused_at_bind() {
    let mut s = setup(Pubkey::default(), false);
    let auth = s.auth.insecure_clone();
    // Transfer Fee cuts on the token side: a transfer hook cannot take it.
    let (author, item, item_mint) = s.gw.item(t::TRANSFER_FEE, params(&[100, 100]));
    let proof = s.gw.deposit_item(&author, &s.mint, &item_mint);
    let ix = s.gw.bind_ix(&auth.pubkey(), &s.mint, 0, kind::OWNED, &item, &proof, vec![Pubkey::new_unique()], &[g::mint_gate(&s.mint).0]);
    s.gw.send(&auth, &[ix]).expect_code(gate_code(G::ItemNotGateable));
    // A Dust Guard the vault does not hold.
    let (_, item, item_mint) = s.gw.item(t::DUST_GUARD, params(&[100]));
    let proof = bordrless_token::client::holding_address(&item_mint, &g::vault(&s.mint).0);
    let ix = s.gw.bind_ix(&auth.pubkey(), &s.mint, 0, kind::OWNED, &item, &proof, vec![], &[g::mint_gate(&s.mint).0]);
    s.gw.send(&auth, &[ix]).expect_fail();
    // The wrong number of extras for the template.
    let (author, item, item_mint) = s.gw.item(t::DUST_GUARD, params(&[100]));
    let proof = s.gw.deposit_item(&author, &s.mint, &item_mint);
    let ix = s.gw.bind_ix(&auth.pubkey(), &s.mint, 0, kind::OWNED, &item, &proof, vec![], &[]);
    s.gw.send(&auth, &[ix]).expect_code(gate_code(G::WrongAccount));
    // Only the gate authority binds.
    let other = s.gw.funded();
    let ix = s.gw.bind_ix(&other.pubkey(), &s.mint, 0, kind::OWNED, &item, &proof, vec![], &[g::mint_gate(&s.mint).0]);
    s.gw.send(&other, &[ix]).expect_code(gate_code(G::NotGateAuthority));
    // A bound item cannot be withdrawn.
    let ix = s.gw.bind_ix(&auth.pubkey(), &s.mint, 0, kind::OWNED, &item, &proof, vec![], &[g::mint_gate(&s.mint).0]);
    s.gw.send(&auth, &[ix]).ok();
    let ix = gate_ix(
        hookwars_gate::accounts::WithdrawItem {
            authority: auth.pubkey(),
            mint_gate: g::mint_gate(&s.mint).0,
            vault: g::vault(&s.mint).0,
            item_mint,
            vault_holding: proof,
            destination: bordrless_token::client::holding_address(&item_mint, &author.pubkey()),
            token_program: bordrless_token::ID,
            token_event_authority: bordrless_token::client::event_authority(),
        },
        hookwars_gate::instruction::WithdrawItem {},
    );
    s.gw.send(&auth, &[ix.clone()]).expect_code(gate_code(G::ItemBound));
    // Unbound, the authority takes it back.
    let un = s.gw.unbind_ix(&auth.pubkey(), &s.mint, 0, &proof);
    s.gw.send(&auth, &[un]).ok();
    s.gw.send(&auth, &[ix]).ok();
    assert_eq!(s.gw.ew.hw.w.env.holding(&item_mint, &author.pubkey()), 1);
}

#[test]
fn execute_outside_a_transfer_and_foreign_registrations_are_refused() {
    let mut s = setup(Pubkey::default(), false);
    bind_owned(&mut s, 0, t::DUST_GUARD, &[100]);
    // Calling Execute directly (not from a Token-2022 transfer) cannot run the bindings.
    let mut accounts = vec![
        AccountMeta::new_readonly(s.a, false),
        AccountMeta::new_readonly(s.mint, false),
        AccountMeta::new_readonly(s.b, false),
        AccountMeta::new_readonly(s.alice.pubkey(), false),
        AccountMeta::new_readonly(g::extra_metas(&s.mint).0, false),
    ];
    let mut extra = s.gw.hook_accounts(&s.mint, &s.a, &s.b);
    extra.truncate(extra.len() - 2);
    accounts.extend(extra);
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: GATE_ID,
        accounts,
        data: hookwars_gate::instruction::Execute { amount: 5 }.data(),
    };
    let alice = s.alice.insecure_clone();
    s.gw.send(&alice, &[ix]).expect_code(gate_code(G::NotTransferring));
    // Only the mint's transfer-hook authority registers it.
    let auth = s.gw.funded();
    let mint = s.gw.hooked_mint(&auth, &GATE_ID);
    let other = s.gw.funded();
    let ix = s.gw.register_ix(&other.pubkey(), &other.pubkey(), &mint, Pubkey::default(), None);
    s.gw.send(&other, &[ix]).expect_code(gate_code(G::NotHookAuthority));
    // A mint hooked by another program cannot register.
    let foreign = s.gw.hooked_mint(&auth, &Pubkey::new_unique());
    s.gw.register(&auth, &foreign, Pubkey::default(), None).expect_code(gate_code(G::NotHooked));
    // Lamports sent to the validation account's address first do not block the registration.
    s.gw.ew.hw.w.env.fund(g::extra_metas(&mint).0, 1_000);
    s.gw.register(&auth, &mint, Pubkey::default(), None).ok();
}

#[test]
fn a_plain_token_2022_mint_is_untouched_and_cannot_buy_licences() {
    let mut gw = Gw::new();
    let auth = gw.funded();
    let mint = gw.plain_mint(&auth);
    let alice = gw.funded();
    let a = gw.account(&mint, &alice.pubkey(), &auth, 1_000, false);
    let b = gw.account(&mint, &Pubkey::new_unique(), &auth, 0, false);
    let ix = transfer_checked_ix(&a, &mint, &b, &alice.pubkey(), 7, vec![]);
    gw.send(&alice, &[ix]).ok();
    assert_eq!(gw.balance(&b), 7);
    let (holder, item, item_mint) = gw.item(t::DUST_GUARD, params(&[100]));
    let author = gw.ew.template_author(&item);
    let ix = gw.ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, SOL, TERM, per::PER_TOKEN, 3, false, true);
    gw.send(&holder, &[ix]).ok();
    let payer = gw.funded();
    let buy = gw.ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &mint, &holder.pubkey(), &author, SOL, false);
    gw.send(&payer, &[buy]).expect_code(market_code(M::WrongAccount));
}

#[test]
fn the_templates_the_app_offers_on_external_tokens_never_cut_and_run_on_transfers() {
    use hookwars_common::{manifest, token_flags};
    for id in [17u16, 18, 19, 20, 22, 26, 39] {
        let (min, _max, _, _) = bordrless_program_tests::armory::test_schema(id);
        let m = manifest(id, &min, 0).expect("manifest");
        assert!(!m.token_cuts(), "template {id} cuts on the token side");
        assert!(m.token_flags & token_flags::BEFORE_TRANSFER != 0, "template {id} has no token-side callback");
    }
}
