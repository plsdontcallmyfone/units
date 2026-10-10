// Changed by Hookwars: new file; pass 4b: the expansion paths (boss claim, five-member coalition form, contribute, joint siege, five-member dissolve, rivalry open and settle).
//! Transaction budgets of the war paths (07 section 3, `budgets.rs::war_*`, kept in its own file):
//! keys, v0 bytes with a lookup table holding every account that may be loaded from one, trace
//! entries, call height and compute units, each asserted under mainnet's limits (64 trace entries,
//! height 5, 1,232 bytes, 1,400,000 units). The figures printed are what 05 and 07 record.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_program_tests::{compute_unit_limit, Tx};
use bordrless_token::client as token;
use hookwars_war::client as war;
use hookwars_war::constants::quest;
use hookwars_war::state::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

/// Sends `ix` in a v0 transaction with a 1.4M unit limit and a lookup table of every key that is
/// neither a signer nor the invoked program; prints and checks the budget.
fn measure(ww: &mut WarWorld, name: &str, ix: Instruction, payer: &Keypair, signers: &[&Keypair]) -> Tx {
    let mut addresses: Vec<Pubkey> = Vec::new();
    for m in &ix.accounts {
        let signs = m.is_signer || signers.iter().any(|s| s.pubkey() == m.pubkey) || m.pubkey == payer.pubkey();
        if !signs && m.pubkey != ix.program_id && !addresses.contains(&m.pubkey) {
            addresses.push(m.pubkey);
        }
    }
    let table = ww.w.env.put_lookup_table(Pubkey::new_unique(), &addresses);
    let ixs = [compute_unit_limit(1_400_000), ix];
    let tx = ww.w.env.send_v0(&ixs, payer, signers, &[table]);
    tx.ok();
    let (height, trace, cu) = (tx.max_height(), tx.trace_len(), tx.cu());
    println!(
        "| {name} | {} | {} | {} | {} | {} |",
        tx.keys.len(),
        tx.size,
        trace,
        height,
        cu
    );
    assert!(height <= 5, "{name}: height {height}");
    assert!(trace <= 64, "{name}: trace {trace}");
    assert!(tx.size <= 1_232, "{name}: {} bytes", tx.size);
    assert!(cu <= 1_400_000, "{name}: {cu} CU");
    tx
}

fn threshold() -> u64 {
    u64::from(OrdersSpec::default().siege_threshold) * TEST_PARAMS.siege_unit_lamports
}

