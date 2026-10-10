// Changed by Hookwars: new file (protocol pass 4a, docs/spec/14-pass-4a.md): the admin queue (review 1
// L-1), access modes and enforce_access (E-1), level gates (E-8), template submissions, external
// templates on both sides (Hook Lab gaps 1 to 4) and the forge of composites.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::{AccountSerialize, Discriminator, InstructionData};
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::*;
use bordrless_program_tests::external::{self, EXT_ID};
use bordrless_program_tests::items::{create_composite, equip_state, module, settle_ix, transfer};
use bordrless_token::client as token;
use hookwars_armory::error::ArmoryError as E;
use hookwars_armory::state::{AccessParams, Approval, LicenceTerms, QueuedAction, TemplateSubmission};
use hookwars_common::access::{self as acc};
use hookwars_common::economy::{self as eco};
use hookwars_common::{ids, pda, template_id as T, EquipConfig, PARAM_FIELDS};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn put(hw: &mut Hw, key: Pubkey, owner: Pubkey, data: Vec<u8>) {
    let lamports = hw.w.env.rent(data.len()).max(1);
    hw.w.env.put(key, solana_account::Account { lamports, data, owner, executable: false, rent_epoch: 0 });
}

fn ser<T: AccountSerialize>(v: &T) -> Vec<u8> {
    let mut out = Vec::new();
    v.try_serialize(&mut out).unwrap();
    out
}

fn set_access_ix(signer: &Pubkey, item: &Pubkey, mode: u8, exclusive: bool, terms: Option<LicenceTerms>, hw: &Hw) -> Instruction {
    let it = hw.read_item(item);
    armory_ix(
        hookwars_armory::accounts::SetAccess {
            signer: *signer,
            config: pda::config().0,
            item: *item,
            template: pda::template(it.template_id).0,
            item_holding: hw.item_holder_holding(item),
            policy: acc::policy_address(item).0,
            lease: hookwars_common::market::lease(item),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetAccess { mode, exclusive, licence_terms: terms },
    )
}

fn approve_ix(signer: &Pubkey, item: &Pubkey, token_mint: &Pubkey, hw: &Hw) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::Approve {
            signer: *signer,
            item: *item,
            item_holding: hw.item_holder_holding(item),
            approval: acc::approval_address(item, token_mint).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::Approve { token_mint: *token_mint },
    )
}

