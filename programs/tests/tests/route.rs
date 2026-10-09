// Changed by Hookwars: new file, M3a route tests.
//! Hookwars M3a: `swap_route` and the DEX-filled route context (spec 03 sections 3.2, 3.3).

use anchor_lang::prelude::{AnchorSerialize, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{
    equip_rule as rule, pool_flags, slot_flags as f, slot_kind as kind, Delta, RouteContext,
    SlotReturn,
};
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::hooks::NewPool;
use bordrless_program_tests::slots::*;
use bordrless_swap::client as swap;
use bordrless_swap::constants::MAX_ROUTE_HOPS;
use bordrless_swap::error::SwapError;
use bordrless_swap::events::{RouteSwapped, Swapped};
use bordrless_swap::state::Pool;
use bordrless_token::client as token;
use bordrless_token::slots::SlotOp;
use hook_tester::client as tester;
use hook_tester::{callback, Script};
use slot_tester::{callback as cb, mode};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SUPPLY: u64 = 1_000_000_000_000_000;
const DEPOSIT: u64 = 1_000_000_000_000;
const AMOUNT: u64 = 5_000_000_000;
const TESTER_FLAGS: u16 = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;

fn code(e: SwapError) -> u32 {
    u32::from(e)
}

/// Four plain mints A, B, C, D and the pools A/B and B/C (each with hook_tester as pool hook, so
/// it records what it is told) and C/D (no hook); `lp` holds every supply and trades.
struct Routes {
    w: World,
    lp: Keypair,
    m: [Pubkey; 4],
    ab: Pubkey,
    bc: Pubkey,
    cd: Pubkey,
}

impl Routes {
    fn new() -> Self {
        let mut w = World::with_slots();
        let lp = w.env.funded(1_000_000_000_000);
        let m = ["AAA", "BBB", "CCC", "DDD"].map(|s| w.mint_to_owner(&lp, 6, SUPPLY, s));
        let mut pool = |w: &mut World, base: Pubkey, quote: Pubkey, hooked: bool| {
            let (p, tx) = w.new_pool(
                &lp,
                &NewPool {
                    base,
                    quote,
                    lp_fee_bps: 30,
                    tester_flags: hooked.then_some(TESTER_FLAGS),
                    extras: vec![],
                    base_amount: DEPOSIT,
                    quote_amount: DEPOSIT,
                },
            );
            tx.ok();
            p
        };
        let ab = pool(&mut w, m[0], m[1], true);
        let bc = pool(&mut w, m[1], m[2], true);
        let cd = pool(&mut w, m[2], m[3], false);
        Self { w, lp, m, ab, bc, cd }
    }

    /// The hop on `pool` in `direction`, the trader's holdings both sides, slices resolved as a
    /// client does (a slot mint's from its slot table).
    fn hop(&self, pool: Pubkey, direction: u8) -> swap::RouteHop {
        hop_of(&self.w, &self.lp.pubkey(), pool, direction)
    }

    fn route(&self, hops: Vec<swap::RouteHop>, min_out: u64, hook_data: Vec<u8>) -> Instruction {
        swap::swap_route(self.lp.pubkey(), AMOUNT, min_out, hops, hook_data)
    }

    fn send(&mut self, ix: Instruction) -> bordrless_program_tests::env::Tx {
        let lp = self.lp.insecure_clone();
        self.w.env.send_paid_by(&[ix], &lp, &[])
    }

    fn script(&self, pool: &Pubkey) -> Script {
        self.w.env.read(&tester::script_address(pool))
    }

    fn bal(&self, mint: &Pubkey) -> u64 {
        self.w.env.holding(mint, &self.lp.pubkey())
    }
}

fn hop_of(w: &World, trader: &Pubkey, pool: Pubkey, direction: u8) -> swap::RouteHop {
    let p: Pool = w.env.read(&pool);
    let (in_mint, out_mint) = if direction == 1 {
        (p.quote_mint, p.base_mint)
    } else {
        (p.base_mint, p.quote_mint)
    };
    let slice = |mint: &Pubkey, src: &Pubkey, dst: &Pubkey, auth: &Pubkey, so: &Pubkey, d: &Pubkey| {
        let mut s = w.env.token_hook_slice(mint, src, dst, auth, so, d);
        let minted: bordrless_token::state::Mint = w.env.read(mint);
        if minted.uses_slots() {
            s = w.slot_extras(mint, SlotOp::Transfer);
        }
        s
    };
    let trader_in = token::holding_address(&in_mint, trader);
    let trader_out = token::holding_address(&out_mint, trader);
    let in_slice = slice(&in_mint, &trader_in, &swap::vault_address(&pool, &in_mint), trader, trader, &pool);
    let out_slice = slice(&out_mint, &swap::vault_address(&pool, &out_mint), &trader_out, &pool, &pool, trader);
    let pool_extras = match p.hook_program {
        Some(hook) => w
            .env
            .pool_hook_extras(&hook, &pool, &p.base_mint, &p.quote_mint, trader),
        None => vec![],
    };
    swap::RouteHop {
        keys: swap::SwapKeys {
            trader: *trader,
            pool,
            base_mint: p.base_mint,
            quote_mint: p.quote_mint,
            trader_base: token::holding_address(&p.base_mint, trader),
            trader_quote: token::holding_address(&p.quote_mint, trader),
            hook_program: p.hook_program,
            base_mint_writable: false,
            quote_mint_writable: false,
        },
        direction,
        in_slice,
        out_slice,
        pool_extras,
    }
}

#[test]
fn a_two_hop_route_feeds_each_hop_what_the_last_delivered() {
    let mut r = Routes::new();
    let c0 = r.bal(&r.m[2]);
    let b0 = r.bal(&r.m[1]);
    // A -> B (sell A on A/B), B -> C (sell B on B/C).
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.bc, 0)], 0, vec![]);
    let tx = r.send(ix);
    tx.ok();
    let hops: Vec<Swapped> = tx.events();
    assert_eq!(hops.len(), 2);
    assert_eq!(hops[0].amount_in, AMOUNT);
    assert_eq!(hops[1].amount_in, hops[0].delivered_out);
    let done: RouteSwapped = tx.event();
    assert_eq!(
        (done.route_input_mint, done.route_output_mint, done.amount_in, done.amount_out, done.pools),
        (r.m[0], r.m[2], AMOUNT, hops[1].delivered_out, vec![r.ab, r.bc])
    );
    assert_eq!(r.bal(&r.m[2]) - c0, done.amount_out);
    // B passed through: the trader's B holding ends where it started.
    assert_eq!(r.bal(&r.m[1]), b0);
    for (i, ev) in hops.iter().enumerate() {
        assert_eq!(
            ev.route,
            RouteContext {
                route_input_mint: r.m[0],
                route_output_mint: r.m[2],
                first_pool: r.ab,
                route_amount_in: AMOUNT,
                hop_index: i as u8,
                hop_count: 2,
            }
        );
    }
}

