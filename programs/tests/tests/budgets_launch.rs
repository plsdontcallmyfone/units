// Changed by Hookwars: new file, M3b measurements of launch-pool paths with slot items (spec 07
// section 3): buy and sell with the kit and 0 to 3 pool items, token-side items on a launch pool,
// buy and graduate with the most slots, a two-hop route into a launch pool (the Raid case),
// prepare_launch and create_prepared_launch.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{equip_rule as rule, slot_flags as f, slot_kind as kind, Delta, SlotReturn};
use bordrless_launch::client as launch;
use bordrless_launch::client::slots as sl;
use bordrless_launch::state::{Launch, LaunchRules};
use bordrless_program_tests::env::{compute_unit_limit, Tx};
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::launch::{presets, VQ};
use bordrless_program_tests::slot_launch::*;
use bordrless_program_tests::slots::*;
use bordrless_swap::client as swap;
use bordrless_swap::instructions::CreatePoolArgs;
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::SlotOp;
use slot_tester::{callback as cb, mode};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

/// Sends `ixs` (after a compute limit), asserting mainnet's limits, and prints the measurement:
/// keys, v0 bytes with no table, with one table holding every account, trace entries, height, CU.
fn measure(w: &mut World, label: &str, payer: &Keypair, signers: &[&Keypair], ixs: Vec<Instruction>) -> Tx {
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

/// A `slot_tester` token item that cuts 1 unit of every transfer into its slot's equip vault.
fn token_item(w: &mut World, owner: &Keypair, mint: &Pubkey, slot: u8) {
    let flags = f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA;
    let answer = answer_bytes(&SlotReturn {
        deltas: vec![Delta {
            amount: 1,
            account: 6,
        }],
        ..SlotReturn::default()
    });
    let item = Pubkey::new_unique();
    w.init_item(owner, &item);
    w.make_equip_vault(owner, mint, slot);
    w.equip(owner, mint, slot, &item, flags).ok();
    w.item_answer(owner, &item, cb::BEFORE_TRANSFER, mode::RETURN, answer);
}

/// A slot launch with the kit (holder rewards and max wallet) and `token_items` token-side
/// cutting items then `pool_items` pool items (each cutting 1 unit of a buy's input and of a
/// sell's output).
fn launch_with(w: &mut World, creator: &Keypair, token_items: usize, pool_items: usize) -> Pubkey {
    launch_with_rules(w, creator, token_items, pool_items, presets::rewards_and_cap())
}

/// As [`launch_with`], with `rules`.
fn launch_with_rules(
    w: &mut World,
    creator: &Keypair,
    token_items: usize,
    pool_items: usize,
    rules: LaunchRules,
) -> Pubkey {
    let mut slots: Vec<SlotInit> = Vec::new();
    for _ in 0..token_items {
        slots.push(item_slot(kind::FEE, rule::VOTE, 100, 0, false));
    }
    for _ in 0..pool_items {
        slots.push(pool_slot(300, false));
    }
    if slots.is_empty() {
        slots.push(pool_slot(300, false));
    }
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    w.prepare_launch(creator, &mint_kp, 100, rules, slots).ok();
    // Slot 0 is the kit's.
    for i in 0..pool_items {
        let slot = 1 + (token_items + i) as u8;
        let item = w.stub_item(creator);
        w.equip_stub(creator, &mint, slot, &item, BOTH).ok();
        w.set_stub(creator, &item, ans(0, 1, 0), ans(0, 1, 0), false, false);
    }
    let ix = w.create_prepared_ix(&creator.pubkey(), &mint, 100, VQ, rules);
    w.env
        .send_paid_by(&[ix], creator, &[&mint_kp])
        .ok();
    // The test token item cuts every transfer, the launch's deposit into the pool included; a
    // real template exempts the launch and its pool (04, Half-Life's exempt accounts). So the test
    // token items are equipped after the launch.
    for i in 0..token_items {
        token_item(w, creator, &mint, 1 + i as u8);
    }
    mint
}

/// Makes the stubs of a launch answer nothing on a buy's after and a sell's before (only quote
/// sides may cut).
fn quote_sides_only(w: &mut World, creator: &Keypair, mint: &Pubkey) {
    let m: bordrless_token::state::Mint = w.env.read(mint);
    for s in m.active_slots() {
        if s.program == pool_item_stub::ID {
            w.set_stub(creator, &s.item, ans(0, 1, 0), ans(0, 0, 0), false, false);
        }
    }
}

#[test]
fn launch_pool_buys_and_sells_with_the_kit_and_slot_items() {
    // The kit is slot 0; MAX_SLOTS - 1 slots remain for items.
    let rest = bordrless_token::constants::MAX_SLOTS - 1;
    for (token_items, pool_items) in [(0, 0), (0, 1), (0, 2), (0, rest), (1, 0), (2, 0), (rest, 0), (1, 2), (2, 1)] {
        let mut w = World::with_slot_launches();
        let creator = w.env.funded(1_000 * SOL);
        let mint = launch_with(&mut w, &creator, token_items, pool_items);
        w.env.warp(60);
        let trader = w.env.funded(100 * SOL);
        w.wrap_sol(&trader, SOL).ok();
        let t = trader.pubkey();
        quote_sides_only(&mut w, &creator, &mint);
        let ixs = vec![
            token::create_holding(t, mint, t),
            w.slot_swap_ix(&t, &t, &mint, 1, SOL / 10),
        ];
        measure(
            &mut w,
            &format!("launch buy, kit + {token_items} token items + {pool_items} pool items"),
            &trader,
            &[],
            ixs,
        );
        // Sells cut the quote output in `after`.
        let m: bordrless_token::state::Mint = w.env.read(&mint);
        for s in m.active_slots() {
            if s.program == pool_item_stub::ID {
                w.set_stub(&creator, &s.item, ans(0, 0, 0), ans(0, 1, 0), false, false);
            }
        }
        let held = w.env.holding(&mint, &t);
        let ix = w.slot_swap_ix(&t, &t, &mint, 0, held / 2);
        measure(
            &mut w,
            &format!("launch sell, kit + {token_items} token items + {pool_items} pool items"),
            &trader,
            &[],
            vec![ix],
        );
    }
}

#[test]
fn buy_and_graduate_with_the_most_slots() {
    let rest = bordrless_token::constants::MAX_SLOTS - 1;
    let r = presets::rewards_and_cap();
    for (token_items, pool_items) in [(0, rest), (rest, 0), (1, rest - 1)] {
        let mut w = World::with_slot_launches();
        let creator = w.env.funded(1_000 * SOL);
        let mint = launch_with_rules(&mut w, &creator, token_items, pool_items, r);
        // Items that take nothing keep the launch's quote math for the fill.
        let m: bordrless_token::state::Mint = w.env.read(&mint);
        for s in m.active_slots() {
            if s.program == pool_item_stub::ID {
                w.set_stub(&creator, &s.item, ans(0, 0, 0), ans(0, 0, 0), false, false);
            }
        }
        w.env.warp(60);
        // Upstream's fill (`World::fill_curve`) through slot swaps: fresh wallets buy within max
        // wallet until the pool needs at most 0.1 SOL more, then the last buy crosses the
        // threshold with `graduate` in the same transaction, as the site sends it.
        let leave = SOL / 10;
        let mut n = 0;
        loop {
            let l: Launch = w.env.read(&launch::launch_address(&mint));
            let p = w.launch_pool(&mint);
            let missing = l.graduation_quote.saturating_sub(p.quote_reserve);
            if missing <= leave {
                break;
            }
            n += 1;
            assert!(n < 300, "the curve does not fill");
            let wallet = w.env.funded(1_000 * SOL);
            let holder = wallet.pubkey();
            let mut amount = w.max_buy_for(&mint, &holder);
            let target = missing - leave;
            if w.launch_quote(&mint, &holder, true, amount).to_reserve_in(true) > target {
                let (mut lo, mut hi) = (0u64, amount);
                while hi - lo > 1 {
                    let mid = lo + (hi - lo) / 2;
                    let q = w.launch_quote(&mint, &holder, true, mid);
                    if q.failure.is_none() && q.to_reserve_in(true) <= target {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                amount = lo;
            }
            // Nothing a buy could add without crossing: the crossing buy below does the rest.
            if amount < 1_000 {
                break;
            }
            w.wrap_sol(&wallet, amount).ok();
            w.slot_buy(&wallet, &mint, amount).ok();
        }
        // The crossing buy: max wallet may need more than one more buyer.
        let last = loop {
            let l: Launch = w.env.read(&launch::launch_address(&mint));
            let p = w.launch_pool(&mint);
            let missing = l.graduation_quote.saturating_sub(p.quote_reserve);
            let wallet = w.env.funded(1_000 * SOL);
            let mut amount = w.crossing_buy_amount(&mint, &wallet.pubkey());
            if amount == 0 {
                // Token items' cuts leave the curve a few base units short of what the helper
                // models; the smallest buy that reaches the threshold, searched upward.
                amount = missing.max(1);
                while amount < SOL {
                    let q = w.launch_quote(&mint, &wallet.pubkey(), true, amount);
                    if q.failure.is_none() && q.to_reserve_in(true) >= missing {
                        break;
                    }
                    amount *= 2;
                }
            }
            let q = w.launch_quote(&mint, &wallet.pubkey(), true, amount);
            n += 1;
            assert!(n < 300, "the curve does not fill");
            if q.failure.is_some() {
                // The curve is dust (graduate.rs, `curve_is_dust`): graduation without a buy.
                break (wallet, 0);
            }
            w.wrap_sol(&wallet, amount).ok();
            if q.to_reserve_in(true) >= missing || p.base_reserve == 0 {
                break (wallet, amount);
            }
            w.slot_buy(&wallet, &mint, amount).ok();
        };
        let (wallet, amount) = last;
        let t = wallet.pubkey();
        let (label, ixs) = if amount > 0 {
            (
                "launch buy and graduate",
                vec![
                    token::create_holding(t, mint, t),
                    w.slot_swap_ix(&t, &t, &mint, 1, amount),
                    w.slot_graduate_ix(&t, &mint),
                ],
            )
        } else {
            ("launch graduate (the curve left dust, no buy can cross)", vec![w.slot_graduate_ix(&t, &mint)])
        };
        measure(
            &mut w,
            &format!("{label}, kit + {token_items} token items + {pool_items} pool items"),
            &wallet,
            &[],
            ixs,
        );
        let l: Launch = w.env.read(&launch::launch_address(&mint));
        assert_eq!(l.status, bordrless_launch::constants::STATUS_GRADUATED);
    }
}

#[test]
fn a_two_hop_route_into_a_launch_pool() {
    let rest = bordrless_token::constants::MAX_SLOTS - 1;
    for pool_items in [0usize, 1, rest] {
        let mut w = World::with_slot_launches();
        let creator = w.env.funded(1_000 * SOL);
        let mint = launch_with(&mut w, &creator, 0, pool_items);
        quote_sides_only(&mut w, &creator, &mint);
        w.env.warp(60);
        // A plain pool X / bridged SOL.
        let owner = w.env.funded(1_000 * SOL);
        let me = owner.pubkey();
        let x = w.mint_to_owner(&owner, 6, 1_000_000_000_000, "X");
        w.wrap_sol(&owner, 100 * SOL).ok();
        let args = CreatePoolArgs {
            lp_fee_bps: 30,
            hook_program: Pubkey::default(),
            hook_flags: 0,
            virtual_base: 0,
            virtual_quote: 0,
            base_amount: 100_000_000_000,
            quote_amount: 50 * SOL,
            base_hook_accounts: 0,
            quote_hook_accounts: 0,
            hook_data: vec![],
        };
        let keys = swap::CreatePoolKeys {
            payer: me,
            authority: me,
            treasury: w.env.treasury.pubkey(),
            base_mint: x,
            quote_mint: w.sol,
            hook_caller: None,
        };
        w.env
            .send_paid_by(&[swap::create_pool(&keys, args, vec![])], &owner, &[])
            .ok();
        let xp = swap::pool_address(&x, &w.sol, 30, None);
        let mut lk = w.launch_keys(&mint);
        lk.burns = true;
        let sol = w.sol;
        let hops = vec![
            swap::RouteHop {
                keys: swap::SwapKeys {
                    trader: me,
                    pool: xp,
                    base_mint: x,
                    quote_mint: sol,
                    trader_base: token::holding_address(&x, &me),
                    trader_quote: token::holding_address(&sol, &me),
                    hook_program: None,
                    base_mint_writable: false,
                    quote_mint_writable: false,
                },
                direction: 0,
                in_slice: vec![],
                out_slice: vec![],
                pool_extras: vec![],
            },
            swap::RouteHop {
                keys: swap::SwapKeys {
                    trader: me,
                    pool: lk.pool(),
                    base_mint: mint,
                    quote_mint: sol,
                    trader_base: token::holding_address(&mint, &me),
                    trader_quote: token::holding_address(&sol, &me),
                    hook_program: Some(bordrless_launch::ID),
                    base_mint_writable: true,
                    quote_mint_writable: false,
                },
                direction: 1,
                in_slice: vec![],
                out_slice: slices_of(&w, &mint, SlotOp::Transfer),
                pool_extras: sl::pool_extras(&mint, &sol, pool_items_of(&w, &mint)),
            },
        ];
        let ixs = vec![
            token::create_holding(me, mint, me),
            swap::swap_route(me, 1_000_000_000, 0, hops, vec![]),
        ];
        measure(
            &mut w,
            &format!("route, 2 hops into a launch pool, kit + {pool_items} pool items"),
            &owner,
            &[],
            ixs,
        );
        if pool_items > 0 {
            let m: bordrless_token::state::Mint = w.env.read(&mint);
            let s = m.active_slots()[1];
            let script = w.stub_script(&s.item);
            // The launch pool's items see the route: it started in X.
            assert_eq!(script.last_route_input, x);
        }
    }
}

#[test]
fn prepare_and_create_prepared_launch() {
    let rest = bordrless_token::constants::MAX_SLOTS - 1;
    for (rules, label) in [(LaunchRules::NONE, "no kit"), (presets::rewards_and_cap(), "kit")] {
        let mut w = World::with_slot_launches();
        let creator = w.env.funded(1_000 * SOL);
        let n = if rules.modules() == 0 { rest + 1 } else { rest };
        let slots: Vec<SlotInit> = (0..n).map(|_| pool_slot(300, false)).collect();
        let mint_kp = Keypair::new();
        let mint = mint_kp.pubkey();
        let ix = sl::prepare_launch(
            creator.pubkey(),
            mint,
            World::prepare_args("SLOT", 100, rules, slots),
        );
        measure(
            &mut w,
            &format!("prepare_launch, {label}, {n} item slots"),
            &creator,
            &[&mint_kp],
            vec![ix],
        );
        let first = if rules.modules() == 0 { 0 } else { 1 };
        for i in 0..n {
            let item = w.stub_item(&creator);
            w.equip_stub(&creator, &mint, first + i as u8, &item, BOTH).ok();
        }
        let ix = w.create_prepared_ix(&creator.pubkey(), &mint, 100, VQ, rules);
        measure(
            &mut w,
            &format!("create_prepared_launch, {label}, {n} pool items"),
            &creator,
            &[&mint_kp],
            vec![ix],
        );
    }
    let _ = AccountMeta::new_readonly(Pubkey::default(), false);
}