fn enforce_ix(hw: &Hw, payer: &Pubkey, mint: &Pubkey, slot: u8, item: Pubkey, proof: Pubkey) -> Instruction {
    let equip = hw.equip_accounts(payer, mint, slot, Some(item), None);
    armory_ix(
        hookwars_armory::accounts::EnforceAccess {
            config: pda::config().0,
            slot_state: pda::slot_state(mint, slot).0,
            proof,
            equip,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::EnforceAccess { slot },
    )
}

fn set_params(hw: &mut Hw, params: AccessParams) {
    let admin = hw.admin.insecure_clone();
    let ix = armory_ix(
        hookwars_armory::accounts::AdminQueued {
            admin: admin.pubkey(),
            config: pda::config().0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetAccessParams { params },
    );
    hw.send_gated(&admin, ix, &[]).ok();
}

/// A one-slot token (a Defense slot, Vote rule) owned by a fresh wallet.
fn defense_token(hw: &mut Hw) -> (Keypair, Pubkey) {
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, vec![bordrless_program_tests::slots::item_slot(slot_kind::DEFENSE, equip_rule::VOTE, 0, 0, false)]);
    (owner, mint)
}

/// A Wall item (Defense, no targets) and its author.
fn wall(hw: &mut Hw) -> (Keypair, Pubkey) {
    let (author, item, _) = hw.item(T::WALL, params(&[1_000]), 0);
    (author, item)
}

/// The `License` of `item` for `token_mint` as the market writes it.
fn put_license(hw: &mut Hw, item: &Pubkey, token_mint: &Pubkey, ends_at: i64, revoked_at: i64) {
    let l = hookwars_market::licence::License {
        bump: 255,
        item: *item,
        token_mint: *token_mint,
        payer: Pubkey::new_unique(),
        price_paid: 1,
        starts_at: hw.w.env.now,
        ends_at,
        revoked_at,
        per: 0,
        counted: true,
    };
    put(hw, acc::license_address(item, token_mint), ids::MARKET_ID, ser(&l));
}

/// Social's skill table (Builder from `licences_sold`, level 1 at 1) and `wallet`'s profile with
/// `licences_sold` = `sold`.
fn put_builder(hw: &mut Hw, wallet: &Pubkey, sold: u64) {
    let mut th = [0u64; eco::MAX_LEVELS];
    th[0] = 1;
    let table = hookwars_social::profiles::SkillTable {
        version: 1,
        bump: 255,
        skills: vec![eco::SkillDef { id: eco::skill::BUILDER, counter: eco::counter::LICENCES_SOLD, thresholds: th }],
        callers: vec![],
        pending: None,
    };
    put(hw, hookwars_common::eco_cpi::skills_address(), eco::SOCIAL_ID, ser(&table));
    let mut counters = [0u64; eco::counter::COUNT];
    counters[usize::from(eco::counter::LICENCES_SOLD)] = sold;
    let p = hookwars_social::profiles::Profile {
        version: 1,
        bump: 255,
        wallet: *wallet,
        counters,
        created_at: 0,
        updated_at: 0,
        reserved: [0; 32],
    };
    put(hw, hookwars_common::eco_cpi::profile_address(wallet), eco::SOCIAL_ID, ser(&p));
}

fn social_metas(wallet: &Pubkey) -> Vec<AccountMeta> {
    vec![
        AccountMeta::new_readonly(eco::SOCIAL_ID, false),
        AccountMeta::new_readonly(hookwars_common::eco_cpi::skills_address(), false),
        AccountMeta::new_readonly(hookwars_common::eco_cpi::profile_address(wallet), false),
    ]
}

// ------------------------------------------------------------------------------ pins

#[test]
fn raw_readers_pin_the_owning_crates() {
    assert_eq!(hookwars_market::licence::License::DISCRIMINATOR, acc::LICENSE_DISC);
    assert_eq!(hookwars_agents::Directive::DISCRIMINATOR, acc::DIRECTIVE_DISC);
    assert_eq!(hookwars_social::profiles::Profile::DISCRIMINATOR, acc::PROFILE_DISC);
    assert_eq!(hookwars_social::profiles::SkillTable::DISCRIMINATOR, acc::SKILL_TABLE_DISC);
    let item = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    assert_eq!(acc::license_address(&item, &mint), hookwars_market::licence::license_address(&item, &mint).0);
    let passport = Pubkey::new_unique();
    assert_eq!(acc::agent_vault(&passport), hookwars_agents::constants::pda::vault(&passport).0);
    assert_eq!(hookwars_common::eco_cpi::profile_address(&item), hookwars_social::profiles::profile_address(&item).0);
    // The launchpad's record of an external pool cut calls this discriminator.
    assert_eq!(hookwars_items::instruction::RecordPoolCut::DISCRIMINATOR, &[119, 109, 14, 167, 96, 253, 125, 130]);
}

// ------------------------------------------------------------------------------ L-1

#[test]
fn gated_admin_actions_wait_out_the_queue_and_bind_their_arguments() {
    let mut hw = Hw::bare();
    hw.init().ok();
    let admin = hw.admin.insecure_clone();
    let reg = |id: u16| {
        let mut ix = armory_ix(
            hookwars_armory::accounts::RegisterTemplate {
                admin: admin.pubkey(),
                config: pda::config().0,
                template: pda::template(id).0,
                template_program: ids::ITEMS_ID,
                programdata: hookwars_common::programdata_address(&ids::ITEMS_ID),
                system_program: anchor_lang::system_program::ID,
                queued: Pubkey::default(),
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::RegisterTemplate { args: Hw::template_args(id) },
        );
        fill_queued(&mut ix, &[ids::ITEMS_ID]);
        ix
    };
    // Not queued: the entry does not exist.
    hw.w.env.send_paid_by(&[reg(1)], &admin, &[]).expect_fail();
    // Queued: refused until the timelock has passed.
    let q = hw.queue(&reg(1).data, &[ids::ITEMS_ID]);
    let e: QueuedAction = hw.w.env.read(&q);
    assert_eq!(e.ready_at, hw.w.env.now + i64::from(TEST_PARAMS.admin_timelock_secs));
    hw.w.env.send_paid_by(&[reg(1)], &admin, &[]).expect_code(armory_code(E::Timelock));
    hw.w.env.warp(i64::from(TEST_PARAMS.admin_timelock_secs));
    hw.w.env.send_paid_by(&[reg(1)], &admin, &[]).ok();
    assert!(hw.w.env.account(&q).is_none(), "the entry closes when it applies");
    // An entry binds its arguments and its template program.
    let q2 = hw.queue(&reg(2).data, &[ids::ITEMS_ID]);
    hw.w.env.warp(i64::from(TEST_PARAMS.admin_timelock_secs));
    let mut wrong = reg(3);
    for m in wrong.accounts.iter_mut() {
        if m.pubkey == queued_for(&reg(3).data, &[ids::ITEMS_ID]) {
            m.pubkey = q2;
        }
    }
    hw.w.env.send_paid_by(&[wrong], &admin, &[]).expect_code(armory_code(E::NotQueued));
    // Cancelled entries never apply.
    let cancel = armory_ix(
        hookwars_armory::accounts::CancelAdmin {
            admin: admin.pubkey(),
            config: pda::config().0,
            queued: q2,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::CancelAdmin {},
    );
    hw.w.env.send_paid_by(&[cancel], &admin, &[]).ok();
    hw.w.env.send_paid_by(&[reg(2)], &admin, &[]).expect_fail();
    // Only the admin queues.
    let stranger = hw.w.env.funded(SOL);
    let (qix, _) = queue_ix(&stranger.pubkey(), &reg(4).data, &[ids::ITEMS_ID]);
    hw.w.env.send_paid_by(&[qix], &stranger, &[]).expect_code(armory_code(E::NotAdmin));
    // The access numbers go through the queue too.
    set_params(&mut hw, TEST_ACCESS);
    assert_eq!(hw.config().access, TEST_ACCESS);
}

// ------------------------------------------------------------------------------ E-1

#[test]
fn an_exclusive_item_runs_on_one_token_at_a_time() {
    let mut hw = Hw::new();
    let (author, item) = wall(&mut hw);
    let ix = set_access_ix(&author.pubkey(), &item, acc::EXCLUSIVE, false, None, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).ok();
    let it = hw.read_item(&item);
    assert!(it.exclusive && it.access_mode == acc::EXCLUSIVE);
    let (a_owner, a) = defense_token(&mut hw);
    let (b_owner, b) = defense_token(&mut hw);
    hw.equip_launch(&a_owner, &a, Hw::entry(0, Some(item), EquipConfig::default())).ok();
    hw.equip_launch(&b_owner, &b, Hw::entry(0, Some(item), EquipConfig::default()))
        .expect_code(armory_code(E::ExclusiveInUse));
}

#[test]
fn a_gated_item_needs_a_live_approval_and_a_revocation_waits_out_the_notice() {
    let mut hw = Hw::new();
    let (author, item) = wall(&mut hw);
    // Approving needs the Gated mode.
    let (owner, mint) = defense_token(&mut hw);
    let ix = approve_ix(&author.pubkey(), &item, &mint, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::WrongAccessMode));
    let ix = set_access_ix(&author.pubkey(), &item, acc::GATED, false, None, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).ok();
    // No proof, then a proof for another token: refused.
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default()))
        .expect_code(armory_code(E::AccessDenied));
    let other = Pubkey::new_unique();
    let ix = approve_ix(&author.pubkey(), &item, &other, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).ok();
    hw.equip_launch_with(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default()), Some(acc::approval_address(&item, &other).0))
        .expect_code(armory_code(E::AccessDenied));
    // Only the holder approves.
    let stranger = hw.w.env.funded(SOL);
    let ix = approve_ix(&stranger.pubkey(), &item, &mint, &hw);
    hw.w.env.send_paid_by(&[ix], &stranger, &[]).expect_code(armory_code(E::NotItemHolder));
    let ix = approve_ix(&author.pubkey(), &item, &mint, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).ok();
    let approval = acc::approval_address(&item, &mint).0;
    hw.equip_launch_with(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default()), Some(approval)).ok();
    assert_eq!(hw.slot_item(&mint, 0), item);
    // Revocation: removal only after the slot's notice (600 s in the TEST entry).
    let revoke = armory_ix(
        hookwars_armory::accounts::RevokeApproval {
            signer: author.pubkey(),
            item,
            item_holding: hw.item_holder_holding(&item),
            approval,
            token_mint: mint,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::RevokeApproval {},
    );
    hw.w.env.send_paid_by(&[revoke.clone()], &author, &[]).expect_code(armory_code(E::WrongAccount));
    let mut with_state = revoke;
    with_state.accounts.push(AccountMeta::new_readonly(pda::slot_state(&mint, 0).0, false));
    let now = hw.w.env.now;
    hw.w.env.send_paid_by(&[with_state], &author, &[]).ok();
    let a: Approval = hw.w.env.read(&approval);
    assert_eq!(a.revoke_after, now + 600);
    let cranker = hw.w.env.funded(SOL);
    let ix = enforce_ix(&hw, &cranker.pubkey(), &mint, 0, item, approval);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).expect_code(armory_code(E::AccessStillValid));
    // Trades keep working during the notice (R38).
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&owner, &mint, &alice.pubkey(), 1_000_000);
    transfer(&mut hw, &alice, &mint, &bob.pubkey(), 1_000).ok();
    hw.w.env.warp(599);
    let ix = enforce_ix(&hw, &cranker.pubkey(), &mint, 0, item, approval);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).expect_code(armory_code(E::AccessStillValid));
    hw.w.env.warp(1);
    let ix = enforce_ix(&hw, &cranker.pubkey(), &mint, 0, item, approval);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    // The launch item was this item: the slot empties.
    assert_eq!(hw.slot_item(&mint, 0), Pubkey::default());
    assert_eq!(hw.read_item(&item).equipped_count, 0);
    transfer(&mut hw, &alice, &mint, &bob.pubkey(), 1_000).ok();
}

