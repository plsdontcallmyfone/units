// Changed by Hookwars: new file (M2), equip rules: holder votes locked in place, quorum, notice,
// execute, cancel, closing, Locked slots, and the Performance revert (docs/spec/02-armory.md 6); security review 2: the registry refresh accounts on a revert.

use anchor_lang::prelude::Pubkey;
use anchor_lang::AccountSerialize;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::*;
use bordrless_program_tests::slots::item_slot;
use bordrless_token::error::TokenError;
use bordrless_token::state::Holding;
use hookwars_armory::error::ArmoryError as E;
use hookwars_armory::state::{proposal_status as S, Item, Proposal, SlotState};
use hookwars_common::{ids, pda, template_id as T, EquipConfig, PerformanceRule};
use hookwars_items::EquipState;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SUPPLY_OWNER: u64 = 1_000;

fn war_params(threshold: u32) -> hookwars_common::Params {
    params(&[threshold, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10])
}

/// A token with the test slots, War orders `a` in slot 0 at launch, and supply: owner 1,000,
/// voter 300, small voter 100.
fn setup(hw: &mut Hw) -> (Keypair, Pubkey, Pubkey, Keypair, Keypair) {
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, a, _) = hw.item(T::WAR_ORDERS, war_params(10), 0);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(a), EquipConfig::default()))
        .ok();
    let voter = hw.w.env.funded(1_000_000_000);
    let small = hw.w.env.funded(1_000_000_000);
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, SUPPLY_OWNER);
    hw.mint_to(&owner, &mint, &voter.pubkey(), 300);
    hw.mint_to(&owner, &mint, &small.pubkey(), 100);
    (owner, mint, a, voter, small)
}

#[test]
fn a_vote_locks_in_place_and_equips_after_its_notice() {
    let mut hw = Hw::new();
    let (owner, mint, a, voter, small) = setup(&mut hw);
    let (_, b, _) = hw.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, proposal) = hw.propose(&voter, &mint, 0, Some(b), EquipConfig::default());
    tx.ok();
    let p: Proposal = hw.w.env.read(&proposal);
    assert_eq!(p.status, S::OPEN);
    assert_eq!(p.executable_at, p.vote_end + 600);
    // One open proposal per slot.
    let (tx, _) = hw.propose(&small, &mint, 0, Some(a), EquipConfig::default());
    tx.expect_fail();

    hw.vote(&voter, &mint, &proposal, true, 300).ok();
    hw.vote(&small, &mint, &proposal, false, 100).ok();
    hw.vote(&small, &mint, &proposal, false, 100).expect_fail();
    // The tokens stay in place, locked until the vote ends.
    let h: Holding = hw.w.env.read(&pda::holding(&mint, &voter.pubkey()));
    assert_eq!((h.amount, h.vote_locked, h.vote_lock_until), (300, 300, p.vote_end));
    let send = hw
        .w
        .slot_transfer_ix(&voter.pubkey(), &mint, &owner.pubkey(), 1);
    hw.w.env
        .send_paid_by(&[send.clone()], &voter, &[])
        .expect_code(anchor_lang::error::ERROR_CODE_OFFSET + TokenError::VoteLocked as u32);

    hw.finalize(&proposal)
        .expect_code(armory_code(E::VotingNotEnded));
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::PASSED);
    // The lock lifted with the vote.
    hw.w.env.send_paid_by(&[send], &voter, &[]).ok();

    hw.execute(&owner, &proposal)
        .expect_code(armory_code(E::NotExecutable));
    hw.w.env.warp(600);
    hw.execute(&owner, &proposal).ok();
    assert_eq!(hw.slot_item(&mint, 0), b);
    assert_eq!(hw.read_item(&a).equipped_count, 0);
    assert_eq!(hw.read_item(&b).equipped_count, 1);
    let st: EquipState = hw.w.env.read(&pda::equip_state(&mint, 0).0);
    assert_eq!(st.item, b);
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::EXECUTED);
    let ss: SlotState = hw.w.env.read(&pda::slot_state(&mint, 0).0);
    assert_eq!(ss.open_proposal, None);
    hw.execute(&owner, &proposal).expect_fail();

    // Closing: votes first, then the proposal; rent goes back.
    let close_proposal = armory_ix(
        hookwars_armory::accounts::CloseProposal {
            proposer: voter.pubkey(),
            proposal,
        },
        hookwars_armory::instruction::CloseProposal {},
    );
    hw.w.env
        .send_paid_by(&[close_proposal.clone()], &owner, &[])
        .expect_code(armory_code(E::VotesOpen));
    for v in [&voter, &small] {
        let ix = armory_ix(
            hookwars_armory::accounts::CloseVote {
                voter: v.pubkey(),
                proposal,
                vote_lock: pda::vote_lock(&proposal, &v.pubkey()).0,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::CloseVote {},
        );
        hw.w.env.send_paid_by(&[ix], &owner, &[]).ok();
    }
    let before = hw.w.env.lamports(&voter.pubkey());
    hw.w.env.send_paid_by(&[close_proposal], &owner, &[]).ok();
    assert!(hw.w.env.lamports(&voter.pubkey()) > before);
    assert!(hw.w.env.account(&proposal).is_none_or(|a| a.lamports == 0));
}

