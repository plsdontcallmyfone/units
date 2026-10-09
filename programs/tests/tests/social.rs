// Changed by Hookwars: new file, `hookwars_social` (docs/spec/10-expansion.md sections 3, 6).
//! Badges (each criterion met and not met, claims open only after the admin delay, one award per
//! recipient, an awarded badge cannot move) and guild halls (threshold, delay, officer changes,
//! SOL and units token spends).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::armory::*;
use bordrless_program_tests::expansion::*;
use bordrless_program_tests::war::WarWorld;
use bordrless_token::client as token;
use bordrless_token::state::Holding;
use hookwars_common::{pda, template_id as t};
use hookwars_social::{BadgeType, Criterion, Guild, GuildActionKind, SocialError};
use hookwars_war::state::WarState;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn badge(env: &bordrless_program_tests::env::Env, id: u32) -> BadgeType {
    env.read(&Pubkey::find_program_address(&[hookwars_social::seeds::BADGE, &id.to_le_bytes()], &hookwars_social::ID).0)
}

#[test]
fn a_forge_level_badge_opens_after_the_delay_and_cannot_move() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let admin = hw.w.env.deployer.insecure_clone();
    let stranger = hw.w.env.funded(SOL);
    // Only the admin creates badges.
    send(&mut hw.w.env, &stranger, &[create_badge_ix(&stranger.pubkey(), 0, "x", Criterion::ForgeLevel { min_level: 1 })])
        .expect_code(social_code(SocialError::NotAdmin));
    send(&mut hw.w.env, &admin, &[create_badge_ix(&admin.pubkey(), 0, "Smith", Criterion::ForgeLevel { min_level: 1 })]).ok();
    send(&mut hw.w.env, &admin, &[create_badge_ix(&admin.pubkey(), 1, "Master", Criterion::ForgeLevel { min_level: 3 })]).ok();
    let (holder, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let extra = |w: &Pubkey| {
        vec![
            AccountMeta::new_readonly(item, false),
            AccountMeta::new_readonly(token::holding_address(&item_mint, w), false),
        ]
    };
    let h = holder.pubkey();
    send(&mut hw.w.env, &holder, &[claim_badge_ix(&h, 0, &h, extra(&h))])
        .expect_code(social_code(SocialError::ClaimsNotOpen));
    hw.w.env.warp(i64::from(TEST_SOCIAL.admin_timelock_secs));
    // Level 1 items do not meet a level 3 badge.
    send(&mut hw.w.env, &holder, &[claim_badge_ix(&h, 1, &h, extra(&h))])
        .expect_code(social_code(SocialError::CriterionNotMet));
    // A wallet that does not hold the item does not meet it either.
    let s = stranger.pubkey();
    send(&mut hw.w.env, &stranger, &[claim_badge_ix(&s, 0, &s, extra(&s))]).expect_fail();
    send(&mut hw.w.env, &holder, &[claim_badge_ix(&h, 0, &h, extra(&h))]).ok();
    let mint = hookwars_social::badge_mint_address(0).0;
    let holding: Holding = hw.w.env.read(&token::holding_address(&mint, &h));
    assert_eq!((holding.amount, holding.frozen), (1, true));
    assert_eq!(badge(&hw.w.env, 0).awarded, 1);
    // One award per recipient.
    send(&mut hw.w.env, &holder, &[claim_badge_ix(&h, 0, &h, extra(&h))]).expect_fail();
    // The badge cannot move.
    let ixs = [
        token::create_holding(h, mint, s),
        token::transfer(h, token::holding_address(&mint, &h), token::holding_address(&mint, &s), mint, None, vec![], 1),
    ];
    hw.w.env.send_paid_by(&ixs, &holder, &[]).expect_fail();
    // A claim for someone else is refused (wallet criteria).
    let other = Keypair::new().pubkey();
    send(&mut hw.w.env, &stranger, &[claim_badge_ix(&s, 0, &other, extra(&other))])
        .expect_code(social_code(SocialError::WrongAccount));
}

/// A launch with a raid slot holding a crafted Raid item at the armory's address, a wallet with
/// `points` raid points this season, and the social program loaded.
fn raid_world(points: u32) -> (WarWorld, Pubkey, Pubkey, Keypair) {
    let mut ww = WarWorld::new();
    let treasury = Keypair::new().pubkey();
    ww.w.env.fund(treasury, SOL);
    load(&mut ww.w.env, treasury);
    let mint = ww.launch("RAIDB", LaunchRules::NONE);
    let item = put_item(&mut ww.w.env, Pubkey::new_unique(), t::RAID, 1);
    ww.add_slot(&mint, WarWorld::raid_slot(item, 0));
    let wallet = ww.w.env.funded(SOL);
    let w = wallet.pubkey();
    ww.w.env.send_paid_by(&[token::create_holding(w, mint, w)], &wallet, &[]).ok();
    let season = ww.config().current_season;
    ww.set_raid(&mint, &w, season, points, 0);
    (ww, mint, item, wallet)
}