#[test]
fn a_licensed_item_reads_the_markets_licence_and_lapses_after_the_notice() {
    let mut hw = Hw::new();
    set_params(&mut hw, TEST_ACCESS);
    let (author, item) = wall(&mut hw);
    let terms = LicenceTerms { price_lamports: SOL / 10, term_secs: 3_600, per: 0, max_live: 2 };
    let ix = set_access_ix(&author.pubkey(), &item, acc::LICENSED, false, Some(terms), &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).ok();
    let (owner, mint) = defense_token(&mut hw);
    let (b_owner, b) = defense_token(&mut hw);
    let ends = hw.w.env.now + 3_600;
    put_license(&mut hw, &item, &mint, ends, 0);
    // A token with no licence: refused.
    hw.equip_launch_with(&b_owner, &b, Hw::entry(0, Some(item), EquipConfig::default()), Some(acc::license_address(&item, &b)))
        .expect_code(armory_code(E::AccessDenied));
    let lic = acc::license_address(&item, &mint);
    hw.equip_launch_with(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default()), Some(lic)).ok();
    let cranker = hw.w.env.funded(SOL);
    let now = hw.w.env.now;
    hw.w.env.warp(ends - now);
    // Lapsed, but the notice (600 s) runs from the lapse.
    let ix = enforce_ix(&hw, &cranker.pubkey(), &mint, 0, item, lic);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).expect_code(armory_code(E::AccessStillValid));
    hw.w.env.warp(600);
    let ix = enforce_ix(&hw, &cranker.pubkey(), &mint, 0, item, lic);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    assert_eq!(hw.slot_item(&mint, 0), Pubkey::default());
    // A revoked licence lapses at its revocation.
    let (c_owner, c) = defense_token(&mut hw);
    let ends = hw.w.env.now + 3_600;
    put_license(&mut hw, &item, &c, ends, 0);
    hw.equip_launch_with(&c_owner, &c, Hw::entry(0, Some(item), EquipConfig::default()), Some(acc::license_address(&item, &c))).ok();
    let now = hw.w.env.now;
    put_license(&mut hw, &item, &c, ends, now);
    hw.w.env.warp(600);
    let ix = enforce_ix(&hw, &cranker.pubkey(), &c, 0, item, acc::license_address(&item, &c));
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
}

