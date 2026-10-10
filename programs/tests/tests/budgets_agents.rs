// Changed by Hookwars: new file, budgets of the agents paths (09 section 12, 07 section 3).
//! Keys, v0 bytes (plain and with one lookup table of every account), trace entries, call height
//! and compute of each agents path, measured in LiteSVM and asserted against mainnet limits.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::agents::*;
use bordrless_program_tests::armory::{params, Hw};
use bordrless_program_tests::env::compute_unit_limit;
use bordrless_program_tests::slots::item_slot;
use bordrless_token::client as token;
use hookwars_agents::constants::{kind, pda as apda, record_kind};
use hookwars_agents::state::{PolicyLimits, TrackedLimit};
use hookwars_common::{ids, template_id, EquipConfig};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn measure(aw: &mut Aw, label: &str, payer: &Keypair, signers: &[&Keypair], ixs: Vec<Instruction>) {
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
    let table = aw.hw.w.env.put_lookup_table(Pubkey::new_unique(), &keys);
    let with_table = aw.hw.w.env.v0_size(&all, payer, signers, &[table]);
    let tx = aw.hw.w.env.send_v0(&all, payer, signers, &[]);
    tx.ok();
    let (n, size, trace, height, cu) = (tx.keys.len(), tx.size, tx.trace_len(), tx.max_height(), tx.cu());
    println!(
        "budget | {label} | keys {n} | v0 bytes {size} | with table {with_table} | trace {trace} | height {height} | CU {cu}"
    );
    assert!(size <= 1_232 || with_table <= 1_232, "{label}: {size} / {with_table} bytes");
    assert!(trace <= 64 && height <= 5 && cu <= 1_400_000, "{label}");
}

#[test]
fn agents_budgets() {
    let mut aw = Aw::new();
    // register_passport (two signers, the badge mint created).
    let operator = aw.hw.w.env.funded(10 * SOL);
    let key = aw.hw.w.env.funded(10 * SOL);
    let (ix, passport) = aw.register_ix(&operator.pubkey(), &key.pubkey(), &operator.pubkey(), profile("budget", kind::ALL));
    measure(&mut aw, "register_passport", &operator, &[&key], vec![ix]);
    let a = Agent {
        operator: operator.insecure_clone(),
        key: key.insecure_clone(),
        passport,
        badge: aw.passport(&passport).badge_mint,
    };
    // issue_badge (after the stand-in equip).
    aw.equip_badge_stub(&operator, &a.badge).ok();
    let ix = aw.issue_ix(&operator.pubkey(), &passport);
    measure(&mut aw, "issue_badge", &operator, &[], vec![ix]);
    // link_social with its ed25519 instruction.
    let stmt = Aw::statement(&a, 1, "budget_tg");
    let ixs = vec![ed25519_ix(&key, &stmt), aw.link_ix(&operator.pubkey(), &passport, 1, "budget_tg")];
    measure(&mut aw, "link_social (with the ed25519 instruction)", &operator, &[], ixs);
    // submit_attestation and an endorsement.
    let ix = aw.attest_ix(&a, aw.attest_args(&a, 86_400));
    measure(&mut aw, "submit_attestation", &key, &[], vec![ix]);
    let v = aw.verifiers[0].insecure_clone();
    let ix = aw.endorse_ix(&passport, &v.pubkey());
    measure(&mut aw, "endorse_attestation", &v, &[], vec![ix]);
    // record through a recorder (the caller's own height is 1 here; record is height 2).
    let ix = aw.record_ix(&passport, &key.pubkey(), record_kind::CRANK, 1_000);
    let payer = aw.hw.w.env.funded(SOL);
    measure(&mut aw, "record (from a recorder)", &payer, &[], vec![ix]);
    // spend: one token transfer from the vault.
    let vault = apda::vault(&passport).0;
    let funder = aw.hw.w.env.funded(10 * SOL);
    let mint = aw.hw.w.mint_to_owner(&funder, 6, 1_000_000, "BUD");
    let ixs = [
        token::create_holding(funder.pubkey(), mint, vault),
        token::transfer(funder.pubkey(), token::holding_address(&mint, &funder.pubkey()), token::holding_address(&mint, &vault), mint, None, vec![], 1_000),
    ];
    aw.hw.w.env.send_paid_by(&ixs, &funder, &[]).ok();
    let limits = PolicyLimits {
        per_action_lamports: SOL,
        per_day_lamports: SOL,
        tracked: vec![TrackedLimit { mint, per_action: 1_000, per_day: 1_000 }],
        targets: vec![bordrless_token::ID],
    };
    let ix = aw.init_policy_ix(&a, limits);
    aw.hw.w.env.send_paid_by(&[ix], &operator, &[]).ok();
    let to = aw.hw.w.env.funded(SOL).pubkey();
    aw.hw.w.env.send_paid_by(&[token::create_holding(funder.pubkey(), mint, to)], &funder, &[]).ok();
    let inner = token::transfer(vault, token::holding_address(&mint, &vault), token::holding_address(&mint, &to), mint, None, vec![], 100);
    let ix = aw.spend_ix(&a, inner);
    measure(&mut aw, "spend (one token transfer)", &key, &[], vec![ix]);
    // post_bond on two treaty proposals.
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.bond_min_passport_age_secs);
    let mut mints = Vec::new();
    for _ in 0..2 {
        let owner = aw.hw.w.env.funded(10 * SOL);
        let m = aw.hw.slot_mint(&owner, vec![item_slot(slot_kind::RELATION, equip_rule::VOTE, 0, 0, false)]);
        aw.hw.equip_launch(&owner, &m, Hw::entry(0, None, EquipConfig::default())).ok();
        // Security review 1 M-2: the proposer must hold the proposal threshold.
        aw.hw.mint_to(&owner, &m, &key.pubkey(), 1_000_000);
        mints.push(m);
    }
    let (_, item, _) = aw.hw.item(template_id::TREATY, params(&[100, 100, 0]), 0);
    let (tx, pa) = aw.hw.propose(&key, &mints[0], 0, Some(item), EquipConfig { targets: vec![mints[1]], role: 0 });
    tx.ok();
    let (tx, pb) = aw.hw.propose(&key, &mints[1], 0, Some(item), EquipConfig { targets: vec![mints[0]], role: 0 });
    tx.ok();
    let ix = agents_ix(
        hookwars_agents::accounts::PostBond {
            agent_key: key.pubkey(),
            config: apda::config().0,
            passport,
            proposal_a: pa,
            proposal_b: pb,
            treaty_item: item,
            bond: apda::bond(&passport, &pa).0,
            mark_a: apda::bond_mark(&pa).0,
            mark_b: apda::bond_mark(&pb).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::PostBond {},
    );
    measure(&mut aw, "post_bond", &key, &[], vec![ix]);
}
