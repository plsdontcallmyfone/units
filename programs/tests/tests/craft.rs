// Changed by Hookwars: new file, `hookwars_craft` (docs/spec/11-hook-economy.md section 5).
//! Drops only from registered caller PDAs and within season caps (R42); drop rules and recipes
//! behind the timelock; craft burns inputs, pays the recipe fee split exactly and asks the output
//! program to make the item; the level gate; wear turns an item dormant exactly at
//! `max_charges`, once, never failing the caller (R38); repair restores; material supply equals
//! emitted minus burned.

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::params;
use bordrless_program_tests::economy::*;
use hookwars_common::economy::{self as eco, counter};
use hookwars_common::template_id as t;
use hookwars_craft::error::CraftError as E;
use hookwars_craft::state::{recipe_kind, source};
use solana_keypair::Keypair;
use solana_signer::Signer;

const MAT: u16 = 1;
const MAT2: u16 = 2;

/// A world with two materials, a SettleCrank rule (1 unit per 1,000 measured) in effect.
fn world(cap: u64) -> Ew {
    let mut ew = Ew::new();
    ew.create_material(MAT, cap);
    ew.create_material(MAT2, cap);
    ew.drop_rule(source::SETTLE_CRANK, MAT, 1, 1_000);
    ew
}

fn supply(ew: &Ew, id: u16) -> u64 {
    let mint: bordrless_token::state::Mint = ew.hw.w.env.read(&hookwars_craft::state::material_mint_address(id).0);
    mint.supply
}

#[test]
fn drops_come_only_from_registered_caller_pdas_and_stop_at_the_season_cap() {
    let mut ew = world(100);
    let alice = ew.funded(SOL).pubkey();
    // A stranger signing as itself, naming the registered program, is refused.
    let stranger = ew.funded(SOL);
    let ix = ew.drop_ix(&stranger.pubkey(), &STUB, &stranger.pubkey(), source::SETTLE_CRANK, MAT, 10_000, &alice);
    ew.send(&stranger, &[ix]).expect_code(craft_code(E::NotCaller));
    // A program not in the config's callers is refused even with its own PDA.
    let other = Pubkey::new_unique();
    let pda = eco::caller_pda(eco::CRAFT_CALLER_SEED, &other).0;
    let mut ix = ew.drop_ix(&pda, &other, &stranger.pubkey(), source::SETTLE_CRANK, MAT, 10_000, &alice);
    ix.accounts[0].is_signer = false;
    ew.send(&stranger, &[ix]).expect_fail();
    // The registered caller drops measured / 1,000.
    ew.stub_drop(source::SETTLE_CRANK, MAT, 42_999, &alice).ok();
    let mint = hookwars_craft::state::material_mint_address(MAT).0;
    assert_eq!(ew.holding(&mint, &alice), 42);
    // Up to the cap, then nothing more this season (never a failure).
    ew.stub_drop(source::SETTLE_CRANK, MAT, 1_000_000, &alice).ok();
    assert_eq!(ew.holding(&mint, &alice), 100);
    ew.stub_drop(source::SETTLE_CRANK, MAT, 1_000_000, &alice).ok();
    assert_eq!(ew.holding(&mint, &alice), 100);
    assert_eq!(ew.material(MAT).emitted_this_season, 100);
    // A new season resets the cap.
    ew.warp(i64::from(TEST_CRAFT.season_secs));
    ew.stub_drop(source::SETTLE_CRANK, MAT, 5_000, &alice).ok();
    assert_eq!(ew.holding(&mint, &alice), 105);
    let m = ew.material(MAT);
    assert_eq!((m.emitted_this_season, m.emitted_total), (5, 105));
    assert_eq!(supply(&ew, MAT), 105);
}