#[test]
fn set_access_checks_the_holder_the_template_the_terms_and_the_builder_level() {
    let mut hw = Hw::new();
    let (author, item) = wall(&mut hw);
    let a = author.pubkey();
    let stranger = hw.w.env.funded(SOL);
    let ix = set_access_ix(&stranger.pubkey(), &item, acc::GATED, false, None, &hw);
    hw.w.env.send_paid_by(&[ix], &stranger, &[]).expect_code(armory_code(E::NotItemHolder));
    for mode in [acc::LEASED, 9] {
        let ix = set_access_ix(&a, &item, mode, false, None, &hw);
        hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::AccessNotAllowed));
    }
    let terms = LicenceTerms { price_lamports: 2 * SOL, term_secs: 3_600, per: 1, max_live: 1 };
    // Before the access numbers are set no term fits; terms on a mode that takes none.
    let ix = set_access_ix(&a, &item, acc::LICENSED, false, Some(terms), &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::BadLicenceTerms));
    let ix = set_access_ix(&a, &item, acc::GATED, false, Some(terms), &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::BadLicenceTerms));
    set_params(&mut hw, TEST_ACCESS);
    let ix = set_access_ix(&a, &item, acc::LICENSED, false, None, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::BadLicenceTerms));
    let short = LicenceTerms { term_secs: 60, ..terms };
    let ix = set_access_ix(&a, &item, acc::LICENSED, false, Some(short), &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::BadLicenceTerms));
    let two = LicenceTerms { max_live: 2, ..terms };
    let ix = set_access_ix(&a, &item, acc::LICENSED, true, Some(two), &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::BadLicenceTerms));
    // E-8: above tier 1 the Builder level is needed.
    let ix = set_access_ix(&a, &item, acc::LICENSED, true, Some(terms), &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::LevelTooLow));
    put_builder(&mut hw, &a, 0);
    let mut ix = set_access_ix(&a, &item, acc::LICENSED, true, Some(terms), &hw);
    ix.accounts.extend(social_metas(&a));
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::LevelTooLow));
    put_builder(&mut hw, &a, 1);
    let mut ix = set_access_ix(&a, &item, acc::LICENSED, true, Some(terms), &hw);
    ix.accounts.extend(social_metas(&a));
    hw.w.env.send_paid_by(&[ix], &author, &[]).ok();
    let p: hookwars_armory::state::AccessPolicy = hw.w.env.read(&acc::policy_address(&item).0);
    assert_eq!((p.mode, p.exclusive, p.licence_terms), (acc::LICENSED, true, Some(terms)));
    // A template may forbid modes (its economy fields, set through the queue).
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
        hookwars_armory::instruction::SetTemplateEconomy {
            template_id: T::WALL,
            author_bps: 0,
            default_access: acc::OPEN,
            allowed_access: 1 << acc::OPEN,
            charges_on_create: 0,
        },
    );
    hw.send_gated(&admin, ix, &[]).ok();
    let ix = set_access_ix(&a, &item, acc::GATED, false, None, &hw);
    hw.w.env.send_paid_by(&[ix], &author, &[]).expect_code(armory_code(E::AccessNotAllowed));
    // A default access mode is what new items get.
    let ix = armory_ix(
        hookwars_armory::accounts::RetireTemplate {
            admin: admin.pubkey(),
            config: pda::config().0,
            template: pda::template(T::WALL).0,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SetTemplateEconomy {
            template_id: T::WALL,
            author_bps: 0,
            default_access: acc::GATED,
            allowed_access: 0,
            charges_on_create: 0,
        },
    );
    hw.send_gated(&admin, ix, &[]).ok();
    let (_, gated) = wall(&mut hw);
    assert_eq!(hw.read_item(&gated).access_mode, acc::GATED);
}

