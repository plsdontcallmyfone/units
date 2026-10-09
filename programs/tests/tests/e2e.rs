// Changed by Hookwars: new file, the integration suite (end to end on the real programs).
//! End to end on the real programs, with no stand-in on any path: the token program, the DEX, the
//! bridge, the launchpad (`prepare_launch`, `equip_prepared`, `create_prepared_launch`,
//! `refresh_pool_registry`), the armory (templates, items, `equip_launch`), the items program
//! (pool and token callbacks, `settle_equip`, the raid ledger) and the war program (`init_war`,
//! the war chest that pool cuts settle into). Each step checks that money is conserved: the
//! token's supply against its holdings, and the `PoolCuts` holding against what the items say
//! they are owed.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_launch::client as launch;
use bordrless_launch::client::slots as sl;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::armory::{armory_events, armory_ix, params, Hw};
use bordrless_program_tests::items::*;
use bordrless_program_tests::launch::VQ;
use bordrless_program_tests::program_bytes;
use bordrless_swap::client as swap;
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::{read_range, SlotOp};
use bordrless_token::state::{Holding, Mint, SlotBounds};
use bordrless_core::policy;
use hookwars_common::raid::{RaidLedger, RaidRange};
use hookwars_common::{ids, pda, template_id as T, EquipConfig};
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

