// Changed by Hookwars: new file (app pass v3). More decoder vectors for the app's explorer: a
// siege, a market sale, a craft through the armory, an order book fill and an operator directive
// (memo plus `set_directive`), recorded from real LiteSVM transactions in the scenarios the suites
// already prove (tests/siege.rs, tests/market.rs, tests/integ3.rs, tests/book.rs,
// tests/directives.rs). The record format is tests/explorer_fixtures.rs's.
//
// With EXPLORER_FIXTURES=<dir> set, each scenario writes <dir>/<name>.json. Without it the
// scenarios run and write nothing.

use std::fmt::Write as _;

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::ToAccountMetas;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::agents::*;
use bordrless_program_tests::armory::{params, token_accounts, Hw};
use bordrless_program_tests::economy::*;
use bordrless_program_tests::env::compute_unit_limit;
use bordrless_program_tests::expansion::{buy_ix, list_ix, load};
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_program_tests::Tx;
use bordrless_token::client as token;
use hookwars_agents::constants::{kind, pda as apda, INSTRUCTIONS_SYSVAR_ID};
use hookwars_agents::handlers::directive::{directive_address, memo_config_address, DirectiveConstraints, MemoParams};
use hookwars_agents::state::PolicyLimits;
use hookwars_armory::cpi::{ARMORY_SIGNER, MINTER};
use hookwars_book::state::side;
use hookwars_common::economy::MEMO_PROGRAM_ID;
use hookwars_common::{eco_cpi, ids, pda, template_id as t};
use hookwars_craft::state::{self as cs, material_mint_address, recipe_kind, source};
use solana_keypair::Keypair;
use solana_message::v0;
use solana_signer::Signer;

fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            c if (c as u32) < 0x20 => { let _ = write!(o, "\\u{:04x}", c as u32); }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// tests/explorer_fixtures.rs `record`: sends `ixs` (the env puts the 1.4M compute limit in front)
/// and, when EXPLORER_FIXTURES is set, writes the transaction as the explorer reads it.
fn record(env_payer: &Keypair, send: impl FnOnce(&[Instruction]) -> Tx, ixs: &[Instruction], name: &str) -> Tx {
    let mut all = vec![compute_unit_limit(1_400_000)];
    all.extend_from_slice(ixs);
    let tx = send(ixs);
    let Ok(dir) = std::env::var("EXPLORER_FIXTURES") else { return tx };
    let msg = v0::Message::try_compile(&env_payer.pubkey(), &all, &[], Default::default()).expect("compile");
    assert_eq!(msg.account_keys, tx.keys, "{name}: the recorded keys are the message's");
    let h = &msg.header;
    let (nrs, nros, nrou) = (usize::from(h.num_required_signatures), usize::from(h.num_readonly_signed_accounts), usize::from(h.num_readonly_unsigned_accounts));
    let n = msg.account_keys.len();
    let signer = |i: usize| i < nrs;
    let writable = |i: usize| if i < nrs { i < nrs - nros } else { i < n - nrou };
    let (ok, logs, inner, cu) = match &tx.result {
        Ok(m) => (true, &m.logs, &m.inner_instructions, m.compute_units_consumed),
        Err(f) => (false, &f.meta.logs, &f.meta.inner_instructions, f.meta.compute_units_consumed),
    };
    let mut j = String::new();
    let _ = write!(j, "{{\"name\":{},\"ok\":{ok},\"computeUnits\":{cu},", esc(name));
    let _ = write!(j, "\"accountKeys\":[{}],", msg.account_keys.iter().map(|k| esc(&k.to_string())).collect::<Vec<_>>().join(","));
    let _ = write!(j, "\"signer\":[{}],", (0..n).map(|i| signer(i).to_string()).collect::<Vec<_>>().join(","));
    let _ = write!(j, "\"writable\":[{}],", (0..n).map(|i| writable(i).to_string()).collect::<Vec<_>>().join(","));
    let ix_json = |p: u8, accts: &[u8], data: &[u8], height: Option<u8>| {
        let a = accts.iter().map(u8::to_string).collect::<Vec<_>>().join(",");
        match height {
            Some(hh) => format!("{{\"programIdIndex\":{p},\"accounts\":[{a}],\"dataHex\":\"{}\",\"stackHeight\":{hh}}}", hex(data)),
            None => format!("{{\"programIdIndex\":{p},\"accounts\":[{a}],\"dataHex\":\"{}\"}}", hex(data)),
        }
    };
    let _ = write!(j, "\"instructions\":[{}],", msg.instructions.iter().map(|c| ix_json(c.program_id_index, &c.accounts, &c.data, None)).collect::<Vec<_>>().join(","));
    let groups: Vec<String> = inner
        .iter()
        .enumerate()
        .filter(|(_, g)| !g.is_empty())
        .map(|(i, g)| {
            let list = g.iter().map(|x| ix_json(x.instruction.program_id_index, &x.instruction.accounts, &x.instruction.data, Some(x.stack_height))).collect::<Vec<_>>().join(",");
            format!("{{\"index\":{i},\"instructions\":[{list}]}}")
        })
        .collect();
    let _ = write!(j, "\"inner\":[{}],", groups.join(","));
    let _ = write!(j, "\"logs\":[{}]}}", logs.iter().map(|l| esc(l)).collect::<Vec<_>>().join(","));
    std::fs::create_dir_all(&dir).expect("fixture dir");
    std::fs::write(format!("{dir}/{name}.json"), j).expect("write fixture");
    tx
}