#[test]
fn an_agent_vault_item_follows_the_agents_live_directive() {
    let mut hw = Hw::new();
    set_params(&mut hw, TEST_ACCESS);
    let (author, item) = wall(&mut hw);
    let operator = hw.w.env.funded(SOL);
    let passport = Pubkey::new_unique();
    // The passport's operator and agent key (the armory reads only those).
    let mut pd = vec![0u8; 200];
    pd[10..42].copy_from_slice(operator.pubkey().as_ref());
    pd[46..78].copy_from_slice(Pubkey::new_unique().as_ref());
    put(&mut hw, passport, ids::AGENTS_ID, pd);
    let vault = acc::agent_vault(&passport);
    let it = hw.read_item(&item);
    hw.give_item(&author, &it.item_mint, &vault);
    HOLDERS.with(|h| h.borrow_mut().push(vault));
    let directive = |allowed: u8, max_price: u64, frozen: bool| hookwars_agents::Directive {
        passport,
        seq: 0,
        memo_hash: [1; 32],
        constraints: hookwars_agents::DirectiveConstraints {
            max_spend_per_action: 0,
            max_spend_per_day: 0,
            allowed_targets: vec![Pubkey::new_unique()],
            allowed_access_modes: allowed,
            max_licence_price: max_price,
            frozen,
        },
        posted_at: 0,
        superseded_by: None,
        bump: 255,
    };
    let dkey = hookwars_agents::directive_address(&passport, 0).0;
    let agent_metas = vec![
        AccountMeta::new_readonly(ids::AGENTS_ID, false),
        AccountMeta::new_readonly(passport, false),
        AccountMeta::new_readonly(dkey, false),
    ];
    put(&mut hw, dkey, ids::AGENTS_ID, ser(&directive((1 << acc::OPEN) | (1 << acc::GATED), SOL / 2, false)));
    let send = |hw: &mut Hw, mode: u8, terms: Option<LicenceTerms>| {
        let mut ix = set_access_ix(&operator.pubkey(), &item, mode, false, terms, hw);
        ix.accounts.extend(agent_metas.clone());
        hw.w.env.send_paid_by(&[ix], &operator, &[])
    };
    send(&mut hw, acc::GATED, None).ok();
    let lic = LicenceTerms { price_lamports: SOL / 10, term_secs: 3_600, per: 0, max_live: 1 };
    send(&mut hw, acc::LICENSED, Some(lic)).expect_code(armory_code(E::DirectiveForbids));
    put(&mut hw, dkey, ids::AGENTS_ID, ser(&directive(1 << acc::LICENSED, SOL / 20, false)));
    send(&mut hw, acc::LICENSED, Some(lic)).expect_code(armory_code(E::DirectiveForbids));
    put(&mut hw, dkey, ids::AGENTS_ID, ser(&directive(1 << acc::LICENSED, SOL, false)));
    send(&mut hw, acc::LICENSED, Some(lic)).ok();
    put(&mut hw, dkey, ids::AGENTS_ID, ser(&directive(0xff, SOL, true)));
    send(&mut hw, acc::OPEN, None).expect_code(armory_code(E::DirectiveForbids));
    // A wallet that is neither the operator nor the agent key: not the holder.
    let stranger = hw.w.env.funded(SOL);
    let mut ix = set_access_ix(&stranger.pubkey(), &item, acc::OPEN, false, None, &hw);
    ix.accounts.extend(agent_metas.clone());
    hw.w.env.send_paid_by(&[ix], &stranger, &[]).expect_code(armory_code(E::NotItemHolder));
}

// ------------------------------------------------------------------------------ submissions

fn submit_ix(submitter: &Pubkey, program: &Pubkey, code_hash: [u8; 32]) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::SubmitTemplate {
            submitter: *submitter,
            config: pda::config().0,
            submission: submission_address(program),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SubmitTemplate { template_program: *program, code_hash, uri_hash: [3; 32] },
    )
}

fn submission_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[hookwars_armory::SUBMISSION_SEED, program.as_ref()], &ids::ARMORY_ID).0
}

fn settle_submission_ix(admin: &Pubkey, program: &Pubkey, submitter: &Pubkey, template: Option<Pubkey>, approved: bool, forfeit: bool) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::SettleSubmission {
            admin: *admin,
            config: pda::config().0,
            submission: submission_address(program),
            submitter: *submitter,
            template,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::SettleSubmission { approved, forfeit },
    )
}

