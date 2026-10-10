// Changed by Hookwars: new file (pass 4b): end to end on the real launchpad, armory, items and war
// programs. Review 2 L-D (a Pool item re-equipped by vote on a slot launch trades at once: the
// armory's `execute` refreshes the pool registry in the same instruction) and review 2 M-B (a live
// self-raid loop through the real Raid callbacks: a raid sold back nets out of the season volume,
// and the season score counts kept raid volume only up to `raid_volume_per_funded` times what the
// chest received). The helpers follow `e2e.rs`.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{equip_rule, slot_kind, AccountSource, HookAccountList};
use bordrless_launch::client as launch;
use bordrless_launch::client::slots as sl;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::armory::{armory_events, armory_ix, params, Hw};
use bordrless_program_tests::items::*;
use bordrless_program_tests::launch::VQ;
use bordrless_program_tests::program_bytes;
use bordrless_program_tests::war::TEST_PARAMS as WAR_PARAMS;
use bordrless_swap::client as swap;
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::SlotOp;
use bordrless_token::state::{Mint, SlotBounds};
use bordrless_core::policy;
use hookwars_common::raid::RaidLedger;
use hookwars_common::{ids, pda, template_id as T, EquipConfig};
use hookwars_war::client as war;
use hookwars_war::events::{CandidateChallenged, CandidateSubmitted};
use hookwars_war::state::{LootEntry, ParamRange, ScoreWeights, WarState};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn slot(kind: u8, data_len: u8, touch: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps: 0,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: touch,
            may_burn: false,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// A v0 transaction with one lookup table of every key, under a 1.4M compute limit.
fn send(env: &mut bordrless_program_tests::env::Env, ixs: &[Instruction], payer: &Keypair, signers: &[&Keypair]) -> bordrless_program_tests::env::Tx {
    let mut all = vec![
        bordrless_program_tests::env::compute_unit_limit(1_400_000),
        bordrless_program_tests::env::heap_frame(256 * 1024),
    ];
    all.extend(ixs.iter().cloned());
    let mut keys: Vec<Pubkey> = Vec::new();
    for i in &all {
        for m in &i.accounts {
            if !keys.contains(&m.pubkey) {
                keys.push(m.pubkey);
            }
        }
    }
    let table = env.put_lookup_table(Pubkey::new_unique(), &keys);
    env.send_v0(&all, payer, signers, &[table])
}

/// The armory world with the real launchpad and war program (war TEST parameters).
fn world() -> Hw {
    let mut hw = arsenal();
    let svm = std::mem::replace(&mut hw.w.env.svm, litesvm::LiteSVM::new());
    hw.w.env.svm = svm.with_log_bytes_limit(None);
    for (name, id) in [("bordrless_launch", bordrless_launch::ID), ("hookwars_war", hookwars_war::ID)] {
        hw.w.env
            .svm
            .add_program(id, &program_bytes(name))
            .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    }
    let deployer = hw.w.env.deployer.insecure_clone();
    hw.w.env.set_upgrade_authority(hookwars_war::ID, Some(deployer.pubkey()));
    let args = hookwars_war::instructions::ConfigArgs {
        admin: deployer.pubkey(),
        protocol_treasury: hw.w.env.treasury.pubkey(),
        randomness_program: Pubkey::default(),
        treaty_template_id: Some(T::TREATY),
        params: WAR_PARAMS,
    };
    hw.w.env.send(&[war::init_config(deployer.pubkey(), args)], &[&deployer]).ok();
    hw
}

fn equip_launch_ix(hw: &Hw, payer: &Pubkey, mint: &Pubkey, entry: hookwars_armory::LaunchEquip) -> Instruction {
    let equip = hw.equip_accounts(payer, mint, entry.slot, None, entry.item);
    armory_ix(
        hookwars_armory::accounts::EquipLaunch {
            launch_caller: pda::armory_caller(mint).0,
            config: pda::config().0,
            slot_state: pda::slot_state(mint, entry.slot).0,
            equip,
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::EquipLaunch { entry },
    )
}

fn pool_items(hw: &Hw, mint: &Pubkey) -> Vec<AccountMeta> {
    let m: Mint = hw.w.env.read(mint);
    let mut v = Vec::new();
    for s in m.active_slots() {
        if !bordrless_launch::instructions::forwards(s) {
            continue;
        }
        v.push(AccountMeta::new_readonly(s.program, false));
        v.push(AccountMeta::new_readonly(sl::item_signer(&s.program), false));
        v.extend(registry_extras(hw, mint, &s.item));
    }
    v
}

fn item_registry(mint: &Pubkey, item: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[bordrless_hook::HOOK_ACCOUNTS_SEED, mint.as_ref(), item.as_ref()], &ids::ITEMS_ID).0
}

fn pool_registries(hw: &Hw, mint: &Pubkey) -> Vec<Pubkey> {
    let m: Mint = hw.w.env.read(mint);
    m.active_slots()
        .iter()
        .filter(|s| bordrless_launch::instructions::forwards(s))
        .map(|s| item_registry(mint, &s.item))
        .collect()
}

/// A slot launch on the real programs (as `e2e.rs`): prepare, equip each entry, create, refresh,
/// `init_war` when it has a War slot (the chest's holding otherwise), the raid ledger.
fn launch_token(hw: &mut Hw, creator: &Keypair, slots: Vec<SlotInit>, equips: Vec<(u8, Pubkey, Vec<Pubkey>)>) -> Pubkey {
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let c = creator.pubkey();
    hw.w.prepare_launch(creator, &mint_kp, 100, LaunchRules::NONE, slots).ok();
    for (slot, item, targets) in equips {
        let entry = hookwars_armory::LaunchEquip {
            slot,
            item: Some(item),
            config: EquipConfig { targets, role: 0 },
            notice_secs: 600,
            rule: None,
        };
        let ix = sl::equip_prepared(c, mint, equip_launch_ix(hw, &c, &mint, entry));
        send(&mut hw.w.env, &[ix], creator, &[]).ok();
    }
    let args = bordrless_program_tests::fixture::World::launch_args("P4B", 100, VQ, LaunchRules::NONE);
    let ix = sl::create_prepared_launch(
        c,
        mint,
        hw.w.env.treasury.pubkey(),
        hw.w.sol,
        policy::LP_FEE_BPS,
        args,
        pool_registries(hw, &mint),
        items_extras(hw, &mint, SlotOp::Transfer),
    );
    send(&mut hw.w.env, &[ix], creator, &[&mint_kp]).ok();
    let m: Mint = hw.w.env.read(&mint);
    if m.active_slots().iter().any(|s| s.kind == slot_kind::WAR) {
        send(&mut hw.w.env, &[war::init_war(c, mint)], creator, &[]).ok();
    } else {
        let chest = hookwars_war::state::chest_address(&mint).0;
        send(&mut hw.w.env, &[token::create_holding(c, hw.w.sol, chest)], creator, &[]).ok();
    }
    init_ledger(hw, creator, &mint).ok();
    mint
}

fn keys(hw: &Hw, mint: &Pubkey) -> launch::LaunchKeys {
    let mut k = hw.w.launch_keys(mint);
    k.burns = true;
    k
}

fn swap_ix(hw: &Hw, trader: &Pubkey, mint: &Pubkey, direction: u8, amount_in: u64) -> Instruction {
    sl::swap(&keys(hw, mint), *trader, *trader, direction, amount_in, 0, items_extras(hw, mint, SlotOp::Transfer), pool_items(hw, mint))
}

fn buy(hw: &mut Hw, trader: &Keypair, mint: &Pubkey, lamports: u64) -> bordrless_program_tests::env::Tx {
    let t = trader.pubkey();
    let ixs = [token::create_holding(t, *mint, t), swap_ix(hw, &t, mint, 1, lamports)];
    send(&mut hw.w.env, &ixs, trader, &[])
}

fn sell(hw: &mut Hw, trader: &Keypair, mint: &Pubkey, amount: u64) -> bordrless_program_tests::env::Tx {
    let t = trader.pubkey();
    let ix = swap_ix(hw, &t, mint, 0, amount);
    send(&mut hw.w.env, &[ix], trader, &[])
}

fn launch_hop(hw: &Hw, trader: &Pubkey, mint: &Pubkey, direction: u8) -> swap::RouteHop {
    let k = keys(hw, mint);
    let sol = hw.w.sol;
    let slices = items_extras(hw, mint, SlotOp::Transfer);
    swap::RouteHop {
        keys: swap::SwapKeys {
            trader: *trader,
            pool: k.pool(),
            base_mint: *mint,
            quote_mint: sol,
            trader_base: token::holding_address(mint, trader),
            trader_quote: token::holding_address(&sol, trader),
            hook_program: Some(bordrless_launch::ID),
            base_mint_writable: true,
            quote_mint_writable: false,
        },
        direction,
        in_slice: if direction == 0 { slices.clone() } else { vec![] },
        out_slice: if direction == 1 { slices } else { vec![] },
        pool_extras: sl::pool_extras(mint, &sol, pool_items(hw, mint)),
    }
}

fn ledger(hw: &Hw, mint: &Pubkey) -> RaidLedger {
    RaidLedger::decode(&hw.w.env.account(&pda::raid_ledger(mint).0).unwrap().data).unwrap()
}

/// The keys a launch pool's registry lists (fixed keys only).
fn registry_keys(hw: &Hw, mint: &Pubkey) -> Vec<Pubkey> {
    let pool = hw.w.launch_pool_key(mint);
    let data = hw.w.env.svm.get_account(&launch::registry_address(&pool)).unwrap().data;
    HookAccountList::decode(&data)
        .unwrap()
        .accounts
        .iter()
        .map(|a| match &a.source {
            AccountSource::Key(k) => *k,
            _ => Pubkey::default(),
        })
        .collect()
}

/// `finalize` of a proposal on a launched token: the launch, its pool's base vault and the launch's
/// own holding (security review 1, M-1).
fn finalize_ix(hw: &Hw, proposal: &Pubkey) -> Instruction {
    let p: hookwars_armory::state::Proposal = hw.w.env.read(proposal);
    let pool = hw.w.launch_pool_key(&p.mint);
    let launch_key = pda::launch(&p.mint).0;
    armory_ix(
        hookwars_armory::accounts::Finalize {
            config: pda::config().0,
            proposal: *proposal,
            slot_state: pda::slot_state(&p.mint, p.slot).0,
            token_mint: p.mint,
            launch: Some(launch_key),
            pool_base_vault: Some(token::holding_address(&p.mint, &pool)),
            launch_holding: Some(token::holding_address(&p.mint, &launch_key)),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::Finalize {},
    )
}

/// Review 2 L-D, end to end: a vote re-equips the Pool slot of a slot launch; `execute` refreshes the
/// pool registry in the same instruction, so the next swap (with the new item's accounts) runs with
/// no separate `refresh_pool_registry`.
#[test]
fn l_d_a_pool_item_re_equipped_by_vote_trades_at_once() {
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let first = hw.item(T::SIZE_TIERS, params(&[1_000_000, 100_000_000, 50, 100, 200]), 0).1;
    let second = hw.item(T::SIZE_TIERS, params(&[2_000_000, 200_000_000, 20, 40, 80]), 0).1;
    let mint = launch_token(&mut hw, &creator, vec![slot(slot_kind::POOL, 0, false)], vec![(0, first, vec![])]);
    assert_eq!(registry_keys(&hw, &mint)[5], ids::ITEMS_ID, "the launch lists the first item");
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, 50 * SOL).ok();
    buy(&mut hw, &trader, &mint, 2 * SOL).ok();
    let held = hw.w.env.holding(&mint, &trader.pubkey());
    assert!(held > 0);

    let (tx, proposal) = hw.propose(&trader, &mint, 0, Some(second), EquipConfig::default());
    tx.ok();
    hw.vote(&trader, &mint, &proposal, true, held).ok();
    hw.w.env.warp(3_600);
    let ix = finalize_ix(&hw, &proposal);
    let payer = hw.w.env.payer.insecure_clone();
    send(&mut hw.w.env, &[ix], &payer, &[]).ok();
    hw.w.env.warp(600);
    // The old item's pool cuts settle first (the armory refuses to unequip a slot with unsettled
    // vaults), into the token's war chest.
    let cranker = hw.w.env.funded(SOL);
    let chest_quote = pda::holding(&hw.w.sol, &hookwars_war::state::chest_address(&mint).0);
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &[(ids::ITEMS_ID, chest_quote)]);
    send(&mut hw.w.env, &[ix], &cranker, &[]).ok();
    // `execute` with the refresh tail of a slot launch: launch program, launch, pool registry, then
    // what the armory forwards after the system program: the launchpad's event authority and
    // program (its `refresh_pool_registry` is an `#[event_cpi]` instruction; the armory does not add
    // them itself, finding recorded in docs/spec/14-pass-4b.md), then the forwarded pool slots' item
    // registries as they will be after the equip.
    let pool = hw.w.launch_pool_key(&mint);
    let mut ix = hw.execute_ix(&trader.pubkey(), &proposal);
    let n = ix.accounts.len();
    ix.accounts.truncate(n - 3);
    ix.accounts.extend([
        AccountMeta::new_readonly(ids::LAUNCH_ID, false),
        AccountMeta::new_readonly(pda::launch(&mint).0, false),
        AccountMeta::new(launch::registry_address(&pool), false),
        AccountMeta::new_readonly(hookwars_common::eco_cpi::event_authority(&ids::LAUNCH_ID), false),
        AccountMeta::new_readonly(ids::LAUNCH_ID, false),
        AccountMeta::new_readonly(item_registry(&mint, &second), false),
    ]);
    // The tail as the armory documents it (no event-cpi accounts) fails against the real launchpad.
    let mut bare = ix.clone();
    bare.accounts.remove(n - 3 + 3);
    bare.accounts.remove(n - 3 + 3);
    send(&mut hw.w.env, &[bare], &trader, &[]).expect_fail();
    let tx = send(&mut hw.w.env, &[ix], &trader, &[]);
    tx.ok();
    println!("execute with the pool registry refresh: cu {}", tx.cu());
    assert_eq!(hw.slot_item(&mint, 0), second);
    let k = registry_keys(&hw, &mint);
    assert_eq!(k[5], ids::ITEMS_ID);
    assert_eq!(k.len(), 5 + 2 + registry_extras(&hw, &mint, &second).len(), "the registry names the new item");

    // No refresh_pool_registry: a buy and a sell go through at once with the new item's accounts.
    let other = hw.w.env.funded(10 * SOL);
    hw.w.wrap_sol(&other, 5 * SOL).ok();
    buy(&mut hw, &other, &mint, SOL).ok();
    let got = hw.w.env.holding(&mint, &other.pubkey());
    assert!(got > 0, "the buy after the re-equip went through");
    sell(&mut hw, &other, &mint, got / 2).ok();
    assert_eq!(hw.w.env.holding(&mint, &other.pubkey()), got - got / 2);
}

