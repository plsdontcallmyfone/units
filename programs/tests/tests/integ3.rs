// Changed by Hookwars: new file, integration pass 3 (docs/spec/13-integration-3.md): the economy wired
// to the real programs. Template economy fields and the protocol bps (E-7, E-2), wear from item
// creation to a dormant item answering the default (E-3, R38), the settle protocol fee and author
// share (E-2, R34), the settle drop (E-4), the armory's `mint_crafted` behind craft (E-5), and the
// social counters the armory and market record (E-6).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{InstructionData, ToAccountMetas};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::{armory_code, armory_events, armory_ix, items_code, params, token_accounts, Hw};
use bordrless_program_tests::economy::*;
use bordrless_program_tests::items::{equip, equip_state, put_launch, settle_ix, transfer};
use bordrless_token::client as token;
use bordrless_token::instructions::SlotInit;
use bordrless_token::state::SlotBounds;
use hookwars_armory::cpi::{ARMORY_SIGNER, MINTER};
use hookwars_armory::error::ArmoryError as AE;
use hookwars_common::economy::{self as eco, counter};
use hookwars_common::{eco_cpi, ids, pda, template_id as t, Params};
use hookwars_craft::state::{self as cs, recipe_kind};
use hookwars_items::ItemsError as IE;
use solana_keypair::Keypair;
use solana_signer::Signer;

const MAT: u16 = 1;

fn world() -> Ew {
    let mut ew = Ew::wired();
    ew.create_material(MAT, 1_000_000);
    ew.drop_rule(eco_cpi::drop_source::SETTLE_CRANK, MAT, 1, 1_000);
    ew
}

fn set_economy_ix(admin: &Pubkey, template_id: u16, author_bps: u16, charges: u32) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::RetireTemplate {
            admin: *admin,
            config: pda::config().0,
            template: pda::template(template_id).0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetTemplateEconomy {
            template_id,
            author_bps,
            default_access: 0,
            allowed_access: 0,
            charges_on_create: charges,
        },
    )
}

fn set_protocol_ix(admin: &Pubkey, bps: u16) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::AdminQueued {
            admin: *admin,
            config: pda::config().0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetItemProtocolBps { item_protocol_bps: bps },
    )
}