/// Sends `ixs` as a v0 transaction with one lookup table holding every key (as a client sends slot
/// launches and their trades, 07 section 3), under a 1.4M compute limit.
fn send(env: &mut bordrless_program_tests::env::Env, ixs: &[Instruction], payer: &Keypair, signers: &[&Keypair]) -> bordrless_program_tests::env::Tx {
    let mut all = vec![bordrless_program_tests::env::compute_unit_limit(1_400_000)];
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

/// The armory world with every program real: the launchpad and the war program replace the
/// stand-ins the armory suites load at their ids; the war program is configured with the war
/// suites' TEST parameters.
fn world() -> Hw {
    let mut hw = arsenal();
    for (name, id) in [
        ("bordrless_launch", bordrless_launch::ID),
        ("hookwars_war", hookwars_war::ID),
    ] {
        hw.w.env
            .svm
            .add_program(id, &program_bytes(name))
            .unwrap_or_else(|e| panic!("load {name}: {e:?}"));
    }
    let deployer = hw.w.env.deployer.insecure_clone();
    hw.w.env
        .set_upgrade_authority(hookwars_war::ID, Some(deployer.pubkey()));
    let args = hookwars_war::instructions::ConfigArgs {
        admin: deployer.pubkey(),
        protocol_treasury: hw.w.env.treasury.pubkey(),
        randomness_program: Pubkey::default(),
        treaty_template_id: Some(T::TREATY),
        params: bordrless_program_tests::war::TEST_PARAMS,
    };
    hw.w.env
        .send(&[hookwars_war::client::init_config(deployer.pubkey(), args)], &[&deployer])
        .ok();
    hw
}

/// The armory's `equip_launch` of one slot, as `equip_prepared` forwards it.
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

/// The pool items' accounts of a launch's swap: per forwarded pool slot, the items program, the
/// launchpad's signer for it, and the item's registry extras.
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

/// The registries of a launch's forwarded pool slots, in slot order.
fn pool_registries(hw: &Hw, mint: &Pubkey) -> Vec<Pubkey> {
    let m: Mint = hw.w.env.read(mint);
    m.active_slots()
        .iter()
        .filter(|s| bordrless_launch::instructions::forwards(s))
        .map(|s| {
            Pubkey::find_program_address(
                &[bordrless_hook::HOOK_ACCOUNTS_SEED, mint.as_ref(), s.item.as_ref()],
                &ids::ITEMS_ID,
            )
            .0
        })
        .collect()
}

/// A whole slot launch on the real programs: `prepare_launch`, one `equip_prepared` per entry,
/// `create_prepared_launch`, `refresh_pool_registry`, `init_war` and the raid ledger.
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
    let args = bordrless_program_tests::fixture::World::launch_args("E2E", 100, VQ, LaunchRules::NONE);
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
    let regs = pool_registries(hw, &mint);
    let ix = sl::refresh_pool_registry(c, mint, hw.w.sol, policy::LP_FEE_BPS, regs);
    send(&mut hw.w.env, &[ix], creator, &[]).ok();
    let m: Mint = hw.w.env.read(&mint);
    if m.active_slots().iter().any(|s| s.kind == slot_kind::WAR) {
        send(&mut hw.w.env, &[hookwars_war::client::init_war(c, mint)], creator, &[]).ok();
    } else {
        // No War slot, so no `init_war`: the chest's bridged SOL holding, where pool cuts settle,
        // is created directly (anyone may).
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

/// A swap on a slot launch with the real items' accounts.
fn swap_ix(hw: &Hw, trader: &Pubkey, mint: &Pubkey, direction: u8, amount_in: u64) -> Instruction {
    sl::swap(
        &keys(hw, mint),
        *trader,
        *trader,
        direction,
        amount_in,
        0,
        items_extras(hw, mint, SlotOp::Transfer),
        pool_items(hw, mint),
    )
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

/// One hop of a launch pool for `swap_route`.
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

/// The sum of what a launch's items say they are owed from `PoolCuts` and have not settled.
fn pool_owed(hw: &Hw, mint: &Pubkey) -> u64 {
    let m: Mint = hw.w.env.read(mint);
    (0..m.active_slots().len() as u8)
        .filter_map(|s| hw.w.env.account(&pda::equip_state(mint, s).0).map(|_| equip_state(hw, mint, s)))
        .map(|st| st.pool_owed - st.pool_settled)
        .sum()
}

/// 07 invariant 3 (pool side): the `PoolCuts` holding holds exactly what the items are owed.
fn assert_pool_cuts(hw: &Hw, mint: &Pubkey) {
    let held = hw.w.env.holding(&hw.w.sol, &pda::pool_cuts(mint).0);
    assert_eq!(held, pool_owed(hw, mint), "PoolCuts holding against the items' records");
}

/// 07 invariant 1 for the holdings named.
fn assert_supply(hw: &Hw, mint: &Pubkey, owners: &[Pubkey]) {
    let m: Mint = hw.w.env.read(mint);
    let sum: u64 = owners.iter().map(|o| hw.w.env.holding(mint, o)).sum();
    assert_eq!(sum, m.supply, "supply against the holdings");
}

fn raid_range(hw: &Hw, mint: &Pubkey, owner: &Pubkey, slot: u8) -> RaidRange {
    let h: Holding = hw.w.env.read(&pda::holding(mint, owner));
    let m: Mint = hw.w.env.read(mint);
    RaidRange::read(&read_range(&h.hook_data, &m.slots[usize::from(slot)]), 0)
}

fn ledger(hw: &Hw, mint: &Pubkey) -> RaidLedger {
    RaidLedger::decode(&hw.w.env.account(&pda::raid_ledger(mint).0).unwrap().data).unwrap()
}

struct Tokens {
    rival: Pubkey,
    target: Pubkey,
    composite: Pubkey,
    raid: Pubkey,
    shield: Pubkey,
    tiers: Pubkey,
    burn: Pubkey,
}

/// Two launches: the rival with a composite (Size Tiers then Sell Burn) in one Pool slot, and the
/// target with Raid and Shield (both naming the rival), Size Tiers and Sell Burn.
fn two_launches(hw: &mut Hw, creator: &Keypair) -> Tokens {
    let tiers_p = [1_000_000, 100_000_000, 50, 100, 200];
    let (tx, composite) = create_composite(
        hw,
        creator,
        vec![
            module(T::SIZE_TIERS, params(&tiers_p), 0, 0),
            module(T::SELL_BURN, params(&[100]), 0, 0),
        ],
        500,
    );
    tx.ok();
    let rival = launch_token(
        hw,
        creator,
        vec![slot(slot_kind::POOL, 0, 0, false, true)],
        vec![(0, composite, vec![])],
    );
    let raid = hw.item(T::RAID, params(&[2_000, 100, 3]), 1_000).1;
    let shield = hw.item(T::SHIELD, params(&[300, 3_600, 0]), 0).1;
    let tiers = hw.item(T::SIZE_TIERS, params(&tiers_p), 0).1;
    let burn = hw.item(T::SELL_BURN, params(&[100]), 0).1;
    let target = launch_token(
        hw,
        creator,
        vec![
            slot(slot_kind::POOL, 0, 12, true, false),
            slot(slot_kind::POOL, 0, 7, false, false),
            slot(slot_kind::POOL, 0, 0, false, false),
            slot(slot_kind::POOL, 0, 0, false, true),
        ],
        vec![
            (0, raid, vec![rival]),
            (1, shield, vec![rival]),
            (2, tiers, vec![]),
            (3, burn, vec![]),
        ],
    );
    Tokens { rival, target, composite, raid, shield, tiers, burn }
}

#[test]
fn a_slot_launch_trades_through_the_real_items_and_settles() {
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let t = two_launches(&mut hw, &creator);
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, 50 * SOL).ok();
    let me = trader.pubkey();

    // A plain buy of the target: Size Tiers cuts the quote, nothing burns, no raid.
    let supply0 = {
        let m: Mint = hw.w.env.read(&t.target);
        m.supply
    };
    buy(&mut hw, &trader, &t.target, SOL).ok();
    assert!(hw.w.env.holding(&t.target, &me) > 0);
    assert!(equip_state(&hw, &t.target, 2).pool_owed > 0, "Size Tiers cut the buy");
    assert_eq!(equip_state(&hw, &t.target, 0).pool_owed, 0, "a plain buy is no raid");
    assert_pool_cuts(&hw, &t.target);

    // A sell: Sell Burn destroys part of the tokens sold.
    let held = hw.w.env.holding(&t.target, &me);
    sell(&mut hw, &trader, &t.target, held / 2).ok();
    let m: Mint = hw.w.env.read(&t.target);
    assert!(m.supply < supply0, "Sell Burn burned on the sell");
    assert_pool_cuts(&hw, &t.target);

    // The rival's composite: one item, two modules, both answering on its pool.
    buy(&mut hw, &trader, &t.rival, SOL).ok();
    let st = equip_state(&hw, &t.rival, 0);
    assert!(st.pool_owed > 0, "the composite's Size Tiers module cut");
    let rival_supply = {
        let m: Mint = hw.w.env.read(&t.rival);
        m.supply
    };
    let rival_held = hw.w.env.holding(&t.rival, &me);
    sell(&mut hw, &trader, &t.rival, rival_held / 4).ok();
    let m: Mint = hw.w.env.read(&t.rival);
    assert!(m.supply < rival_supply, "the composite's Sell Burn module burned");
    assert_pool_cuts(&hw, &t.rival);

    // Settle every item: royalty, bounty, then each module's destination (the war chest).
    let cranker = hw.w.env.funded(SOL);
    for (mint, slots) in [(t.target, vec![2u8, 3]), (t.rival, vec![0u8])] {
        let chest = hookwars_war::state::chest_address(&mint).0;
        let chest_quote = pda::holding(&hw.w.sol, &chest);
        for s in slots {
            let st = equip_state(&hw, &mint, s);
            let it: hookwars_armory::state::Item = hw.w.env.read(&st.item);
            let n = if it.template_id == T::COMPOSITE { 2 } else { 1 };
            let dests: Vec<(Pubkey, Pubkey)> = (0..n).map(|_| (ids::ITEMS_ID, chest_quote)).collect();
            let ix = settle_ix(&hw, &cranker.pubkey(), &mint, s, &dests);
            send(&mut hw.w.env, &[ix], &cranker, &[]).ok();
            let after = equip_state(&hw, &mint, s);
            assert_eq!(after.pool_owed, after.pool_settled, "slot {s} fully settled");
        }
        assert_pool_cuts(&hw, &mint);
        assert!(hw.w.env.holding(&hw.w.sol, &chest) > 0, "the chest received the cuts");
    }
    // The composite's owner (its author) earned one royalty.
    assert!(hw.w.env.holding(&hw.w.sol, &pda::royalty_owner(&t.composite).0) > 0);
    let _ = (t.raid, t.shield, t.tiers, t.burn);
}

#[test]
fn a_raid_through_the_rivals_pool_counts_and_a_forged_first_hop_does_not() {
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let t = two_launches(&mut hw, &creator);
    let raider = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&raider, 50 * SOL).ok();
    let me = raider.pubkey();
    buy(&mut hw, &raider, &t.rival, 2 * SOL).ok();
    let x = hw.w.env.holding(&t.rival, &me);
    assert!(x > 0);

    // The real raid: sell the rival on its own launch pool, buy the target, in one route.
    let ixs = [
        token::create_holding(me, t.target, me),
        swap::swap_route(
            me,
            x / 4,
            0,
            vec![launch_hop(&hw, &me, &t.rival, 0), launch_hop(&hw, &me, &t.target, 1)],
            vec![],
        ),
    ];
    send(&mut hw.w.env, &ixs, &raider, &[]).ok();
    let l = ledger(&hw, &t.target);
    assert_eq!(l.inbound[0].rival_mint, t.rival);
    let raided = l.inbound[0].volume;
    assert!(raided > 0, "the raid counted");
    assert!(equip_state(&hw, &t.target, 0).pool_owed > 0, "the Raid toll was cut");
    let points = raid_range(&hw, &t.target, &me, 0).raid_points;
    assert!(points > 0, "the delivery stamped raid points");
    assert_pool_cuts(&hw, &t.target);

    // The forgery (security review M-5): an ordinary pool of the rival the attacker owns, as the
    // first hop. The route still reads "sold the rival", but the first pool is not the rival's
    // launch pool, so the Raid item answers nothing.
    let attacker = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&attacker, 50 * SOL).ok();
    let a = attacker.pubkey();
    buy(&mut hw, &attacker, &t.rival, 2 * SOL).ok();
    let ax = hw.w.env.holding(&t.rival, &a);
    let sol = hw.w.sol;
    let rival_extras = items_extras(&hw, &t.rival, SlotOp::Transfer);
    let create = swap::create_pool(
        &swap::CreatePoolKeys {
            payer: a,
            authority: a,
            treasury: hw.w.env.treasury.pubkey(),
            base_mint: t.rival,
            quote_mint: sol,
            hook_caller: None,
        },
        bordrless_swap::instructions::CreatePoolArgs {
            lp_fee_bps: 30,
            hook_program: Pubkey::default(),
            hook_flags: 0,
            virtual_base: 0,
            virtual_quote: 0,
            base_amount: ax / 2,
            quote_amount: SOL,
            base_hook_accounts: rival_extras.len() as u8,
            quote_hook_accounts: 0,
            hook_data: vec![],
        },
        rival_extras.clone(),
    );
    send(&mut hw.w.env, &[create], &attacker, &[]).ok();
    let fake_pool = swap::pool_address(&t.rival, &sol, 30, None);
    let fake_hop = swap::RouteHop {
        keys: swap::SwapKeys {
            trader: a,
            pool: fake_pool,
            base_mint: t.rival,
            quote_mint: sol,
            trader_base: token::holding_address(&t.rival, &a),
            trader_quote: token::holding_address(&sol, &a),
            hook_program: None,
            base_mint_writable: false,
            quote_mint_writable: false,
        },
        direction: 0,
        in_slice: rival_extras,
        out_slice: vec![],
        pool_extras: vec![],
    };
    let owed_before = equip_state(&hw, &t.target, 0).pool_owed;
    let ixs = [
        token::create_holding(a, t.target, a),
        swap::swap_route(a, ax / 8, 0, vec![fake_hop, launch_hop(&hw, &a, &t.target, 1)], vec![]),
    ];
    send(&mut hw.w.env, &ixs, &attacker, &[]).ok();
    let l = ledger(&hw, &t.target);
    assert_eq!(l.inbound[0].volume, raided, "a forged first hop adds no raid volume");
    assert_eq!(equip_state(&hw, &t.target, 0).pool_owed, owed_before, "and pays no Raid toll");
    assert_eq!(raid_range(&hw, &t.target, &a, 0).raid_points, 0, "and stamps no points");
    assert!(hw.w.env.holding(&t.target, &a) > 0, "the buy itself still went through");
    assert_pool_cuts(&hw, &t.target);
}