#[test]
fn every_hop_s_pool_hook_is_told_the_route() {
    let mut r = Routes::new();
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.bc, 0), r.hop(r.cd, 0)], 0, vec![]);
    r.send(ix).ok();
    for (pool, index) in [(r.ab, 0u8), (r.bc, 1u8)] {
        for which in [callback::BEFORE_SWAP, callback::AFTER_SWAP] {
            let told = r.script(&pool).told_pool(which).expect("told");
            assert_eq!(
                told.route,
                RouteContext {
                    route_input_mint: r.m[0],
                    route_output_mint: r.m[3],
                    first_pool: r.ab,
                    route_amount_in: AMOUNT,
                    hop_index: index,
                    hop_count: 3,
                }
            );
        }
    }
}

#[test]
fn a_plain_swap_is_a_one_hop_route_and_hook_data_cannot_forge_one() {
    let mut r = Routes::new();
    let forged = RouteContext {
        route_input_mint: r.m[3],
        route_output_mint: r.m[1],
        first_pool: r.cd,
        route_amount_in: 1,
        hop_index: 1,
        hop_count: 2,
    };
    let mut data = Vec::new();
    forged.serialize(&mut data).unwrap();
    let hop = r.hop(r.ab, 1);
    let ix = swap::swap(
        &hop.keys,
        bordrless_swap::instructions::SwapArgs {
            direction: 1,
            amount_in: AMOUNT,
            min_amount_out: 0,
            in_hook_accounts: 0,
            out_hook_accounts: 0,
            hook_data: data.clone(),
        },
        hop.pool_extras,
    );
    let tx = r.send(ix);
    tx.ok();
    let told = r.script(&r.ab).told_pool(callback::BEFORE_SWAP).unwrap();
    // B in, A out on A/B.
    assert_eq!(told.route, RouteContext::single(r.m[1], r.m[0], r.ab, AMOUNT));
    assert_eq!(told.hook_data, data);
    let ev: Swapped = tx.event();
    assert_eq!(ev.route, told.route);
}

#[test]
fn the_build_maximum_of_hops_runs_and_one_more_is_refused() {
    let mut r = Routes::new();
    assert_eq!(MAX_ROUTE_HOPS, 3);
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.bc, 0), r.hop(r.cd, 0)], 0, vec![]);
    r.send(ix).ok();
    let ix = r.route(
        vec![r.hop(r.ab, 0), r.hop(r.bc, 0), r.hop(r.cd, 0), r.hop(r.cd, 1)],
        0,
        vec![],
    );
    r.send(ix).expect_code(code(SwapError::RouteTooLong));
    let ix = r.route(vec![], 0, vec![]);
    r.send(ix).expect_code(code(SwapError::EmptyRoute));
}

