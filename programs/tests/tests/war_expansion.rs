// Changed by Hookwars: new file (pass 4b, 10 sections 8, 11.1, 11.3).
//! The war side of spec 10 section 17: the boss pool (funded from the prize split, sealed from the
//! boss's raid ledger, claimed per source chest), coalitions with a shared chest (form, contribute
//! within the item's cap, joint siege, raze, pro rata dissolve) and rivalry budgets (ring-fenced in
//! the token's own chest, score only, nothing moves between chests, R36). Chest solvency after
//! every step.

use anchor_lang::prelude::Pubkey;
use anchor_lang::Discriminator;
use bordrless_hook::slot_kind;
use bordrless_launch::client as launch;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_token::client as token;
use hookwars_war::client::{self as war, Member};
use hookwars_war::constants::{COALITION_TEMPLATE, RIVALRY_TEMPLATE};
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use hookwars_war::foreign::{template_address, PARAM_FIELDS};
use hookwars_war::state::*;
use solana_signer::Signer;

fn by_raid_volume() -> ScoreWeights {
    ScoreWeights {
        raid_volume_won: 1,
        ..ScoreWeights::default()
    }
}

fn wrap(owner: &Pubkey) -> anchor_lang::solana_program::instruction::Instruction {
    bordrless_bridge::client::wrap_sol(*owner, 0)
}

fn unwrap(owner: &Pubkey) -> anchor_lang::solana_program::instruction::Instruction {
    bordrless_bridge::client::unwrap_sol(*owner, 0)
}

/// Sends `lamports` into the prize vault (as the DEX's fee collection would).
fn fill_vault(ww: &mut WarWorld, lamports: u64) {
    let donor = ww.w.env.funded(lamports + SOL);
    let vault = prize_vault_address().0;
    ww.w.env
        .send_paid_by(
            &[anchor_lang::solana_program::system_instruction::transfer(&donor.pubkey(), &vault, lamports)],
            &donor,
            &[],
        )
        .ok();
}

#[test]
fn the_equip_state_discriminator_war_decodes_is_the_items_one() {
    assert_eq!(
        hookwars_war::foreign::disc::EQUIP_STATE,
        <hookwars_items::EquipState as Discriminator>::DISCRIMINATOR
    );
}

// ---- boss -------------------------------------------------------------------------------------------

#[test]
fn the_boss_pool_takes_its_share_and_pays_source_chests_by_volume() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("SRCA", OrdersSpec::default());
    let b = ww.war_token("SRCB", OrdersSpec::default());
    let c = ww.war_token("NONE", OrdersSpec::default());
    let boss = Pubkey::new_unique();
    let admin = ww.w.env.deployer.insecure_clone();
    let s = ww.open_season(by_raid_volume());
    // Only the admin names the boss.
    let stranger = ww.w.env.funded(SOL);
    ww.w.env
        .send(&[war::init_boss_pool(stranger.pubkey(), s.number, boss)], &[&stranger])
        .expect_code(war_code(WarError::NotAdmin));
    ww.w.env.send(&[war::init_boss_pool(admin.pubkey(), s.number, boss)], &[&admin]).ok();
    // Review 3 L-7: a named boss takes no share before the admin timelock.
    ww.w.env.warp(i64::from(TEST_PARAMS.admin_timelock_secs));

    fill_vault(&mut ww, 10 * SOL);
    let vault = prize_vault_address().0;
    let available = ww.w.env.lamports(&vault) - ww.w.env.rent(0);
    let cranker = ww.w.env.funded(SOL);
    let treasury = ww.w.env.treasury.pubkey();
    let inner = [unwrap(&vault)];
    let ix = war::split_protocol_fees_with_boss(cranker.pubkey(), treasury, None, s.number, &inner);
    let tx = ww.w.env.send_paid_by(&[ix], &cranker, &[]);
    let funded = tx.event::<BossPoolFunded>();
    let paid = tx.event::<PrizePaid>();
    assert_eq!(funded.amount, available * u64::from(TEST_PARAMS.boss_share_bps) / 10_000);
    assert_eq!(funded.amount + paid.to_treasury + paid.bounty, available);
    let pool: BossPool = ww.w.env.read(&BossPool::address(s.number).0);
    assert_eq!(pool.funded, funded.amount);
    println!("split with boss: cu {}", tx.cu());

    // The boss ledger: A raided 100, B 150 + 50 over two windows; season 0 (the Boss item names
    // no war config).
    let now = ww.now();
    ww.put_ledger(&boss, 0, 300, &[(a.mint, now, 100, 0), (b.mint, now, 150, 50)]);
    ww.w.env
        .send(&[war::seal_boss_pool(s.number, boss)], &[])
        .expect_code(war_code(WarError::SeasonNotEnded));
    ww.w.env.warp(s.ends_at - ww.now() + 1);
    let tx = ww.w.env.send(&[war::seal_boss_pool(s.number, boss)], &[]);
    let sealed = tx.event::<BossPoolSealed>();
    assert_eq!((sealed.total_volume, sealed.sources, sealed.to_share), (300, 2, funded.amount));
    ww.w.env
        .send(&[war::seal_boss_pool(s.number, boss)], &[])
        .expect_code(war_code(WarError::BossPoolState));
    // A sealed pool takes no more funding (review 3 M-3: the split goes on without it).
    fill_vault(&mut ww, SOL);
    let ix = war::split_protocol_fees_with_boss(cranker.pubkey(), treasury, None, s.number, &inner);
    ww.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    assert_eq!(ww.w.env.read::<BossPool>(&BossPool::address(s.number).0).funded, funded.amount);

    let before = ww.chest_balance(&a.mint);
    let funded_before = ww.state(&a.mint).funded_total;
    let tx = ww.w.env.send(&[war::claim_boss_share(s.number, a.mint, &[wrap(&WarWorld::chest(&a.mint))])], &[]);
    let claim = tx.event::<BossShareClaimed>();
    assert_eq!(claim.amount, funded.amount * 100 / 300);
    assert_eq!(ww.chest_balance(&a.mint), before + claim.amount);
    let st = ww.state(&a.mint);
    assert_eq!(st.received_other, claim.amount);
    // Not season funding: the M-B score cap does not grow with it.
    assert_eq!(st.funded_total, funded_before);
    ww.assert_solvent(&a.mint);
    println!("claim_boss_share: cu {}", tx.cu());
    // Once per source.
    ww.w.env.warp(1);
    ww.w.env
        .send(&[war::claim_boss_share(s.number, a.mint, &[wrap(&WarWorld::chest(&a.mint))])], &[])
        .expect_code(war_code(WarError::NoBossShare));
    // A token that never raided the boss has no share.
    ww.w.env
        .send(&[war::claim_boss_share(s.number, c.mint, &[wrap(&WarWorld::chest(&c.mint))])], &[])
        .expect_code(war_code(WarError::NoBossShare));
    let tx = ww.w.env.send(&[war::claim_boss_share(s.number, b.mint, &[wrap(&WarWorld::chest(&b.mint))])], &[]);
    assert_eq!(tx.event::<BossShareClaimed>().amount, funded.amount * 200 / 300);
    ww.assert_solvent(&b.mint);
    let pool: BossPool = ww.w.env.read(&BossPool::address(s.number).0);
    assert!(pool.paid <= pool.funded && pool.funded - pool.paid <= 1);
}

