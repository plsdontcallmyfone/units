// Changed by Hookwars: new file (M2), the armory: init, params timelock, templates, items, loot,
// equip at launch (docs/spec/02-armory.md, 07 M2 row).
// M3b: Raid's registry gained our own launch (extra_count 6).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_token::state::{Holding, Mint};
use bordrless_program_tests::armory::*;
use bordrless_program_tests::env::write_programdata_header;
use bordrless_program_tests::slots::item_slot;
use hookwars_armory::error::ArmoryError as E;
use hookwars_armory::state::{source, ArmoryConfig, Item, Template};
use hookwars_common::{ids, manifest, pda, template_id as T, EquipConfig};
use hookwars_items::EquipState;
use solana_keypair::Keypair;
use solana_signer::Signer;

#[test]
fn init_needs_the_upgrade_authority_once() {
    let mut hw = Hw::bare();
    let stranger = hw.w.env.funded(1_000_000_000);
    let real = hw.admin.insecure_clone();
    hw.admin = stranger;
    hw.init().expect_code(armory_code(E::NotUpgradeAuthority));
    hw.admin = real;
    hw.init().ok();
    let c: ArmoryConfig = hw.config();
    assert_eq!(c.params, TEST_PARAMS);
    assert_eq!(c.admin, hw.admin.pubkey());
    hw.init().expect_fail();
}