#[test]
fn drop_rules_and_caps_wait_for_the_timelock() {
    let mut ew = Ew::new();
    ew.create_material(MAT, 1_000);
    ew.create_material(MAT2, 1_000);
    let alice = ew.funded(SOL).pubkey();
    let d = ew.deployer();
    let ix = ew.set_drop_rule_ix(source::RAID_REVEAL, hookwars_craft::state::DropTerms { material_id: MAT, per_unit_num: 2, per_unit_den: 1 });
    ew.send(&d, &[ix]).ok();
    // Not yet in effect: nothing drops, nothing fails.
    ew.stub_drop(source::RAID_REVEAL, MAT, 10, &alice).ok();
    let mint = hookwars_craft::state::material_mint_address(MAT).0;
    assert_eq!(ew.holding(&mint, &alice), 0);
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    ew.stub_drop(source::RAID_REVEAL, MAT, 10, &alice).ok();
    assert_eq!(ew.holding(&mint, &alice), 20);
    // A change points the rule at MAT2; until the delay it still drops MAT.
    let ix = ew.set_drop_rule_ix(source::RAID_REVEAL, hookwars_craft::state::DropTerms { material_id: MAT2, per_unit_num: 1, per_unit_den: 1 });
    ew.send(&d, &[ix]).ok();
    ew.stub_drop(source::RAID_REVEAL, MAT, 1, &alice).ok();
    assert_eq!(ew.holding(&mint, &alice), 22);
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    // The rule now names MAT2: passing MAT is refused, MAT2 drops.
    ew.stub_drop(source::RAID_REVEAL, MAT, 1, &alice).expect_fail();
    ew.stub_drop(source::RAID_REVEAL, MAT2, 3, &alice).ok();
    assert_eq!(ew.holding(&hookwars_craft::state::material_mint_address(MAT2).0, &alice), 3);
    // A malformed rule is refused.
    let ix = ew.set_drop_rule_ix(source::COUNT, hookwars_craft::state::DropTerms { material_id: MAT, per_unit_num: 1, per_unit_den: 1 });
    ew.send(&d, &[ix]).expect_code(craft_code(E::BadDropRule));
    let ix = ew.set_drop_rule_ix(source::QUEST_CLAIM, hookwars_craft::state::DropTerms { material_id: MAT, per_unit_num: 1, per_unit_den: 0 });
    ew.send(&d, &[ix]).expect_code(craft_code(E::BadDropRule));
    // Only the admin sets rules.
    let stranger = ew.funded(SOL);
    let mut ix = ew.set_drop_rule_ix(source::QUEST_CLAIM, hookwars_craft::state::DropTerms { material_id: MAT, per_unit_num: 1, per_unit_den: 1 });
    ix.accounts[0].pubkey = stranger.pubkey();
    ew.send(&stranger, &[ix]).expect_code(craft_code(E::NotAdmin));
}

/// A crafter holding `n` of MAT and MAT2.
fn crafter(ew: &mut Ew, n: u64) -> Keypair {
    let c = ew.funded(10 * SOL);
    ew.drop_rule(source::SEASON_FINISH, MAT2, 1, 1);
    ew.stub_drop(source::SETTLE_CRANK, MAT, n * 1_000, &c.pubkey()).ok();
    ew.stub_drop(source::SEASON_FINISH, MAT2, n, &c.pubkey()).ok();
    c
}

#[test]
fn craft_burns_inputs_pays_the_fee_split_and_calls_the_output_program() {
    let mut ew = world(10_000);
    let c = crafter(&mut ew, 50);
    let inputs = [(MAT, 10), (MAT2, 4)];
    let fee = 2_000_003;
    let mut r = recipe(recipe_kind::CRAFT, &inputs, fee, t::TRANSFER_FEE);
    r.param_min = params(&[10, 0]);
    r.param_max = params(&[200, 0]);
    ew.create_recipe(1, r).ok();
    // Not usable before the delay.
    let ix = ew.craft_item_ix(&c.pubkey(), 1, &inputs);
    ew.send(&c, &[ix]).expect_code(craft_code(E::RecipeClosed));
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    ew.open_profile(&c);
    let (t0, p0) = (ew.lamports(&ew.craft_treasury), ew.lamports(&ew.season_pool));
    let ix = ew.craft_item_ix(&c.pubkey(), 1, &inputs);
    let tx = ew.send(&c, &[ix]);
    tx.ok();
    let protocol = fee * u64::from(TEST_CRAFT.recipe_protocol_bps) / 10_000;
    assert_eq!(ew.lamports(&ew.craft_treasury) - t0, protocol);
    assert_eq!(ew.lamports(&ew.season_pool) - p0, fee - protocol);
    assert_eq!(ew.craft_config().protocol_fees_total, u128::from(protocol));
    let m1 = hookwars_craft::state::material_mint_address(MAT).0;
    let m2 = hookwars_craft::state::material_mint_address(MAT2).0;
    assert_eq!(ew.holding(&m1, &c.pubkey()), 40);
    assert_eq!(ew.holding(&m2, &c.pubkey()), 46);
    assert_eq!(ew.material(MAT).burned_total, 10);
    // The output program got the request signed by the craft signer, with the recipe's ranges.
    let signer = hookwars_craft::state::signer_address().0;
    assert!(logged(&tx, &format!("mint_crafted signer={signer} crafter={} template={} recipe=1 min0=10 max0=200", c.pubkey(), t::TRANSFER_FEE)));
    // The crafter's counter moved through social (craft is a registered caller).
    assert_eq!(ew.profile(&c.pubkey()).counters[usize::from(counter::ITEMS_CRAFTED)], 1);
    // Supply conservation: minted minus burned.
    for id in [MAT, MAT2] {
        let m = ew.material(id);
        assert_eq!(supply(&ew, id), m.emitted_total - m.burned_total);
    }
}