// ---- coalitions -------------------------------------------------------------------------------------

/// Writes the Coalition template and gives `t` a Coalition item naming `id` with `bps`.
fn join(ww: &mut WarWorld, t: &WarToken, id: u32, bps: u32) -> Member {
    let mut params = [0u32; PARAM_FIELDS];
    params[0] = id;
    params[1] = bps;
    let item = ww.put_item(COALITION_TEMPLATE, params);
    ww.add_slot(&t.mint, WarWorld::named_slot(slot_kind::RELATION, item));
    Member {
        mint: t.mint,
        item,
        template: template_address(COALITION_TEMPLATE),
    }
}

fn coalition_template(ww: &mut WarWorld) {
    let mut max = [0u32; PARAM_FIELDS];
    max[0] = u32::MAX;
    max[1] = 10_000;
    ww.put_template(COALITION_TEMPLATE, slot_kind::RELATION, 2, [0; PARAM_FIELDS], max, false);
}

fn contribute_ix(ww: &WarWorld, cranker: &Pubkey, id: u32, m: Member, amount: u64) -> anchor_lang::solana_program::instruction::Instruction {
    let shared = Coalition::chest(id).0;
    let inner = [
        unwrap(&WarWorld::chest(&m.mint)),
        wrap(&shared),
        token::create_holding(*cranker, ww.w.sol, shared),
    ];
    war::contribute(*cranker, id, m, amount, &inner)
}

fn shared_balance(ww: &WarWorld, id: u32) -> u64 {
    ww.w.env.holding(&ww.w.sol, &Coalition::chest(id).0)
}

#[track_caller]
fn assert_coalition_solvent(ww: &WarWorld, id: u32) {
    let c: Coalition = ww.w.env.read(&Coalition::address(id).0);
    let b = shared_balance(ww, id);
    assert_eq!(c.expected_balance(), Some(b), "coalition totals vs balance");
    assert_eq!(c.last_seen_balance, b);
}