#[test]
fn templates_register_in_order_and_refuse_what_rule_3_forbids() {
    let mut hw = Hw::bare();
    hw.init().ok();
    let admin = hw.admin.insecure_clone();
    // Changed by Hookwars M3b: ids may be sparse (the template list), but must be known.
    let mut unknown = Hw::template_args(2);
    unknown.id = 13;
    hw.register_with(&admin, ids::ITEMS_ID, unknown)
        .expect_code(armory_code(E::InvalidSchema));
    // Only the admin.
    let stranger = hw.w.env.funded(1_000_000_000);
    hw.register_with(&stranger, ids::ITEMS_ID, Hw::template_args(1))
        .expect_code(armory_code(E::NotAdmin));
    // A program upgradeable by anyone else.
    hw.w.env
        .set_upgrade_authority(ids::ITEMS_ID, Some(Pubkey::new_unique()));
    hw.register_with(&admin, ids::ITEMS_ID, Hw::template_args(1))
        .expect_code(armory_code(E::TemplateUpgradeable));
    hw.w.env
        .set_upgrade_authority(ids::ITEMS_ID, Some(ids::PROTOCOL_AUTHORITY));
    // A wrong ProgramData account.
    let mut ix = armory_ix(
        hookwars_armory::accounts::RegisterTemplate {
            admin: admin.pubkey(),
            config: pda::config().0,
            template: pda::template(1).0,
            template_program: ids::ITEMS_ID,
            programdata: Pubkey::new_unique(),
            system_program: anchor_lang::system_program::ID,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::RegisterTemplate {
            args: Hw::template_args(1),
        },
    );
    ix.accounts[4] = AccountMeta::new_readonly(Pubkey::new_unique(), false);
    hw.send_gated(&admin, ix, &[ids::ITEMS_ID])
        .expect_code(armory_code(E::ProgramDataMissing));
    // A protocol program, even with an allowed upgrade authority.
    hw.w.env
        .set_upgrade_authority(bordrless_token::ID, Some(ids::PROTOCOL_AUTHORITY));
    hw.register_with(&admin, bordrless_token::ID, Hw::template_args(1))
        .expect_code(armory_code(E::InvalidTemplateProgram));
    // The items program's shape must match the template id.
    let mut wrong = Hw::template_args(1);
    wrong.kind = slot_kind::FEE;
    hw.register_with(&admin, ids::ITEMS_ID, wrong)
        .expect_code(armory_code(E::InvalidSchema));
    let mut rich = Hw::template_args(1);
    rich.loot_royalty_bps = TEST_PARAMS.max_royalty_bps + 1;
    hw.register_with(&admin, ids::ITEMS_ID, rich)
        .expect_code(armory_code(E::RoyaltyTooHigh));
    for id in 1..=9u16 {
        hw.register(id).ok();
    }
    assert_eq!(hw.config().templates, 9);
    let t: Template = hw.w.env.read(&pda::template(T::RAID).0);
    assert_eq!(t.program, ids::ITEMS_ID);
    assert!(t.deploy_slot.is_some());
    assert_eq!((t.kind, t.field_count, t.max_targets), (slot_kind::POOL, 3, 3));
    let t: Template = hw.w.env.read(&pda::template(T::TREATY).0);
    assert!(!t.forge_enabled);
}

#[test]
fn items_are_supply_one_tokens_with_their_manifest() {
    let mut hw = Hw::new();
    let p = params(&[1_000, 200, 50]);
    let (author, item, item_mint) = hw.item(T::RAID, p, 700);
    let m: Mint = hw.w.env.read(&item_mint);
    assert_eq!((m.supply, m.max_supply, m.decimals), (1, 1, 0));
    assert_eq!(m.mint_authority, None);
    assert_eq!(hw.w.env.holding(&item_mint, &author.pubkey()), 1);
    let i: Item = hw.read_item(&item);
    assert_eq!(i.item_mint, item_mint);
    assert_eq!(i.params, p);
    assert_eq!((i.royalty_bps, i.level, i.source), (700, 1, source::AUTHORED));
    assert_eq!(i.author, author.pubkey());
    assert_eq!(i.equipped_count, 0);
    let expected = manifest(T::RAID, &p, 3).unwrap();
    assert_eq!(i.manifest, expected);
    assert_eq!(i.manifest.max_cut_buy_bps, 200);
    assert_eq!(i.manifest.max_discount_bps, 1_000);
    assert_eq!(hw.config().items_minted, 1);

    // Refusals.
    let a = hw.w.env.funded(1_000_000_000);
    let (tx, _, _) = hw.create_item(&a, T::RAID, params(&[5_001, 0, 0]), 0);
    tx.expect_code(armory_code(E::ParamOutOfRange));
    let (tx, _, _) = hw.create_item(&a, T::RAID, params(&[1, 1, 1, 1]), 0);
    tx.expect_code(armory_code(E::ParamOutOfRange));
    let (tx, _, _) = hw.create_item(&a, T::RAID, p, TEST_PARAMS.max_royalty_bps + 1);
    tx.expect_code(armory_code(E::RoyaltyTooHigh));
    // A cross-field rule in the items program: counter_short_secs < counter_long_secs.
    let war = params(&[10, 100, 600, 500, 3_600, 600, 600, 100, 0, 1, 10]);
    let (tx, _, _) = hw.create_item(&a, T::WAR_ORDERS, war, 0);
    tx.expect_code(items_code(hookwars_items::ItemsError::BadParams));
    // A retired template.
    let admin = hw.admin.insecure_clone();
    let ix = armory_ix(
        hookwars_armory::accounts::RetireTemplate {
            admin: admin.pubkey(),
            config: pda::config().0,
            template: pda::template(T::WALL).0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::RetireTemplate {
            template_id: T::WALL,
        },
    );
    hw.send_gated(&admin, ix, &[]).ok();
    let (tx, _, _) = hw.create_item(&a, T::WALL, params(&[500]), 0);
    tx.expect_code(armory_code(E::TemplateClosed));
}

#[test]
fn loot_comes_only_from_the_war_program() {
    let mut hw = Hw::new();
    let payer = hw.w.env.funded(1_000_000_000);
    let owner = Pubkey::new_unique();
    let p = params(&[100, 60, 1]);
    let (ix, item) = hw.mint_loot_ix(&payer.pubkey(), &owner, T::SHIELD, p);
    hw.w.env
        .send_paid_by(&[as_war(ix)], &payer, &[])
        .ok();
    let i: Item = hw.read_item(&item);
    assert_eq!(i.source, source::LOOT);
    assert_eq!(i.royalty_bps, 500);
    assert_eq!(i.author, pda::loot_signer().0);
    assert_eq!(hw.w.env.holding(&i.item_mint, &owner), 1);
    // Anyone else: the loot signer cannot sign.
    let (mut ix, _) = hw.mint_loot_ix(&payer.pubkey(), &owner, T::SHIELD, p);
    let fake = Keypair::new();
    ix.accounts[0] = AccountMeta::new_readonly(fake.pubkey(), true);
    hw.w.env
        .send_paid_by(&[ix], &payer, &[&fake])
        .expect_code(armory_code(E::NotLootSigner));
}

#[test]
fn params_and_admin_wait_out_the_timelock() {
    let mut hw = Hw::new();
    let admin = hw.admin.insecure_clone();
    let mut next = TEST_PARAMS;
    next.vote_quorum_bps = 2_000;
    let propose = armory_ix(
        hookwars_armory::accounts::ProposeParams {
            admin: admin.pubkey(),
            config: pda::config().0,
            pending: pda::pending().0,
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::ProposeParams { params: next },
    );
    hw.w.env.send_paid_by(&[propose], &admin, &[]).ok();
    let apply = armory_ix(
        hookwars_armory::accounts::ApplyParams {
            admin: admin.pubkey(),
            config: pda::config().0,
            pending: pda::pending().0,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::ApplyParams {},
    );
    let payer = hw.w.env.payer.insecure_clone();
    hw.w.env
        .send(&[apply.clone()], &[&payer])
        .expect_code(armory_code(E::Timelock));
    hw.w.env.warp(i64::from(TEST_PARAMS.admin_timelock_secs));
    hw.w.env.send(&[apply], &[&payer]).ok();
    assert_eq!(hw.config().params.vote_quorum_bps, 2_000);

    let new_admin = hw.w.env.funded(1_000_000_000);
    let propose = armory_ix(
        hookwars_armory::accounts::AdminOnly {
            admin: admin.pubkey(),
            config: pda::config().0,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::ProposeAdmin {
            admin: new_admin.pubkey(),
        },
    );
    hw.w.env.send_paid_by(&[propose], &admin, &[]).ok();
    let accept = armory_ix(
        hookwars_armory::accounts::AcceptAdmin {
            new_admin: new_admin.pubkey(),
            config: pda::config().0,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::AcceptAdmin {},
    );
    hw.w.env
        .send_paid_by(&[accept.clone()], &new_admin, &[])
        .expect_code(armory_code(E::Timelock));
    hw.w.env.warp(i64::from(TEST_PARAMS.admin_timelock_secs));
    hw.w.env.send_paid_by(&[accept], &new_admin, &[]).ok();
    assert_eq!(hw.config().admin, new_admin.pubkey());
}

#[test]
fn the_launchpad_equips_a_fresh_mint_once() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let rival = Pubkey::new_unique();
    let (_, raid, _) = hw.item(T::RAID, params(&[1_000, 100, 10]), 500);
    let cfg = EquipConfig {
        targets: vec![rival],
        role: 0,
    };
    // Not the launchpad's caller.
    let mut e = Hw::entry(1, Some(raid), cfg.clone());
    let equip = hw.equip_accounts(&owner.pubkey(), &mint, 1, None, Some(raid));
    let fake = Keypair::new();
    let ix = armory_ix(
        hookwars_armory::accounts::EquipLaunch {
            launch_caller: fake.pubkey(),
            config: pda::config().0,
            slot_state: pda::slot_state(&mint, 1).0,
            equip,
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::EquipLaunch { entry: e.clone() },
    );
    hw.w.env
        .send_paid_by(&[ix], &owner, &[&fake])
        .expect_code(armory_code(E::NotLaunchCaller));
    // The wrong kind: a Raid (Pool) in the Fee slot.
    e.slot = 2;
    hw.equip_launch(&owner, &mint, e.clone())
        .expect_code(armory_code(E::KindMismatch));
    // The Pool slot.
    e.slot = 1;
    hw.equip_launch(&owner, &mint, e.clone()).ok();
    assert_eq!(hw.slot_item(&mint, 1), raid);
    let i: Item = hw.read_item(&raid);
    assert_eq!(i.equipped_count, 1);
    let m: Mint = hw.w.env.read(&mint);
    assert_eq!(m.slots[1].flags, i.manifest.token_flags);
    assert_eq!(m.slots[1].pool_flags, u16::from(i.manifest.pool_flags));
    assert_eq!(m.slots[1].program, ids::ITEMS_ID);
    let st: EquipState = hw.w.env.read(&pda::equip_state(&mint, 1).0);
    assert_eq!((st.item, st.template_id), (raid, T::RAID));
    assert_eq!(st.config, cfg);
    // The registry: the Item, the EquipState, then Raid's extras (ledger, war config, our launch,
    // the target's launch). Changed by Hookwars M3b: our own launch joined Raid's extras.
    let reg = Pubkey::find_program_address(
        &[bordrless_hook::HOOK_ACCOUNTS_SEED, mint.as_ref(), raid.as_ref()],
        &ids::ITEMS_ID,
    )
    .0;
    assert!(hw.w.env.account(&reg).is_some());
    assert_eq!(m.slots[1].extra_count, 6);
    // A pool cut: the royalty owner's bridged-SOL holding exists.
    let owner_r = pda::royalty_owner(&raid).0;
    let h: Holding = hw.w.env.read(&pda::holding(&hw.w.sol, &owner_r));
    assert_eq!(h.amount, 0);
    // Once supply exists, no more launch equips.
    hw.mint_to(&owner, &mint, &owner.pubkey(), 1_000);
    let (_, wo, _) = hw.item(
        T::WAR_ORDERS,
        params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]),
        0,
    );
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(wo), EquipConfig::default()))
        .expect_code(armory_code(E::NotFreshMint));
}

#[test]
fn equip_checks_data_ranges_duplicates_and_staleness() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    // A Pool slot whose range is too small for a Raid (11 bytes + the epoch byte).
    let small = vec![
        item_slot(slot_kind::POOL, equip_rule::VOTE, 0, 6, true),
        item_slot(slot_kind::POOL, equip_rule::VOTE, 0, 12, true),
    ];
    let mint = hw.slot_mint(&owner, small);
    let (_, raid, _) = hw.item(T::RAID, params(&[1_000, 100, 10]), 500);
    let cfg = EquipConfig {
        targets: vec![Pubkey::new_unique()],
        role: 0,
    };
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(raid), cfg.clone()))
        .expect_code(armory_code(E::DataRangeTooSmall));
    hw.equip_launch(&owner, &mint, Hw::entry(1, Some(raid), cfg.clone()))
        .ok();
    // The same item cannot run in two slots of one token.
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(raid), cfg.clone()))
        .expect_fail();
    // Too many targets for the template.
    let mint2 = hw.slot_mint(&owner, test_slots());
    let many = EquipConfig {
        targets: (0..4).map(|_| Pubkey::new_unique()).collect(),
        role: 0,
    };
    hw.equip_launch(&owner, &mint2, Hw::entry(1, Some(raid), many))
        .expect_code(items_code(hookwars_items::ItemsError::BadTargets));
    // The items program upgraded since registration.
    let (pd, _) = Pubkey::find_program_address(&[ids::ITEMS_ID.as_ref()], &ids::BPF_LOADER_UPGRADEABLE_ID);
    let mut acc = hw.w.env.account(&pd).unwrap();
    let slot = u64::from_le_bytes(acc.data[4..12].try_into().unwrap());
    write_programdata_header(&mut acc.data, slot + 1, Some(ids::PROTOCOL_AUTHORITY));
    hw.w.env.put(pd, acc);
    hw.equip_launch(&owner, &mint2, Hw::entry(1, Some(raid), cfg))
        .expect_code(armory_code(E::TemplateChanged));
}
