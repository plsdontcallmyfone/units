// Changed by Hookwars: new file, budgets of the expansion programs (10): keys, v0 bytes with and
// without one lookup table, trace entries, call height and compute of each market and social
// instruction a user or cranker sends.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_program_tests::armory::*;
use bordrless_program_tests::env::{compute_unit_limit, Tx};
use bordrless_program_tests::expansion::*;
use bordrless_program_tests::fixture::World;
use bordrless_token::client as token;
use hookwars_common::{template_id as t, EquipConfig};
use hookwars_market::state as ms;
use hookwars_social::{Criterion, GuildActionKind};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn measure(w: &mut World, label: &str, payer: &Keypair, signers: &[&Keypair], ix: Instruction) -> Tx {
    let ixs = [compute_unit_limit(1_400_000), ix];
    let mut keys: Vec<Pubkey> = Vec::new();
    for i in &ixs {
        for m in &i.accounts {
            if !keys.contains(&m.pubkey) {
                keys.push(m.pubkey);
            }
        }
    }
    let table = w.env.put_lookup_table(Pubkey::new_unique(), &keys);
    let with_table = w.env.v0_size(&ixs, payer, signers, &[table]);
    let tx = w.env.send_v0(&ixs, payer, signers, &[]);
    tx.ok();
    let (n_keys, size, trace, height, cu) = (tx.keys.len(), tx.size, tx.trace_len(), tx.max_height(), tx.cu());
    println!(
        "budget | {label} | keys {n_keys} | v0 bytes {size} | with table {with_table} | trace {trace} | height {height} | CU {cu}"
    );
    assert!(with_table <= 1_232 && trace <= 64 && height <= 5 && cu <= 1_400_000, "{label}");
    tx
}

#[test]
fn market_budgets() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let (author, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let a = author.pubkey();
    measure(&mut hw.w, "market list", &author, &[], list_ix(&a, &item, &item_mint, SOL, 0));
    let buyer = hw.w.env.funded(5 * SOL);
    let ix = buy_ix(&buyer.pubkey(), &a, &a, &treasury, &item, &item_mint, SOL);
    measure(&mut hw.w, "market buy", &buyer, &[], ix);

    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, test_slots());
    let (lessor, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let ix = offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 1_000, SOL, TEST_MARKET.lease_min_secs);
    measure(&mut hw.w, "market offer_lease", &lessor, &[], ix);
    let payer = hw.w.env.funded(5 * SOL);
    let ix = accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey());
    measure(&mut hw.w, "market accept_lease", &payer, &[], ix);
    hw.w.env.warp(i64::from(TEST_MARKET.lease_min_secs));
    let cranker = hw.w.env.funded(SOL);
    let mut ix = close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.push(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(mint, false));
    measure(&mut hw.w, "market end_lease", &cranker, &[], ix);

    let creator = hw.w.env.funded(10 * SOL);
    let ix = open_commission_ix(&creator.pubkey(), &mint, 0, 2, TEST_MARKET.commission_min_lamports, 3_600);
    measure(&mut hw.w, "market open_commission", &creator, &[], ix);
    let commission = ms::commission_address(&mint, 0).0;
    let (submitter, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[200, 0]), 100);
    let ix = submit_ix(&submitter.pubkey(), &commission, &mint, &item, &item_mint);
    measure(&mut hw.w, "market submit", &submitter, &[], ix);
    // Equip the submission (launch path), then pay.
    let cfg = EquipConfig {
        targets: vec![Pubkey::new_unique()],
        role: 0,
    };
    hw.equip_launch(&owner, &mint, Hw::entry(2, Some(item), cfg)).ok();
    hw.w.env.warp(3_600);
    let ix = pay_commission_ix(&commission, &mint, &item, &submitter.pubkey());
    measure(&mut hw.w, "market pay_commission", &cranker, &[], ix);
}

#[test]
fn social_budgets() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let admin = hw.w.env.deployer.insecure_clone();
    let ix = create_badge_ix(&admin.pubkey(), 0, "Smith", Criterion::ForgeLevel { min_level: 1 });
    measure(&mut hw.w, "social create_badge", &admin, &[], ix);
    hw.w.env.warp(i64::from(TEST_SOCIAL.admin_timelock_secs));
    let (holder, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let h = holder.pubkey();
    let extra = vec![
        AccountMeta::new_readonly(item, false),
        AccountMeta::new_readonly(token::holding_address(&item_mint, &h), false),
        AccountMeta::new(item_claim_marker(0, &item), false),
    ];
    measure(&mut hw.w, "social claim_badge (forge level)", &holder, &[], claim_badge_ix(&h, 0, &h, extra));

    let founder = hw.w.env.funded(10 * SOL);
    let f = founder.pubkey();
    measure(&mut hw.w, "social create_guild", &founder, &[], create_guild_ix(&f, 0, "Vanguard"));
    measure(&mut hw.w, "social deposit_sol", &founder, &[], deposit_sol_ix(&f, 0, 2 * SOL));
    let to = Keypair::new().pubkey();
    hw.w.env.fund(to, SOL);
    let spend = GuildActionKind::SpendSol { to, lamports: SOL };
    measure(&mut hw.w, "social propose_action", &founder, &[], propose_action_ix(&f, 0, 0, spend));
    hw.w.env.warp(i64::from(TEST_SOCIAL.guild_timelock_secs));
    let anyone = hw.w.env.funded(SOL);
    measure(&mut hw.w, "social execute_action (SOL)", &anyone, &[], execute_action_ix(0, 0, &to, vec![]));
}
