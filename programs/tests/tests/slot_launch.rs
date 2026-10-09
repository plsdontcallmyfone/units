// Changed by Hookwars: new file, M3b slot launches (spec 03 sections 4 and 5).
//! Slot launches: `prepare_launch`, equipping, `create_prepared_launch`, the launch pool hook
//! forwarding to pool items and merging their answers, `refresh_pool_registry`, graduation with a
//! slot mint, and `equip_prepared` through the real armory.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use bordrless_hook::{AccountSource, ExtraAccount, HookAccountList, HOOK_ACCOUNTS_SEED};
use bordrless_launch::client as launch;
use bordrless_launch::client::slots as sl;
use bordrless_launch::constants::hookwars::SLOT_LAUNCH;
use bordrless_launch::constants::STATUS_GRADUATED;
use bordrless_launch::error::LaunchError;
use bordrless_launch::state::{Launch, LaunchRules, PreparedLaunch};
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::launch::{presets, VQ};
use bordrless_program_tests::slot_launch::*;
use bordrless_token::client as token;
use bordrless_token::slots::SlotOp;
use bordrless_token::state::Mint;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn code(e: LaunchError) -> u32 {
    u32::from(e)
}

#[test]
fn prepare_then_launch_makes_a_slot_mint_with_no_supply_until_the_launch() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(100 * SOL);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    w.prepare_launch(&creator, &mint_kp, 100, LaunchRules::NONE, vec![pool_slot(300, false)])
        .ok();
    let m: Mint = w.env.read(&mint);
    assert!(m.uses_slots());
    assert_eq!(m.supply, 0);
    assert_eq!(m.mint_authority, Some(launch::launch_address(&mint)));
    assert_eq!(m.hook_program, None);
    let p: PreparedLaunch = w.env.read(&sl::prepared_address(&mint));
    assert_eq!(p.creator, creator.pubkey());
    assert!(!p.launched);
    assert_eq!(p.slot_count, 1);

    // Someone else cannot launch the prepared mint (it signs, and the creator must be the one
    // who prepared it).
    let other = w.env.funded(100 * SOL);
    let ix = w.create_prepared_ix(&other.pubkey(), &mint, 100, VQ, LaunchRules::NONE);
    w.env
        .send_paid_by(&[ix], &other, &[&mint_kp])
        .expect_code(code(LaunchError::WrongCreator));
    // Other rules than the prepared ones are refused.
    let ix = w.create_prepared_ix(&creator.pubkey(), &mint, 200, VQ, LaunchRules::NONE);
    w.env
        .send_paid_by(&[ix], &creator, &[&mint_kp])
        .expect_code(code(LaunchError::PreparedMismatch));

    let ix = w.create_prepared_ix(&creator.pubkey(), &mint, 100, VQ, LaunchRules::NONE);
    w.env.send_paid_by(&[ix], &creator, &[&mint_kp]).ok();
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    assert_eq!(l.slot_launch, SLOT_LAUNCH);
    let m: Mint = w.env.read(&mint);
    assert_eq!(m.mint_authority, None);
    assert_eq!(m.supply, l.curve_tokens + l.reserve_tokens);
    let p: PreparedLaunch = w.env.read(&sl::prepared_address(&mint));
    assert!(p.launched);
    // The PoolCuts quote holding exists from the launch on.
    let cuts = sl::pool_cuts_holding(&mint, &w.sol);
    assert!(w.env.svm.get_account(&cuts).is_some());
    // A prepared launch launches once.
    let ix = w.create_prepared_ix(&creator.pubkey(), &mint, 100, VQ, LaunchRules::NONE);
    w.env.send_paid_by(&[ix], &creator, &[&mint_kp]).expect_fail();
}

#[test]
fn prepare_refuses_locked_item_slots_and_too_many_slots() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(100 * SOL);
    let mut locked = pool_slot(0, false);
    locked.kind = bordrless_hook::slot_kind::LOCKED;
    w.prepare_launch(&creator, &Keypair::new(), 100, LaunchRules::NONE, vec![locked])
        .expect_code(code(LaunchError::InvalidSlots));
    w.prepare_launch(&creator, &Keypair::new(), 100, LaunchRules::NONE, vec![])
        .expect_code(code(LaunchError::InvalidSlots));
    // With kit rules the kit takes slot 0, so only MAX_SLOTS - 1 item slots fit.
    let n = bordrless_token::constants::MAX_SLOTS;
    let slots = (0..n).map(|_| pool_slot(100, false)).collect();
    w.prepare_launch(&creator, &Keypair::new(), 100, presets::rewards_and_cap(), slots)
        .expect_code(code(LaunchError::InvalidSlots));
}

