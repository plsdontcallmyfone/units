// Changed by Hookwars: new file (gating, docs/spec/18-gating.md parts A and B): scarce templates and
// premium (Licensed by default) templates.
//! A tracked template caps new copies: authors issue up to `max_supply - loot_reserve`, drops
//! (loot and craft) up to the whole cap, forges are transformations; `AUTHOR_ONLY` lets only the
//! minter issue (also for a template closed to authoring, and per composite module); the `Supply`
//! cannot be left out; caps only fall; the minter hands issuance over. A premium template's items
//! start Licensed, may never be switched to Open, and a token without a live licence cannot equip
//! them. Untracked templates behave as before.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::*;
use bordrless_program_tests::items::{arsenal, module};
use bordrless_program_tests::Tx;
use bordrless_token::client as token;
use hookwars_armory::cpi::{ARMORY_SIGNER, MINTER};
use hookwars_armory::error::ArmoryError as E;
use hookwars_armory::state::LicenceTerms;
use hookwars_armory::supply::{minter_rule, supply_address, Supply};
use hookwars_common::access as acc;
use hookwars_common::composite::{CompositeItem, Module};
use hookwars_common::{ids, pda, template_id as T, EquipConfig, Params};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn gated(hw: &mut Hw, accounts: impl ToAccountMetas, data: impl InstructionData) -> Tx {
    let admin = hw.admin.insecure_clone();
    let ix = armory_ix(accounts, data);
    hw.send_gated(&admin, ix, &[])
}