#[test]
fn a_template_submission_posts_its_bond_with_the_builder_discount_and_settles() {
    let mut hw = Hw::new();
    set_params(&mut hw, TEST_ACCESS);
    let admin = hw.admin.insecure_clone();
    let a = hw.w.env.funded(10 * SOL);
    let p1 = Pubkey::new_unique();
    let before = hw.w.env.lamports(&a.pubkey());
    hw.w.env.send_paid_by(&[submit_ix(&a.pubkey(), &p1, [9; 32])], &a, &[]).ok();
    let s: TemplateSubmission = hw.w.env.read(&submission_address(&p1));
    assert_eq!(s.bond, TEST_ACCESS.lab_bond_lamports);
    assert!(before - hw.w.env.lamports(&a.pubkey()) >= TEST_ACCESS.lab_bond_lamports);
    // Forfeited: the bond goes to the admin.
    let admin_before = hw.w.env.lamports(&admin.pubkey());
    hw.w.env.send_paid_by(&[settle_submission_ix(&admin.pubkey(), &p1, &a.pubkey(), None, false, true)], &admin, &[]).ok();
    assert!(hw.w.env.lamports(&admin.pubkey()) > admin_before + TEST_ACCESS.lab_bond_lamports - SOL / 100);
    assert!(hw.w.env.account(&submission_address(&p1)).is_none());
    // A Builder at the discount level posts half (TEST 5,000 bps); approving needs the template.
    let b = hw.w.env.funded(10 * SOL);
    put_builder(&mut hw, &b.pubkey(), 1);
    external::load(&mut hw);
    let p2 = ext_template::ID;
    let mut ix = submit_ix(&b.pubkey(), &p2, [7; 32]);
    ix.accounts.extend(social_metas(&b.pubkey()));
    hw.w.env.send_paid_by(&[ix], &b, &[]).ok();
    let s: TemplateSubmission = hw.w.env.read(&submission_address(&p2));
    assert_eq!(s.bond, TEST_ACCESS.lab_bond_lamports / 2);
    hw.w.env
        .send_paid_by(&[settle_submission_ix(&admin.pubkey(), &p2, &b.pubkey(), None, true, false)], &admin, &[])
        .expect_code(armory_code(E::SubmissionClosed));
    hw.register_external(&admin, ext_template::ID, external::args(EXT_ID, slot_kind::FEE, 1), external::fee_manifest(500)).ok();
    let b_before = hw.w.env.lamports(&b.pubkey());
    hw.w.env
        .send_paid_by(&[settle_submission_ix(&admin.pubkey(), &p2, &b.pubkey(), Some(pda::template(EXT_ID).0), true, false)], &admin, &[])
        .ok();
    assert!(hw.w.env.lamports(&b.pubkey()) >= b_before + TEST_ACCESS.lab_bond_lamports / 2);
}

// ------------------------------------------------------------------------------ external templates

#[test]
fn an_external_template_registers_authors_equips_cuts_and_settles() {
    let mut hw = Hw::new();
    let admin = hw.admin.insecure_clone();
    external::load(&mut hw);
    // The built-in path refuses an external id; the external path refuses a built-in id, the
    // items program and a declared kind that is not the manifest's.
    hw.register_with(&admin, ext_template::ID, external::args(EXT_ID, slot_kind::FEE, 1))
        .expect_code(armory_code(E::InvalidSchema));
    hw.register_external(&admin, ext_template::ID, external::args(10, slot_kind::FEE, 1), external::fee_manifest(500))
        .expect_code(armory_code(E::InvalidSchema));
    hw.register_external(&admin, ids::ITEMS_ID, external::args(EXT_ID, slot_kind::FEE, 1), external::fee_manifest(500))
        .expect_code(armory_code(E::InvalidSchema));
    hw.register_external(&admin, ext_template::ID, external::args(EXT_ID, slot_kind::DEFENSE, 1), external::fee_manifest(500))
        .expect_code(armory_code(E::InvalidSchema));
    hw.register_external(&admin, ext_template::ID, external::args(EXT_ID, slot_kind::FEE, 1), external::fee_manifest(500)).ok();
    // Fields outside the template's ceiling are refused.
    let author = hw.w.env.funded(10 * SOL);
    let (tx, _, _) = hw.create_item(&author, EXT_ID, params(&[501]), 0);
    tx.expect_code(armory_code(E::ParamOutOfRange));
    // A slot bound below the declared ceiling refuses the item.
    let owner = hw.w.env.funded(100 * SOL);
    let low = hw.slot_mint(&owner, vec![external::fee_slot(300, 6)]);
    let (_, item, _) = hw.item(EXT_ID, params(&[100]), 0);
    hw.equip_launch(&owner, &low, Hw::entry(0, Some(item), EquipConfig::default()))
        .expect_code(armory_code(E::OverBounds));
    external::cut_flow(&mut hw, EXT_ID, 200, 500);
}

#[test]
fn an_external_template_pays_its_rest_to_its_first_target() {
    let mut hw = Hw::new();
    external::register(&mut hw);
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, vec![external::fee_slot(500, 6)]);
    let (_, item, _) = hw.item(EXT_ID, params(&[300]), 0);
    let target = hw.w.env.funded(SOL);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(item), EquipConfig { targets: vec![target.pubkey()], role: 0 })).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&owner, &mint, &alice.pubkey(), 10_000_000);
    transfer(&mut hw, &alice, &mint, &bob.pubkey(), 1_000_000).ok();
    bordrless_program_tests::items::create_holding(&mut hw, &target, &mint, &target.pubkey());
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &[(pda::holding(&mint, &target.pubkey()), ids::ITEMS_ID)]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let cut = 30_000u64;
    let bounty = cut * 50 / 10_000;
    assert_eq!(hw.w.env.holding(&mint, &target.pubkey()), cut - bounty);
    assert_eq!(hw.w.env.holding(&mint, &cranker.pubkey()), bounty);
    assert_eq!(equip_state(&hw, &mint, 0).token_unsettled[0], 0);
}