#[test]
fn a_kit_launch_in_slots_has_the_kit_locked_in_slot_0_and_trades() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let r = presets::rewards_and_cap();
    let (mint, _) = w.slot_launch(&creator, 100, r, vec![pool_slot(300, false)], &[]);
    let m: Mint = w.env.read(&mint);
    let (index, kit) = m.locked_slot().expect("kit slot");
    assert_eq!(index, 0);
    assert_eq!(kit.program, bordrless_kit::ID);
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    assert!(l.rewards_on());
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, SOL).ok();
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    let held = w.env.holding(&mint, &trader.pubkey());
    assert!(held > 0);
    w.slot_sell(&trader, &mint, held / 2).ok();
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    assert!(l.creator_fees_accrued > 0);
    // The kit runs from its Locked slot: it counts the trader's tokens as eligible supply (holder
    // fees start once holders hold the kit's threshold, as upstream).
    let kit = w.env.kit_config(&mint);
    assert_eq!(kit.eligible, w.env.holding(&mint, &trader.pubkey()));
}

fn market() -> (World, Keypair, Pubkey, Pubkey, Keypair) {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(
        &creator,
        100,
        LaunchRules::NONE,
        vec![pool_slot(500, false)],
        &[(0, BOTH)],
    );
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, 10 * SOL).ok();
    (w, creator, mint, items[0], trader)
}

#[test]
fn a_buy_forwards_to_the_pool_item_and_its_cut_lands_in_pool_cuts() {
    let (mut w, creator, mint, item, trader) = market();
    let cut = 1_000_000;
    w.set_stub(&creator, &item, ans(0, cut, 0), ans(0, 0, 0), false, false);
    let cuts = sl::pool_cuts_holding(&mint, &w.sol);
    let before = w.env.holding(&w.sol.clone(), &sl::pool_cuts_owner(&mint));
    let amount = SOL / 10;
    w.slot_buy(&trader, &mint, amount).ok();
    let after = w.env.holding(&w.sol.clone(), &sl::pool_cuts_owner(&mint));
    assert_eq!(after - before, cut);
    let s = w.stub_script(&item);
    assert_eq!((s.before_calls, s.after_calls), (1, 1));
    assert_eq!(s.collected, cut);
    assert_eq!(s.last_slot, 0);
    assert_eq!(s.last_item, item);
    assert_eq!(s.last_direction, 1);
    assert_eq!(s.last_hop_count, 1);
    assert_eq!(s.last_route_input, w.sol);
    assert_eq!(s.last_actor, trader.pubkey());
    let _ = cuts;
}

#[test]
fn the_before_context_names_the_launch_fee_and_what_is_left_of_the_input() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(
        &creator,
        100,
        LaunchRules::NONE,
        vec![pool_slot(500, false)],
        &[(0, BEFORE)],
    );
    let item = items[0];
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, 10 * SOL).ok();
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 0, 0), false, false);
    // Past the sniper window so only the creator fee applies.
    w.env.warp(60);
    let amount = SOL / 10;
    w.slot_buy(&trader, &mint, amount).ok();
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    let s = w.stub_script(&item);
    // A buy's before callback acts on the quote input: the launch's cut is its creator fee.
    assert_eq!(s.last_direction, 1);
    assert_eq!(s.last_launch_fee_bps, 100);
    assert_eq!(l.creator_fees_accrued, s.last_launch_cut);
    assert_eq!(s.last_side_amount, amount - s.last_launch_cut);
    assert_eq!((s.before_calls, s.after_calls), (1, 0));
}

#[test]
fn a_sell_cut_comes_from_the_quote_output_after_the_launch_fees() {
    let (mut w, creator, mint, item, trader) = market();
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 0, 0), false, false);
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    let cut = 500_000;
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, cut, 0), false, false);
    let owner = sl::pool_cuts_owner(&mint);
    let sol = w.sol;
    let before = w.env.holding(&sol, &owner);
    let held = w.env.holding(&mint, &trader.pubkey());
    w.slot_sell(&trader, &mint, held / 2).ok();
    assert_eq!(w.env.holding(&sol, &owner) - before, cut);
}

