// Changed by Hookwars: new file, M1 measurements (docs/spec/07-budgets-tests.md section 3).
//! What slots cost: a wallet transfer, a DEX buy and a DEX sell of a slot mint with 0 to 3
//! cutting item slots (each item answering one cut, the worst case), without and with a lookup
//! table holding every account the message may load from one. Each line printed is one
//! measurement; each asserts mainnet's limits (1,232 bytes, 64 trace entries, height 5,
//! 1,400,000 compute units). Also the size and rent of a mint with the slot table.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{equip_rule as rule, slot_flags as f, slot_kind as kind, Delta, SlotReturn};
use bordrless_program_tests::env::{compute_unit_limit, Tx};
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::slots::*;
use bordrless_swap::client as swap;
use bordrless_swap::instructions::{CreatePoolArgs, SwapArgs};
use bordrless_token::client as token;
use bordrless_token::slots::SlotOp;
use bordrless_token::state::Mint;
use slot_tester::{callback as cb, mode};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;
const SUPPLY: u64 = 1_000_000_000_000;

/// A slot mint with `n` Fee slots (100 bps each), each with an item answering a 1-unit cut, or
/// (n = 0) a plain mint; `owner` holds `SUPPLY`.
fn mint_with(w: &mut World, owner: &Keypair, n: usize) -> Pubkey {
    if n == 0 {
        return w.mint_to_owner(owner, 6, SUPPLY, "PLN");
    }
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let slots = (0..n)
        .map(|_| item_slot(kind::FEE, rule::VOTE, 100, 0, false))
        .collect();
    w.create_slot_mint(owner, &mint_kp, slots, true).ok();
    let flags = f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA;
    let answer = answer_bytes(&SlotReturn {
        deltas: vec![Delta {
            amount: 1,
            account: 6,
        }],
        ..SlotReturn::default()
    });
    for i in 0..n as u8 {
        let item = Pubkey::new_unique();
        w.init_item(owner, &item);
        w.make_equip_vault(owner, &mint, i);
        w.equip(owner, &mint, i, &item, flags).ok();
        w.item_answer(owner, &item, cb::BEFORE_TRANSFER, mode::RETURN, answer.clone());
    }
    let ix = w.slot_mint_ix(&owner.pubkey(), &mint, &owner.pubkey(), SUPPLY);
    w.env.send_paid_by(&[ix], owner, &[]).ok();
    mint
}

fn extras(w: &World, mint: &Pubkey, n: usize) -> Vec<AccountMeta> {
    if n == 0 {
        vec![]
    } else {
        w.slot_extras(mint, SlotOp::Transfer)
    }
}

/// Sends `ix` twice under the same state (the second time with a lookup table of every account
/// it names), records both and asserts mainnet's limits. Answers the first.
fn measure(w: &mut World, label: &str, payer: &Keypair, ix: Instruction) -> Tx {
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
    let with_table = w.env.v0_size(&ixs, payer, &[], &[table]);
    let tx = w.env.send_v0(&ixs, payer, &[], &[]);
    tx.ok();
    let (n_keys, size, trace, height, cu) = (
        tx.keys.len(),
        tx.size,
        tx.trace_len(),
        tx.max_height(),
        tx.cu(),
    );
    println!(
        "budget | {label} | keys {n_keys} | v0 bytes {size} | with table {with_table} | trace {trace} | height {height} | CU {cu}"
    );
    assert!(with_table <= 1_232, "{label}: {with_table} bytes with a table");
    assert!(trace <= 64 && height <= 5 && cu <= 1_400_000, "{label}");
    tx
}

#[test]
fn transfers_buys_and_sells_with_zero_to_three_cutting_slots() {
    for n in 0..=3usize {
        let mut w = World::with_slots();
        let owner = w.env.funded(100 * SOL);
        let base = mint_with(&mut w, &owner, n);
        let quote = w.mint_to_owner(&owner, 9, SUPPLY, "QTE");
        let bob = Pubkey::new_unique();
        w.holdings(&owner, base, &[bob]);

        // Wallet transfer.
        let ix = token::transfer_with(
            owner.pubkey(),
            token::holding_address(&base, &owner.pubkey()),
            token::holding_address(&base, &bob),
            base,
            None,
            extras(&w, &base, n),
            1_000_000,
        );
        let tx = measure(&mut w, &format!("transfer, {n} cutting slots"), &owner, ix);
        if n > 0 {
            let ev: bordrless_token::events::Transferred = tx.event();
            assert_eq!(ev.slot_cuts.len(), n);
        }

        // An ordinary pool (no pool hook) of the slot mint against a plain quote.
        let slice = extras(&w, &base, n);
        let args = CreatePoolArgs {
            lp_fee_bps: 30,
            hook_program: Pubkey::default(),
            hook_flags: 0,
            virtual_base: 0,
            virtual_quote: 0,
            base_amount: 1_000_000_000,
            quote_amount: 1_000_000_000,
            base_hook_accounts: slice.len() as u8,
            quote_hook_accounts: 0,
            hook_data: vec![],
        };
        let keys = swap::CreatePoolKeys {
            payer: owner.pubkey(),
            authority: owner.pubkey(),
            treasury: w.env.treasury.pubkey(),
            base_mint: base,
            quote_mint: quote,
            hook_caller: None,
        };
        w.env
            .send_paid_by(&[swap::create_pool(&keys, args, slice)], &owner, &[])
            .ok();
        let pool = swap::pool_address(&base, &quote, 30, None);
        let swap_ix = |w: &World, direction: u8, amount_in: u64| {
            let slice = extras(w, &base, n);
            let (in_n, out_n) = if direction == 1 {
                (0, slice.len() as u8)
            } else {
                (slice.len() as u8, 0)
            };
            swap::swap(
                &swap::SwapKeys {
                    trader: owner.pubkey(),
                    pool,
                    base_mint: base,
                    quote_mint: quote,
                    trader_base: token::holding_address(&base, &owner.pubkey()),
                    trader_quote: token::holding_address(&quote, &owner.pubkey()),
                    hook_program: None,
                    base_mint_writable: false,
                    quote_mint_writable: false,
                },
                SwapArgs {
                    direction,
                    amount_in,
                    min_amount_out: 0,
                    in_hook_accounts: in_n,
                    out_hook_accounts: out_n,
                    hook_data: vec![],
                },
                slice,
            )
        };
        let ix = swap_ix(&w, 1, 10_000_000);
        measure(&mut w, &format!("buy, {n} cutting slots"), &owner, ix);
        let ix = swap_ix(&w, 0, 10_000_000);
        measure(&mut w, &format!("sell, {n} cutting slots"), &owner, ix);
    }
}

#[test]
fn the_slot_table_size_and_rent() {
    let w = World::with_slots();
    let len = Mint::LEN;
    let upstream = 519;
    println!(
        "budget | mint size | {len} bytes (upstream {upstream}) | rent {} lamports (upstream {})",
        w.env.rent(len),
        w.env.rent(upstream)
    );
    assert_eq!(len, upstream + 33 + 1 + 113 * bordrless_token::constants::MAX_SLOTS);
}