#[test]
fn the_launchs_own_deposit_is_never_cut_by_token_items() {
    // A token-side item (Transfer Fee, 3%) equipped before the supply: the launch's deposit of the
    // curve tokens into its pool is not cut (the pool holds exactly the curve allocation), and a
    // wallet transfer afterwards is.
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let fee = hw.item(T::TRANSFER_FEE, params(&[300, 10_000]), 0).1;
    let mint = launch_token(
        &mut hw,
        &creator,
        vec![slot(slot_kind::FEE, 5_000, 0, false, false)],
        vec![(0, fee, vec![creator.pubkey()])],
    );
    let k = keys(&hw, &mint);
    let l: bordrless_launch::state::Launch = hw.w.env.read(&launch::launch_address(&mint));
    let vault = swap::vault_address(&k.pool(), &mint);
    let pool_base = {
        let h: Holding = hw.w.env.read(&vault);
        h.amount
    };
    assert_eq!(pool_base, l.curve_tokens, "the deposit reached the pool uncut");
    assert_eq!(hw.w.env.holding(&mint, &pda::equip_state(&mint, 0).0), 0);
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, 10 * SOL).ok();
    buy(&mut hw, &trader, &mint, SOL).ok();
    let got = hw.w.env.holding(&mint, &trader.pubkey());
    let friend = hw.w.env.funded(SOL);
    transfer(&mut hw, &trader, &mint, &friend.pubkey(), got / 2).ok();
    assert!(hw.w.env.holding(&mint, &pda::equip_state(&mint, 0).0) > 0, "a wallet transfer is cut");
    assert_supply(
        &hw,
        &mint,
        &[
            launch::launch_address(&mint),
            k.pool(),
            trader.pubkey(),
            friend.pubkey(),
            pda::equip_state(&mint, 0).0,
        ],
    );
}

