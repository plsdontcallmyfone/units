// Changed by Hookwars: new file (M3b), what the items program's paths cost (07 section 3): a
// transfer through each token-side template, each pool callback, a composite, and settle_equip.
// Pool callbacks are measured through `launch_stub` (its forwarding adds one call level and its
// own compute); the launchpad's real forwarding is measured by the launchpad branch.

use anchor_lang::prelude::Pubkey;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::{params, Hw};
use bordrless_program_tests::env::Tx;
use bordrless_program_tests::items::*;
use bordrless_token::instructions::SlotInit;
use bordrless_token::state::SlotBounds;
use hookwars_common::{ids, pda, template_id as T};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn slot(kind: u8, max_cut_bps: u16, data_len: u8, touch: bool, burn: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: touch,
            may_burn: burn,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

fn row(label: &str, tx: &Tx) {
    tx.ok();
    println!(
        "budget | {label} | {} keys | {} bytes | {} trace | height {} | {} CU",
        tx.keys.len(),
        tx.size,
        tx.trace_len(),
        tx.max_height(),
        tx.cu()
    );
}

fn world() -> (Hw, Keypair, Pubkey, Keypair) {
    let mut hw = arsenal();
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(
        &owner,
        vec![
            slot(slot_kind::FEE, 5_000, 6, false, false),
            slot(slot_kind::POOL, 5_000, 24, true, true),
            slot(slot_kind::DEFENSE, 0, 0, false, false),
            slot(slot_kind::RELATION, 0, 0, false, false),
        ],
    );
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(&mut hw, &mint, &pool.pubkey(), now);
    (hw, owner, mint, pool)
}

#[test]
fn items_budgets() {
    // Token side: one transfer through each token-side template.
    for (label, template, p, slot_i, targets) in [
        ("transfer, Half-Life", T::HALF_LIFE, vec![200_000u32, 3_600, 4], 0u8, 0usize),
        ("transfer, Transfer Fee", T::TRANSFER_FEE, vec![100, 5_000], 0, 1),
        ("transfer, Max Transaction", T::MAX_TRANSACTION, vec![500], 2, 0),
        ("transfer, Dust Guard", T::DUST_GUARD, vec![1], 2, 0),
    ] {
        let (mut hw, owner, mint, _pool) = world();
        let it = hw.item(template, params(&p), 0).1;
        let tg: Vec<Pubkey> = (0..targets).map(|_| Pubkey::new_unique()).collect();
        equip(&mut hw, &owner, &mint, slot_i, it, tg, 0).ok();
        let a = hw.w.env.funded(SOL);
        let b = hw.w.env.funded(SOL);
        hw.mint_to(&owner, &mint, &a.pubkey(), 1_000_000);
        create_holding(&mut hw, &a, &mint, &b.pubkey());
        let tx = transfer(&mut hw, &a, &mint, &b.pubkey(), 10_000);
        row(label, &tx);
    }

    // Pool side: each pool template's quote-side callback.
    for (label, template, p) in [
        ("pool_before_swap buy, Side Skew", T::SIDE_SKEW, vec![100u32, 200]),
        ("pool_before_swap buy, Size Tiers", T::SIZE_TIERS, vec![1_000_000, 100_000_000, 50, 100, 200]),
        ("pool_before_swap buy, Launch Decay", T::LAUNCH_DECAY, vec![300, 100, 3_600]),
    ] {
        let (mut hw, owner, mint, pool) = world();
        let it = hw.item(template, params(&p), 0).1;
        equip(&mut hw, &owner, &mint, 1, it, vec![], 0).ok();
        let pk = pool.pubkey();
        let call = PoolCall {
            before: true,
            direction: 1,
            actor: Pubkey::new_unique(),
            recipient: Pubkey::new_unique(),
            amount_in: 5_000_000,
            side_amount: 5_000_000,
            route: plain_route(&mint, &pk, 5_000_000),
        };
        let (tx, _) = pool_call(&mut hw, &mint, &pk, 1, &call);
        row(label, &tx);
    }

    // Raid: before and after on a raid buy, then the delivery that stamps.
    let (mut hw, owner, mint, pool) = world();
    put_war_config(&mut hw, 1);
    let rival = Pubkey::new_unique();
    let rival_pool = Pubkey::new_unique();
    let now = hw.w.env.now;
    put_launch(&mut hw, &rival, &rival_pool, now);
    let raid = hw.item(T::RAID, params(&[2_000, 100, 3]), 0).1;
    equip(&mut hw, &owner, &mint, 1, raid, vec![rival], 0).ok();
    let payer = owner.insecure_clone();
    init_ledger(&mut hw, &payer, &mint).ok();
    let buyer = hw.w.env.funded(SOL);
    let pk = pool.pubkey();
    let mut call = PoolCall {
        before: true,
        direction: 1,
        actor: buyer.pubkey(),
        recipient: buyer.pubkey(),
        amount_in: 20_000_000,
        side_amount: 20_000_000,
        route: raid_route(&rival, &rival_pool, &mint, 20_000_000),
    };
    let (tx, _) = pool_call(&mut hw, &mint, &pk, 1, &call);
    row("pool_before_swap raid buy, Raid", &tx);
    call.before = false;
    let (tx, _) = pool_call(&mut hw, &mint, &pk, 1, &call);
    row("pool_after_swap raid buy (mark, inbound), Raid", &tx);
    hw.w.env.fund(pk, SOL);
    hw.mint_to(&owner, &mint, &pk, 1_000_000);
    create_holding(&mut hw, &buyer, &mint, &buyer.pubkey());
    let tx = transfer(&mut hw, &pool, &mint, &buyer.pubkey(), 100_000);
    row("delivery transfer stamping raid points, Raid", &tx);

    // A composite of three modules on both sides, then settle_equip.
    let (mut hw, owner, mint, pool) = world();
    let author = hw.w.env.funded(10 * SOL);
    let (tx, comp) = create_composite(
        &mut hw,
        &author,
        vec![
            module(T::SIDE_SKEW, params(&[100, 0]), 0, 0),
            module(T::SIZE_TIERS, params(&[1_000_000, 100_000_000, 50, 100, 200]), 0, 0),
            module(T::HALF_LIFE, params(&[200_000, 3_600, 4]), 0, 0),
        ],
        500,
    );
    row("create_composite, 3 modules", &tx);
    equip(&mut hw, &owner, &mint, 1, comp, vec![], 0).ok();
    let pk = pool.pubkey();
    let call = PoolCall {
        before: true,
        direction: 1,
        actor: Pubkey::new_unique(),
        recipient: Pubkey::new_unique(),
        amount_in: 5_000_000,
        side_amount: 5_000_000,
        route: plain_route(&mint, &pk, 5_000_000),
    };
    let (tx, _) = pool_call(&mut hw, &mint, &pk, 1, &call);
    row("pool_before_swap buy, composite of 3", &tx);
    let a = hw.w.env.funded(SOL);
    let b = hw.w.env.funded(SOL);
    hw.mint_to(&owner, &mint, &a.pubkey(), 1_000_000);
    create_holding(&mut hw, &a, &mint, &b.pubkey());
    let tx = transfer(&mut hw, &a, &mint, &b.pubkey(), 10_000);
    row("transfer, composite of 3 (Half-Life module cuts)", &tx);
    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&mint).0, 100_000);
    let sol = hw.w.sol;
    let chest = Pubkey::find_program_address(&[b"war-chest", mint.as_ref()], &ids::WAR_ID).0;
    create_holding(&mut hw, &funder, &sol, &chest);
    let ch = pda::holding(&sol, &chest);
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 1, &[(ids::ITEMS_ID, ch), (ids::ITEMS_ID, ch), (mint, ids::ITEMS_ID)]);
    let tx = hw.w.env.send_paid_by(&[ix], &cranker, &[]);
    row("settle_equip, composite of 3 (2 quote payouts, 1 burn)", &tx);
}
