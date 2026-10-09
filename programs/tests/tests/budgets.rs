// Changed by Hookwars: new file, M1 measurements (docs/spec/07-budgets-tests.md section 3); M2:
// the armory's execute (an equip by vote) and forge; M3a route and observation ring measurements; R21 slot size.
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
    // Hookwars R21: a `Slot` is 114 bytes since `SlotBounds.may_burn` (was 113 at M1).
    assert_eq!(len, upstream + 33 + 1 + 114 * bordrless_token::constants::MAX_SLOTS);
}

/// M2: `execute` of a passed vote that moves a slot from one item to another (close_equip,
/// init_equip with its registry, set_slot_item), and `forge` of two items.
#[test]
fn armory_execute_and_forge() {
    use bordrless_program_tests::armory::*;
    use hookwars_common::{template_id as t, EquipConfig};
    for (label, template, a_params, b_params, slot, cfg) in [
        (
            "execute equip, War orders (no vault, no royalty holding)",
            t::WAR_ORDERS,
            params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]),
            params(&[20, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]),
            0u8,
            EquipConfig::default(),
        ),
        (
            "execute equip, Transfer Fee (equip vault, royalty holding)",
            t::TRANSFER_FEE,
            params(&[100, 0]),
            params(&[200, 0]),
            2u8,
            EquipConfig {
                targets: vec![Pubkey::new_unique()],
                role: 0,
            },
        ),
    ] {
        let mut hw = Hw::new();
        let owner = hw.w.env.funded(100 * SOL);
        let mint = hw.slot_mint(&owner, test_slots());
        let (_, a, _) = hw.item(template, a_params, 100);
        hw.equip_launch(&owner, &mint, Hw::entry(slot, Some(a), cfg.clone())).ok();
        let o = owner.pubkey();
        hw.mint_to(&owner, &mint, &o, 1_000);
        let (_, b, _) = hw.item(template, b_params, 100);
        let (tx, proposal) = hw.propose(&owner, &mint, slot, Some(b), cfg);
        tx.ok();
        hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
        hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
        hw.finalize(&proposal).ok();
        hw.w.env.warp(600);
        let ix = hw.execute_ix(&o, &proposal);
        measure(&mut hw.w, label, &owner, ix);
    }
    let mut hw = Hw::new();
    let (forger, a, _) = hw.item(t::RAID, params(&[100, 100, 10]), 100);
    let (other, b, b_mint) = hw.item(t::RAID, params(&[300, 50, 20]), 200);
    hw.give_item(&other, &b_mint, &forger.pubkey());
    let (ix, _) = hw.forge_ix(&forger.pubkey(), &a, &b);
    measure(&mut hw.w, "forge, two Raid items", &forger, ix);

/// Hookwars M3a: what a multi-hop route costs (`swap_route`, spec 03 section 3.3): 2 and 3 hops of
/// plain pools, and 2 hops whose last delivery is a slot mint with 3 cutting items. Also the
/// observation ring's size and rent.
#[test]
fn route_budgets() {
    use bordrless_swap::constants::{MAX_ROUTE_HOPS, OBS_RING_LEN};
    let mut w = World::with_slots();
    let owner = w.env.funded(100 * SOL);
    let me = owner.pubkey();
    let plain: Vec<Pubkey> = (0..4).map(|i| w.mint_to_owner(&owner, 6, SUPPLY, &format!("M{i}"))).collect();
    let slot = mint_with(&mut w, &owner, 3);
    let pool_of = |w: &mut World, base: Pubkey, quote: Pubkey| {
        let slice = if base == slot { extras(w, &base, 3) } else { vec![] };
        let args = CreatePoolArgs {
            lp_fee_bps: 30,
            hook_program: Pubkey::default(),
            hook_flags: 0,
            virtual_base: 0,
            virtual_quote: 0,
            base_amount: 100_000_000_000,
            quote_amount: 100_000_000_000,
            base_hook_accounts: slice.len() as u8,
            quote_hook_accounts: 0,
            hook_data: vec![],
        };
        let keys = swap::CreatePoolKeys {
            payer: me,
            authority: me,
            treasury: w.env.treasury.pubkey(),
            base_mint: base,
            quote_mint: quote,
            hook_caller: None,
        };
        w.env
            .send_paid_by(&[swap::create_pool(&keys, args, slice)], &owner, &[])
            .ok();
        swap::pool_address(&base, &quote, 30, None)
    };
    let p01 = pool_of(&mut w, plain[0], plain[1]);
    let p12 = pool_of(&mut w, plain[1], plain[2]);
    let p23 = pool_of(&mut w, plain[2], plain[3]);
    let ps1 = pool_of(&mut w, slot, plain[1]);
    let hop = |w: &World, pool: Pubkey, base: Pubkey, quote: Pubkey, direction: u8| {
        let out_is_slot = direction == 1 && base == slot;
        swap::RouteHop {
            keys: swap::SwapKeys {
                trader: me,
                pool,
                base_mint: base,
                quote_mint: quote,
                trader_base: token::holding_address(&base, &me),
                trader_quote: token::holding_address(&quote, &me),
                hook_program: None,
                base_mint_writable: false,
                quote_mint_writable: false,
            },
            direction,
            in_slice: vec![],
            out_slice: if out_is_slot { extras(w, &base, 3) } else { vec![] },
            pool_extras: vec![],
        }
    };
    let routes: Vec<(&str, Vec<swap::RouteHop>)> = vec![
        ("route, 2 hops, plain pools", vec![hop(&w, p01, plain[0], plain[1], 0), hop(&w, p12, plain[1], plain[2], 0)]),
        (
            "route, 3 hops, plain pools",
            vec![
                hop(&w, p01, plain[0], plain[1], 0),
                hop(&w, p12, plain[1], plain[2], 0),
                hop(&w, p23, plain[2], plain[3], 0),
            ],
        ),
        (
            "route, 2 hops, delivering a 3-cutting-slot mint",
            vec![hop(&w, p01, plain[0], plain[1], 0), hop(&w, ps1, slot, plain[1], 1)],
        ),
    ];
    assert_eq!(MAX_ROUTE_HOPS, 3);
    for (label, hops) in routes {
        let ix = swap::swap_route(me, 1_000_000_000, 0, hops, vec![]);
        measure(&mut w, label, &owner, ix);
    }
    let len = bordrless_core::observations::account_len(OBS_RING_LEN);
    println!(
        "budget | observation ring | {OBS_RING_LEN} entries | {len} bytes | rent {} lamports | per entry {} bytes",
        w.env.rent(len),
        bordrless_core::observations::ENTRY_LEN
    );
}