#[test]
fn a_companion_makes_a_slot_launch_through_launch_slots() {
    use bordrless_companion::client as companion;
    use bordrless_companion::instructions::CreateArgs;
    use bordrless_companion::state::Split;
    let mut hw = world();
    let launcher = hw.w.env.funded(1_000 * SOL);
    let tiers = hw.item(T::SIZE_TIERS, params(&[1_000_000, 100_000_000, 50, 100, 200]), 0).1;
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let l = launcher.pubkey();
    let args = CreateArgs {
        split: Split { buyback_bps: 0, holders_bps: 0, beneficiary_bps: 10_000, war_bps: 0 },
        bounty_bps: 50,
        max_buyback: SOL,
        buyback_interval: 60,
        vest_secs: 0,
        fund: 2 * SOL,
    };
    send(&mut hw.w.env, &[companion::create(l, l, mint, args)], &launcher, &[&mint_kp])
        .ok();
    let creator = companion::creator_address(&mint);

    // Step 1: prepare, the creator address signing through the companion.
    let prep = sl::prepare_launch(
        creator,
        mint,
        bordrless_program_tests::fixture::World::prepare_args(
            "COMP",
            100,
            LaunchRules::NONE,
            vec![slot(slot_kind::POOL, 0, 0, false, false)],
        ),
    );
    send(&mut hw.w.env, &[companion::launch_slots(l, mint, &prep)], &launcher, &[&mint_kp])
        .ok();
    // Step 2: equip the launch item through the armory.
    let entry = hookwars_armory::LaunchEquip {
        slot: 0,
        item: Some(tiers),
        config: EquipConfig::default(),
        notice_secs: 600,
        rule: None,
    };
    let equip = sl::equip_prepared(creator, mint, equip_launch_ix(&hw, &creator, &mint, entry));
    send(&mut hw.w.env, &[companion::launch_slots(l, mint, &equip)], &launcher, &[])
        .ok();
    // Anything else is refused.
    let other = token::create_holding(creator, mint, creator);
    let mut bad = companion::launch_slots(l, mint, &prep);
    bad.data = anchor_lang::InstructionData::data(&bordrless_companion::instruction::LaunchSlots {
        data: other.data.clone(),
    });
    send(&mut hw.w.env, &[bad], &launcher, &[&mint_kp])
        .expect_code(u32::from(bordrless_companion::error::CompanionError::NotALaunchStep));
    // Step 3: the launch.
    let cargs = bordrless_program_tests::fixture::World::launch_args("COMP", 100, VQ, LaunchRules::NONE);
    let create = sl::create_prepared_launch(
        creator,
        mint,
        hw.w.env.treasury.pubkey(),
        hw.w.sol,
        policy::LP_FEE_BPS,
        cargs,
        pool_registries(&hw, &mint),
        items_extras(&hw, &mint, SlotOp::Transfer),
    );
    let ixs = vec![
        bordrless_program_tests::env::compute_unit_limit(1_400_000),
        companion::launch_slots(l, mint, &create),
    ];
    let mut keys: Vec<Pubkey> = Vec::new();
    for i in &ixs {
        for m in &i.accounts {
            if !keys.contains(&m.pubkey) {
                keys.push(m.pubkey);
            }
        }
    }
    let table = hw.w.env.put_lookup_table(Pubkey::new_unique(), &keys);
    let tx = hw.w.env.send_v0(&ixs, &launcher, &[&mint_kp], &[table]);
    tx.ok();
    println!(
        "budget | companion slot launch, create_prepared_launch, 1 pool item | keys {} | v0 bytes with table {} | trace {} | height {} | CU {}",
        tx.keys.len(),
        tx.size,
        tx.trace_len(),
        tx.max_height(),
        tx.cu()
    );
    assert!(tx.max_height() <= 5);
    let c: bordrless_companion::state::Companion = hw.w.env.read(&companion::companion_address(&mint));
    assert!(c.launched);
    let lr: bordrless_launch::state::Launch = hw.w.env.read(&launch::launch_address(&mint));
    assert_eq!(lr.creator, creator);
    assert!(lr.is_slot_launch());
    let regs = pool_registries(&hw, &mint);
    let ix = sl::refresh_pool_registry(l, mint, hw.w.sol, policy::LP_FEE_BPS, regs);
    send(&mut hw.w.env, &[ix], &launcher, &[]).ok();
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, 10 * SOL).ok();
    buy(&mut hw, &trader, &mint, SOL).ok();
    assert!(equip_state(&hw, &mint, 0).pool_owed > 0, "the item cut the companion launch's buy");
    assert_pool_cuts(&hw, &mint);
}