#[test]
fn a_raid_points_badge_reads_the_wallets_raid_range_this_season() {
    let (mut ww, mint, item, wallet) = raid_world(40);
    let admin = ww.w.env.deployer.insecure_clone();
    send(&mut ww.w.env, &admin, &[create_badge_ix(&admin.pubkey(), 0, "Raider", Criterion::RaidPoints { mint, min: 50 })]).ok();
    send(&mut ww.w.env, &admin, &[create_badge_ix(&admin.pubkey(), 1, "Scout", Criterion::RaidPoints { mint, min: 40 })]).ok();
    ww.w.env.warp(i64::from(TEST_SOCIAL.admin_timelock_secs));
    let w = wallet.pubkey();
    let extra = vec![
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(token::holding_address(&mint, &w), false),
        AccountMeta::new_readonly(item, false),
        AccountMeta::new_readonly(pda::war_config().0, false),
    ];
    send(&mut ww.w.env, &wallet, &[claim_badge_ix(&w, 0, &w, extra.clone())])
        .expect_code(social_code(SocialError::CriterionNotMet));
    send(&mut ww.w.env, &wallet, &[claim_badge_ix(&w, 1, &w, extra.clone())]).ok();
    // A wrong war config is refused.
    let mut bad = extra;
    bad[3] = AccountMeta::new_readonly(Pubkey::new_unique(), false);
    send(&mut ww.w.env, &wallet, &[claim_badge_ix(&w, 0, &w, bad)]).expect_fail();
}

#[test]
fn a_first_siege_badge_goes_to_the_token_war_chest() {
    let mut ww = WarWorld::new();
    let treasury = Keypair::new().pubkey();
    ww.w.env.fund(treasury, SOL);
    load(&mut ww.w.env, treasury);
    let mint = ww.launch("SIEGE", LaunchRules::NONE);
    ww.put_war_state(&mint);
    let admin = ww.w.env.deployer.insecure_clone();
    send(&mut ww.w.env, &admin, &[create_badge_ix(&admin.pubkey(), 0, "First blood", Criterion::FirstSiege { mint })]).ok();
    ww.w.env.warp(i64::from(TEST_SOCIAL.admin_timelock_secs));
    let chest = hookwars_social::war_chest(&mint);
    let anyone = ww.w.env.funded(SOL);
    let extra = vec![AccountMeta::new_readonly(pda::war_state(&mint).0, false)];
    send(&mut ww.w.env, &anyone, &[claim_badge_ix(&anyone.pubkey(), 0, &chest, extra.clone())])
        .expect_code(social_code(SocialError::CriterionNotMet));
    // The chest spends on a siege (written directly: the siege itself is the war suites').
    let key = pda::war_state(&mint).0;
    let mut s: WarState = ww.w.env.read(&key);
    s.spent_siege = 1;
    put_anchor(&mut ww.w.env, key, hookwars_war::ID, &s, WarState::LEN);
    // Awarding a wallet instead of the chest is refused.
    send(&mut ww.w.env, &anyone, &[claim_badge_ix(&anyone.pubkey(), 0, &anyone.pubkey(), extra.clone())])
        .expect_fail();
    send(&mut ww.w.env, &anyone, &[claim_badge_ix(&anyone.pubkey(), 0, &chest, extra)]).ok();
    let b = hookwars_social::badge_mint_address(0).0;
    assert_eq!(ww.w.env.holding(&b, &chest), 1);
}

