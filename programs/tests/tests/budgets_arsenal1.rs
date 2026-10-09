// Changed by Hookwars: new file (arsenal waves B and C): one budget row per template, on the path
// it runs (a buy delivery or wallet send for token-side templates, a pool callback for pool
// templates), measured with every account in one lookup table (07 section 3).

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_program_tests::armory::Hw;
use bordrless_program_tests::arsenal1::*;
use bordrless_program_tests::env::{compute_unit_limit, Tx};
use bordrless_program_tests::items::{equip, items_extras, pool_call_ix};
use bordrless_token::client as token;
use bordrless_token::slots::SlotOp;
use hookwars_common::template_id as T;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn measure(hw: &mut Hw, label: &str, payer: &Keypair, ix: Instruction) -> Tx {
    let ixs = [compute_unit_limit(1_400_000), ix];
    let mut keys: Vec<Pubkey> = Vec::new();
    for i in &ixs {
        for m in &i.accounts {
            if !keys.contains(&m.pubkey) {
                keys.push(m.pubkey);
            }
        }
    }
    let table = hw.w.env.put_lookup_table(Pubkey::new_unique(), &keys);
    let with_table = hw.w.env.v0_size(&ixs, payer, &[], &[table]);
    let tx = hw.w.env.send_v0(&ixs, payer, &[], &[]);
    tx.ok();
    let (n_keys, size, trace, height, cu) = (tx.keys.len(), tx.size, tx.trace_len(), tx.max_height(), tx.cu());
    println!(
        "budget | {label} | keys {n_keys} | v0 bytes {size} | with table {with_table} | trace {trace} | height {height} | CU {cu}"
    );
    assert!(with_table <= 1_232, "{label}: {with_table} bytes with a table");
    assert!(trace <= 64 && height <= 5 && cu <= 1_400_000, "{label}");
    tx
}

fn transfer_ix(hw: &Hw, authority: &Pubkey, mint: &Pubkey, from: &Pubkey, to: &Pubkey, amount: u64) -> Instruction {
    token::transfer_with(
        *authority,
        token::holding_address(mint, from),
        token::holding_address(mint, to),
        *mint,
        None,
        items_extras(hw, mint, SlotOp::Transfer),
        amount,
    )
}

#[test]
fn token_side_templates_on_a_buy_delivery() {
    for (id, slot, p) in [
        (T::COOLDOWN, 1u8, vec![600u32]),
        (T::FLASH_GUARD, 1, vec![2]),
        (T::DAILY_SELL_CAP, 1, vec![2_500]),
        (T::STREAK, 2, vec![30]),
        (T::GUILD_TAG, 2, vec![1]),
    ] {
        let mut hw = world(&[id]);
        let t = tok(&mut hw);
        let it = item(&mut hw, id, &p, 0);
        equip(&mut hw, &t.owner, &t.mint, slot, it, vec![], 0).ok();
        let pool = t.pool.pubkey();
        hw.mint_to(&t.owner, &t.mint, &pool, 1_000_000);
        let alice = hw.w.env.funded(SOL);
        hw.w.env
            .send_paid_by(&[token::create_holding(alice.pubkey(), t.mint, alice.pubkey())], &alice, &[])
            .ok();
        let ix = transfer_ix(&hw, &pool, &t.mint, &pool, &alice.pubkey(), 10_000);
        let payer = t.pool.insecure_clone();
        measure(&mut hw, &format!("template {id}, buy delivery"), &payer, ix);
    }
}

#[test]
fn rank_badge_on_a_buy_delivery_reading_the_ring() {
    let mut hw = world(&[T::RANK_BADGE]);
    let (owner, mint) = tok_market(&mut hw);
    let rb = item(&mut hw, T::RANK_BADGE, &[100, 10, 5], 0);
    equip(&mut hw, &owner, &mint, 2, rb, vec![], 0).ok();
    let now = hw.w.env.now;
    let pool = put_market(&mut hw, &mint, 0, 2 * Q64, now - 600, &flat(now - 600));
    hw.mint_to(&owner, &mint, &pool, 1_000_000);
    let d = delegate(&mut hw, &mint, &pool);
    let alice = hw.w.env.funded(SOL);
    hw.w.env
        .send_paid_by(&[token::create_holding(alice.pubkey(), mint, alice.pubkey())], &alice, &[])
        .ok();
    let ix = transfer_ix(&hw, &d.pubkey(), &mint, &pool, &alice.pubkey(), 1_000);
    measure(&mut hw, "template 35, buy delivery", &d, ix);
}

#[test]
fn pool_templates_on_a_buy_input_callback() {
    for (id, p) in [
        (T::VELOCITY_FEE, vec![300u32, 2, 50, 300]),
        (T::IMPACT_FEE, vec![100, 300]),
        (T::VOLATILITY_FEE, vec![300, 1_200, 500, 100]),
        (T::RUSH_HOUR, vec![22, 4, 200, 20]),
        (T::DUMP_BRAKE, vec![300, 1_200, 1_000, 200]),
    ] {
        let mut hw = world(&[id]);
        let (owner, mint) = tok_market(&mut hw);
        let it = item(&mut hw, id, &p, 0);
        equip(&mut hw, &owner, &mint, 0, it, vec![], 0).ok();
        let now = hw.w.env.now;
        let pool = put_market(&mut hw, &mint, 10, Q64 * 6 / 5, now - 600, &two_prices(now - 2_000, now - 600, Q64, 4));
        // Dump Brake answers on a sell's output; the others on a buy's input.
        let call = if id == T::DUMP_BRAKE { sell(1_000_000, &mint, &pool, false) } else { buy(1_000_000, &mint, &pool, true) };
        let ix = pool_call_ix(&hw, &mint, &pool, 0, &call);
        let payer = hw.w.env.payer.insecure_clone();
        measure(&mut hw, &format!("template {id}, pool callback (through launch_stub)"), &payer, ix);
    }
}