#[test]
fn a_coalition_forms_contributes_sieges_razes_and_dissolves_pro_rata() {
    let mut ww = WarWorld::new();
    coalition_template(&mut ww);
    let a = ww.war_token("CA", OrdersSpec::default());
    let b = ww.war_token("CB", OrdersSpec::default());
    let c = ww.war_token("CC", OrdersSpec::default());
    let rival = ww.launch("RIV", LaunchRules::NONE);
    ww.buyer(&rival, SOL);
    let id = 7;
    let ma = join(&mut ww, &a, id, 5_000);
    let mb = join(&mut ww, &b, id, 5_000);
    let mc = join(&mut ww, &c, id + 1, 5_000);
    let payer = ww.w.env.payer.insecure_clone();
    // Refused: one member; a member whose item names another coalition; a duplicate.
    ww.w.env.send(&[war::form_coalition(payer.pubkey(), id, 86_400, &[ma])], &[]).expect_code(war_code(WarError::InvalidCoalition));
    ww.w.env.send(&[war::form_coalition(payer.pubkey(), id, 86_400, &[ma, mc])], &[]).expect_code(war_code(WarError::InvalidCoalition));
    ww.w.env.send(&[war::form_coalition(payer.pubkey(), id, 86_400, &[ma, ma])], &[]).expect_code(war_code(WarError::InvalidCoalition));
    let tx = ww.w.env.send(&[war::form_coalition(payer.pubkey(), id, 86_400, &[ma, mb])], &[]);
    let formed = tx.event::<CoalitionFormed>();
    assert_eq!(formed.members, vec![a.mint, b.mint]);
    println!("form_coalition (2): cu {}", tx.cu());

    ww.fund_chest(&a.mint, 10 * SOL);
    ww.fund_chest(&b.mint, 10 * SOL);
    // Above the item's cap (50% of the chest), then within it, then inside the interval.
    let (tx, _) = ww.crank(|k, ww| contribute_ix(ww, k, id, ma, 5 * SOL + 1));
    tx.expect_code(war_code(WarError::ContributionLimit));
    let (tx, _) = ww.crank(|k, ww| contribute_ix(ww, k, id, ma, 2 * SOL));
    assert_eq!(tx.event::<CoalitionContributed>().contributed, 2 * SOL);
    println!("contribute: cu {}", tx.cu());
    let (tx, _) = ww.crank(|k, ww| contribute_ix(ww, k, id, ma, SOL));
    tx.expect_code(war_code(WarError::ContributionLimit));
    ww.crank(|k, ww| contribute_ix(ww, k, id, mb, 3 * SOL)).0.ok();
    assert_eq!(ww.state(&a.mint).sent_coalition, 2 * SOL);
    ww.assert_solvent(&a.mint);
    ww.assert_solvent(&b.mint);
    assert_eq!(shared_balance(&ww, id), 5 * SOL);
    assert_coalition_solvent(&ww, id);
    // A non-member may not contribute.
    let (tx, _) = ww.crank(|k, ww| contribute_ix(ww, k, id, mc, SOL));
    tx.expect_code(war_code(WarError::InvalidCoalition));

    // The joint siege: A's War orders and raid ledger make it due.
    ww.w.env.warp(120);
    let now = ww.now();
    let threshold = u64::from(OrdersSpec::default().siege_threshold) * TEST_PARAMS.siege_unit_lamports;
    ww.put_ledger(&a.mint, 0, threshold, &[(rival, now, threshold, 0), (b.mint, now, threshold, 0)]);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let siege = |target: Pubkey| {
        move |k: &Pubkey, ww: &WarWorld| {
            let shared = Coalition::chest(id).0;
            let keys = ww.w.launch_keys(&target);
            let inner = [
                launch::swap_with_base_slice(&keys, shared, shared, 1, 1, 0, vec![]),
                token::create_holding(*k, target, shared),
                unwrap(&shared),
            ];
            war::coalition_siege(*k, id, a.mint, a.orders, target, ww.w.launch_pool_key(&target), None, vec![], &inner)
        }
    };
    // A member is not a target.
    let (tx, _) = ww.crank(siege(b.mint));
    tx.expect_code(war_code(WarError::SelfSiege));
    let (tx, _) = ww.crank(siege(rival));
    let e = tx.event::<CoalitionSiegeExecuted>();
    assert!(e.spent > 0 && e.bought > 0);
    println!("coalition_siege: spent {} bought {} cu {}", e.spent, e.bought, tx.cu());
    let co: Coalition = ww.w.env.read(&Coalition::address(id).0);
    assert_eq!((co.captured[0].rival_mint, co.captured[0].amount), (rival, e.bought));
    assert_eq!(ww.w.env.holding(&rival, &Coalition::chest(id).0), e.bought);
    assert_coalition_solvent(&ww, id);
    // The interval holds for the coalition.
    ww.w.env.warp(1);
    let (tx, _) = ww.crank(siege(rival));
    tx.expect_code(war_code(WarError::SiegeNotDue));

    let raze = |k: &Pubkey, ww: &WarWorld| {
        let shared = Coalition::chest(id).0;
        let keys = ww.w.launch_keys(&rival);
        let inner = [
            launch::swap_with_base_slice(&keys, shared, shared, 0, 1, 0, vec![]),
            token::create_holding(*k, rival, shared),
            unwrap(&shared),
        ];
        war::coalition_raze(*k, id, rival, ww.w.launch_pool_key(&rival), vec![], &inner)
    };
    // During the term: the raze rate limit (25% of the window's base here).
    let (tx, _) = ww.crank(raze);
    let r = tx.event::<CoalitionRazed>();
    assert_eq!(r.sold, e.bought * u64::from(TEST_PARAMS.raze_max_bps_per_interval) / 10_000);
    assert_coalition_solvent(&ww, id);
    ww.w.env.warp(1);
    let (tx, _) = ww.crank(raze);
    tx.expect_code(war_code(WarError::RazeLimit));
    // Not before the term's end, and not with captured tokens left.
    let dissolve = |_: &WarWorld| {
        let shared = Coalition::chest(id).0;
        let inner = [unwrap(&shared), wrap(&WarWorld::chest(&a.mint)), wrap(&WarWorld::chest(&b.mint))];
        war::dissolve_coalition(id, &[a.mint, b.mint], &inner)
    };
    let ix = dissolve(&ww);
    ww.w.env.send(&[ix], &[]).expect_code(war_code(WarError::CoalitionNotDone));
    ww.w.env.warp(86_400);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let ix = dissolve(&ww);
    ww.w.env.send(&[ix], &[]).expect_code(war_code(WarError::CoalitionNotDone));
    // After the term the shared chest winds down without the rate limit.
    for _ in 0..4 {
        let co: Coalition = ww.w.env.read(&Coalition::address(id).0);
        if co.captured[0].amount == 0 {
            break;
        }
        ww.crank(raze).0.ok();
        ww.w.env.warp(1);
    }
    let co: Coalition = ww.w.env.read(&Coalition::address(id).0);
    assert_eq!(co.captured[0].amount, 0);
    assert_coalition_solvent(&ww, id);
    // No contribution after the term.
    let (tx, _) = ww.crank(|k, ww| contribute_ix(ww, k, id, mb, SOL));
    tx.expect_code(war_code(WarError::CoalitionClosed));

    let total = shared_balance(&ww, id);
    let (ba, bb) = (ww.chest_balance(&a.mint), ww.chest_balance(&b.mint));
    let ix = dissolve(&ww);
    let tx = ww.w.env.send(&[ix], &[]);
    let d = tx.event::<CoalitionDissolved>();
    let share_a = (u128::from(total) * 2 / 5) as u64;
    assert_eq!(d.returned, vec![share_a, total - share_a]);
    assert_eq!(ww.chest_balance(&a.mint), ba + share_a);
    assert_eq!(ww.chest_balance(&b.mint), bb + total - share_a);
    assert_eq!(ww.state(&a.mint).received_other, share_a);
    ww.assert_solvent(&a.mint);
    ww.assert_solvent(&b.mint);
    assert_eq!(shared_balance(&ww, id), 0);
    assert_coalition_solvent(&ww, id);
    println!("dissolve_coalition (2): cu {}", tx.cu());
    let ix = dissolve(&ww);
    ww.w.env.warp(1);
    ww.w.env.send(&[ix], &[]).expect_code(war_code(WarError::CoalitionClosed));
}

