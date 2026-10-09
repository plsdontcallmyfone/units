// Changed by Hookwars: new file, templates 43 Coalition, 44 Boss, 45 Rivalry (10 sections 8, 11.1,
// 11.3): shapes, params rules, manifests, what each item says, Boss's source rule, and the armory
// registering them and minting their items.

use anchor_lang::prelude::Pubkey;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{Phase, PoolHookArgs, PoolOp, RouteContext};
use bordrless_program_tests::armory::*;
use hookwars_common::{composable, ids, kind, manifest, pool_flags, shape, template_id as t, validate, ParamsError};
use hookwars_items::templates::{boss, coalition, launch_pool_address, rivalry, Env, BUY, SELL};
use solana_signer::Signer;

#[test]
fn the_three_shapes_rules_and_manifests() {
    let c = shape(t::COALITION).unwrap();
    let b = shape(t::BOSS).unwrap();
    let r = shape(t::RIVALRY).unwrap();
    assert_eq!((c.kind, c.field_count, c.forgeable), (kind::RELATION, 2, false));
    assert_eq!((b.kind, b.field_count, b.forgeable), (kind::POOL, 1, false));
    assert_eq!((r.kind, r.field_count, r.forgeable), (kind::RELATION, 3, false));
    for id in [t::COALITION, t::BOSS, t::RIVALRY] {
        assert!(!composable(id), "{id} is a standalone config item");
    }
    assert_eq!(validate(t::COALITION, &params(&[0, 100])), Err(ParamsError::BadParams));
    assert_eq!(validate(t::COALITION, &params(&[1, 0])), Err(ParamsError::BadParams));
    assert_eq!(validate(t::COALITION, &params(&[1, 10_001])), Err(ParamsError::BadParams));
    assert_eq!(validate(t::COALITION, &params(&[1, 2_000])), Ok(()));
    assert_eq!(validate(t::BOSS, &params(&[0])), Err(ParamsError::BadParams));
    assert_eq!(validate(t::BOSS, &params(&[86_400])), Ok(()));
    assert_eq!(validate(t::RIVALRY, &params(&[0, 0, 100])), Err(ParamsError::BadParams));
    assert_eq!(validate(t::RIVALRY, &params(&[0, 60, 10_001])), Err(ParamsError::BadParams));
    assert_eq!(validate(t::RIVALRY, &params(&[1_000, 60, 500])), Ok(()));
    let mb = manifest(t::BOSS, &params(&[86_400]), 0).unwrap();
    assert_eq!(mb.pool_flags, pool_flags::AFTER_SWAP | pool_flags::MARKS);
    assert_eq!((mb.max_cut_buy_bps, mb.max_cut_sell_bps, mb.max_discount_bps, mb.may_burn), (0, 0, 0, false));
    for id in [t::COALITION, t::RIVALRY] {
        let m = manifest(id, &params(&[1, 1, 1]), 1).unwrap();
        assert_eq!((m.kind, m.pool_flags, m.token_flags, m.may_refuse), (kind::RELATION, 0, 0, false));
    }
}

#[test]
fn what_coalition_and_rivalry_items_say() {
    let c = coalition::read(&params(&[7, 2_500]));
    assert_eq!((c.coalition_id, c.max_contribution_bps), (7, 2_500));
    let p = params(&[1_000, 600, 300]);
    let r = rivalry::read(&p);
    assert_eq!((r.starts_at, r.ends_at, r.budget_bps), (1_000, 1_600, 300));
    let rival = Pubkey::new_unique();
    let targets = [rival];
    assert!(!rivalry::live(&p, &targets, &rival, 999));
    assert!(rivalry::live(&p, &targets, &rival, 1_000));
    assert!(rivalry::live(&p, &targets, &rival, 1_599));
    assert!(!rivalry::live(&p, &targets, &rival, 1_600));
    assert!(!rivalry::live(&p, &targets, &Pubkey::new_unique(), 1_100));
}