#[test]
fn a_discount_waives_part_of_the_creator_fee_never_the_lp_fee() {
    let mut fees = Vec::new();
    for discount in [0u16, 5_000, 10_000] {
        let (mut w, creator, mint, item, trader) = market();
        w.set_stub(&creator, &item, ans(discount, 0, 0), ans(0, 0, 0), false, false);
        w.env.warp(60);
        w.slot_buy(&trader, &mint, SOL / 10).ok();
        let l: Launch = w.env.read(&launch::launch_address(&mint));
        let p = w.launch_pool(&mint);
        fees.push((l.creator_fees_accrued, p.protocol_fees_quote));
    }
    let (full, lp_full) = fees[0];
    assert!(full > 0);
    assert_eq!(fees[1].0, full / 2);
    assert_eq!(fees[2].0, 0);
    // The protocol's LP fee is never discounted; only its share of the creator fee moves.
    assert!(fees[2].1 > 0 && fees[2].1 <= lp_full);
}

#[test]
fn item_answers_on_the_wrong_side_or_over_the_bound_are_refused() {
    // A cut in a buy's after callback (the base output) is on the wrong side.
    let (mut w, creator, mint, item, trader) = market();
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 1, 0), false, false);
    w.slot_buy(&trader, &mint, SOL / 10)
        .expect_code(code(LaunchError::ItemCutWrongSide));
    // A burn where the slot does not allow one.
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 0, 1), false, false);
    w.slot_buy(&trader, &mint, SOL / 10)
        .expect_code(code(LaunchError::ItemBurnWrongSide));
    // More than what is left of the side.
    w.set_stub(&creator, &item, ans(0, SOL / 10, 0), ans(0, 0, 0), false, false);
    w.slot_buy(&trader, &mint, SOL / 10)
        .expect_code(code(LaunchError::ItemCutOutOfBounds));
    // A discount above 10,000.
    w.set_stub(&creator, &item, ans(10_001, 0, 0), ans(0, 0, 0), false, false);
    w.slot_buy(&trader, &mint, SOL / 10)
        .expect_code(code(LaunchError::ItemDiscountTooHigh));
    // An item that fails fails the swap.
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 0, 0), true, false);
    w.slot_buy(&trader, &mint, SOL / 10).expect_fail();
}

#[test]
fn a_burn_slot_burns_base_on_a_buys_output() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(
        &creator,
        100,
        LaunchRules::NONE,
        vec![pool_slot(500, true)],
        &[(0, BOTH)],
    );
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, SOL).ok();
    let burn = 1_000_000;
    w.set_stub(&creator, &items[0], ans(0, 0, 0), ans(0, 0, burn), false, false);
    let supply = w.env.read::<Mint>(&mint).supply;
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    assert_eq!(supply - w.env.read::<Mint>(&mint).supply, burn);
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    assert_eq!(l.burned_on_trades, burn);
}

#[test]
fn only_subscribed_callbacks_are_forwarded_and_items_run_in_slot_order() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(
        &creator,
        100,
        LaunchRules::NONE,
        vec![pool_slot(300, false), pool_slot(300, false), pool_slot(300, false)],
        &[(0, BEFORE), (1, AFTER), (2, BOTH)],
    );
    for i in &items {
        w.set_stub(&creator, i, ans(0, 10, 0), ans(0, 0, 0), false, false);
    }
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, SOL).ok();
    let owner = sl::pool_cuts_owner(&mint);
    let sol = w.sol;
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    let calls: Vec<(u32, u32)> = items
        .iter()
        .map(|i| {
            let s = w.stub_script(i);
            (s.before_calls, s.after_calls)
        })
        .collect();
    assert_eq!(calls, vec![(1, 0), (0, 1), (1, 1)]);
    // The two before-subscribed items' cuts merged into one delta.
    assert_eq!(w.env.holding(&sol, &owner), 20);
}

#[test]
fn missing_or_wrong_item_accounts_fail_the_swap() {
    let (mut w, creator, mint, item, trader) = market();
    w.set_stub(&creator, &item, ans(0, 0, 0), ans(0, 0, 0), false, false);
    let t = trader.pubkey();
    let keys = w.launch_keys(&mint);
    // No item accounts.
    let ix = sl::swap(&keys, t, t, 1, SOL / 10, 0, vec![], vec![]);
    w.env
        .send_paid_by(&[token::create_holding(t, mint, t), ix], &trader, &[])
        .expect_code(code(LaunchError::ItemAccountsMissing));
    // The wrong program in the item's place.
    let items = vec![
        AccountMeta::new_readonly(slot_tester::ID, false),
        AccountMeta::new_readonly(sl::item_signer(&slot_tester::ID), false),
        AccountMeta::new(pool_item_stub::script_address(&item), false),
    ];
    let ix = sl::swap(&keys, t, t, 1, SOL / 10, 0, vec![], items);
    w.env
        .send_paid_by(&[token::create_holding(t, mint, t), ix], &trader, &[])
        .expect_code(code(LaunchError::WrongItemProgram));
    // A pool-cuts holding that is not the items program's.
    let mut ix = w.slot_swap_ix(&t, &t, &mint, 1, SOL / 10);
    let cuts = sl::pool_cuts_holding(&mint, &w.sol);
    for m in ix.accounts.iter_mut() {
        if m.pubkey == cuts {
            m.pubkey = token::holding_address(&w.sol, &t);
        }
    }
    w.env
        .send_paid_by(&[token::create_holding(t, mint, t), ix], &trader, &[])
        .expect_code(code(LaunchError::WrongPoolCuts));
}

