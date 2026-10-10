// Changed by Hookwars: new file, budgets of the hook economy paths (11, 07 section 3): keys, v0
// bytes with and without one lookup table, trace entries, call height and compute of each craft,
// book, licence, profile and directive instruction a user, cranker or protocol caller sends.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_program_tests::agents::*;
use bordrless_program_tests::armory::params;
use bordrless_program_tests::economy::*;
use bordrless_program_tests::env::{compute_unit_limit, Tx};
use bordrless_program_tests::fixture::World;
use hookwars_agents::constants::{kind, pda as apda, INSTRUCTIONS_SYSVAR_ID};
use hookwars_agents::handlers::directive::{directive_address, memo_config_address, DirectiveConstraints, MemoParams};
use hookwars_agents::state::PolicyLimits;
use hookwars_book::state::{side, ClassKey};
use hookwars_common::economy::{counter, MEMO_PROGRAM_ID};
use hookwars_common::{ids, template_id as t, PARAM_FIELDS};
use hookwars_craft::state::{recipe_kind, source};
use hookwars_market::licence::per;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn measure_with(w: &mut World, label: &str, payer: &Keypair, signers: &[&Keypair], ixs: Vec<Instruction>) -> Tx {
    let mut all = vec![compute_unit_limit(1_400_000)];
    all.extend(ixs);
    let mut keys: Vec<Pubkey> = Vec::new();
    for i in &all {
        for m in &i.accounts {
            if !keys.contains(&m.pubkey) {
                keys.push(m.pubkey);
            }
        }
    }
    let table = w.env.put_lookup_table(Pubkey::new_unique(), &keys);
    let with_table = w.env.v0_size(&all, payer, signers, &[table]);
    let tx = w.env.send_v0(&all, payer, signers, &[]);
    tx.ok();
    let (n_keys, size, trace, height, cu) = (tx.keys.len(), tx.size, tx.trace_len(), tx.max_height(), tx.cu());
    println!(
        "budget | {label} | keys {n_keys} | v0 bytes {size} | with table {with_table} | trace {trace} | height {height} | CU {cu}"
    );
    assert!(with_table <= 1_232 && trace <= 64 && height <= 5 && cu <= 1_400_000, "{label}");
    tx
}

fn measure(w: &mut World, label: &str, payer: &Keypair, ix: Instruction) -> Tx {
    measure_with(w, label, payer, &[], vec![ix])
}

const MAT: u16 = 1;
const MAT2: u16 = 2;