#[test]
fn craft_refusals_and_the_crafter_level_gate() {
    let mut ew = world(10_000);
    let c = crafter(&mut ew, 40);
    let inputs = [(MAT, 5)];
    let mut gated = recipe(recipe_kind::CRAFT, &inputs, 1_000, t::TRANSFER_FEE);
    gated.min_level = 2;
    ew.create_recipe(2, gated).ok();
    ew.create_recipe(3, recipe(recipe_kind::CRAFT, &inputs, 1_000, t::TRANSFER_FEE)).ok();
    ew.create_recipe(4, recipe(recipe_kind::REPAIR, &inputs, 1_000, t::TRANSFER_FEE)).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    // Level 0 without a profile.
    let ix = ew.craft_item_ix(&c.pubkey(), 2, &inputs);
    ew.send(&c, &[ix]).expect_code(craft_code(E::LevelTooLow));
    ew.open_profile(&c);
    // Earned through crafting: Crafter thresholds 1, 3 (TEST).
    for _ in 0..3 {
        let ix = ew.craft_item_ix(&c.pubkey(), 3, &inputs);
        ew.send(&c, &[ix]).ok();
    }
    let ix = ew.craft_item_ix(&c.pubkey(), 2, &inputs);
    ew.send(&c, &[ix]).ok();
    // Another wallet's profile cannot be credited (nor lend its level).
    let other = ew.funded(SOL);
    ew.open_profile(&other);
    let mut ix = ew.craft_item_ix(&c.pubkey(), 3, &inputs);
    ix.accounts[8].pubkey = hookwars_social::profile_address(&other.pubkey()).0;
    ew.send(&c, &[ix]).expect_fail();
    assert_eq!(ew.profile(&other.pubkey()).counters[usize::from(counter::ITEMS_CRAFTED)], 0);
    // A repair recipe cannot craft.
    let ix = ew.craft_item_ix(&c.pubkey(), 4, &inputs);
    ew.send(&c, &[ix]).expect_code(craft_code(E::WrongRecipe));
    // Not enough material: the burn fails, nothing is paid.
    let poor = ew.funded(SOL);
    let t0 = ew.lamports(&ew.craft_treasury);
    let ix = ew.craft_item_ix(&poor.pubkey(), 3, &inputs);
    ew.send(&poor, &[ix]).expect_fail();
    assert_eq!(ew.lamports(&ew.craft_treasury), t0);
    // Closing a recipe waits for the delay too.
    let mut closed = recipe(recipe_kind::CRAFT, &inputs, 1_000, t::TRANSFER_FEE);
    closed.active = false;
    ew.set_recipe(3, closed).ok();
    let ix = ew.craft_item_ix(&c.pubkey(), 3, &inputs);
    ew.send(&c, &[ix]).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    let ix = ew.craft_item_ix(&c.pubkey(), 3, &inputs);
    ew.send(&c, &[ix]).expect_code(craft_code(E::RecipeClosed));
    // Malformed recipes are refused.
    ew.create_recipe(9, recipe(recipe_kind::CRAFT, &[], 0, t::TRANSFER_FEE)).expect_code(craft_code(E::BadRecipe));
    ew.create_recipe(9, recipe(recipe_kind::CRAFT, &[(MAT, 1), (MAT, 2)], 0, t::TRANSFER_FEE))
        .expect_code(craft_code(E::BadRecipe));
    ew.create_recipe(9, recipe(recipe_kind::CRAFT, &[(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)], 0, t::TRANSFER_FEE))
        .expect_code(craft_code(E::BadRecipe));
}