/// Opens war season 1 with `weights` and a loot table of one loot-enabled template at its floors.
fn open_season(hw: &mut Hw, weights: ScoreWeights) -> hookwars_war::state::Season {
    let admin = hw.w.env.deployer.insecure_clone();
    let (id, t) = (1..=64u16)
        .filter_map(|id| {
            let a = hw.w.env.account(&pda::template(id).0)?;
            let t = <hookwars_armory::state::Template as anchor_lang::AccountDeserialize>::try_deserialize(&mut &a.data[..]).ok()?;
            t.loot_enabled.then_some((id, t))
        })
        .next()
        .expect("a loot-enabled template");
    let mut ranges = [ParamRange::default(); hookwars_common::PARAM_FIELDS];
    for (i, r) in ranges.iter_mut().enumerate() {
        *r = ParamRange { min: t.field_min[i], max: t.field_min[i] };
    }
    let starts_at = hw.w.env.now + WAR_PARAMS.admin_timelock_secs;
    let args = hookwars_war::instructions::SeasonArgs {
        number: 1,
        starts_at,
        weights,
        penalize_besieged: false,
    };
    let ixs = [
        war::propose_season(admin.pubkey(), args),
        war::propose_loot_table(admin.pubkey(), 1, vec![LootEntry { template_id: id, weight: 1, ranges }], &[pda::template(id).0]),
    ];
    hw.w.env.send(&ixs, &[&admin]).ok();
    hw.w.env.warp(WAR_PARAMS.admin_timelock_secs);
    hw.w.env.send(&[war::open_season(0)], &[]).ok();
    hw.w.env.read(&hookwars_war::state::Season::address(1).0)
}