// ---- rivalries --------------------------------------------------------------------------------------

/// Gives `t` a Rivalry item aimed at `rival` (its `EquipState` written in the items layout);
/// answers the item and its slot.
fn rivalry(ww: &mut WarWorld, t: &WarToken, rival: &Pubkey, starts_at: i64, secs: u32, bps: u32) -> (Pubkey, u8) {
    let mut max = [0u32; PARAM_FIELDS];
    max[..3].copy_from_slice(&[u32::MAX, u32::MAX, 10_000]);
    ww.put_template(RIVALRY_TEMPLATE, slot_kind::RELATION, 3, [0; PARAM_FIELDS], max, false);
    let mut params = [0u32; PARAM_FIELDS];
    params[..3].copy_from_slice(&[starts_at as u32, secs, bps]);
    let item = ww.put_item(RIVALRY_TEMPLATE, params);
    let slot = ww.add_slot(&t.mint, WarWorld::named_slot(slot_kind::RELATION, item));
    let (key, bump) = hookwars_common::pda::equip_state(&t.mint, slot);
    let es = hookwars_items::EquipState {
        version: 1,
        bump,
        mint: t.mint,
        slot,
        item,
        template_id: RIVALRY_TEMPLATE,
        config: hookwars_common::EquipConfig {
            targets: vec![*rival],
            role: 0,
        },
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
    (item, slot)
}

#[test]
fn a_rivalry_ring_fences_its_budget_and_settles_into_the_score_only() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("RA", OrdersSpec::default());
    let b = ww.war_token("RB", OrdersSpec::default());
    ww.buyer(&b.mint, SOL);
    ww.fund_chest(&a.mint, 10 * SOL);
    let now = ww.now();
    let (item, slot) = rivalry(&mut ww, &a, &b.mint, now - 10, 86_400, 2_000);
    let tx = ww.w.env.send(&[war::open_rivalry(a.mint, item, slot)], &[]);
    let o = tx.event::<RivalryOpened>();
    assert_eq!((o.rival_mint, o.budget), (b.mint, 2 * SOL));
    println!("open_rivalry: cu {}", tx.cu());
    ww.w.env.warp(1);
    ww.w.env.send(&[war::open_rivalry(a.mint, item, slot)], &[]).expect_code(war_code(WarError::RivalryState));
    let st = ww.state(&a.mint);
    let t = ww.now();
    // Ring-fenced: other targets see the chest less the budget; the rival sees all of it.
    let other = Pubkey::new_unique();
    assert_eq!(hookwars_war::instructions::siege_base(&st.rivalry, &other, t, 10 * SOL), 8 * SOL);
    assert_eq!(hookwars_war::instructions::siege_base(&st.rivalry, &b.mint, t, 10 * SOL), 10 * SOL);
    assert_eq!(hookwars_war::instructions::counter_base(&st, t, 10 * SOL), 8 * SOL);

    // A siege of the rival spends the budget first.
    ww.w.env.warp(120);
    let t = ww.now();
    let threshold = u64::from(OrdersSpec::default().siege_threshold) * TEST_PARAMS.siege_unit_lamports;
    ww.put_ledger(&a.mint, 0, threshold, &[(b.mint, t, threshold, 0)]);
    let pool = ww.w.launch_pool_key(&b.mint);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let (tx, _) = ww.crank(|k, ww| ww.siege_ix(k, &a, &b.mint, true, None));
    let e = tx.event::<SiegeExecuted>();
    assert_eq!(ww.state(&a.mint).rivalry.spent, e.spent.min(2 * SOL));
    ww.assert_solvent(&a.mint);

    // Not settled before the end.
    ww.w.env.send(&[war::settle_rivalry(a.mint, b.mint)], &[]).expect_code(war_code(WarError::RivalryState));
    // The result: A's ledger shows 500 from B, B's ledger 100 from A.
    ww.w.env.warp(86_400);
    let t = ww.now();
    ww.put_ledger(&a.mint, 0, 0, &[(b.mint, t, 500, 0)]);
    ww.put_ledger(&b.mint, 0, 0, &[(a.mint, t, 100, 0)]);
    let (ca, cb) = (ww.chest_balance(&a.mint), ww.chest_balance(&b.mint));
    let tx = ww.w.env.send(&[war::settle_rivalry(a.mint, b.mint)], &[]);
    let r = tx.event::<RivalrySettled>();
    assert!(r.won && !r.early);
    assert_eq!((r.ours, r.theirs), (500, 100));
    println!("settle_rivalry: cu {}", tx.cu());
    let st = ww.state(&a.mint);
    assert_eq!(st.season.rivalry_wins, 1);
    assert!(!st.rivalry.is_open());
    // Nothing moved between the chests (R36).
    assert_eq!((ww.chest_balance(&a.mint), ww.chest_balance(&b.mint)), (ca, cb));
    ww.assert_solvent(&a.mint);
    // The score weighs the counter.
    let season = Season {
        version: 1,
        bump: 0,
        number: 1,
        starts_at: 0,
        ends_at: 0,
        weights: ScoreWeights {
            rivalry_wins: 7,
            ..ScoreWeights::default()
        },
        penalize_besieged: false,
        eta: 0,
        opened: true,
        leader: None,
        leader_score: 0,
        finalized: false,
        prize_paid: 0,
        reserved: [0; 32],
    };
    assert_eq!(season.score(&st.season), Some(7));
}