// ---- security review 2 regressions on the real programs (Changed by Hookwars)

/// The raid of `two_launches`: `raider` sells a quarter of its rival tokens on the rival's own
/// launch pool and buys the target in one route.
fn raid(hw: &mut Hw, t: &Tokens, raider: &Keypair) {
    let me = raider.pubkey();
    buy(hw, raider, &t.rival, 2 * SOL).ok();
    let x = hw.w.env.holding(&t.rival, &me);
    let ixs = [
        token::create_holding(me, t.target, me),
        swap::swap_route(
            me,
            x / 4,
            0,
            vec![launch_hop(hw, &me, &t.rival, 0), launch_hop(hw, &me, &t.target, 1)],
            vec![],
        ),
    ];
    send(&mut hw.w.env, &ixs, raider, &[]).ok();
}

#[test]
fn h_a_a_token_with_raid_and_shield_items_trades_both_ways() {
    // Raid and Shield answer the default on a plain buy and a plain sell (review 2 H-A).
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let t = two_launches(&mut hw, &creator);
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, 10 * SOL).ok();
    buy(&mut hw, &trader, &t.target, SOL / 2).ok();
    let got = hw.w.env.holding(&t.target, &trader.pubkey());
    assert!(got > 0);
    sell(&mut hw, &trader, &t.target, got).ok();
    assert_eq!(equip_state(&hw, &t.target, 0).pool_owed, 0, "no raid, no Raid toll");
    assert_eq!(equip_state(&hw, &t.target, 1).pool_owed, 0, "no raided tokens, no Shield cut");
    assert_pool_cuts(&hw, &t.target);
}