/// Review 2 M-B, live: raids through the real Raid callbacks. A raid sold back nets out of the
/// season volume (items); the kept raid volume scores only up to `raid_volume_per_funded` times what
/// the chest received in the season (war), so the loop's fees cannot buy the score.
#[test]
fn m_b_a_live_self_raid_loop_cannot_buy_the_season_score() {
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let tiers = hw.item(T::SIZE_TIERS, params(&[1_000_000, 100_000_000, 50, 100, 200]), 0).1;
    let rival = launch_token(&mut hw, &creator, vec![slot(slot_kind::POOL, 0, false)], vec![(0, tiers, vec![])]);
    let raid_item = hw.item(T::RAID, params(&[2_000, 100, 3]), 0).1;
    let orders = hw.item(T::WAR_ORDERS, params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]), 0).1;
    let target = launch_token(
        &mut hw,
        &creator,
        vec![slot(slot_kind::POOL, 12, true), slot(slot_kind::WAR, 0, false)],
        vec![(0, raid_item, vec![rival]), (1, orders, vec![])],
    );
    assert!(hw.w.env.account(&WarState::address(&target).0).is_some(), "the target is a war token");
    let s = open_season(&mut hw, ScoreWeights { raid_volume_won: 1, ..ScoreWeights::default() });

    let raider = hw.w.env.funded(200 * SOL);
    hw.w.wrap_sol(&raider, 100 * SOL).ok();
    let me = raider.pubkey();
    buy(&mut hw, &raider, &rival, 10 * SOL).ok();
    let raid = |hw: &mut Hw, amount: u64| {
        let ixs = [
            token::create_holding(me, target, me),
            swap::swap_route(me, amount, 0, vec![launch_hop(hw, &me, &rival, 0), launch_hop(hw, &me, &target, 1)], vec![]),
        ];
        send(&mut hw.w.env, &ixs, &raider, &[]).ok();
    };
    // The wash loop: raid, then sell the raided tokens straight back, three times.
    let x = hw.w.env.holding(&rival, &me);
    for _ in 0..3 {
        raid(&mut hw, x / 20);
        let held = hw.w.env.holding(&target, &me);
        sell(&mut hw, &raider, &target, held).ok();
        assert_eq!(ledger(&hw, &target).outbound_volume_season, 0, "a raid sold back counts nothing");
        let back = hw.w.env.holding(&hw.w.sol.clone(), &me).min(SOL / 2);
        buy(&mut hw, &raider, &rival, back).ok();
    }
    // Kept raids count in the ledger.
    let x = hw.w.env.holding(&rival, &me);
    raid(&mut hw, x / 10);
    raid(&mut hw, x / 10);
    let l = ledger(&hw, &target);
    assert_eq!(l.season_id, s.number);
    let volume = l.outbound_volume_season;
    assert!(volume > 0);

    // After the season: the chest received nothing this season, so raid volume scores 0.
    hw.w.env.warp(s.ends_at - hw.w.env.now + 1);
    let submitter = hw.w.env.funded(SOL);
    let tx = hw.w.env.send(&[war::submit_candidate(submitter.pubkey(), 1, target, true)], &[&submitter]);
    assert_eq!(tx.event::<CandidateSubmitted>().score, 0);
    // Real money into the chest raises the cap to exactly `raid_volume_per_funded` per lamport.
    let funding = volume / WAR_PARAMS.raid_volume_per_funded / 2;
    assert!(funding > 0);
    let donor = hw.w.env.funded(10 * SOL);
    hw.w.wrap_sol(&donor, SOL).ok();
    let chest = hookwars_war::state::chest_address(&target).0;
    let sol = hw.w.sol;
    let ixs = [
        token::transfer(donor.pubkey(), token::holding_address(&sol, &donor.pubkey()), token::holding_address(&sol, &chest), sol, None, vec![], funding),
        war::record_funding(target),
    ];
    hw.w.env.send_paid_by(&ixs, &donor, &[]).ok();
    let tx = hw.w.env.send(&[war::submit_candidate(submitter.pubkey(), 1, target, true)], &[&submitter]);
    let e = tx.event::<CandidateChallenged>();
    let st: WarState = hw.w.env.read(&WarState::address(&target).0);
    assert_eq!(st.season.funded, funding);
    assert_eq!(e.score, i128::from(funding * WAR_PARAMS.raid_volume_per_funded));
    assert!(e.score < i128::from(volume), "the cap binds: the loop's volume is not the score");
}