/// Pass 5 (review 3 I-4): a passive rival (no raids back) gives no win; a contested rivalry does,
/// once per rival per season.
#[test]
fn p5_i4_rivalry_wins_need_a_contest_and_count_once_per_rival_per_season() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("FA", OrdersSpec::default());
    let b = ww.war_token("FB", OrdersSpec::default());
    ww.fund_chest(&a.mint, 10 * SOL);
    let settle = |ww: &mut WarWorld, theirs: u64| -> bool {
        let now = ww.now();
        let (item, slot) = rivalry(ww, &a, &b.mint, now - 10, 60, 1_000);
        ww.w.env.send(&[war::open_rivalry(a.mint, item, slot)], &[]).ok();
        ww.w.env.warp(61);
        let t = ww.now();
        ww.put_ledger(&a.mint, 0, 0, &[(b.mint, t, 500, 0)]);
        if theirs > 0 {
            ww.put_ledger(&b.mint, 0, 0, &[(a.mint, t, theirs, 0)]);
        }
        let tx = ww.w.env.send(&[war::settle_rivalry(a.mint, b.mint)], &[]);
        let won = tx.event::<RivalrySettled>().won;
        // Free the slot for the next rivalry item.
        ww.write_mint(&a.mint, |m| {
            m.slots[usize::from(slot)] = Default::default();
            m.slot_count -= 1;
        });
        won
    };
    // Passive rival: no win.
    assert!(!settle(&mut ww, 0));
    assert_eq!(ww.state(&a.mint).season.rivalry_wins, 0);
    // Contested: a win.
    assert!(settle(&mut ww, 100));
    assert_eq!(ww.state(&a.mint).season.rivalry_wins, 1);
    // The same rival again in the same season: not counted.
    assert!(!settle(&mut ww, 100));
    assert_eq!(ww.state(&a.mint).season.rivalry_wins, 1);
}