#[test]
fn craft_and_profile_budgets() {
    let mut ew = Ew::new();
    let d = ew.deployer();
    let ix = ew.create_material_ix(MAT, "Iron", 1_000_000);
    measure(&mut ew.hw.w, "craft create_material", &d, ix);
    ew.create_material(MAT2, 1_000_000);
    ew.drop_rule(source::SETTLE_CRANK, MAT, 1, 1);
    ew.drop_rule(source::RAID_REVEAL, MAT2, 1, 1);
    let c = ew.funded(10 * SOL);
    ew.open_profile(&c);
    let payer = ew.funded(SOL);
    let caller = stub_pda(hookwars_common::economy::CRAFT_CALLER_SEED);
    let ix = via_stub(hookwars_common::economy::CRAFT_CALLER_SEED, ew.drop_ix(&caller, &STUB, &payer.pubkey(), source::SETTLE_CRANK, MAT, 500, &c.pubkey()));
    measure(&mut ew.hw.w, "craft drop (new holding, via caller)", &payer, ix);
    ew.stub_drop(source::RAID_REVEAL, MAT2, 500, &c.pubkey()).ok();
    let inputs = [(MAT, 2), (MAT2, 1)];
    let mut r = recipe(recipe_kind::CRAFT, &inputs, 100_000, t::TRANSFER_FEE);
    r.min_level = 0;
    ew.create_recipe(1, r).ok();
    let mut gated = recipe(recipe_kind::CRAFT, &[(MAT, 1), (MAT2, 1), (3, 1), (4, 1)], 100_000, t::TRANSFER_FEE);
    gated.min_level = 1;
    ew.create_material(3, 1_000);
    ew.create_material(4, 1_000);
    ew.drop_rule(source::SEASON_FINISH, 3, 1, 1);
    ew.drop_rule(source::QUEST_CLAIM, 4, 1, 1);
    ew.stub_drop(source::SEASON_FINISH, 3, 10, &c.pubkey()).ok();
    ew.stub_drop(source::QUEST_CLAIM, 4, 10, &c.pubkey()).ok();
    ew.create_recipe(2, gated).ok();
    ew.warp(600);
    let ix = ew.craft_item_ix(&c.pubkey(), 1, &inputs);
    measure(&mut ew.hw.w, "craft craft (2 inputs, counter)", &c, ix);
    let ix = ew.craft_item_ix(&c.pubkey(), 2, &[(MAT, 1), (MAT2, 1), (3, 1), (4, 1)]);
    measure(&mut ew.hw.w, "craft craft (4 inputs, level gate, counter)", &c, ix);

    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let ix = ew.init_wear_ix(&caller, &STUB, &payer.pubkey(), &item, 10);
    measure(&mut ew.hw.w, "craft init_wear (via caller)", &payer, via_stub(hookwars_common::economy::CRAFT_CALLER_SEED, ix));
    let ix = ew.wear_ix(&caller, &STUB, &item, 10);
    measure(&mut ew.hw.w, "craft wear turning dormant (via caller)", &payer, via_stub(hookwars_common::economy::CRAFT_CALLER_SEED, ix));
    ew.stub_drop(source::SETTLE_CRANK, MAT, 10, &holder.pubkey()).ok();
    ew.create_recipe(3, recipe(recipe_kind::REPAIR, &[(MAT, 2)], 100_000, t::TRANSFER_FEE)).ok();
    ew.warp(600);
    let ix = ew.repair_ix(&holder.pubkey(), 3, &[(MAT, 2)], &item, &item_mint);
    measure(&mut ew.hw.w, "craft repair (1 input)", &holder, ix);

    let ix = ew.open_profile_ix(&payer.pubkey(), &payer.pubkey());
    measure(&mut ew.hw.w, "social open_profile", &payer, ix);
    let sc = stub_pda(hookwars_common::economy::SOCIAL_CALLER_SEED);
    let ix = ew.record_wallet_ix(&sc, &STUB, &payer.pubkey(), counter::BOOK_FILLS, 1);
    measure(&mut ew.hw.w, "social record_wallet (via caller)", &payer, via_stub(hookwars_common::economy::SOCIAL_CALLER_SEED, ix));
}

#[test]
fn book_budgets() {
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(source::QUEST_CLAIM, MAT, 1, 1);
    let creator = ew.funded(SOL);
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, 1_000, 1);
    measure(&mut ew.hw.w, "book create_market", &creator, ix);
    let base = hookwars_craft::state::material_mint_address(MAT).0;
    let mut makers = Vec::new();
    for i in 0..u64::from(TEST_BOOK.slots) {
        let k = ew.funded(10 * SOL);
        ew.stub_drop(source::QUEST_CLAIM, MAT, 10, &k.pubkey()).ok();
        let ix = ew.place_ix(&k.pubkey(), &base, side::ASK, (5 + i) * 1_000, 10, false, 0, &[]);
        let label = if i == 0 { "book place ask resting (no fill)" } else { "book place ask resting (deeper)" };
        measure(&mut ew.hw.w, label, &k, ix);
        makers.push(k.pubkey());
    }
    let buyer = ew.funded(100 * SOL);
    ew.open_profile(&buyer);
    let n = makers.len();
    let ix = ew.place_ix(&buyer.pubkey(), &base, side::BID, 100_000, 10 * n as u64, false, 0, &makers);
    measure(&mut ew.hw.w, &format!("book place bid filling {n} makers (counter)"), &buyer, ix);
    // A full side and an eviction.
    let mut bidders = Vec::new();
    for i in 0..u64::from(TEST_BOOK.slots) {
        let k = ew.funded(10 * SOL);
        let ix = ew.place_ix(&k.pubkey(), &base, side::BID, (1 + i) * 1_000, 1, false, 0, &[]);
        ew.send(&k, &[ix]).ok();
        bidders.push(k.pubkey());
    }
    let late = ew.funded(10 * SOL);
    let ix = ew.place_ix(&late.pubkey(), &base, side::BID, 50_000, 1, false, 0, &[bidders[0]]);
    measure(&mut ew.hw.w, "book place bid evicting the worst", &late, ix);
    let id = ew.book(&base).bids[0].id;
    let ix = ew.cancel_ix(&late.pubkey(), &base, id);
    measure(&mut ew.hw.w, "book cancel bid", &late, ix);

    let bidder = ew.funded(10 * SOL);
    let class = ClassKey {
        template_id: t::TRANSFER_FEE,
        min_level: 0,
        param_min: [0; PARAM_FIELDS],
        param_max: [u32::MAX; PARAM_FIELDS],
    };
    let ix = ew.place_class_bid_ix(&bidder.pubkey(), 1, class, SOL, 0);
    measure(&mut ew.hw.w, "book place_class_bid", &bidder, ix);
    let (seller, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let ix = ew.match_class_ix(&seller.pubkey(), &bidder.pubkey(), 1, &item, &item_mint);
    measure(&mut ew.hw.w, "book match_class", &seller, ix);
}