/// `create_item` with the optional craft init-wear and social suffixes.
fn create_item_ix(hw: &Hw, author: &Pubkey, template_id: u16, p: Params, royalty_bps: u16, wear: bool, social: bool) -> (Instruction, Pubkey, Pubkey) {
    let n = hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let mut ix = armory_ix(
        hookwars_armory::accounts::CreateItem {
            author: *author,
            config: pda::config().0,
            template: pda::template(template_id).0,
            minter: MINTER,
            item_mint,
            item,
            recipient_holding: token::holding_address(&item_mint, author),
            armory_signer: ARMORY_SIGNER,
            items_program: ids::ITEMS_ID,
            token: token_accounts(),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::CreateItem { template_id, params: p, royalty_bps },
    );
    if wear {
        ix.accounts.extend(eco_cpi::init_wear_metas(&ids::ARMORY_ID, &item));
    }
    if social {
        ix.accounts.extend(eco_cpi::social_metas(&ids::ARMORY_ID, author));
    }
    (ix, item, item_mint)
}

#[test]
fn template_economy_and_protocol_bps_are_admin_only_and_bounded() {
    let mut ew = world();
    let admin = ew.deployer();
    let stranger = ew.funded(SOL);
    ew.hw.send_gated_all(&stranger, vec![set_economy_ix(&stranger.pubkey(), t::HALF_LIFE, 1_000, 3)]).expect_code(armory_code(AE::NotAdmin));
    ew.hw.send_gated_all(&admin, vec![set_economy_ix(&admin.pubkey(), t::HALF_LIFE, 10_001, 3)]).expect_code(armory_code(AE::RoyaltyTooHigh));
    ew.hw.send_gated_all(&admin, vec![set_economy_ix(&admin.pubkey(), t::HALF_LIFE, 1_000, 3)]).ok();
    let tp: hookwars_armory::state::Template = ew.hw.w.env.read(&pda::template(t::HALF_LIFE).0);
    assert_eq!((tp.author_bps, tp.charges_on_create), (1_000, 3));
    // The protocol share is bounded by `max_royalty_bps` and set by the admin only.
    let max = ew.hw.config().params.max_royalty_bps;
    ew.hw.send_gated_all(&admin, vec![set_protocol_ix(&admin.pubkey(), max + 1)]).expect_code(armory_code(AE::RoyaltyTooHigh));
    ew.hw.send_gated_all(&stranger, vec![set_protocol_ix(&stranger.pubkey(), 100)]).expect_code(armory_code(AE::NotAdmin));
    ew.hw.send_gated_all(&admin, vec![set_protocol_ix(&admin.pubkey(), 100)]).ok();
    assert_eq!(ew.hw.config().item_protocol_bps, 100);
}

#[test]
fn a_wearing_template_needs_the_wear_accounts_and_counters_reach_social() {
    let mut ew = world();
    let admin = ew.deployer();
    ew.hw.send_gated_all(&admin, vec![set_economy_ix(&admin.pubkey(), t::HALF_LIFE, 0, 4)]).ok();
    let author = ew.funded(10 * SOL);
    ew.open_profile(&author);
    let p = params(&[200_000, 3_600, 4]);
    // No wear accounts: refused, so no item of a wearing template exists without its `Wear`.
    let (ix, _, _) = create_item_ix(&ew.hw, &author.pubkey(), t::HALF_LIFE, p, 1_000, false, true);
    ew.send(&author, &[ix]).expect_code(armory_code(AE::WearAccountsMissing));
    let (ix, item, _) = create_item_ix(&ew.hw, &author.pubkey(), t::HALF_LIFE, p, 1_000, true, true);
    ew.send(&author, &[ix]).ok();
    let it = ew.hw.read_item(&item);
    assert!(it.has_wear);
    let w = ew.wear(&item);
    assert_eq!((w.max_charges, w.used, w.dormant), (4, 0, false));
    assert_eq!(ew.profile(&author.pubkey()).counters[usize::from(counter::ITEMS_AUTHORED)], 1);
    // A template that does not wear makes no `Wear` and takes no suffix.
    let (ix, plain, _) = create_item_ix(&ew.hw, &author.pubkey(), t::TRANSFER_FEE, params(&[100, 500]), 1_000, false, true);
    ew.send(&author, &[ix]).ok();
    assert!(!ew.hw.read_item(&plain).has_wear);
    assert!(ew.hw.w.env.account(&cs::wear_address(&plain).0).is_none());
    assert_eq!(ew.profile(&author.pubkey()).counters[usize::from(counter::ITEMS_AUTHORED)], 2);
    // Without the social suffix the item is made and no counter moves.
    let (ix, _, _) = create_item_ix(&ew.hw, &author.pubkey(), t::TRANSFER_FEE, params(&[100, 500]), 1_000, false, false);
    ew.send(&author, &[ix]).ok();
    assert_eq!(ew.profile(&author.pubkey()).counters[usize::from(counter::ITEMS_AUTHORED)], 2);
}

fn fee_slot() -> SlotInit {
    SlotInit {
        kind: slot_kind::FEE,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds { max_cut_bps: 5_000, may_refuse: true, may_write_data: true, may_answer_touch: false, may_burn: false },
        data_len: 6,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// The craft settle suffix for `item` held by `holder` (wear, the SettleCrank drop of MAT).
fn craft_settle_metas(item: &Pubkey, item_mint: &Pubkey, holder: &Pubkey) -> Vec<AccountMeta> {
    let mat_mint = cs::material_mint_address(MAT).0;
    vec![
        AccountMeta::new_readonly(eco::CRAFT_ID, false),
        AccountMeta::new_readonly(eco_cpi::craft_config(), false),
        AccountMeta::new_readonly(eco_cpi::event_authority(&eco::CRAFT_ID), false),
        AccountMeta::new_readonly(eco::caller_pda(eco::CRAFT_CALLER_SEED, &ids::ITEMS_ID).0, false),
        AccountMeta::new(eco_cpi::wear_address(item), false),
        AccountMeta::new(cs::drop_address(eco_cpi::drop_source::SETTLE_CRANK).0, false),
        AccountMeta::new(cs::material_address(MAT).0, false),
        AccountMeta::new(mat_mint, false),
        AccountMeta::new_readonly(cs::minter_address().0, false),
        AccountMeta::new_readonly(*holder, false),
        AccountMeta::new(token::holding_address(&mat_mint, holder), false),
        AccountMeta::new_readonly(token::holding_address(item_mint, holder), false),
    ]
}

/// The fee suffix: the template, the admin's token holding, the author's token and quote holdings.
fn fee_metas(template_id: u16, mint: &Pubkey, admin: &Pubkey, registrant: &Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(pda::template(template_id).0, false),
        AccountMeta::new(pda::holding(mint, admin), false),
        AccountMeta::new(pda::holding(mint, registrant), false),
        AccountMeta::new(pda::holding(&ids::BRIDGED_SOL_MINT, registrant), false),
    ]
}

#[test]
fn settle_takes_the_protocol_fee_and_author_share_wears_the_item_and_drops_to_its_holder() {
    let mut ew = world();
    let admin = ew.deployer();
    // TEST values (none is a decision): protocol 5%, author 20% of the royalty, one charge.
    ew.hw.send_gated_all(&admin, vec![set_protocol_ix(&admin.pubkey(), 500), set_economy_ix(&admin.pubkey(), t::HALF_LIFE, 2_000, 1)]).ok();
    let holder = ew.funded(10 * SOL);
    let (ix, item, item_mint) = create_item_ix(&ew.hw, &holder.pubkey(), t::HALF_LIFE, params(&[200_000, 3_600, 4]), 1_000, true, false);
    ew.send(&holder, &[ix]).ok();
    let owner = ew.funded(100 * SOL);
    let mint = ew.hw.slot_mint(&owner, vec![fee_slot()]);
    let now = ew.now();
    put_launch(&mut ew.hw, &mint, &Pubkey::new_unique(), now);
    equip(&mut ew.hw, &owner, &mint, 0, item, vec![], 0).ok();
    let alice = ew.funded(SOL);
    let bob = ew.funded(SOL);
    ew.hw.mint_to(&owner, &mint, &alice.pubkey(), 1_000_000);
    // One run: a 20% cut of 100,000.
    transfer(&mut ew.hw, &alice, &mint, &bob.pubkey(), 100_000).ok();
    assert_eq!(equip_state(&ew.hw, &mint, 0).token_unsettled[0], 20_000);
    token_holding(&mut ew, &admin, &mint, &admin.pubkey());

    let cranker = ew.funded(SOL);
    let base = settle_ix(&ew.hw, &cranker.pubkey(), &mint, 0, &[(mint, ids::ITEMS_ID)]);
    // Once the protocol share is set, a settle without the fee suffix is refused.
    let mut ix = base.clone();
    ix.accounts.extend(craft_settle_metas(&item, &item_mint, &holder.pubkey()));
    ew.send(&cranker, &[ix]).expect_code(items_code(IE::WrongAccount));
    // An item that wears needs the craft suffix once it has run.
    let mut ix = base.clone();
    ix.accounts.extend(fee_metas(t::HALF_LIFE, &mint, &admin.pubkey(), &admin.pubkey()));
    ew.send(&cranker, &[ix]).expect_code(items_code(IE::WrongAccount));
    let mut ix = base;
    ix.accounts.extend(craft_settle_metas(&item, &item_mint, &holder.pubkey()));
    ix.accounts.extend(fee_metas(t::HALF_LIFE, &mint, &admin.pubkey(), &admin.pubkey()));
    let before = ew.holding(&mint, &admin.pubkey());
    ew.send(&cranker, &[ix]).ok();
    let x = 20_000u64;
    let protocol = x * 500 / 10_000;
    let royalty = (x - protocol) * 1_000 / 10_000;
    let bounty = (x - protocol - royalty) * 50 / 10_000;
    let author = royalty * 2_000 / 10_000;
    // The registrant is the armory admin here, so both shares land in one holding.
    assert_eq!(ew.holding(&mint, &admin.pubkey()) - before, protocol + author);
    assert_eq!(ew.holding(&mint, &pda::royalty_owner(&item).0), royalty - author);
    assert_eq!(ew.holding(&mint, &cranker.pubkey()), bounty);
    // The run wore the item's only charge. Pass 5 (review 3 M-11): the drop is measured in
    // bridged SOL settled, so token-side cuts (raw units of a token anyone can launch) drop none.
    assert!(ew.wear(&item).dormant);
    assert_eq!(ew.holding(&cs::material_mint_address(MAT).0, &holder.pubkey()), 0);
    // Dormant: the item answers the default, so a transfer is no longer cut (R38).
    let carol = ew.funded(SOL);
    transfer(&mut ew.hw, &alice, &mint, &carol.pubkey(), 100_000).ok();
    assert_eq!(ew.holding(&mint, &carol.pubkey()), 100_000);
    assert_eq!(equip_state(&ew.hw, &mint, 0).token_unsettled[0], 0);
}

fn token_holding(ew: &mut Ew, payer: &Keypair, mint: &Pubkey, owner: &Pubkey) {
    let ix = token::create_holding(payer.pubkey(), *mint, *owner);
    ew.send(payer, &[ix]).ok();
}

#[test]
fn craft_mints_through_the_armory_and_only_its_signer_may() {
    let mut ew = world();
    let c = ew.funded(10 * SOL);
    ew.stub_drop(eco_cpi::drop_source::SETTLE_CRANK, MAT, 50_000, &c.pubkey()).ok();
    let inputs = [(MAT, 10)];
    let mut r = recipe(recipe_kind::CRAFT, &inputs, 1_000, t::TRANSFER_FEE);
    r.param_min = params(&[100, 500]);
    r.param_max = params(&[300, 1_500]);
    ew.create_recipe(1, r).ok();
    ew.warp(i64::from(TEST_CRAFT.admin_timelock_secs));
    let n = ew.hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let accounts = hookwars_armory::accounts::MintCrafted {
        craft_signer: cs::signer_address().0,
        payer: c.pubkey(),
        owner: c.pubkey(),
        config: pda::config().0,
        template: pda::template(t::TRANSFER_FEE).0,
        minter: MINTER,
        item_mint,
        item,
        recipient_holding: token::holding_address(&item_mint, &c.pubkey()),
        armory_signer: ARMORY_SIGNER,
        items_program: ids::ITEMS_ID,
        token: token_accounts(),
        system_program: anchor_lang::system_program::ID,
        event_authority: armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    // Anyone else signing in the craft signer's place is refused.
    let mut forged = accounts.clone();
    forged[0] = AccountMeta::new_readonly(c.pubkey(), true);
    let ix = Instruction {
        program_id: ids::ARMORY_ID,
        accounts: forged,
        data: hookwars_armory::instruction::MintCrafted {
            crafter: c.pubkey(),
            template_id: t::TRANSFER_FEE,
            param_min: vec![100, 500],
            param_max: vec![300, 1_500],
            recipe_id: 1,
        }
        .data(),
    };
    ew.send(&c, &[ix]).expect_code(armory_code(AE::NotCraftSigner));
    // Through craft: the inputs burn and the armory makes the item for the crafter.
    let mut ix = ew.craft_item_ix(&c.pubkey(), 1, &inputs);
    ix.accounts.extend(accounts.into_iter().skip(1));
    ew.send(&c, &[ix]).ok();
    let it = ew.hw.read_item(&item);
    assert_eq!(it.source, hookwars_armory::state::source::CRAFTED);
    assert_eq!((it.params[0], it.params[1]), (200, 1_000));
    assert_eq!(ew.holding(&item_mint, &c.pubkey()), 1);
    assert_eq!(ew.holding(&cs::material_mint_address(MAT).0, &c.pubkey()), 40);
}

fn fuse_ix(hw: &Hw, author: &Pubkey, parts: &[(Pubkey, Pubkey, u16)], targets: Vec<(u8, u8)>) -> (Instruction, Pubkey) {
    use hookwars_common::composite::CompositeItem;
    let n = hw.config().items_minted;
    let item_mint = pda::item_mint(n).0;
    let item = pda::item(&item_mint).0;
    let mut accounts = hookwars_armory::accounts::CreateComposite {
        author: *author,
        config: pda::config().0,
        template: pda::template(t::COMPOSITE).0,
        minter: MINTER,
        item_mint,
        item,
        composite: CompositeItem::address(&item).0,
        recipient_holding: token::holding_address(&item_mint, author),
        token: token_accounts(),
        system_program: anchor_lang::system_program::ID,
        event_authority: armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    for (_, _, tid) in parts {
        accounts.push(AccountMeta::new_readonly(pda::template(*tid).0, false));
    }
    for (it, mint, _) in parts {
        accounts.push(AccountMeta::new_readonly(*it, false));
        accounts.push(AccountMeta::new(*mint, false));
        accounts.push(AccountMeta::new(token::holding_address(mint, author), false));
    }
    let ix = Instruction {
        program_id: ids::ARMORY_ID,
        accounts,
        data: hookwars_armory::instruction::Fuse {
            targets: targets.iter().map(|&(start, count)| hookwars_armory::FuseTarget { start, count }).collect(),
            royalty_bps: 500,
        }
        .data(),
    };
    (ix, item)
}

#[test]
fn fuse_burns_the_components_into_one_composite() {
    let mut ew = world();
    let author = ew.funded(10 * SOL);
    let (tx, a, a_mint) = ew.hw.create_item(&author, t::SIDE_SKEW, params(&[100, 0]), 0);
    tx.ok();
    let (tx, b, b_mint) = ew.hw.create_item(&author, t::HALF_LIFE, params(&[200_000, 3_600, 4]), 0);
    tx.ok();
    let parts = [(a, a_mint, t::SIDE_SKEW), (b, b_mint, t::HALF_LIFE)];
    // One component is not a fusion; a wallet that does not hold the components cannot fuse them.
    let (ix, _) = fuse_ix(&ew.hw, &author.pubkey(), &parts[..1], vec![(0, 0)]);
    ew.send(&author, &[ix]).expect_code(armory_code(AE::WrongAccount));
    let stranger = ew.funded(10 * SOL);
    let (ix, _) = fuse_ix(&ew.hw, &stranger.pubkey(), &parts, vec![(0, 0), (0, 0)]);
    ew.send(&stranger, &[ix]).expect_code(armory_code(AE::NotItemHolder));
    let (ix, comp) = fuse_ix(&ew.hw, &author.pubkey(), &parts, vec![(0, 0), (0, 0)]);
    ew.send(&author, &[ix]).ok();
    let it = ew.hw.read_item(&comp);
    assert_eq!(it.template_id, t::COMPOSITE);
    assert_eq!(ew.holding(&it.item_mint, &author.pubkey()), 1);
    let data = ew.hw.w.env.account(&hookwars_common::composite::CompositeItem::address(&comp).0).expect("composite").data;
    let c = hookwars_common::composite::CompositeItem::decode(&data).expect("decode");
    assert_eq!(c.modules.iter().map(|m| m.template_id).collect::<Vec<_>>(), vec![t::SIDE_SKEW, t::HALF_LIFE]);
    assert_eq!(c.provenance, vec![a, b]);
    // The components are burned: one-way, nothing left to fuse again.
    assert_eq!(ew.holding(&a_mint, &author.pubkey()), 0);
    assert_eq!(ew.holding(&b_mint, &author.pubkey()), 0);
    let (ix, _) = fuse_ix(&ew.hw, &author.pubkey(), &parts, vec![(0, 0), (0, 0)]);
    ew.send(&author, &[ix]).expect_code(armory_code(AE::NotItemHolder));
}

#[test]
fn the_hand_built_cpis_use_the_programs_discriminators() {
    use anchor_lang::Discriminator;
    assert_eq!(eco_cpi::INIT_WEAR_DISC, hookwars_craft::instruction::InitWear::DISCRIMINATOR);
    assert_eq!(eco_cpi::WEAR_DISC, hookwars_craft::instruction::Wear::DISCRIMINATOR);
    assert_eq!(eco_cpi::DROP_DISC, hookwars_craft::instruction::Drop::DISCRIMINATOR);
    assert_eq!(eco_cpi::RECORD_WALLET_DISC, hookwars_social::instruction::RecordWallet::DISCRIMINATOR);
    assert_eq!(eco_cpi::MINT_CRAFTED_DISC, hookwars_armory::instruction::MintCrafted::DISCRIMINATOR);
    assert_eq!(eco_cpi::MINT_CRAFTED_DISC, [121, 196, 34, 30, 90, 249, 235, 177]);
}

fn preset_address(id: u16) -> Pubkey {
    Pubkey::find_program_address(&[b"preset".as_ref(), &id.to_le_bytes()], &ids::ARMORY_ID).0
}

#[test]
fn presets_fix_the_module_order_and_only_the_admin_registers_them() {
    use bordrless_program_tests::items::module;
    use hookwars_common::composite::CompositeItem;
    let mut ew = world();
    let admin = ew.deployer();
    let reg = |who: &Pubkey, ids_: Vec<u16>| {
        armory_ix(
            hookwars_armory::accounts::RegisterPreset {
                admin: *who,
                config: pda::config().0,
                preset: preset_address(1),
                system_program: anchor_lang::system_program::ID,
                queued: Pubkey::default(),
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::RegisterPreset { id: 1, template_ids: ids_, name: "TEST preset".into() },
        )
    };
    let stranger = ew.funded(SOL);
    ew.hw.send_gated_all(&stranger, vec![reg(&stranger.pubkey(), vec![t::SIDE_SKEW, t::HALF_LIFE])]).expect_code(armory_code(AE::NotAdmin));
    ew.hw.send_gated_all(&admin, vec![reg(&admin.pubkey(), vec![t::SIDE_SKEW])]).expect_code(armory_code(AE::InvalidSchema));
    ew.hw.send_gated_all(&admin, vec![reg(&admin.pubkey(), vec![t::SIDE_SKEW, t::HALF_LIFE])]).ok();
    let author = ew.funded(10 * SOL);
    let mint_ix = |hw: &Hw, modules: Vec<hookwars_common::composite::Module>| {
        let n = hw.config().items_minted;
        let item_mint = pda::item_mint(n).0;
        let item = pda::item(&item_mint).0;
        let mut accounts = hookwars_armory::accounts::CreateComposite {
            author: author.pubkey(),
            config: pda::config().0,
            template: pda::template(t::COMPOSITE).0,
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
        accounts.push(AccountMeta::new_readonly(preset_address(1), false));
        for m in &modules {
            accounts.push(AccountMeta::new_readonly(pda::template(m.template_id).0, false));
        }
        let ix = Instruction {
            program_id: ids::ARMORY_ID,
            accounts,
            data: hookwars_armory::instruction::MintComposite { preset_id: 1, modules, royalty_bps: 0 }.data(),
        };
        (ix, item)
    };
    let skew = module(t::SIDE_SKEW, params(&[100, 0]), 0, 0);
    let half = module(t::HALF_LIFE, params(&[200_000, 3_600, 4]), 0, 0);
    // Out of the preset's order: refused.
    let (ix, _) = mint_ix(&ew.hw, vec![half.clone(), skew.clone()]);
    ew.send(&author, &[ix]).expect_code(armory_code(AE::InvalidSchema));
    let (ix, comp) = mint_ix(&ew.hw, vec![skew, half]);
    ew.send(&author, &[ix]).ok();
    assert_eq!(ew.hw.read_item(&comp).template_id, t::COMPOSITE);
}