#[test]
fn an_external_pool_template_cuts_through_the_launchpad_which_records_it_for_settlement() {
    use bordrless_launch::state::LaunchRules;
    let mut hw = Hw::new();
    hw.w.env.svm.add_program(bordrless_launch::ID, &bordrless_program_tests::program_bytes("bordrless_launch")).unwrap();
    external::load(&mut hw);
    let admin = hw.admin.insecure_clone();
    let manifest = hookwars_common::Manifest {
        kind: slot_kind::POOL,
        pool_flags: 3,
        max_cut_buy_bps: 300,
        max_cut_sell_bps: 300,
        ..Default::default()
    };
    hw.register_external(&admin, ext_template::ID, external::args(EXT_ID, slot_kind::POOL, 0), manifest).ok();
    let creator = hw.w.env.funded(1_000 * SOL);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    hw.w.prepare_launch(&creator, &mint_kp, 100, LaunchRules::NONE, vec![bordrless_program_tests::slot_launch::pool_slot(300, false)])
        .ok();
    let (_, item, _) = hw.item(EXT_ID, params(&[0, 100]), 1_000);
    let c = creator.pubkey();
    let equip = hw.equip_accounts(&c, &mint, 0, None, Some(item));
    let inner = armory_ix(
        hookwars_armory::accounts::EquipLaunch {
            launch_caller: pda::armory_caller(&mint).0,
            config: pda::config().0,
            slot_state: pda::slot_state(&mint, 0).0,
            equip,
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::EquipLaunch { entry: Hw::entry(0, Some(item), EquipConfig::default()) },
    );
    let ix = bordrless_launch::client::slots::equip_prepared(c, mint, inner);
    hw.w.env.send_paid_by(&[ix], &creator, &[]).ok();
    let m: bordrless_token::state::Mint = hw.w.env.read(&mint);
    assert_eq!(m.slots[0].program, ext_template::ID);
    let ix = hw.w.create_prepared_ix(&c, &mint, 100, bordrless_program_tests::launch::VQ, LaunchRules::NONE);
    hw.w.env.send_paid_by(&[ix], &creator, &[&mint_kp]).ok();
    let trader = hw.w.env.funded(100 * SOL);
    hw.w.wrap_sol(&trader, SOL).ok();
    let cuts = pda::pool_cuts(&mint).0;
    let sol = ids::BRIDGED_SOL_MINT;
    let before = hw.w.env.holding(&sol, &cuts);
    hw.w.slot_buy(&trader, &mint, SOL / 10).ok();
    let collected = hw.w.env.holding(&sol, &cuts) - before;
    assert!(collected > 0);
    let st = equip_state(&hw, &mint, 0);
    assert_eq!(st.pool_unsettled[0], collected);
    assert_eq!(st.pool_owed, collected);
    // Settle: the royalty and the rest to the item's royalty holding, the bounty to the cranker.
    let cranker = hw.w.env.funded(SOL);
    let royalty = pda::royalty_owner(&item).0;
    let ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &[(ids::ITEMS_ID, pda::holding(&sol, &royalty))]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let roy = collected * 1_000 / 10_000;
    let bounty = (collected - roy) * 50 / 10_000;
    assert_eq!(hw.w.env.holding(&sol, &cranker.pubkey()), bounty);
    assert_eq!(hw.w.env.holding(&sol, &royalty), collected - bounty);
    assert_eq!(equip_state(&hw, &mint, 0).pool_unsettled[0], 0);
    // Only the launchpad's signer records a cut, and only for an external equip.
    let ix = Instruction {
        program_id: ids::ITEMS_ID,
        accounts: vec![
            AccountMeta::new_readonly(trader.pubkey(), true),
            AccountMeta::new(pda::equip_state(&mint, 0).0, false),
        ],
        data: hookwars_items::instruction::RecordPoolCut { mint, slot: 0, item, cut: 1, side: 1 }.data(),
    };
    hw.w.env.send_paid_by(&[ix], &trader, &[]).expect_fail();
}

// ------------------------------------------------------------------------------ forge

#[test]
fn composites_with_the_same_modules_forge_into_one() {
    use hookwars_armory::state::CompositeItem;
    let addr = |i: &Pubkey| hookwars_common::composite::CompositeItem::address(i).0;
    let mut hw = bordrless_program_tests::items::arsenal();
    let forger = hw.w.env.funded(100 * SOL);
    let ms = |a: u32, b: u32| {
        vec![
            module(T::SIDE_SKEW, params(&[a, a]), 0, 0),
            module(T::HALF_LIFE, params(&[b, 3_600, 4]), 0, 0),
        ]
    };
    let (tx, c1) = create_composite(&mut hw, &forger, ms(100, 100), 500);
    tx.ok();
    let (tx, c2) = create_composite(&mut hw, &forger, ms(200, 50), 700);
    tx.ok();
    let (tx, other) = create_composite(&mut hw, &forger, vec![module(T::HALF_LIFE, params(&[50, 3_600, 4]), 0, 0), module(T::SIDE_SKEW, params(&[10, 10]), 0, 0)], 0);
    tx.ok();
    let forge = |hw: &Hw, a: Pubkey, b: Pubkey| {
        let ia = hw.read_item(&a);
        let ib = hw.read_item(&b);
        let n = hw.config().items_minted;
        let item_mint = pda::item_mint(n).0;
        let item = pda::item(&item_mint).0;
        let ca: CompositeItem = hw.w.env.read(&addr(&a));
        let mut ix = armory_ix(
            hookwars_armory::accounts::Forge {
                forger: forger.pubkey(),
                config: pda::config().0,
                forge_counter: pda::forge_counter(&forger.pubkey()).0,
                item_a: a,
                item_b: b,
                mint_a: ia.item_mint,
                mint_b: ib.item_mint,
                holding_a: token::holding_address(&ia.item_mint, &forger.pubkey()),
                holding_b: token::holding_address(&ib.item_mint, &forger.pubkey()),
                template: pda::template(T::COMPOSITE).0,
                minter: hookwars_armory::cpi::MINTER,
                item_mint,
                item,
                recipient_holding: token::holding_address(&item_mint, &forger.pubkey()),
                armory_signer: hookwars_armory::cpi::ARMORY_SIGNER,
                items_program: ids::ITEMS_ID,
                token: token_accounts(),
                system_program: anchor_lang::system_program::ID,
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::Forge {},
        );
        ix.accounts.push(AccountMeta::new(addr(&a), false));
        ix.accounts.push(AccountMeta::new(addr(&b), false));
        ix.accounts.push(AccountMeta::new(addr(&item), false));
        for m in &ca.modules {
            ix.accounts.push(AccountMeta::new_readonly(pda::template(m.template_id).0, false));
        }
        (ix, item)
    };
    let (ix, _) = forge(&hw, c1, other);
    hw.w.env.send_paid_by(&[ix], &forger, &[]).expect_code(armory_code(E::ModulesMismatch));
    let (ix, forged) = forge(&hw, c1, c2);
    hw.w.env.send_paid_by(&[ix], &forger, &[]).ok();
    let f = hw.read_item(&forged);
    assert_eq!((f.template_id, f.level, f.royalty_bps, f.source), (T::COMPOSITE, 2, 700, hookwars_armory::state::source::FORGED));
    let list: CompositeItem = hw.w.env.read(&addr(&forged));
    assert_eq!(list.provenance, vec![c1, c2]);
    assert_eq!(list.modules.len(), 2);
    // Each module combined by its own forge rule (the same pure function the items program runs).
    let gain = TEST_PARAMS.forge_gain_bps;
    for (i, m) in list.modules.iter().enumerate() {
        let t: hookwars_armory::state::Template = hw.w.env.read(&pda::template(m.template_id).0);
        let a = ms(100, 100)[i].params;
        let b = ms(200, 50)[i].params;
        let mut want = hookwars_common::combine(m.template_id, &t.field_min, &t.field_max, gain, &a, &b).unwrap();
        for (k, w) in want.iter_mut().enumerate().take(PARAM_FIELDS) {
            if k < usize::from(t.field_count) {
                let s = hookwars_common::shape(m.template_id).unwrap();
                if !(s.zero_off[k] && *w == 0) {
                    *w = (*w).clamp(t.field_min[k], t.field_max[k]);
                }
            } else {
                *w = 0;
            }
        }
        assert_eq!(m.params, want, "module {i}");
    }
    assert!(hw.w.env.account(&c1).is_none() && hw.w.env.account(&addr(&c1)).is_none());
}

/// Keeps the helpers this suite shares in one place.
trait HolderHolding {
    fn item_holder_holding(&self, item: &Pubkey) -> Pubkey;
}

impl HolderHolding for Hw {
    /// The holding that holds `item`'s token: the author's, an agent vault's, or whatever the
    /// suite moved it to (found by owner among the known candidates).
    fn item_holder_holding(&self, item: &Pubkey) -> Pubkey {
        let it = self.read_item(item);
        let author = token::holding_address(&it.item_mint, &it.author);
        if self.w.env.holding(&it.item_mint, &it.author) == 1 {
            return author;
        }
        HOLDERS.with(|h| {
            for owner in h.borrow().iter() {
                if self.w.env.holding(&it.item_mint, owner) == 1 {
                    return token::holding_address(&it.item_mint, owner);
                }
            }
            author
        })
    }
}

thread_local! {
    static HOLDERS: std::cell::RefCell<Vec<Pubkey>> = const { std::cell::RefCell::new(Vec::new()) };
}