#[test]
fn m_b_a_raid_sold_back_nets_out_of_the_season_volume() {
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let t = two_launches(&mut hw, &creator);
    let raider = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&raider, 50 * SOL).ok();
    raid(&mut hw, &t, &raider);
    let counted = ledger(&hw, &t.target).outbound_volume_season;
    assert!(counted > 0, "the raid counted");
    assert!(raid_range(&hw, &t.target, &raider.pubkey(), 0).raid_points > 0);
    // The wash: sell everything the raid bought back into the target's pool.
    let held = hw.w.env.holding(&t.target, &raider.pubkey());
    sell(&mut hw, &raider, &t.target, held).ok();
    assert_eq!(ledger(&hw, &t.target).outbound_volume_season, 0, "a round trip counts nothing");
    // A second raid that is kept counts again.
    raid(&mut hw, &t, &raider);
    assert!(ledger(&hw, &t.target).outbound_volume_season > 0);
    assert_pool_cuts(&hw, &t.target);
}

#[test]
fn l_b_the_raid_origin_follows_tokens_to_an_off_curve_owner() {
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let t = two_launches(&mut hw, &creator);
    let raider = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&raider, 50 * SOL).ok();
    raid(&mut hw, &t, &raider);
    let held = hw.w.env.holding(&t.target, &raider.pubkey());
    let vault_owner = Pubkey::find_program_address(&[b"review2"], &ids::ITEMS_ID).0;
    assert!(!vault_owner.is_on_curve());
    transfer(&mut hw, &raider, &t.target, &vault_owner, held / 2).ok();
    let h: Holding = hw.w.env.read(&pda::holding(&t.target, &vault_owner));
    let m: Mint = hw.w.env.read(&t.target);
    let range = read_range(&h.hook_data, &m.slots[1]);
    assert_eq!(range[0], 0x02, "the Shield range was written");
    assert_eq!(range[1], 1, "with the raid's origin (target 0)");
}