#[test]
fn refresh_pool_registry_names_each_pool_items_accounts() {
    let (mut w, creator, mint, item, _) = market();
    // The stub keeps no registry; one is written at the item registry's address for this test
    // (the real items program writes it in `init_equip`, 02 M2 notes).
    let (registry, _) = Pubkey::find_program_address(
        &[HOOK_ACCOUNTS_SEED, mint.as_ref(), item.as_ref()],
        &pool_item_stub::ID,
    );
    let list = HookAccountList::new(vec![ExtraAccount {
        writable: true,
        source: AccountSource::Key(pool_item_stub::script_address(&item)),
    }]);
    let bytes = list.encode();
    w.env.svm
        .set_account(
            registry,
            solana_account::Account {
                lamports: 10_000_000,
                data: bytes,
                owner: pool_item_stub::ID,
                executable: false,
                rent_epoch: 0,
            },
        )
        .unwrap();
    let ix = sl::refresh_pool_registry(
        creator.pubkey(),
        mint,
        w.sol,
        bordrless_core::policy::LP_FEE_BPS,
        vec![registry],
    );
    w.env.send_paid_by(&[ix], &creator, &[]).ok();
    let pool = w.launch_pool_key(&mint);
    let data = w
        .env
        .svm
        .get_account(&launch::registry_address(&pool))
        .unwrap()
        .data;
    let got = HookAccountList::decode(&data).unwrap();
    let keys: Vec<Pubkey> = got
        .accounts
        .iter()
        .map(|a| match &a.source {
            AccountSource::Key(k) => *k,
            _ => Pubkey::default(),
        })
        .collect();
    // Upstream's four, the PoolCuts holding, then the item.
    assert_eq!(keys.len(), 5 + 3);
    assert_eq!(keys[4], sl::pool_cuts_holding(&mint, &w.sol));
    assert_eq!(keys[5], pool_item_stub::ID);
    assert_eq!(keys[6], sl::item_signer(&pool_item_stub::ID));
    assert_eq!(keys[7], pool_item_stub::script_address(&item));
    // A wrong registry is refused.
    let ix = sl::refresh_pool_registry(
        creator.pubkey(),
        mint,
        w.sol,
        bordrless_core::policy::LP_FEE_BPS,
        vec![Pubkey::new_unique()],
    );
    w.env
        .send_paid_by(&[ix], &creator, &[])
        .expect_code(code(LaunchError::StaleRegistry));
}

#[test]
fn a_slot_launch_with_the_kit_and_a_pool_item_graduates() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let r = presets::rewards_and_cap();
    let (mint, items) = w.slot_launch(&creator, 100, r, vec![pool_slot(300, false)], &[(1, BOTH)]);
    // Items that take nothing keep the launch's own quote math, so the fill helpers apply.
    w.set_stub(&creator, &items[0], ans(0, 0, 0), ans(0, 0, 0), false, false);
    w.env.warp(60);
    let mut n = 0;
    loop {
        let l: Launch = w.env.read(&launch::launch_address(&mint));
        let p = w.launch_pool(&mint);
        if p.quote_reserve >= l.graduation_quote || p.base_reserve == 0 {
            break;
        }
        n += 1;
        assert!(n < 300, "the curve does not fill");
        let wallet = w.env.funded(1_000 * SOL);
        let amount = w.crossing_buy_amount(&mint, &wallet.pubkey());
        w.wrap_sol(&wallet, amount).ok();
        w.slot_buy(&wallet, &mint, amount).ok();
    }
    let cranker = w.env.funded(SOL);
    let ix = w.slot_graduate_ix(&cranker.pubkey(), &mint);
    w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    assert_eq!(l.status, STATUS_GRADUATED);
    // Trading goes on in the same pool, items still forwarded.
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, SOL).ok();
    let calls = w.stub_script(&items[0]).before_calls;
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    assert_eq!(w.stub_script(&items[0]).before_calls, calls + 1);
    let _ = slices_of(&w, &mint, SlotOp::Burn);
}