#[test]
fn unequipping_the_rivalry_ends_it_early_with_no_win_and_no_transfer() {
    let mut ww = WarWorld::new();
    let a = ww.war_token("EA", OrdersSpec::default());
    let b = ww.war_token("EB", OrdersSpec::default());
    ww.fund_chest(&a.mint, 10 * SOL);
    let now = ww.now();
    let (item, slot) = rivalry(&mut ww, &a, &b.mint, now - 10, 86_400, 1_000);
    ww.w.env.send(&[war::open_rivalry(a.mint, item, slot)], &[]).ok();
    let t = ww.now();
    ww.put_ledger(&a.mint, 0, 0, &[(b.mint, t, 500, 0)]);
    ww.w.env.send(&[war::settle_rivalry(a.mint, b.mint)], &[]).expect_code(war_code(WarError::RivalryState));
    ww.write_mint(&a.mint, |m| m.slots[usize::from(slot)].item = Pubkey::default());
    let ca = ww.chest_balance(&a.mint);
    let tx = ww.w.env.send(&[war::settle_rivalry(a.mint, b.mint)], &[]);
    let r = tx.event::<RivalrySettled>();
    assert!(r.early && !r.won);
    assert_eq!(ww.state(&a.mint).season.rivalry_wins, 0);
    assert_eq!(ww.chest_balance(&a.mint), ca);
    ww.assert_solvent(&a.mint);
    // A token that equips no Rivalry item cannot open one.
    let other = Pubkey::new_unique();
    ww.w.env
        .send(&[war::open_rivalry(b.mint, other, 2)], &[])
        .expect_code(war_code(WarError::ItemNotEquipped));
}

// ---- the economy from war (11 E-4, E-6) ------------------------------------------------------------

/// Review: `reveal` with the craft drop suffix and the social suffix drops one material unit per
/// revealed raid ticket to the raider (`RAID_REVEAL`) and counts `RAIDS` on the raider's profile;
/// without the suffixes it is unchanged (the `loot.rs` suite).
#[test]
fn a_revealed_raid_drops_through_craft_and_counts_raids_in_social() {
    use anchor_lang::prelude::AccountMeta;
    use anchor_lang::solana_program::instruction::Instruction;
    use anchor_lang::{InstructionData, ToAccountMetas};
    use bordrless_program_tests::armory::Hw;
    use bordrless_program_tests::economy::{Ew, STUB};
    use hookwars_common::eco_cpi::drop_source;
    use hookwars_common::economy::counter;

    const MAT: u16 = 1;
    let mut ww = WarWorld::new();
    let t = ww.war_token("DROP", OrdersSpec::default());
    let s = ww.open_season(ScoreWeights::default());
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, 0, 2);
    let loot_log = Pubkey::find_program_address(&[b"loot-log", holder.pubkey().as_ref()], &war_armory_stub::ID).0;
    let log = war_armory_stub::LootLog {
        owner: holder.pubkey(),
        ..Default::default()
    };
    ww.put_anchor(loot_log, war_armory_stub::ID, &log, 0);

    // Craft and social loaded and configured with war as a caller of both.
    let WarWorld { w } = ww;
    let admin = w.env.deployer.insecure_clone();
    let mut ew = Ew::with(Hw { w, admin }, STUB, vec![STUB, hookwars_war::ID], vec![hookwars_war::ID]);
    let material_mint = ew.create_material(MAT, 1_000_000);
    ew.drop_rule(drop_source::RAID_REVEAL, MAT, 1, 1);
    ew.open_profile(&holder);
    let mut ww = WarWorld { w: ew.hw.w };

    let randomness_of = |roll: &Pubkey| Pubkey::find_program_address(&[b"randomness", roll.as_ref()], &randomness_stub::ID).0;
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let roll = RollRequest::address(&holding, 1).0;
    let ix = war::roll(holder.pubkey(), t.mint, 1, RAID_SLOT, randomness_stub::ID, randomness_of(&roll), vec![]);
    ww.w.env.send_paid_by(&[ix], &holder, &[]).ok();
    ww.w.env.warp(1);
    let oracle = ww.w.env.funded(SOL);
    let fulfill = Instruction {
        program_id: randomness_stub::ID,
        accounts: randomness_stub::accounts::Fulfill {
            authority: oracle.pubkey(),
            randomness: randomness_of(&roll),
        }
        .to_account_metas(None),
        data: randomness_stub::instruction::Fulfill { value: [7; 32] }.data(),
    };
    ww.w.env.send(&[fulfill], &[&oracle]).ok();
    let mut extra = vec![AccountMeta::new(loot_log, false)];
    extra.extend(war::drop_common());
    extra.extend(war::drop_suffix(drop_source::RAID_REVEAL, MAT, holder.pubkey()));
    extra.extend(war::social_suffix(holder.pubkey()));
    let revealer = ww.w.env.funded(SOL);
    let ix = war::reveal(revealer.pubkey(), holder.pubkey(), holding, 1, s.number, randomness_of(&roll), extra);
    let tx = ww.w.env.send_paid_by(&[ix], &revealer, &[]);
    tx.event::<RollRevealed>();
    println!("reveal with drop and RAIDS: cu {}", tx.cu());
    assert_eq!(ww.w.env.holding(&material_mint, &holder.pubkey()), 1, "one unit per revealed raid");
    let p: hookwars_social::Profile = ww.w.env.read(&hookwars_common::eco_cpi::profile_address(&holder.pubkey()));
    assert_eq!(p.counters[usize::from(counter::RAIDS)], 1);
    // A drop suffix naming another recipient is refused.
    let roll2 = RollRequest::address(&holding, 2).0;
    let ix = war::roll(holder.pubkey(), t.mint, 2, RAID_SLOT, randomness_stub::ID, randomness_of(&roll2), vec![]);
    ww.w.env.send_paid_by(&[ix], &holder, &[]).ok();
    ww.w.env.warp(1);
    let fulfill = Instruction {
        program_id: randomness_stub::ID,
        accounts: randomness_stub::accounts::Fulfill {
            authority: oracle.pubkey(),
            randomness: randomness_of(&roll2),
        }
        .to_account_metas(None),
        data: randomness_stub::instruction::Fulfill { value: [9; 32] }.data(),
    };
    ww.w.env.send(&[fulfill], &[&oracle]).ok();
    let mut extra = vec![AccountMeta::new(loot_log, false)];
    extra.extend(war::drop_common());
    extra.extend(war::drop_suffix(drop_source::RAID_REVEAL, MAT, revealer.pubkey()));
    let ix = war::reveal(revealer.pubkey(), holder.pubkey(), holding, 2, s.number, randomness_of(&roll2), extra);
    ww.w.env.send_paid_by(&[ix], &revealer, &[]).expect_code(war_code(WarError::WrongAccount));
}