#[test]
fn explorer_fixtures_war_siege() {
    // tests/siege.rs `a_siege_buys_the_rival_and_keeps_it`.
    let mut ww = WarWorld::new();
    let tk = ww.war_token("ATK", OrdersSpec::default());
    let rival = ww.launch("RIV", LaunchRules::NONE);
    ww.buyer(&rival, SOL);
    ww.w.env.warp(120);
    let now = ww.now();
    let volume = u64::from(OrdersSpec::default().siege_threshold) * TEST_PARAMS.siege_unit_lamports;
    ww.put_ledger(&tk.mint, 0, volume, &[(rival, now, volume, 0)]);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    ww.fund_chest(&tk.mint, 10 * SOL);
    let cranker = ww.w.env.funded(10 * SOL);
    let ix = ww.siege_ix(&cranker.pubkey(), &tk, &rival, false, None);
    record(&cranker, |ixs: &[Instruction]| ww.w.env.send_paid_by(ixs, &cranker, &[]), &[ix], "war_siege").ok();
}

#[test]
fn explorer_fixtures_market_list_and_buy() {
    // tests/market.rs `a_sale_pays_fee_resale_and_seller_exactly_and_moves_the_item`.
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let (author, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let seller = hw.w.env.funded(10 * SOL);
    hw.give_item(&author, &item_mint, &seller.pubkey());
    let price = 10 * SOL;
    let ix = list_ix(&seller.pubkey(), &item, &item_mint, price, 0);
    record(&seller, |ixs: &[Instruction]| hw.w.env.send_paid_by(ixs, &seller, &[]), &[ix], "market_list").ok();
    let buyer = hw.w.env.funded(20 * SOL);
    let ix = buy_ix(&buyer.pubkey(), &seller.pubkey(), &author.pubkey(), &treasury, &item, &item_mint, price);
    record(&buyer, |ixs: &[Instruction]| hw.w.env.send_paid_by(ixs, &buyer, &[]), &[ix], "market_buy").ok();
}

#[test]
fn explorer_fixtures_craft_through_the_armory() {
    // tests/integ3.rs `craft_mints_through_the_armory_and_only_its_signer_may`.
    const MAT: u16 = 1;
    let mut ew = Ew::wired();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(eco_cpi::drop_source::SETTLE_CRANK, MAT, 1, 1_000);
    let c = ew.funded(10 * SOL);
    ew.stub_drop(eco_cpi::drop_source::SETTLE_CRANK, MAT, 50_000, &c.pubkey()).ok();
    let inputs = [(MAT, 10)];
    let mut r = recipe(recipe_kind::CRAFT, &inputs, 1_000, t::TRANSFER_FEE);
    r.param_min = params(&[100, 500]);
    r.param_max = params(&[300, 1_500]);
    ew.create_recipe(1, r).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    let n = ew.hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let accounts = hookwars_armory::accounts::MintCrafted {
        craft_signer: cs::signer_address().0,
        payer: c.pubkey(),
        owner: c.pubkey(),
        config: pda::config().0,
        template: pda::template(t::TRANSFER_FEE).0,
        minter: MINTER,
        item_mint,
        item,
        recipient_holding: token::holding_address(&item_mint, &c.pubkey()),
        armory_signer: ARMORY_SIGNER,
        items_program: ids::ITEMS_ID,
        token: token_accounts(),
        system_program: anchor_lang::system_program::ID,
        event_authority: bordrless_program_tests::armory::armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    let mut ix = ew.craft_item_ix(&c.pubkey(), 1, &inputs);
    ix.accounts.extend(accounts.into_iter().skip(1));
    record(&c, |ixs: &[Instruction]| ew.hw.w.env.send_paid_by(ixs, &c, &[]), &[ix], "craft_item").ok();
}

#[test]
fn explorer_fixtures_book_fill() {
    // tests/book.rs `price_time_priority_partial_fills_and_exact_fees`.
    const MAT: u16 = 1;
    const TICK: u64 = 1_000;
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(source::QUEST_CLAIM, MAT, 1, 1);
    let creator = ew.funded(SOL);
    let ix = ew.create_market_ix(&creator.pubkey(), MAT, TICK, 1);
    ew.send(&creator, &[ix]).ok();
    let base = material_mint_address(MAT).0;
    let maker = ew.funded(100 * SOL);
    ew.stub_drop(source::QUEST_CLAIM, MAT, 100, &maker.pubkey()).ok();
    let ix = ew.place_ix(&maker.pubkey(), &base, side::ASK, 4 * TICK, 10, false, 0, &[]);
    record(&maker, |ixs: &[Instruction]| ew.hw.w.env.send_paid_by(ixs, &maker, &[]), &[ix], "book_place_ask").ok();
    let buyer = ew.funded(100 * SOL);
    ew.open_profile(&buyer);
    let ix = ew.place_ix(&buyer.pubkey(), &base, side::BID, 5 * TICK, 6, false, 0, &[maker.pubkey()]);
    record(&buyer, |ixs: &[Instruction]| ew.hw.w.env.send_paid_by(ixs, &buyer, &[]), &[ix], "book_fill").ok();
}

#[test]
fn explorer_fixtures_directive_memo() {
    // tests/directives.rs `a_directive_binds_its_memo_hash_and_writes_the_policy`.
    let mut aw = Aw::new();
    let admin = aw.hw.admin.insecure_clone();
    let ix = agents_ix(
        hookwars_agents::accounts::InitMemoConfig {
            admin: admin.pubkey(),
            config: apda::config().0,
            memo_config: memo_config_address().0,
            system_program: anchor_lang::system_program::ID,
        },
        hookwars_agents::instruction::InitMemoConfig { params: MemoParams { memo_max_bytes: 600, postage_lamports: 5_000, min_proof: 1 } },
    );
    aw.hw.w.env.send_paid_by(&[ix], &admin, &[]).ok();
    let a = aw.agent("director", kind::AUTHOR);
    let limits = PolicyLimits { per_action_lamports: SOL, per_day_lamports: SOL, tracked: vec![], targets: vec![bordrless_token::ID] };
    let ix = aw.init_policy_ix(&a, limits);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    let p = a.passport;
    let op = a.operator.insecure_clone();
    let dc = DirectiveConstraints {
        max_spend_per_action: 1_000,
        max_spend_per_day: 10_000,
        allowed_targets: vec![bordrless_token::ID],
        allowed_access_modes: 0b1,
        max_licence_price: SOL,
        frozen: false,
    };
    let ch = hookwars_agents::handlers::directive::constraints_hash_hex(&dc).unwrap();
    let text = units_memo::directive_message(&p.to_string(), 0, "https://example.invalid/rules.md", "00ff", &ch).encode();
    let memo = Instruction { program_id: MEMO_PROGRAM_ID, accounts: vec![AccountMeta::new_readonly(op.pubkey(), true)], data: text.as_bytes().to_vec() };
    let set = agents_ix(
        hookwars_agents::accounts::SetDirective {
            operator: op.pubkey(),
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
            constraints: dc,
        },
    );
    record(&op, |ixs: &[Instruction]| aw.hw.w.env.send_paid_by(ixs, &op, &[]), &[memo, set], "memo_directive").ok();
}