#[test]
fn upstream_launches_are_not_slot_launches() {
    let mut w = World::new();
    let creator = w.env.funded(100 * SOL);
    let (mint, tx) = w.create_launch(&creator, "UP", 100, VQ);
    tx.ok();
    let l: Launch = w.env.read(&launch::launch_address(&mint));
    assert_eq!(l.slot_launch, 0);
    assert!(!l.is_slot_launch());
}

/// The armory's `equip_launch` of one slot, as `equip_prepared` forwards it.
fn equip_launch_ix(
    hw: &bordrless_program_tests::armory::Hw,
    payer: &Pubkey,
    mint: &Pubkey,
    entry: hookwars_armory::LaunchEquip,
) -> anchor_lang::solana_program::instruction::Instruction {
    use bordrless_program_tests::armory::{armory_events, armory_ix};
    use hookwars_common::{ids, pda};
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

#[test]
fn equip_prepared_equips_launch_items_through_the_armory_before_the_supply() {
    use bordrless_program_tests::armory::{params, Hw};
    use bordrless_program_tests::program_bytes;
    use hookwars_common::{pda, template_id as T, EquipConfig};
    let mut hw = Hw::new();
    // The real launchpad at its id (the armory world loads launch_stub there).
    hw.w.env
        .svm
        .add_program(bordrless_launch::ID, &program_bytes("bordrless_launch"))
        .unwrap();
    let creator = hw.w.env.funded(1_000 * SOL);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let slots = vec![
        bordrless_program_tests::slots::item_slot(
            bordrless_hook::slot_kind::WAR,
            bordrless_hook::equip_rule::VOTE,
            0,
            0,
            false,
        ),
        pool_slot(300, false),
    ];
    hw.w.prepare_launch(&creator, &mint_kp, 100, LaunchRules::NONE, slots)
        .ok();
    let (_, war_orders, _) = hw.item(
        T::WAR_ORDERS,
        params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]),
        0,
    );
    let entry = |slot: u8, item: Option<Pubkey>| hookwars_armory::LaunchEquip {
        slot,
        item,
        config: EquipConfig::default(),
        notice_secs: 600,
        rule: None,
    };
    let c = creator.pubkey();

    // Not the creator: refused.
    let other = hw.w.env.funded(10 * SOL);
    let ix = sl::equip_prepared(
        other.pubkey(),
        mint,
        equip_launch_ix(&hw, &other.pubkey(), &mint, entry(0, Some(war_orders))),
    );
    hw.w.env
        .send_paid_by(&[ix], &other, &[])
        .expect_code(code(LaunchError::WrongCreator));
    // Not equip_launch: refused.
    let mut ix = sl::equip_prepared(
        c,
        mint,
        equip_launch_ix(&hw, &c, &mint, entry(0, Some(war_orders))),
    );
    let bad = bordrless_launch::instruction::EquipPrepared {
        data: vec![0; 16],
    };
    ix.data = anchor_lang::InstructionData::data(&bad);
    hw.w.env
        .send_paid_by(&[ix], &creator, &[])
        .expect_code(code(LaunchError::NotEquipLaunch));

    // The War orders in slot 0, the pool slot left empty with its SlotState (Vote rule).
    let ix = sl::equip_prepared(c, mint, equip_launch_ix(&hw, &c, &mint, entry(0, Some(war_orders))));
    hw.w.env.send_paid_by(&[ix], &creator, &[]).ok();
    let ix = sl::equip_prepared(c, mint, equip_launch_ix(&hw, &c, &mint, entry(1, None)));
    hw.w.env.send_paid_by(&[ix], &creator, &[]).ok();
    let m: Mint = hw.w.env.read(&mint);
    assert_eq!(m.slots[0].item, war_orders);
    assert!(hw.w.env.svm.get_account(&pda::slot_state(&mint, 1).0).is_some());

    let ix = hw.w.create_prepared_ix(&c, &mint, 100, VQ, LaunchRules::NONE);
    hw.w.env
        .send_paid_by(
            &[ix],
            &creator,
            &[&mint_kp],
        )
        .ok();
    let l: Launch = hw.w.env.read(&launch::launch_address(&mint));
    assert!(l.is_slot_launch());
    // After the launch nothing more is equipped through the launchpad.
    let ix = sl::equip_prepared(c, mint, equip_launch_ix(&hw, &c, &mint, entry(1, None)));
    hw.w.env
        .send_paid_by(&[ix], &creator, &[])
        .expect_code(code(LaunchError::NotPrepared));
    // The token trades with its War orders in place (no callbacks).
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, SOL).ok();
    hw.w.slot_buy(&trader, &mint, SOL / 10).ok();
    assert!(hw.w.env.holding(&mint, &trader.pubkey()) > 0);
}