// ---- security review 3 --------------------------------------------------------------------------------

/// M-3: the cranker cannot leave the boss pool out of the split (any other account at its place is
/// refused); L-7: a newly named boss takes no share before the admin timelock.
#[test]
fn sf3_the_boss_share_is_not_the_crankers_choice_and_waits_for_the_timelock() {
    let mut ww = WarWorld::new();
    let admin = ww.w.env.deployer.insecure_clone();
    let s = ww.open_season(by_raid_volume());
    ww.w.env.send(&[war::init_boss_pool(admin.pubkey(), s.number, Pubkey::new_unique())], &[&admin]).ok();
    fill_vault(&mut ww, 10 * SOL);
    let vault = prize_vault_address().0;
    let cranker = ww.w.env.funded(SOL);
    let treasury = ww.w.env.treasury.pubkey();
    let inner = [unwrap(&vault)];
    // The review's proof: the boss pool's place filled with anything else.
    for other in [Pubkey::new_unique(), BossPool::address(s.number + 1).0] {
        let mut ix = war::split_protocol_fees(cranker.pubkey(), treasury, None, s.number, &inner);
        ix.accounts[7] = anchor_lang::solana_program::instruction::AccountMeta::new(other, false);
        ww.w.env.send_paid_by(&[ix], &cranker, &[]).expect_code(war_code(WarError::WrongAccount));
    }
    // Before the timelock: the split goes on, the pool takes nothing.
    let ix = war::split_protocol_fees(cranker.pubkey(), treasury, None, s.number, &inner);
    ww.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    assert_eq!(ww.w.env.read::<BossPool>(&BossPool::address(s.number).0).funded, 0);
    // After it: funded.
    ww.w.env.warp(TEST_PARAMS.admin_timelock_secs);
    fill_vault(&mut ww, 10 * SOL);
    let available = ww.w.env.lamports(&vault) - ww.w.env.rent(0);
    let ix = war::split_protocol_fees(cranker.pubkey(), treasury, None, s.number, &inner);
    let tx = ww.w.env.send_paid_by(&[ix], &cranker, &[]);
    assert_eq!(tx.event::<BossPoolFunded>().amount, available * u64::from(TEST_PARAMS.boss_share_bps) / 10_000);
}