#[test]
fn below_quorum_fails_and_frees_the_slot() {
    let mut hw = Hw::new();
    let (owner, mint, _a, _voter, small) = setup(&mut hw);
    let (_, b, _) = hw.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, proposal) = hw.propose(&small, &mint, 0, Some(b), EquipConfig::default());
    tx.ok();
    // 100 of 1,400 eligible is below the TEST quorum of 10%.
    hw.vote(&small, &mint, &proposal, true, 100).ok();
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    assert_eq!(hw.w.env.read::<Proposal>(&proposal).status, S::FAILED);
    hw.execute(&owner, &proposal)
        .expect_code(armory_code(E::NotExecutable));
    let (tx, _) = hw.propose(&small, &mint, 0, Some(b), EquipConfig::default());
    tx.ok();
}

fn cancel_ix(mint: &Pubkey, proposal: Pubkey, who: &Keypair) -> anchor_lang::solana_program::instruction::Instruction {
    armory_ix(
        hookwars_armory::accounts::Cancel {
            proposer: who.pubkey(),
            proposal,
            slot_state: pda::slot_state(mint, 0).0,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::Cancel {},
    )
}

#[test]
fn the_proposer_cancels_only_without_votes() {
    let mut hw = Hw::new();
    let (_owner, mint, _a, voter, small) = setup(&mut hw);
    let (_, b, _) = hw.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, proposal) = hw.propose(&voter, &mint, 0, Some(b), EquipConfig::default());
    tx.ok();
    // Not the proposer.
    hw.w.env
        .send_paid_by(&[cancel_ix(&mint, proposal, &small)], &small, &[])
        .expect_fail();
    hw.vote(&small, &mint, &proposal, true, 10).ok();
    hw.w.env
        .send_paid_by(&[cancel_ix(&mint, proposal, &voter)], &voter, &[])
        .expect_code(armory_code(E::VotesExist));
    // Without votes: cancelled, and the slot is free again.
    let mut hw2 = Hw::new();
    let (_o2, mint2, _, voter2, _) = setup(&mut hw2);
    let (_, b2, _) = hw2.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, p2) = hw2.propose(&voter2, &mint2, 0, Some(b2), EquipConfig::default());
    tx.ok();
    hw2.w.env
        .send_paid_by(&[cancel_ix(&mint2, p2, &voter2)], &voter2, &[])
        .ok();
    assert_eq!(hw2.w.env.read::<Proposal>(&p2).status, S::CANCELLED);
    let ss: SlotState = hw2.w.env.read(&pda::slot_state(&mint2, 0).0);
    assert_eq!(ss.open_proposal, None);
}