fn swap_args(direction: u8, input: Pubkey, first_pool: Pubkey, hop_index: u8) -> PoolHookArgs {
    PoolHookArgs {
        op: PoolOp::Swap,
        phase: Phase::After,
        pool: Pubkey::new_unique(),
        base_mint: Pubkey::new_unique(),
        quote_mint: ids::BRIDGED_SOL_MINT,
        actor: Pubkey::new_unique(),
        recipient: Pubkey::new_unique(),
        direction,
        amount_in: 1_000_000,
        amount_out: 1,
        base_reserve: 0,
        quote_reserve: 0,
        virtual_base: 0,
        virtual_quote: 0,
        lp_fee_bps: 0,
        protocol_fee_bps: 0,
        swap_count: 0,
        created_at: 0,
        lp_amount: 0,
        hook_data: vec![],
        route: RouteContext {
            route_input_mint: input,
            route_output_mint: Pubkey::new_unique(),
            first_pool,
            route_amount_in: 1_000_000,
            hop_index,
            hop_count: hop_index + 1,
        },
    }
}

/// Boss counts a buy only when its route started by selling another units token on that token's
/// own launch pool (an attacker-made pool cannot stand in: review 1 M-5).
#[test]
fn boss_counts_only_routes_from_a_source_launch_pool() {
    let boss_mint = Pubkey::new_unique();
    let env = Env {
        mint: boss_mint,
        slot: 0,
        item: Pubkey::new_unique(),
        module: 0,
        params: params(&[86_400]),
        targets: &[],
        role: 0,
        extras: &[],
        now: 0,
        clock_slot: 0,
    };
    let src = Pubkey::new_unique();
    let canonical = launch_pool_address(&src);
    assert_eq!(boss::source(&env, &swap_args(BUY, src, canonical, 1)), Some(src));
    assert_eq!(boss::source(&env, &swap_args(BUY, src, Pubkey::new_unique(), 1)), None);
    assert_eq!(boss::source(&env, &swap_args(SELL, src, canonical, 1)), None);
    assert_eq!(boss::source(&env, &swap_args(BUY, src, canonical, 0)), None);
    assert_eq!(boss::source(&env, &swap_args(BUY, boss_mint, launch_pool_address(&boss_mint), 1)), None);
    let sol = ids::BRIDGED_SOL_MINT;
    assert_eq!(boss::source(&env, &swap_args(BUY, sol, launch_pool_address(&sol), 1)), None);
    // Before-swap and idle sides answer nothing (no extras needed).
    let ctx = ItemPoolContext::default();
    let out = boss::pool(&env, &swap_args(BUY, src, canonical, 1), &ctx, true).unwrap();
    assert_eq!((out.cut, out.burn, out.discount_bps), (0, 0, 0));
    let out = boss::pool(&env, &swap_args(SELL, src, canonical, 1), &ctx, false).unwrap();
    assert_eq!((out.cut, out.burn, out.discount_bps), (0, 0, 0));
}

/// The armory registers the three templates from the items program and mints their items.
#[test]
fn the_armory_registers_them_and_mints_their_items() {
    let mut hw = Hw::new();
    let admin = hw.admin.insecure_clone();
    for (id, fields, max) in [
        (t::COALITION, 2u8, params(&[u32::MAX, 10_000])),
        (t::BOSS, 1, params(&[u32::MAX])),
        (t::RIVALRY, 3, params(&[u32::MAX, u32::MAX, 10_000])),
    ] {
        let s = shape(id).unwrap();
        let args = hookwars_armory::RegisterTemplateArgs {
            id,
            code_hash: [id as u8; 32],
            kind: s.kind,
            field_count: fields,
            field_min: params(&[]),
            field_max: max,
            open_authoring: true,
            loot_enabled: false,
            forge_enabled: false,
            max_level: 1,
            loot_royalty_bps: 0,
            max_targets: 1,
            name: format!("Template {id}"),
        };
        hw.register_with(&admin, ids::ITEMS_ID, args).ok();
    }
    let author = hw.w.env.funded(10_000_000_000);
    let (tx, item, _) = hw.create_item(&author, t::COALITION, params(&[3, 1_500]), 0);
    tx.ok();
    assert_eq!(hw.read_item(&item).template_id, t::COALITION);
    let (tx, item, _) = hw.create_item(&author, t::BOSS, params(&[604_800]), 0);
    tx.ok();
    assert_eq!(hw.read_item(&item).manifest.pool_flags, pool_flags::AFTER_SWAP | pool_flags::MARKS);
    let (tx, _, _) = hw.create_item(&author, t::RIVALRY, params(&[0, 0, 100]), 0);
    tx.expect_fail();
    let (tx, item, _) = hw.create_item(&author, t::RIVALRY, params(&[1_000, 86_400, 100]), 0);
    tx.ok();
    assert_eq!(hw.read_item(&item).author, author.pubkey());
}