#[test]
fn broken_and_repeating_routes_are_refused() {
    let mut r = Routes::new();
    // A -> B, then C -> D: the second hop's input is not the first's output.
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.cd, 0)], 0, vec![]);
    r.send(ix).expect_code(code(SwapError::RouteBroken));
    // A -> B -> A on the same pool.
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.ab, 1)], 0, vec![]);
    r.send(ix).expect_code(code(SwapError::RoutePoolRepeated));
    // The output delivered to someone else's holding cannot feed the next hop.
    let mut first = r.hop(r.ab, 0);
    let stranger = Pubkey::new_unique();
    let lp = r.lp.insecure_clone();
    r.w.holdings(&lp, r.m[1], &[stranger]);
    first.keys.trader_quote = token::holding_address(&r.m[1], &stranger);
    let ix = r.route(vec![first, r.hop(r.bc, 0)], 0, vec![]);
    r.send(ix).expect_code(code(SwapError::RouteBroken));
}

#[test]
fn only_the_last_hop_checks_the_minimum() {
    // Two identical markets: the first measures what the route delivers.
    let mut probe = Routes::new();
    let ix = probe.route(vec![probe.hop(probe.ab, 0), probe.hop(probe.bc, 0)], 0, vec![]);
    let out = probe.send(ix).event::<RouteSwapped>().amount_out;
    let mut r = Routes::new();
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.bc, 0)], out + 1, vec![]);
    r.send(ix).expect_code(code(SwapError::Slippage));
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(r.bc, 0)], out, vec![]);
    r.send(ix).ok();
}

#[test]
fn a_slot_mint_on_a_hop_runs_its_items() {
    let mut r = Routes::new();
    // S: a slot mint whose one Fee item cuts 1 unit from every transfer into its equip vault.
    let lp = r.lp.insecure_clone();
    let s_kp = Keypair::new();
    let s = s_kp.pubkey();
    r.w.create_slot_mint(&lp, &s_kp, vec![item_slot(kind::FEE, rule::VOTE, 100, 0, false)], true)
        .ok();
    let flags = f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA;
    let item = Pubkey::new_unique();
    r.w.init_item(&lp, &item);
    let vault = r.w.make_equip_vault(&lp, &s, 0);
    r.w.equip(&lp, &s, 0, &item, flags).ok();
    let answer = answer_bytes(&SlotReturn {
        deltas: vec![Delta { amount: 1, account: 6 }],
        ..SlotReturn::default()
    });
    r.w.item_answer(&lp, &item, cb::BEFORE_TRANSFER, mode::RETURN, answer);
    let ix = r.w.slot_mint_ix(&lp.pubkey(), &s, &lp.pubkey(), SUPPLY);
    r.w.env.send_paid_by(&[ix], &lp, &[]).ok();
    // Pool S/B (base S): its deposit pays the item one unit.
    let base_slice = r.w.slot_extras(&s, SlotOp::Transfer);
    let args = bordrless_swap::instructions::CreatePoolArgs {
        lp_fee_bps: 30,
        hook_program: Pubkey::default(),
        hook_flags: 0,
        virtual_base: 0,
        virtual_quote: 0,
        base_amount: DEPOSIT,
        quote_amount: DEPOSIT,
        base_hook_accounts: base_slice.len() as u8,
        quote_hook_accounts: 0,
        hook_data: vec![],
    };
    let keys = swap::CreatePoolKeys {
        payer: lp.pubkey(),
        authority: lp.pubkey(),
        treasury: r.w.env.treasury.pubkey(),
        base_mint: s,
        quote_mint: r.m[1],
        hook_caller: None,
    };
    r.w.env
        .send_paid_by(&[swap::create_pool(&keys, args, base_slice)], &lp, &[])
        .ok();
    let sb = swap::pool_address(&s, &r.m[1], 30, None);
    let vault_before = r.w.env.read::<bordrless_token::state::Holding>(&vault).amount;
    let s_before = r.bal(&s);
    // A -> B on A/B, then B -> S (buy S on S/B): the delivery of S runs S's item.
    let ix = r.route(vec![r.hop(r.ab, 0), r.hop(sb, 1)], 0, vec![]);
    let tx = r.send(ix);
    tx.ok();
    let done: RouteSwapped = tx.event();
    assert_eq!(done.route_output_mint, s);
    assert_eq!(r.bal(&s) - s_before, done.amount_out);
    let vault_after = r.w.env.read::<bordrless_token::state::Holding>(&vault).amount;
    assert_eq!(vault_after - vault_before, 1);
}