#[test]
fn a_locked_slot_takes_no_proposals() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(
        &owner,
        vec![item_slot(slot_kind::WAR, equip_rule::LOCKED, 0, 0, false)],
    );
    let (_, a, _) = hw.item(T::WAR_ORDERS, war_params(10), 0);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(a), EquipConfig::default()))
        .ok();
    assert_eq!(hw.slot_item(&mint, 0), a);
    let (_, b, _) = hw.item(T::WAR_ORDERS, war_params(20), 0);
    let (tx, _) = hw.propose(&owner, &mint, 0, Some(b), EquipConfig::default());
    tx.expect_code(armory_code(E::SlotLocked));
}

/// Puts a fake launch, pool and observation ring for `mint` (the DEX part is built in parallel,
/// M3): entries `(ts, swap_count)`, the pool's current `swap_count`.
fn fake_market(hw: &mut Hw, mint: &Pubkey, entries: &[(i64, u64)], swaps_now: u64) {
    let pool = Pubkey::new_unique();
    let mut launch = vec![0u8; 565];
    launch[..8].copy_from_slice(&hookwars_common::LAUNCH_DISCRIMINATOR);
    launch[10..42].copy_from_slice(mint.as_ref());
    launch[74..106].copy_from_slice(pool.as_ref());
    let rent = hw.w.env.rent(launch.len());
    hw.w.env.put(
        pda::launch(mint).0,
        Account {
            lamports: rent,
            data: launch,
            owner: ids::LAUNCH_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    let p = bordrless_swap::state::Pool {
        version: 1,
        bump: 255,
        lp_mint_bump: 255,
        base_mint: *mint,
        quote_mint: hw.w.sol,
        lp_mint: Pubkey::new_unique(),
        base_vault: Pubkey::new_unique(),
        quote_vault: Pubkey::new_unique(),
        hook_program: None,
        hook_flags: 0,
        lp_fee_bps: 30,
        protocol_fee_bps: 0,
        base_reserve: 1,
        quote_reserve: 1,
        virtual_base: 0,
        virtual_quote: 0,
        lp_supply: 0,
        protocol_fees_quote: 0,
        curve: false,
        creator: Pubkey::new_unique(),
        created_at: 0,
        last_swap_at: 0,
        swap_count: swaps_now,
        base_volume: 0,
        quote_volume: 0,
        hook_signer_bump: 0,
        fee_model: 0,
        protocol_share_bps: 0,
        reserved: [0; 60],
    };
    let mut data = Vec::new();
    p.try_serialize(&mut data).unwrap();
    let rent = hw.w.env.rent(data.len());
    hw.w.env.put(
        pool,
        Account {
            lamports: rent,
            data,
            owner: ids::SWAP_ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    // Hookwars M3b: the ring lives in the pool's tail (03 M3a notes).
    let e: Vec<bordrless_program_tests::ring::RingEntry> =
        entries.iter().map(|(t, sc)| (*t, 0, 0, *sc)).collect();
    let last_ts = entries.last().map_or(0, |e| e.0);
    let mut account = hw.w.env.account(&pool).expect("pool");
    account.data = bordrless_program_tests::ring::with_ring(
        account.data,
        &bordrless_program_tests::ring::ring(&pool, 0, last_ts, &e),
    );
    account.lamports = hw.w.env.rent(account.data.len());
    hw.w.env.put(pool, account);
}

#[test]
fn a_performance_slot_reverts_to_its_launch_item() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let partner = Pubkey::new_unique();
    let cfg = EquipConfig {
        targets: vec![partner],
        role: 0,
    };
    let (_, t1, _) = hw.item(T::TREATY, params(&[50, 50, 0]), 0);
    let (_, t2, _) = hw.item(T::TREATY, params(&[100, 100, 1]), 0);
    let rule = PerformanceRule {
        metric: 2,
        window_secs: 60,
        base_window_secs: 600,
        op: 0,
        ratio_bps: 10_000,
        hold_secs: 120,
    };
    // A Performance slot needs its rule.
    hw.equip_launch(&owner, &mint, Hw::entry(3, Some(t1), cfg.clone()))
        .expect_code(armory_code(E::InvalidRule));
    let mut e = Hw::entry(3, Some(t1), cfg.clone());
    e.rule = Some(rule);
    hw.equip_launch(&owner, &mint, e).ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    // Holders vote in t2.
    let (tx, proposal) = hw.propose(&owner, &mint, 3, Some(t2), cfg.clone());
    tx.ok();
    hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    hw.w.env.warp(600);
    hw.execute(&owner, &proposal).ok();
    assert_eq!(hw.slot_item(&mint, 3), t2);

    // No swaps in the last minute against 100 over ten: the condition holds.
    let now = hw.w.env.now;
    fake_market(&mut hw, &mint, &[(now - 600, 0), (now - 60, 100)], 100);
    let check = |hw: &Hw| {
        let launch = pda::launch(&mint).0;
        let data = hw.w.env.account(&launch).unwrap().data;
        let pool = Pubkey::new_from_array(data[74..106].try_into().unwrap());
        let equip = hw.equip_accounts(&o, &mint, 3, Some(t2), Some(t1));
        let ix = armory_ix(
            hookwars_armory::accounts::CheckPerformance {
                config: pda::config().0,
                slot_state: pda::slot_state(&mint, 3).0,
                launch,
                pool,
                open_proposal: None,
                equip,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::CheckPerformance {},
        );
        // Security review 2, L-D: the registry refresh's accounts.
        let mut ix = ix;
        ix.accounts.extend(hw.refresh_tail(&mint, 3));
        ix
    };
    let ix = check(&hw);
    hw.w.env.send_paid_by(&[ix], &owner, &[]).ok();
    let ss: SlotState = hw.w.env.read(&pda::slot_state(&mint, 3).0);
    assert_eq!(ss.condition_since, Some(now));
    assert_eq!(hw.slot_item(&mint, 3), t2);
    hw.w.env.warp(120);
    let now = hw.w.env.now;
    fake_market(&mut hw, &mint, &[(now - 600, 0), (now - 60, 100)], 100);
    let ix = check(&hw);
    hw.w.env.send_paid_by(&[ix], &owner, &[]).ok();
    assert_eq!(hw.slot_item(&mint, 3), t1);
    // Re-aimed as at launch.
    let es: EquipState = hw.w.env.read(&pda::equip_state(&mint, 3).0);
    assert_eq!((es.item, &es.config), (t1, &cfg));
    let i1: Item = hw.read_item(&t1);
    let i2: Item = hw.read_item(&t2);
    assert_eq!((i1.equipped_count, i2.equipped_count), (1, 0));
    let ss: SlotState = hw.w.env.read(&pda::slot_state(&mint, 3).0);
    assert_eq!(ss.condition_since, None);
}

#[test]
fn a_rate_that_recovers_clears_the_condition() {
    let rule = PerformanceRule {
        metric: 2,
        window_secs: 60,
        base_window_secs: 600,
        op: 0,
        ratio_bps: 10_000,
        hold_secs: 120,
    };
    // Built directly from the reader: 20 swaps in the last minute against 100 over ten (rate 1/3
    // against 1/6 per second) is not "below".
    let now = 10_000i64;
    let pool = Pubkey::new_unique();
    let r = bordrless_program_tests::ring::ring(&pool, 0, now, &[(now - 600, 0, 0, 0), (now - 60, 0, 0, 80)]);
    let read = |w: i64| {
        bordrless_core::observations::window_read(&r, &bordrless_swap::obs::OBSERVATIONS_DISCRIMINATOR, now, w, 1, 0, 100)
            .ok()
            .map(|x| hookwars_common::WindowRead {
                twap_q64: x.twap_q64,
                quote_volume: x.quote_volume,
                swaps: x.swaps,
                seconds: x.span_secs,
            })
    };
    let short = read(60);
    let base = read(600);
    assert!(!rule.holds(short, base));
    // Too short a history is no signal.
    assert!(read(3_600).is_none());
    assert!(!rule.holds(None, base));
}