#[test]
fn a_guild_spends_only_with_its_threshold_after_the_delay() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let founder = hw.w.env.funded(10 * SOL);
    let second = hw.w.env.funded(SOL);
    let f = founder.pubkey();
    send(&mut hw.w.env, &founder, &[create_guild_ix(&f, 0, "Vanguard")]).ok();
    let donor = hw.w.env.funded(10 * SOL);
    send(&mut hw.w.env, &donor, &[deposit_sol_ix(&donor.pubkey(), 0, 5 * SOL)]).ok();
    let vault = hookwars_social::treasury_address(0).0;
    assert_eq!(hw.w.env.lamports(&vault), 5 * SOL);

    // Officers become two with threshold 2.
    let set = GuildActionKind::SetOfficers {
        officers: vec![f, second.pubkey()],
        threshold: 2,
    };
    send(&mut hw.w.env, &founder, &[propose_action_ix(&f, 0, 0, set)]).ok();
    let anyone = hw.w.env.funded(SOL);
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 0, &f, vec![])])
        .expect_code(social_code(SocialError::TooEarly));
    hw.w.env.warp(i64::from(TEST_SOCIAL.guild_timelock_secs));
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 0, &f, vec![])]).ok();
    let g: Guild = hw.w.env.read(&guild_address(0));
    assert_eq!((g.officers.len(), g.threshold, g.officers_version), (2, 2, 1));

    // A stranger cannot propose; one approval is not enough; two are.
    let to = Keypair::new().pubkey();
    hw.w.env.fund(to, SOL);
    let spend = GuildActionKind::SpendSol { to, lamports: 2 * SOL };
    send(&mut hw.w.env, &anyone, &[propose_action_ix(&anyone.pubkey(), 0, 1, spend.clone())])
        .expect_code(social_code(SocialError::NotOfficer));
    send(&mut hw.w.env, &founder, &[propose_action_ix(&f, 0, 1, spend)]).ok();
    hw.w.env.warp(i64::from(TEST_SOCIAL.guild_timelock_secs));
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 1, &to, vec![])])
        .expect_code(social_code(SocialError::NotEnoughApprovals));
    send(&mut hw.w.env, &second, &[approve_action_ix(&second.pubkey(), 0, 1)]).ok();
    let t0 = hw.w.env.lamports(&to);
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 1, &to, vec![])]).ok();
    assert_eq!(hw.w.env.lamports(&to) - t0, 2 * SOL);
    assert_eq!(hw.w.env.lamports(&vault), 3 * SOL);
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 1, &to, vec![])])
        .expect_code(social_code(SocialError::AlreadyExecuted));

    // An action proposed before an officer change can no longer execute.
    let spend = GuildActionKind::SpendSol { to, lamports: SOL };
    send(&mut hw.w.env, &founder, &[propose_action_ix(&f, 0, 2, spend)]).ok();
    send(&mut hw.w.env, &second, &[approve_action_ix(&second.pubkey(), 0, 2)]).ok();
    let set = GuildActionKind::SetOfficers {
        officers: vec![f],
        threshold: 1,
    };
    send(&mut hw.w.env, &founder, &[propose_action_ix(&f, 0, 3, set)]).ok();
    send(&mut hw.w.env, &second, &[approve_action_ix(&second.pubkey(), 0, 3)]).ok();
    hw.w.env.warp(i64::from(TEST_SOCIAL.guild_timelock_secs));
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 3, &f, vec![])]).ok();
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 2, &to, vec![])])
        .expect_code(social_code(SocialError::StaleAction));
    // Bad officer sets are refused.
    let bad = GuildActionKind::SetOfficers {
        officers: vec![f, f],
        threshold: 1,
    };
    send(&mut hw.w.env, &founder, &[propose_action_ix(&f, 0, 4, bad)])
        .expect_code(social_code(SocialError::BadOfficers));
}

#[test]
fn a_guild_treasury_spends_units_tokens() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let founder = hw.w.env.funded(10 * SOL);
    let f = founder.pubkey();
    send(&mut hw.w.env, &founder, &[create_guild_ix(&f, 0, "Hoard")]).ok();
    let vault = hookwars_social::treasury_address(0).0;
    let mint = hw.w.mint_to_owner(&founder, 6, 1_000_000, "HRD");
    let ixs = [
        token::create_holding(f, mint, vault),
        token::transfer(f, token::holding_address(&mint, &f), token::holding_address(&mint, &vault), mint, None, vec![], 400_000),
    ];
    hw.w.env.send_paid_by(&ixs, &founder, &[]).ok();
    let to = Keypair::new().pubkey();
    hw.w.env.send_paid_by(&[token::create_holding(f, mint, to)], &founder, &[]).ok();
    let spend = GuildActionKind::SpendToken { mint, to, amount: 150_000 };
    send(&mut hw.w.env, &founder, &[propose_action_ix(&f, 0, 0, spend)]).ok();
    hw.w.env.warp(i64::from(TEST_SOCIAL.guild_timelock_secs));
    let extra = vec![
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(token::holding_address(&mint, &vault), false),
        AccountMeta::new(token::holding_address(&mint, &to), false),
    ];
    let anyone = hw.w.env.funded(SOL);
    send(&mut hw.w.env, &anyone, &[execute_action_ix(0, 0, &to, extra)]).ok();
    assert_eq!(hw.w.env.holding(&mint, &to), 150_000);
    assert_eq!(hw.w.env.holding(&mint, &vault), 250_000);
}