#[test]
fn war_budgets() {
    println!("| path | keys | v0 bytes | trace | height | CU |");
    println!("| --- | --- | --- | --- | --- | --- |");
    let mut ww = WarWorld::new();

    // init_war
    let mint = ww.launch("BUDG", LaunchRules::NONE);
    let item = ww.put_item(WAR_ORDERS_TEMPLATE, OrdersSpec::default().params());
    ww.add_slot(&mint, WarWorld::war_slot(item));
    ww.add_slot(&mint, WarWorld::raid_slot(Pubkey::new_unique(), 0));
    let payer = ww.w.env.funded(10 * SOL);
    measure(&mut ww, "init_war", war::init_war(payer.pubkey(), mint), &payer, &[]);
    let t = WarToken {
        mint,
        orders: war::Orders {
            item,
            template: hookwars_war::foreign::template_address(WAR_ORDERS_TEMPLATE),
        },
        pool: ww.w.launch_pool_key(&mint),
        raid_item: Pubkey::default(),
    };

    // siege (the rival a war token too, so it is marked)
    let r = ww.war_token("RIVL", OrdersSpec::default());
    ww.buyer(&r.mint, SOL);
    ww.buyer(&t.mint, 2 * SOL);
    ww.w.env.warp(3_600);
    let now = ww.now();
    ww.put_ledger(&t.mint, 0, threshold(), &[(r.mint, now, threshold(), 0)]);
    let spot = ww.spot(&r.pool);
    ww.flat_observations(&r.pool, spot, 3_600);
    ww.fund_chest(&t.mint, 10 * SOL);
    let cranker = ww.w.env.funded(10 * SOL);
    let ix = ww.siege_ix(&cranker.pubkey(), &t, &r.mint, true, None);
    measure(&mut ww, "siege", ix, &cranker, &[]);

    // raze
    let ix = ww.raze_ix(&cranker.pubkey(), &t, &r.mint);
    measure(&mut ww, "raze", ix, &cranker, &[]);

    // counter_strike
    let spot = ww.spot(&t.pool);
    let o = OrdersSpec::default();
    ww.falling_observations(&t.pool, 2 * spot, spot, i64::from(o.counter_short_secs), i64::from(o.counter_long_secs));
    let ix = ww.counter_ix(&cranker.pubkey(), &t);
    measure(&mut ww, "counter_strike", ix, &cranker, &[]);

    // claim_bounty
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 0, 100, 0);
    let ix = ww.bounty_ix(&holder.pubkey(), &t);
    measure(&mut ww, "claim_bounty", ix, &holder, &[]);

    // A season, then roll, reveal, claim_quest.
    let s = ww.open_season(ScoreWeights {
        raid_volume_won: 1,
        ..ScoreWeights::default()
    });
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, 3 * TEST_PARAMS.quest_raid_points, 2);
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let roll = RollRequest::address(&holding, 1).0;
    let randomness = Pubkey::find_program_address(&[b"randomness", roll.as_ref()], &randomness_stub::ID).0;
    let ix = war::roll(holder.pubkey(), t.mint, 1, RAID_SLOT, randomness_stub::ID, randomness, vec![]);
    measure(&mut ww, "roll", ix, &holder, &[]);
    ww.w.env.warp(1);
    let oracle = ww.w.env.funded(SOL);
    let fulfill = Instruction {
        program_id: randomness_stub::ID,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(
            &randomness_stub::accounts::Fulfill {
                authority: oracle.pubkey(),
                randomness,
            },
            None,
        ),
        data: anchor_lang::InstructionData::data(&randomness_stub::instruction::Fulfill { value: [5; 32] }),
    };
    ww.w.env.send(&[fulfill], &[&oracle]).ok();
    let log = Pubkey::find_program_address(&[b"loot-log", holder.pubkey().as_ref()], &war_armory_stub::ID).0;
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(
        &war_armory_stub::LootLog {
            owner: holder.pubkey(),
            ..Default::default()
        },
        &mut data,
    )
    .unwrap();
    let lamports = ww.w.env.rent(data.len());
    ww.w.env.put(
        log,
        solana_account::Account {
            lamports,
            data,
            owner: war_armory_stub::ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let ix = war::reveal(cranker.pubkey(), holder.pubkey(), holding, 1, s.number, randomness, vec![AccountMeta::new(log, false)]);
    measure(&mut ww, "reveal (stub armory)", ix, &cranker, &[]);
    let p = hookwars_war::instructions::quests::period_at(&s, ww.now(), TEST_PARAMS.quest_period_secs);
    let ix = war::claim_quest(holder.pubkey(), t.mint, s.number, quest::RAID, p, RAID_SLOT, vec![]);
    measure(&mut ww, "claim_quest (raid)", ix, &holder, &[]);

    // submit_candidate, then the prize split.
    ww.put_ledger(&t.mint, s.number, 1_000, &[]);
    ww.w.env.warp(s.ends_at - ww.now());
    let ix = war::submit_candidate(cranker.pubkey(), s.number, t.mint, true);
    measure(&mut ww, "submit_candidate", ix, &cranker, &[]);
    ww.w.env.warp(TEST_PARAMS.challenge_secs);
    ww.w.env.send(&[war::finalize_season(s.number)], &[]).ok();
    let vault = prize_vault_address().0;
    ww.w.env.fund(vault, ww.w.env.rent(0) + SOL);
    let treasury = ww.w.env.treasury.pubkey();
    let chest = WarWorld::chest(&t.mint);
    let inner = [
        bordrless_bridge::client::unwrap_sol(vault, 0),
        bordrless_bridge::client::wrap_sol(chest, 0),
    ];
    let ix = war::split_protocol_fees(cranker.pubkey(), treasury, Some((t.mint, s.number)), &inner);
    measure(&mut ww, "split_protocol_fees", ix, &cranker, &[]);

    // The account sizes and rents the layout constants set (MAX_CAPTURED, LOOT_TABLE_LEN).
    for (name, len) in [
        ("WarConfig", WarConfig::LEN),
        ("WarState", WarState::LEN),
        ("Season", Season::LEN),
        ("LootTable", LootTable::LEN),
        ("RollRequest", RollRequest::LEN),
        ("QuestMark", QuestMark::LEN),
    ] {
        println!("account {name}: {len} bytes, rent {} lamports", ww.w.env.rent(len));
    }
}

/// Pass 4b (10 sections 8, 11.1, 11.3): the expansion paths at their worst case (a coalition of
/// `COALITION_MAX_MEMBERS`).
#[test]
fn war_expansion_budgets() {
    use bordrless_hook::slot_kind;
    use bordrless_launch::client as launch;
    use hookwars_war::client::Member;
    use hookwars_war::constants::{COALITION_MAX_MEMBERS, COALITION_TEMPLATE, RIVALRY_TEMPLATE};
    use hookwars_war::foreign::{template_address, PARAM_FIELDS};

    println!("| path | keys | v0 bytes | trace | height | CU |");
    println!("| --- | --- | --- | --- | --- | --- |");
    let mut ww = WarWorld::new();
    let mut max = [0u32; PARAM_FIELDS];
    max[..3].copy_from_slice(&[u32::MAX, u32::MAX, 10_000]);
    ww.put_template(COALITION_TEMPLATE, slot_kind::RELATION, 2, [0; PARAM_FIELDS], max, false);
    ww.put_template(RIVALRY_TEMPLATE, slot_kind::RELATION, 3, [0; PARAM_FIELDS], max, false);
    let id = 1;
    let mut members = Vec::new();
    let mut tokens = Vec::new();
    for i in 0..COALITION_MAX_MEMBERS {
        let t = ww.war_token(&format!("M{i}"), OrdersSpec::default());
        let mut p = [0u32; PARAM_FIELDS];
        p[0] = id;
        p[1] = 5_000;
        let item = ww.put_item(COALITION_TEMPLATE, p);
        ww.add_slot(&t.mint, WarWorld::named_slot(slot_kind::RELATION, item));
        ww.fund_chest(&t.mint, 10 * SOL);
        members.push(Member { mint: t.mint, item, template: template_address(COALITION_TEMPLATE) });
        tokens.push(t);
    }
    let payer = ww.w.env.payer.insecure_clone();
    measure(&mut ww, "form_coalition (5)", war::form_coalition(payer.pubkey(), id, 86_400, &members), &payer, &[]);
    let shared = Coalition::chest(id).0;
    for m in &members {
        let cranker = ww.w.env.funded(SOL);
        let inner = [
            bordrless_bridge::client::unwrap_sol(WarWorld::chest(&m.mint), 0),
            bordrless_bridge::client::wrap_sol(shared, 0),
            token::create_holding(cranker.pubkey(), ww.w.sol, shared),
        ];
        let ix = war::contribute(cranker.pubkey(), id, *m, SOL, &inner);
        measure(&mut ww, "contribute", ix, &cranker, &[]);
    }
    // The joint siege.
    let rival = ww.launch("RIV", LaunchRules::NONE);
    ww.buyer(&rival, SOL);
    ww.w.env.warp(120);
    let now = ww.now();
    let a = tokens[0];
    ww.put_ledger(&a.mint, 0, threshold(), &[(rival, now, threshold(), 0)]);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let cranker = ww.w.env.funded(SOL);
    let keys = ww.w.launch_keys(&rival);
    let inner = [
        launch::swap_with_base_slice(&keys, shared, shared, 1, 1, 0, vec![]),
        token::create_holding(cranker.pubkey(), rival, shared),
        bordrless_bridge::client::unwrap_sol(shared, 0),
    ];
    let ix = war::coalition_siege(cranker.pubkey(), id, a.mint, a.orders, rival, pool, None, vec![], &inner);
    measure(&mut ww, "coalition_siege", ix, &cranker, &[]);
    // Wind down after the term, then dissolve all five.
    ww.w.env.warp(86_400);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    for _ in 0..4 {
        let c: Coalition = ww.w.env.read(&Coalition::address(id).0);
        if c.captured[0].amount == 0 {
            break;
        }
        let cranker = ww.w.env.funded(SOL);
        let inner = [
            launch::swap_with_base_slice(&keys, shared, shared, 0, 1, 0, vec![]),
            token::create_holding(cranker.pubkey(), rival, shared),
            bordrless_bridge::client::unwrap_sol(shared, 0),
        ];
        let ix = war::coalition_raze(cranker.pubkey(), id, rival, pool, vec![], &inner);
        measure(&mut ww, "coalition_raze (after the term)", ix, &cranker, &[]);
        ww.w.env.warp(1);
    }
    let mints: Vec<Pubkey> = members.iter().map(|m| m.mint).collect();
    let mut inner = vec![bordrless_bridge::client::unwrap_sol(shared, 0)];
    inner.extend(mints.iter().map(|m| bordrless_bridge::client::wrap_sol(WarWorld::chest(m), 0)));
    let ix = war::dissolve_coalition(id, &mints, &inner);
    let payer = ww.w.env.payer.insecure_clone();
    measure(&mut ww, "dissolve_coalition (5)", ix, &payer, &[]);

    // A rivalry between the first two members.
    let b = tokens[1];
    let mut p = [0u32; PARAM_FIELDS];
    p[..3].copy_from_slice(&[(ww.now() - 10) as u32, 3_600, 2_000]);
    let item = ww.put_item(RIVALRY_TEMPLATE, p);
    let slot = ww.add_slot(&a.mint, WarWorld::named_slot(slot_kind::RELATION, item));
    let (key, bump) = hookwars_common::pda::equip_state(&a.mint, slot);
    let es = hookwars_items::EquipState {
        version: 1,
        bump,
        mint: a.mint,
        slot,
        item,
        template_id: RIVALRY_TEMPLATE,
        config: hookwars_common::EquipConfig { targets: vec![b.mint], role: 0 },
        equipped_at: 0,
        runs: 0,
        collected_token: 0,
        pool_owed: 0,
        pool_settled: 0,
        token_unsettled: [0; hookwars_common::MAX_MODULES],
        pool_unsettled: [0; hookwars_common::MAX_MODULES],
        runs_at_settle: 0,
        reserved: [0; 24],
    };
    ww.put_anchor(key, bordrless_token::constants::ITEMS_ID, &es, hookwars_items::EquipState::space(1));
    measure(&mut ww, "open_rivalry", war::open_rivalry(a.mint, item, slot), &payer, &[]);
    ww.w.env.warp(3_600);
    measure(&mut ww, "settle_rivalry", war::settle_rivalry(a.mint, b.mint), &payer, &[]);
}