/// L-3: a least term, and a consenting token joins before the first contribution. M-6: after the
/// term plus the grace period the shared chest dissolves with captured tokens left; razes in
/// caller-sized steps keep paying into the chest and a later dissolve returns that too.
#[test]
fn sf3_coalitions_take_late_joiners_and_dissolve_after_the_grace_with_captured_tokens() {
    let mut ww = WarWorld::new();
    coalition_template(&mut ww);
    let a = ww.war_token("SA", OrdersSpec::default());
    let b = ww.war_token("SB", OrdersSpec::default());
    let c = ww.war_token("SC", OrdersSpec::default());
    let rival = ww.launch("SRIV", LaunchRules::NONE);
    ww.buyer(&rival, SOL);
    let id = 9;
    let ma = join(&mut ww, &a, id, 5_000);
    let mb = join(&mut ww, &b, id, 5_000);
    let mc = join(&mut ww, &c, id, 5_000);
    let payer = ww.w.env.payer.insecure_clone();
    let term = 86_400;
    // A throwaway one-second term is refused.
    ww.w.env
        .send(&[war::form_coalition(payer.pubkey(), id, TEST_PARAMS.coalition_min_term_secs - 1, &[ma, mb])], &[])
        .expect_code(war_code(WarError::InvalidCoalition));
    ww.w.env.send(&[war::form_coalition(payer.pubkey(), id, term, &[ma, mb])], &[]).ok();
    // C consented (its item names the id) and joins; twice is refused.
    let tx = ww.w.env.send(&[war::join_coalition(id, mc)], &[]);
    assert_eq!(tx.event::<CoalitionJoined>().count, 3);
    ww.w.env.warp(1);
    ww.w.env.send(&[war::join_coalition(id, mc)], &[]).expect_code(war_code(WarError::InvalidCoalition));
    ww.fund_chest(&a.mint, 10 * SOL);
    ww.crank(|k, ww| contribute_ix(ww, k, id, ma, 2 * SOL)).0.ok();
    // After the first contribution nobody joins.
    let d = ww.war_token("SD", OrdersSpec::default());
    let md = join(&mut ww, &d, id, 5_000);
    ww.w.env.send(&[war::join_coalition(id, md)], &[]).expect_code(war_code(WarError::CoalitionClosed));

    // A siege captures rival tokens.
    ww.w.env.warp(120);
    let now = ww.now();
    let threshold = u64::from(OrdersSpec::default().siege_threshold) * TEST_PARAMS.siege_unit_lamports;
    ww.put_ledger(&a.mint, 0, threshold, &[(rival, now, threshold, 0)]);
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let (tx, _) = ww.crank(|k, ww| {
        let shared = Coalition::chest(id).0;
        let keys = ww.w.launch_keys(&rival);
        let inner = [
            launch::swap_with_base_slice(&keys, shared, shared, 1, 1, 0, vec![]),
            token::create_holding(*k, rival, shared),
            unwrap(&shared),
        ];
        war::coalition_siege(*k, id, a.mint, a.orders, rival, ww.w.launch_pool_key(&rival), None, vec![], &inner)
    });
    let bought = tx.event::<CoalitionSiegeExecuted>().bought;
    assert!(bought > 0);
    let dissolve = |ww: &WarWorld| {
        let _ = ww;
        let shared = Coalition::chest(id).0;
        let inner = [unwrap(&shared), wrap(&WarWorld::chest(&a.mint)), wrap(&WarWorld::chest(&b.mint)), wrap(&WarWorld::chest(&c.mint))];
        war::dissolve_coalition(id, &[a.mint, b.mint, c.mint], &inner)
    };
    // After the term, inside the grace: captured tokens still block.
    ww.w.env.warp(term);
    ww.w.env.send(&[dissolve(&ww)], &[]).expect_code(war_code(WarError::CoalitionNotDone));
    // After the grace (a rival's sell cap could have blocked every raze): it dissolves anyway.
    ww.w.env.warp(TEST_PARAMS.coalition_grace_secs);
    let total = shared_balance(&ww, id);
    let tx = ww.w.env.send(&[dissolve(&ww)], &[]);
    assert_eq!(tx.event::<CoalitionDissolved>().returned.iter().sum::<u64>(), total);
    let co: Coalition = ww.w.env.read(&Coalition::address(id).0);
    assert!(co.dissolved && co.captured[0].amount == bought);
    // A raze in a step the caller sizes, then a second dissolve returns its proceeds.
    let pool = ww.w.launch_pool_key(&rival);
    let spot = ww.spot(&pool);
    ww.flat_observations(&pool, spot, 3_600);
    let step = bought / 4;
    let (tx, _) = ww.crank(|k, ww| {
        let shared = Coalition::chest(id).0;
        let keys = ww.w.launch_keys(&rival);
        let inner = [
            launch::swap_with_base_slice(&keys, shared, shared, 0, 1, 0, vec![]),
            token::create_holding(*k, rival, shared),
            unwrap(&shared),
        ];
        war::coalition_raze_max(*k, id, rival, ww.w.launch_pool_key(&rival), vec![], &inner, step)
    });
    let r = tx.event::<CoalitionRazed>();
    assert_eq!(r.sold, step);
    let again = shared_balance(&ww, id);
    assert!(again > 0);
    let tx = ww.w.env.send(&[dissolve(&ww)], &[]);
    assert_eq!(tx.event::<CoalitionDissolved>().returned.iter().sum::<u64>(), again);
    assert_eq!(shared_balance(&ww, id), 0);
    ww.assert_solvent(&a.mint);
}
