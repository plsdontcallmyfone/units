// Changed by Hookwars: new file.
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
            template: hookwars_war::foreign::Template::address(WAR_ORDERS_TEMPLATE),
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