#[test]
fn licence_budgets() {
    let mut ew = Ew::new();
    let (holder, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let ix = ew.set_offer_ix(&holder.pubkey(), &item, &item_mint, SOL, 86_400, per::PER_PERIOD, 2, false, true);
    measure(&mut ew.hw.w, "market set_licence_offer", &holder, ix);
    ew.open_profile(&holder);
    let funder = ew.funded(10 * SOL);
    let tok = ew.hw.w.mint_to_owner(&funder, 6, 1_000, "TOK");
    let payer = ew.funded(10 * SOL);
    let author = ew.template_author(&item);
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, SOL, false);
    measure(&mut ew.hw.w, "market buy_license (2 counters)", &payer, ix);
    let ix = ew.buy_license_ix(&payer.pubkey(), &item, &item_mint, &tok, &holder.pubkey(), &author, SOL, true);
    measure(&mut ew.hw.w, "market renew_license", &payer, ix);
    let ix = ew.revoke_license_ix(&holder.pubkey(), &item, &item_mint, &tok, &payer.pubkey());
    measure(&mut ew.hw.w, "market revoke_license", &holder, ix);
}

#[test]
fn directive_budgets() {
    let mut aw = Aw::new();
    let admin = aw.hw.admin.insecure_clone();
    let ix = agents_ix(
        hookwars_agents::accounts::InitMemoConfig {
            admin: admin.pubkey(),
            config: apda::config().0,
            memo_config: memo_config_address().0,
            system_program: anchor_lang::system_program::ID,
        },
        hookwars_agents::instruction::InitMemoConfig {
            params: MemoParams { memo_max_bytes: 600, postage_lamports: 5_000, min_proof: 1 },
        },
    );
    aw.hw.w.env.send_paid_by(&[ix], &admin, &[]).ok();
    let a = aw.agent("budget", kind::AUTHOR);
    let limits = PolicyLimits { per_action_lamports: SOL, per_day_lamports: SOL, tracked: vec![], targets: vec![bordrless_token::ID] };
    let ix = aw.init_policy_ix(&a, limits);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    let p = a.passport;
    let text = units_memo::directive_message(&p.to_string(), 0, "https://example.invalid/rules.md", &"ab".repeat(32)).encode();
    let memo = Instruction {
        program_id: MEMO_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(a.operator.pubkey(), true)],
        data: text.into_bytes(),
    };
    let set = agents_ix(
        hookwars_agents::accounts::SetDirective {
            operator: a.operator.pubkey(),
            config: apda::config().0,
            memo_config: memo_config_address().0,
            passport: p,
            policy: apda::policy(&p).0,
            directive: directive_address(&p, 0).0,
            previous: None,
            instructions: INSTRUCTIONS_SYSVAR_ID,
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::SetDirective {
            seq: 0,
            constraints: DirectiveConstraints {
                max_spend_per_action: SOL,
                max_spend_per_day: SOL,
                allowed_targets: vec![bordrless_token::ID],
                allowed_access_modes: 0b11111,
                max_licence_price: SOL,
                frozen: false,
            },
        },
    );
    let op = a.operator.insecure_clone();
    measure_with(&mut aw.hw.w, "agents memo + set_directive (seq 0)", &op, &[], vec![memo, set]);
}