#[test]
fn wear_turns_an_item_dormant_exactly_at_max_charges_and_repair_restores() {
    let mut ew = world(10_000);
    let (author, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    // Only a registered caller opens wear.
    let stranger = ew.funded(SOL);
    let ix = ew.init_wear_ix(&stranger.pubkey(), &STUB, &stranger.pubkey(), &item, 10);
    ew.send(&stranger, &[ix]).expect_code(craft_code(E::NotCaller));
    ew.stub_init_wear(&item, 10).ok();
    ew.stub_wear(&item, 9).ok();
    let w = ew.wear(&item);
    assert!(!w.dormant && w.used == 9 && w.left() == 1);
    let turn = ew.stub_wear(&item, 1);
    turn.ok();
    assert!(ew.wear(&item).dormant);
    // Past the limit: never a failure, stays dormant, used stays at max, and `ItemWorn` (an
    // event CPI) is not emitted again.
    let again = ew.stub_wear(&item, 5);
    again.ok();
    assert_eq!(turn.inner_len(), again.inner_len() + 1);
    let w = ew.wear(&item);
    assert!(w.dormant && w.used == 10);
    // Only the holder repairs, with a repair recipe of the item's template.
    let inputs = [(MAT, 3)];
    ew.drop_rule(source::QUEST_CLAIM, MAT, 1, 1);
    ew.stub_drop(source::QUEST_CLAIM, MAT, 10, &author.pubkey()).ok();
    ew.stub_drop(source::QUEST_CLAIM, MAT, 10, &stranger.pubkey()).ok();
    ew.create_recipe(5, recipe(recipe_kind::REPAIR, &inputs, 50_000, t::TRANSFER_FEE)).ok();
    ew.create_recipe(6, recipe(recipe_kind::REPAIR, &inputs, 50_000, t::SHIELD)).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    let ix = ew.repair_ix(&stranger.pubkey(), 5, &inputs, &item, &item_mint);
    ew.send(&stranger, &[ix]).expect_fail();
    let ix = ew.repair_ix(&author.pubkey(), 6, &inputs, &item, &item_mint);
    ew.send(&author, &[ix]).expect_code(craft_code(E::WrongRecipe));
    let (t0, p0) = (ew.lamports(&ew.craft_treasury), ew.lamports(&ew.season_pool));
    let ix = ew.repair_ix(&author.pubkey(), 5, &inputs, &item, &item_mint);
    ew.send(&author, &[ix]).ok();
    let w = ew.wear(&item);
    assert!(!w.dormant && w.used == 5 && w.repairs == 1);
    assert_eq!(ew.lamports(&ew.craft_treasury) - t0, 5_000);
    assert_eq!(ew.lamports(&ew.season_pool) - p0, 45_000);
    assert_eq!(ew.holding(&hookwars_craft::state::material_mint_address(MAT).0, &author.pubkey()), 7);
    // Wearing again turns it dormant once more.
    ew.stub_wear(&item, 5).ok();
    assert!(ew.wear(&item).dormant);
}

#[test]
fn items_with_zero_max_charges_never_wear_and_cannot_be_repaired() {
    let mut ew = world(10_000);
    let (author, item, item_mint) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    ew.stub_init_wear(&item, 0).ok();
    ew.stub_wear(&item, 1_000_000).ok();
    let w = ew.wear(&item);
    assert!(!w.dormant && w.used == 0 && w.left() == u32::MAX);
    let inputs = [(MAT, 1)];
    ew.drop_rule(source::QUEST_CLAIM, MAT, 1, 1);
    ew.stub_drop(source::QUEST_CLAIM, MAT, 5, &author.pubkey()).ok();
    ew.create_recipe(5, recipe(recipe_kind::REPAIR, &inputs, 0, t::TRANSFER_FEE)).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    let ix = ew.repair_ix(&author.pubkey(), 5, &inputs, &item, &item_mint);
    ew.send(&author, &[ix]).expect_code(craft_code(E::NeverWears));
    // An item without a Wear account reads as never wearing.
    let (_, other, _) = ew.hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    assert!(ew.hw.w.env.account(&hookwars_craft::state::wear_address(&other).0).is_none());
}

#[test]
fn config_changes_wait_for_the_timelock() {
    let mut ew = world(10_000);
    let d = ew.deployer();
    let mut p = TEST_CRAFT;
    p.recipe_protocol_bps = 2_000;
    let new_treasury = ew.funded(SOL).pubkey();
    let propose = craft_ix(
        hookwars_craft::accounts::ProposeConfig {
            admin: d.pubkey(),
            config: hookwars_craft::state::config_address().0,
            pending: hookwars_craft::state::pending_address().0,
            system_program: anchor_lang::system_program::ID,
            event_authority: bordrless_program_tests::expansion::events(&hookwars_craft::ID),
            program: hookwars_craft::ID,
        },
        hookwars_craft::instruction::ProposeConfig {
            treasury: new_treasury,
            season_pool: ew.season_pool,
            output_program: STUB,
            callers: vec![STUB],
            params: p,
        },
    );
    ew.send(&d, &[propose]).ok();
    let apply = craft_ix(
        hookwars_craft::accounts::ApplyConfig {
            config: hookwars_craft::state::config_address().0,
            pending: hookwars_craft::state::pending_address().0,
        },
        hookwars_craft::instruction::ApplyConfig {},
    );
    ew.send(&d, std::slice::from_ref(&apply)).expect_code(craft_code(E::NotReady));
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    ew.send(&d, &[apply]).ok();
    let c = ew.craft_config();
    assert_eq!((c.treasury, c.params.recipe_protocol_bps), (new_treasury, 2_000));
}