fn init_supply(hw: &mut Hw, template_id: u16, max_supply: u32, loot_reserve: u32, rule: u8, minter: Pubkey) -> Tx {
    let admin = hw.admin.pubkey();
    gated(
        hw,
        hookwars_armory::accounts::InitSupply {
            admin,
            config: pda::config().0,
            template: pda::template(template_id).0,
            supply: supply_address(template_id).0,
            system_program: anchor_lang::system_program::ID,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::InitSupply { template_id, max_supply, loot_reserve, minter_rule: rule, minter },
    )
}

fn set_cap(hw: &mut Hw, template_id: u16, max_supply: u32, loot_reserve: u32) -> Tx {
    let admin = hw.admin.pubkey();
    gated(
        hw,
        hookwars_armory::accounts::SetSupply {
            admin,
            config: pda::config().0,
            supply: supply_address(template_id).0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetSupplyCap { template_id, max_supply, loot_reserve },
    )
}

fn supply(hw: &Hw, template_id: u16) -> Supply {
    hw.w.env.read(&supply_address(template_id).0)
}

/// `create_item` by `author` with the `Supply` of `template_id` first among its remaining
/// accounts when `with_supply`.
fn create(hw: &mut Hw, author: &Keypair, template_id: u16, p: Params, with_supply: bool) -> (Tx, Pubkey, Pubkey) {
    let n = hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let mut ix = armory_ix(
        hookwars_armory::accounts::CreateItem {
            author: author.pubkey(),
            config: pda::config().0,
            template: pda::template(template_id).0,
            minter: MINTER,
            item_mint,
            item,
            recipient_holding: token::holding_address(&item_mint, &author.pubkey()),
            armory_signer: ARMORY_SIGNER,
            items_program: ids::ITEMS_ID,
            token: token_accounts(),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::CreateItem { template_id, params: p, royalty_bps: 0 },
    );
    if with_supply {
        ix.accounts.push(AccountMeta::new(supply_address(template_id).0, false));
    }
    let tx = hw.w.env.send_paid_by(&[ix], author, &[]);
    (tx, item, item_mint)
}

fn composite(hw: &mut Hw, author: &Keypair, modules: Vec<Module>, supplies: &[u16]) -> Tx {
    let n = hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let mut accounts = hookwars_armory::accounts::CreateComposite {
        author: author.pubkey(),
        config: pda::config().0,
        template: pda::template(T::COMPOSITE).0,
        minter: MINTER,
        item_mint,
        item,
        composite: CompositeItem::address(&item).0,
        recipient_holding: token::holding_address(&item_mint, &author.pubkey()),
        token: token_accounts(),
        system_program: anchor_lang::system_program::ID,
        event_authority: armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    for id in supplies {
        accounts.push(AccountMeta::new(supply_address(*id).0, false));
    }
    for m in &modules {
        accounts.push(AccountMeta::new_readonly(pda::template(m.template_id).0, false));
    }
    let ix = Instruction {
        program_id: ids::ARMORY_ID,
        accounts,
        data: hookwars_armory::instruction::CreateComposite { modules, royalty_bps: 0 }.data(),
    };
    hw.w.env.send_paid_by(&[ix], author, &[])
}

const FEE: u16 = T::TRANSFER_FEE;

fn fee_params() -> Params {
    params(&[100, 100])
}

#[test]
fn authors_issue_up_to_the_author_cap_and_only_the_minter_issues() {
    let mut hw = Hw::new();
    let alice = hw.w.env.funded(10 * SOL);
    let bob = hw.w.env.funded(10 * SOL);
    // Untracked: anyone authors, with or without accounts.
    create(&mut hw, &bob, FEE, fee_params(), false).0.ok();
    // The cap must hold what authors may issue plus the reserve.
    init_supply(&mut hw, FEE, 3, 4, minter_rule::AUTHOR_ONLY, alice.pubkey()).expect_code(armory_code(E::InvalidSchema));
    init_supply(&mut hw, FEE, 3, 1, minter_rule::AUTHOR_ONLY, alice.pubkey()).ok();
    let t: hookwars_armory::state::Template = hw.w.env.read(&pda::template(FEE).0);
    assert_eq!(t.supply_flags & 1, 1);
    // Leaving the Supply out is refused; a stranger is not the minter.
    create(&mut hw, &alice, FEE, fee_params(), false).0.expect_code(armory_code(E::SupplyMissing));
    create(&mut hw, &bob, FEE, fee_params(), true).0.expect_code(armory_code(E::NotMinter));
    create(&mut hw, &alice, FEE, fee_params(), true).0.ok();
    create(&mut hw, &alice, FEE, fee_params(), true).0.ok();
    // Author cap = 3 - 1: the third author copy is refused.
    create(&mut hw, &alice, FEE, fee_params(), true).0.expect_code(armory_code(E::SupplyExhausted));
    let s = supply(&hw, FEE);
    assert_eq!((s.issued, s.drops, s.circulating()), (2, 0, 2));
    // The minter hands issuance over; a stranger cannot.
    let hand = |by: &Keypair, to: Pubkey| {
        armory_ix(
            hookwars_armory::accounts::HandOverMinter {
                minter: by.pubkey(),
                supply: supply_address(FEE).0,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::HandOverMinter { new_minter: to },
        )
    };
    hw.w.env.send_paid_by(&[hand(&bob, bob.pubkey())], &bob, &[]).expect_code(armory_code(E::NotMinter));
    hw.w.env.send_paid_by(&[hand(&alice, bob.pubkey())], &alice, &[]).ok();
    assert_eq!(supply(&hw, FEE).minter, bob.pubkey());
}

#[test]
fn a_closed_template_issues_through_its_minter_only() {
    let mut hw = Hw::new();
    let alice = hw.w.env.funded(10 * SOL);
    let bob = hw.w.env.funded(10 * SOL);
    // A template registered closed to authoring.
    let mut args = Hw::template_args(T::DUST_GUARD);
    args.open_authoring = false;
    let admin = hw.admin.insecure_clone();
    hw.register_with(&admin, ids::ITEMS_ID, args).ok();
    create(&mut hw, &alice, T::DUST_GUARD, params(&[10]), false).0.expect_code(armory_code(E::TemplateClosed));
    init_supply(&mut hw, T::DUST_GUARD, 0, 0, minter_rule::AUTHOR_ONLY, alice.pubkey()).ok();
    create(&mut hw, &bob, T::DUST_GUARD, params(&[10]), true).0.expect_code(armory_code(E::TemplateClosed));
    // Uncapped but tracked: the minter issues and every copy is counted.
    for _ in 0..3 {
        create(&mut hw, &alice, T::DUST_GUARD, params(&[10]), true).0.ok();
    }
    assert_eq!(supply(&hw, T::DUST_GUARD).issued, 3);
}

#[test]
fn drops_use_the_whole_cap_and_forges_only_transform() {
    let mut hw = Hw::new();
    let alice = hw.w.env.funded(10 * SOL);
    init_supply(&mut hw, FEE, 3, 1, minter_rule::AUTHOR_ONLY, alice.pubkey()).ok();
    let (_, a, _) = create(&mut hw, &alice, FEE, fee_params(), true);
    let (_, b, _) = create(&mut hw, &alice, FEE, fee_params(), true);
    let loot = |hw: &mut Hw, with: bool| {
        let payer = hw.w.env.payer.pubkey();
        let (mut ix, _) = hw.mint_loot_ix(&payer, &alice.pubkey(), FEE, fee_params());
        if with {
            ix.accounts.push(AccountMeta::new(supply_address(FEE).0, false));
        }
        hw.w.env.send(&[as_war(ix)], &[])
    };
    loot(&mut hw, false).expect_code(armory_code(E::SupplyMissing));
    // The reserved copy drops; the next drop finds the cap spent.
    loot(&mut hw, true).ok();
    loot(&mut hw, true).expect_code(armory_code(E::SupplyExhausted));
    // A forge burns two and makes one, outside the cap.
    let (mut ix, _) = hw.forge_ix(&alice.pubkey(), &a, &b);
    let without = ix.clone();
    hw.w.env.send_paid_by(&[without], &alice, &[]).expect_code(armory_code(E::SupplyMissing));
    ix.accounts.push(AccountMeta::new(supply_address(FEE).0, false));
    hw.w.env.send_paid_by(&[ix], &alice, &[]).ok();
    let s = supply(&hw, FEE);
    assert_eq!((s.issued, s.drops, s.forged, s.burned), (2, 1, 1, 2));
    assert_eq!(s.circulating(), 2);
}

#[test]
fn a_cap_only_falls_and_never_below_what_was_made() {
    let mut hw = Hw::new();
    let alice = hw.w.env.funded(10 * SOL);
    init_supply(&mut hw, FEE, 10, 2, minter_rule::OPEN_UNTIL_CAP, alice.pubkey()).ok();
    let bob = hw.w.env.funded(10 * SOL);
    for _ in 0..4 {
        create(&mut hw, &bob, FEE, fee_params(), true).0.ok();
    }
    set_cap(&mut hw, FEE, 11, 2).expect_code(armory_code(E::CapMayOnlyFall));
    set_cap(&mut hw, FEE, 0, 0).expect_code(armory_code(E::CapMayOnlyFall));
    set_cap(&mut hw, FEE, 3, 0).expect_code(armory_code(E::CapMayOnlyFall));
    // Authors issued 4: the author cap must still cover them.
    set_cap(&mut hw, FEE, 5, 2).expect_code(armory_code(E::CapMayOnlyFall));
    set_cap(&mut hw, FEE, 5, 1).ok();
    let s = supply(&hw, FEE);
    assert_eq!((s.max_supply, s.loot_reserve), (5, 1));
    create(&mut hw, &bob, FEE, fee_params(), true).0.expect_code(armory_code(E::SupplyExhausted));
}

#[test]
fn each_composite_module_counts_against_its_template() {
    let mut hw = arsenal();
    let alice = hw.w.env.funded(10 * SOL);
    let bob = hw.w.env.funded(10 * SOL);
    init_supply(&mut hw, T::MAX_TRANSACTION, 1, 0, minter_rule::AUTHOR_ONLY, alice.pubkey()).ok();
    let mods = || vec![module(T::MAX_TRANSACTION, params(&[100]), 0, 0), module(T::DUST_GUARD, params(&[10]), 0, 0)];
    composite(&mut hw, &bob, mods(), &[]).expect_code(armory_code(E::SupplyMissing));
    composite(&mut hw, &bob, mods(), &[T::MAX_TRANSACTION]).expect_code(armory_code(E::NotMinter));
    composite(&mut hw, &alice, mods(), &[T::MAX_TRANSACTION]).ok();
    composite(&mut hw, &alice, mods(), &[T::MAX_TRANSACTION]).expect_code(armory_code(E::SupplyExhausted));
    assert_eq!(supply(&hw, T::MAX_TRANSACTION).issued, 1);
}

fn set_access(hw: &Hw, signer: &Keypair, item: &Pubkey, mode: u8, terms: Option<LicenceTerms>) -> Instruction {
    let it = hw.read_item(item);
    armory_ix(
        hookwars_armory::accounts::SetAccess {
            signer: signer.pubkey(),
            config: pda::config().0,
            item: *item,
            template: pda::template(it.template_id).0,
            item_holding: token::holding_address(&it.item_mint, &signer.pubkey()),
            policy: acc::policy_address(item).0,
            lease: hookwars_common::market::lease(item),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetAccess { mode, exclusive: false, licence_terms: terms },
    )
}

#[test]
fn a_premium_templates_items_start_licensed_and_need_a_live_licence() {
    let mut hw = Hw::new();
    let admin = hw.admin.pubkey();
    // Premium: Licensed by default; Licensed or Leased only.
    gated(
        &mut hw,
        hookwars_armory::accounts::RetireTemplate {
            admin,
            config: pda::config().0,
            template: pda::template(T::WALL).0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetTemplateEconomy {
            template_id: T::WALL,
            author_bps: 0,
            default_access: acc::LICENSED,
            allowed_access: (1 << acc::LICENSED) | (1 << acc::LEASED),
            charges_on_create: 0,
        },
    )
    .ok();
    let (author, item, _) = hw.item(T::WALL, params(&[1_000]), 0);
    assert_eq!(hw.read_item(&item).access_mode, acc::LICENSED);
    // The holder may not switch it to Open.
    let ix = set_access(&hw, &author, &item, acc::OPEN, None);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::AccessNotAllowed));
    // A token without a licence cannot equip it at launch.
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, vec![bordrless_program_tests::slots::item_slot(slot_kind::DEFENSE, equip_rule::VOTE, 0, 0, false)]);
    hw.equip_launch_with(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default()), None)
        .expect_code(armory_code(E::AccessDenied));
    hw.equip_launch_with(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default()), Some(acc::license_address(&item, &mint)))
        .expect_code(armory_code(E::AccessDenied));
}

#[test]
fn a_craft_drop_counts_against_the_whole_cap_and_reverts_when_spent() {
    use bordrless_program_tests::economy::{recipe, Ew, TEST_CRAFT};
    use hookwars_common::eco_cpi;
    use hookwars_craft::state::{self as cs, recipe_kind};
    const MAT: u16 = 1;
    let mut ew = Ew::wired();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(eco_cpi::drop_source::SETTLE_CRANK, MAT, 1, 1_000);
    let c = ew.funded(10 * SOL);
    ew.stub_drop(eco_cpi::drop_source::SETTLE_CRANK, MAT, 50_000, &c.pubkey()).ok();
    let inputs = [(MAT, 10)];
    let mut r = recipe(recipe_kind::CRAFT, &inputs, 1_000, FEE);
    r.param_min = params(&[100, 500]);
    r.param_max = params(&[300, 1_500]);
    ew.create_recipe(1, r).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    // One copy, all of it kept for drops: authors may issue none, craft one.
    init_supply(&mut ew.hw, FEE, 1, 1, minter_rule::AUTHOR_ONLY, c.pubkey()).ok();
    create(&mut ew.hw, &c, FEE, fee_params(), true).0.expect_code(armory_code(E::SupplyExhausted));
    let craft = |ew: &mut Ew, with: bool| {
        let n = ew.hw.config().items_minted;
        let item_mint = pda::item_mint(n).0;
        let accounts = hookwars_armory::accounts::MintCrafted {
            craft_signer: cs::signer_address().0,
            payer: c.pubkey(),
            owner: c.pubkey(),
            config: pda::config().0,
            template: pda::template(FEE).0,
            minter: MINTER,
            item_mint,
            item: pda::item(&item_mint).0,
            recipient_holding: token::holding_address(&item_mint, &c.pubkey()),
            armory_signer: ARMORY_SIGNER,
            items_program: ids::ITEMS_ID,
            token: token_accounts(),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        }
        .to_account_metas(None);
        let mut ix = ew.craft_item_ix(&c.pubkey(), 1, &inputs);
        ix.accounts.extend(accounts.into_iter().skip(1));
        if with {
            ix.accounts.push(AccountMeta::new(supply_address(FEE).0, false));
        }
        let payer = c.insecure_clone();
        ew.send(&payer, &[ix])
    };
    craft(&mut ew, false).expect_code(armory_code(E::SupplyMissing));
    craft(&mut ew, true).ok();
    // Spent: the craft reverts and the crafter keeps the materials.
    let before = ew.holding(&cs::material_mint_address(MAT).0, &c.pubkey());
    craft(&mut ew, true).expect_code(armory_code(E::SupplyExhausted));
    assert_eq!(ew.holding(&cs::material_mint_address(MAT).0, &c.pubkey()), before);
    assert_eq!(supply(&ew.hw, FEE).drops, 1);
}