#[test]
fn m_a_and_l_c_stray_tokens_and_a_missing_destination_do_not_freeze_a_slot() {
    // Transfer Fee (3%) to the creator. A stranger sends tokens straight to the slot's equip
    // vault; settling burns what no module recorded (M-A). The creator's holding does not exist at
    // first, so that module stays owed and the rest settles (L-C); once it exists, all settles and
    // the vault is empty, which is what `close_equip` requires.
    let mut hw = world();
    let creator = hw.w.env.funded(1_000 * SOL);
    let fee = hw.item(T::TRANSFER_FEE, params(&[300, 10_000]), 0).1;
    let mint = launch_token(
        &mut hw,
        &creator,
        vec![slot(slot_kind::FEE, 5_000, 0, false, false)],
        vec![(0, fee, vec![creator.pubkey()])],
    );
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, 10 * SOL).ok();
    buy(&mut hw, &trader, &mint, SOL).ok();
    let got = hw.w.env.holding(&mint, &trader.pubkey());
    let friend = hw.w.env.funded(SOL);
    transfer(&mut hw, &trader, &mint, &friend.pubkey(), got / 2).ok();
    let state = pda::equip_state(&mint, 0).0;
    let recorded = equip_state(&hw, &mint, 0).token_unsettled[0];
    assert!(recorded > 0);
    let vault = pda::holding(&mint, &state);
    let before = hw.w.env.holding(&mint, &state);
    assert_eq!(before, recorded);
    // The donation: a raw transfer into the vault.
    let donate = token::transfer_with(
        friend.pubkey(),
        token::holding_address(&mint, &friend.pubkey()),
        vault,
        mint,
        None,
        items_extras(&hw, &mint, SlotOp::Transfer),
        1_000,
    );
    hw.w.env.send_paid_by(&[donate], &friend, &[]).ok();
    let recorded = equip_state(&hw, &mint, 0).token_unsettled[0];
    let stray = hw.w.env.holding(&mint, &state) - recorded;
    assert!(stray > 0, "the donation sits unrecorded in the vault");
    let supply0 = {
        let m: Mint = hw.w.env.read(&mint);
        m.supply
    };
    let cranker = hw.w.env.funded(SOL);
    let dests = [(pda::holding(&mint, &creator.pubkey()), ids::ITEMS_ID)];
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &dests);
    send(&mut hw.w.env, &[ix], &cranker, &[]).ok();
    let m: Mint = hw.w.env.read(&mint);
    assert_eq!(m.supply, supply0 - stray, "the stray tokens were burned");
    assert_eq!(equip_state(&hw, &mint, 0).token_unsettled[0], recorded, "no destination yet: still owed");
    assert_eq!(hw.w.env.holding(&mint, &state), recorded);
    // The destination appears; the next settle pays it and empties the vault.
    hw.w.env
        .send_paid_by(&[token::create_holding(cranker.pubkey(), mint, creator.pubkey())], &cranker, &[])
        .ok();
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &dests);
    send(&mut hw.w.env, &[ix], &cranker, &[]).ok();
    assert_eq!(equip_state(&hw, &mint, 0).token_unsettled[0], 0);
    assert_eq!(hw.w.env.holding(&mint, &state), 0, "the vault is empty: the slot can close");
    assert!(hw.w.env.holding(&mint, &creator.pubkey()) > 0);
}
