// Changed by Hookwars: new file (explorer v2). Records real LiteSVM transactions as decoder vectors
// for the app's explorer (app/packages/sdk/src/hookwars/explore.ts). Nothing here is asserted about
// the programs beyond "the scenario ran as the suites already prove"; the point is the bytes.
//
// With EXPLORER_FIXTURES=<dir> set, each scenario writes <dir>/<name>.json: the message's keys
// (static keys only, no lookup tables), signer and writable flags from the compiled header, the
// top-level instructions, the inner instructions as the runtime recorded them, and the logs.
// Without it the scenarios run and write nothing.

use std::fmt::Write as _;

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::armory::{params, Hw};
use bordrless_program_tests::env::compute_unit_limit;
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::items::*;
use bordrless_program_tests::slot_launch::*;
use bordrless_program_tests::Tx;
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::SlotOp;
use bordrless_token::state::SlotBounds;
use hookwars_common::{ids, template_id as T};
use solana_keypair::Keypair;
use solana_message::v0;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;
const SUPPLY: u64 = 1_000_000_000;

fn slot(kind: u8, max_cut_bps: u16, data_len: u8, touch: bool, burn: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds { max_cut_bps, may_refuse: true, may_write_data: data_len > 0, may_answer_touch: touch, may_burn: burn },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// The slot table of tests/items.rs: 0 Fee, 1 Pool, 2 Defense, 3 Relation.
fn slots() -> Vec<SlotInit> {
    vec![
        slot(slot_kind::FEE, 5_000, 6, false, false),
        slot(slot_kind::POOL, 5_000, 24, true, true),
        slot(slot_kind::DEFENSE, 0, 0, false, false),
        slot(slot_kind::RELATION, 0, 0, false, false),
    ]
}

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

/// Sends `ixs` paid by `payer` (the env adds the 1.4M compute limit in front, as `send_paid_by`
/// does) and, when EXPLORER_FIXTURES is set, writes the transaction as the explorer reads it.
fn record(env_payer: &Keypair, send: impl FnOnce(&[Instruction]) -> Tx, ixs: &[Instruction], name: &str) -> Tx {
    let mut all = vec![compute_unit_limit(1_400_000)];
    all.extend_from_slice(ixs);
    let tx = send(ixs);
    let Ok(dir) = std::env::var("EXPLORER_FIXTURES") else { return tx };
    // The same compile the env does (no tables): key order does not depend on the blockhash.
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

fn transfer_ixs(hw: &Hw, from: &Keypair, mint: &Pubkey, to: &Pubkey, amount: u64) -> Vec<Instruction> {
    vec![
        token::create_holding(from.pubkey(), *mint, *to),
        token::transfer_with(from.pubkey(), token::holding_address(mint, &from.pubkey()), token::holding_address(mint, to), *mint, None, items_extras(hw, mint, SlotOp::Transfer), amount),
    ]
}

macro_rules! send_by {
    ($hw:expr, $payer:expr) => {
        |ixs: &[Instruction]| $hw.w.env.send_paid_by(ixs, $payer, &[])
    };
}

#[test]
fn explorer_fixtures_slot_items_settle_and_refusal() {
    let mut hw = arsenal();
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(&mut hw, &mint, &pool.pubkey(), now);

    // Half-Life in slot 0: a transfer pays a cut into the slot's equip vault.
    let hl = hw.item(T::HALF_LIFE, params(&[200_000, 3_600, 4]), 1_000).1;
    equip(&mut hw, &owner, &mint, 0, hl, vec![], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&owner, &mint, &alice.pubkey(), 1_000_000);
    let ixs = transfer_ixs(&hw, &alice, &mint, &bob.pubkey(), 100_000);
    record(&alice, send_by!(hw, &alice), &ixs, "transfer_half_life_cut").ok();

    // The settle: royalty, cranker bounty, the rest burned.
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &[(mint, ids::ITEMS_ID)]);
    record(&cranker, send_by!(hw, &cranker), &[ix], "settle_half_life").ok();

    // A refused transfer: Transfer Fee caps wallets at half the supply (a second mint).
    let mint2 = hw.slot_mint(&owner, slots());
    put_launch(&mut hw, &mint2, &Pubkey::new_unique(), now);
    let tf = hw.item(T::TRANSFER_FEE, params(&[100, 5_000]), 0).1;
    let collector = hw.w.env.funded(SOL);
    equip(&mut hw, &owner, &mint2, 0, tf, vec![collector.pubkey()], 0).ok();
    hw.mint_to(&owner, &mint2, &alice.pubkey(), SUPPLY);
    let ixs = transfer_ixs(&hw, &alice, &mint2, &bob.pubkey(), SUPPLY * 6 / 10);
    record(&alice, send_by!(hw, &alice), &ixs, "transfer_refused").expect_fail();
}

#[test]
fn explorer_fixtures_raid_mark() {
    let mut hw = arsenal();
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(&mut hw, &mint, &pool.pubkey(), now);
    put_war_config(&mut hw, 1);
    let rival = Pubkey::new_unique();
    let rival_pool = Pubkey::new_unique();
    put_launch(&mut hw, &rival, &rival_pool, now);
    let raid = hw.item(T::RAID, params(&[2_000, 100, 3]), 0).1;
    equip(&mut hw, &owner, &mint, 1, raid, vec![rival], 0).ok();
    let payer = owner.insecure_clone();
    init_ledger(&mut hw, &payer, &mint).ok();
    let buyer = hw.w.env.funded(SOL);
    let amount = 20 * 1_000_000u64;
    let p = pool.pubkey();
    let mut call = PoolCall { before: true, direction: 1, actor: Pubkey::new_unique(), recipient: buyer.pubkey(), amount_in: amount, side_amount: amount, route: raid_route(&rival, &rival_pool, &mint, amount) };
    let env_payer = hw.w.env.payer.insecure_clone();
    let ix = pool_call_ix(&hw, &mint, &p, 1, &call);
    record(&env_payer, |ixs: &[Instruction]| hw.w.env.send_paid_by(ixs, &env_payer, &[]), &[ix], "raid_before").ok();
    call.before = false;
    let ix = pool_call_ix(&hw, &mint, &p, 1, &call);
    record(&env_payer, |ixs: &[Instruction]| hw.w.env.send_paid_by(ixs, &env_payer, &[]), &[ix], "raid_mark").ok();
    // The delivery in the same slot stamps the points.
    hw.w.env.fund(pool.pubkey(), SOL);
    hw.mint_to(&owner, &mint, &pool.pubkey(), 1_000_000);
    let ixs = transfer_ixs(&hw, &pool, &mint, &buyer.pubkey(), 100_000);
    record(&pool, send_by!(hw, &pool), &ixs, "raid_delivery").ok();
}

#[test]
fn explorer_fixtures_slot_launch_pool_item_swaps() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(&creator, 100, LaunchRules::NONE, vec![pool_slot(500, true)], &[(0, BOTH)]);
    let item = items[0];
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, 10 * SOL).ok();
    w.set_stub(&creator, &item, ans(0, 1_000_000, 0), ans(0, 0, 0), false, false);
    let t = trader.pubkey();
    let ixs = vec![token::create_holding(t, mint, t), w.slot_swap_ix(&t, &t, &mint, 1, SOL / 10)];
    record(&trader, |ixs: &[Instruction]| w.env.send_paid_by(ixs, &trader, &[]), &ixs, "slot_buy_pool_item").ok();
    let held = w.env.holding(&mint, &t);
    // The same before-swap cut on a sell: refused (a pool item cuts a sell's quote output, after).
    let ixs = vec![w.slot_swap_ix(&t, &t, &mint, 0, held / 4)];
    record(&trader, |ixs: &[Instruction]| w.env.send_paid_by(ixs, &trader, &[]), &ixs, "slot_sell_wrong_side").expect_fail();
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 500_000, 0), false, false);
    let ixs = vec![w.slot_swap_ix(&t, &t, &mint, 0, held / 2)];
    record(&trader, |ixs: &[Instruction]| w.env.send_paid_by(ixs, &trader, &[]), &ixs, "slot_sell_pool_item").ok();
}
