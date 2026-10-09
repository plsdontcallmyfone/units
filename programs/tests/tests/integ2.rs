// Changed by Hookwars: new file, integration pass 2 (docs/spec/12-integration-2.md): the lease gate
// and the lease end revert (10 section 17 I-3), the listed-item claim (I-4), the lessor's rent at
// settlement (I-7), the merged items error codes, and the composite module conflicts (08 arsenal 2
// request 2).

use anchor_lang::prelude::Pubkey;
use anchor_lang::ToAccountMetas;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::*;
use bordrless_program_tests::expansion::*;
use bordrless_program_tests::items::{arsenal, put_launch, settle_ix, transfer};
use bordrless_token::instructions::SlotInit;
use bordrless_token::state::SlotBounds;
use hookwars_armory::error::ArmoryError;
use hookwars_common::{ids, pda, template_id as t, EquipConfig};
use hookwars_items::ItemsError;
use hookwars_market::state as ms;
use anchor_lang::solana_program::instruction::AccountMeta;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn fee_config() -> EquipConfig {
    EquipConfig {
        targets: vec![Pubkey::new_unique()],
        role: 0,
    }
}

/// The accounts `end_lease` appends so the market reverts slot `slot` of `mint` (still holding
/// `item`) to `launch_item` through the armory's `revert_for_lease_end`.
fn revert_tail(hw: &Hw, cranker: &Pubkey, mint: &Pubkey, slot: u8, item: Pubkey, launch_item: Option<Pubkey>) -> Vec<AccountMeta> {
    let mut metas = hookwars_armory::accounts::RevertForLeaseEnd {
        market_caller: hookwars_common::market::caller().0,
        config: pda::config().0,
        slot_state: pda::slot_state(mint, slot).0,
        equip: hw.equip_accounts(cranker, mint, slot, Some(item), launch_item),
        event_authority: armory_events(),
        program: ids::ARMORY_ID,
    }
    .to_account_metas(None);
    // The market signs as `["market-caller"]` inside the instruction.
    metas[0].is_signer = false;
    metas.extend(hw.refresh_tail(mint, slot));
    metas
}

#[test]
fn a_leased_item_equips_only_where_its_active_lease_names_and_reverts_when_the_lease_ends() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, test_slots());
    let other = hw.slot_mint(&owner, test_slots());
    let (_, incumbent, _) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let cfg = fee_config();
    hw.equip_launch(&owner, &mint, Hw::entry(2, Some(incumbent), cfg.clone())).ok();
    hw.equip_launch(&owner, &other, Hw::entry(2, Some(incumbent), cfg.clone())).ok();
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 1_000);
    hw.mint_to(&owner, &other, &o, 1_000);
    let (lessor, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    let term = TEST_MARKET.lease_min_secs;
    send(&mut hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 2, 1_000, 0, term)]).ok();

    // Offered, not yet active: no equip anywhere.
    let (tx, _) = hw.propose(&owner, &mint, 2, Some(item), cfg.clone());
    tx.expect_code(armory_code(ArmoryError::ItemLeasedElsewhere));
    let payer = hw.w.env.funded(SOL);
    send(&mut hw.w.env, &payer, &[accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey())]).ok();
    // Active: refused on another token, accepted on the (token, slot) the lease names.
    let (tx, _) = hw.propose(&owner, &other, 2, Some(item), cfg.clone());
    tx.expect_code(armory_code(ArmoryError::ItemLeasedElsewhere));
    let (tx, proposal) = hw.propose(&owner, &mint, 2, Some(item), cfg);
    tx.ok();
    hw.vote(&owner, &mint, &proposal, true, 1_000).ok();
    hw.w.env.warp(i64::from(TEST_PARAMS.vote_period_secs));
    hw.finalize(&proposal).ok();
    hw.w.env.warp(600);
    hw.execute(&owner, &proposal).ok();
    assert_eq!(hw.slot_item(&mint, 2), item);

    // The term ends: the market reverts the slot to its launch item, then returns the token.
    hw.w.env.warp(i64::from(term));
    let cranker = hw.w.env.funded(SOL);
    let mut ix = close_lease_ix(&cranker.pubkey(), &item, &item_mint, &lessor.pubkey(), true);
    ix.accounts.extend(revert_tail(&hw, &cranker.pubkey(), &mint, 2, item, Some(incumbent)));
    send(&mut hw.w.env, &cranker, &[ix]).ok();
    assert_eq!(hw.slot_item(&mint, 2), incumbent, "reverted to the launch item");
    assert_eq!(hw.w.env.holding(&item_mint, &lessor.pubkey()), 1);
    assert!(hw.w.env.account(&ms::lease_address(&item).0).is_none());
}

#[test]
fn revert_for_lease_end_answers_only_the_market() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, item, _) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    hw.equip_launch(&owner, &mint, Hw::entry(2, Some(item), fee_config())).ok();
    // A stranger signing in the market caller's place.
    let fake = hw.w.env.funded(SOL);
    let mut metas = revert_tail(&hw, &fake.pubkey(), &mint, 2, item, None);
    metas[0] = AccountMeta::new_readonly(fake.pubkey(), true);
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ids::ARMORY_ID,
        accounts: metas,
        data: anchor_lang::InstructionData::data(&hookwars_armory::instruction::RevertForLeaseEnd { slot: 2, item }),
    };
    hw.w.env.send_paid_by(&[ix], &fake, &[]).expect_code(armory_code(ArmoryError::NotMarketCaller));
    assert_eq!(hw.slot_item(&mint, 2), item);
}

#[test]
fn a_listed_item_earns_but_its_royalty_waits_for_the_holder() {
    let mut hw = Hw::new();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, test_slots());
    let (author, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 100);
    hw.equip_launch(&owner, &mint, Hw::entry(2, Some(item), fee_config())).ok();
    let royalty_owner = pda::royalty_owner(&item).0;
    hw.mint_to(&owner, &mint, &royalty_owner, 50);
    send(&mut hw.w.env, &author, &[list_ix(&author.pubkey(), &item, &item_mint, SOL, 0)]).ok();
    let claim = |claimant: &Keypair| {
        armory_ix(
            hookwars_armory::accounts::ClaimRoyalty {
                claimant: claimant.pubkey(),
                item,
                item_holding: bordrless_token::client::holding_address(&item_mint, &claimant.pubkey()),
                royalty_owner,
                cut_mint: mint,
                royalty_holding: pda::holding(&mint, &royalty_owner),
                destination: pda::holding(&mint, &claimant.pubkey()),
                token: token_accounts(),
                event_authority: armory_events(),
                program: ids::ARMORY_ID,
            },
            hookwars_armory::instruction::ClaimRoyalty { amount: u64::MAX },
        )
    };
    // R31: listed, the seller no longer holds the item, so it cannot claim; nor can anyone else.
    hw.w.env.send_paid_by(&[claim(&author)], &author, &[]).expect_fail();
    assert_eq!(hw.w.env.holding(&mint, &royalty_owner), 50);
    // Delisted, the holder claims everything accrued while listed.
    send(&mut hw.w.env, &author, &[unlist_ix(&author.pubkey(), &author.pubkey(), &item_mint, false)]).ok();
    hw.w.env.send_paid_by(&[claim(&author)], &author, &[]).ok();
    assert_eq!(hw.w.env.holding(&mint, &author.pubkey()), 50);
}

fn fee_slot() -> SlotInit {
    SlotInit {
        kind: slot_kind::FEE,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps: 5_000,
            may_refuse: true,
            may_write_data: true,
            may_answer_touch: false,
            may_burn: false,
        },
        data_len: 6,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

#[test]
fn a_lessor_takes_its_rent_out_of_the_royalty_never_on_top() {
    let mut hw = arsenal();
    let treasury = Keypair::new().pubkey();
    hw.w.env.fund(treasury, SOL);
    load(&mut hw.w.env, treasury);
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, vec![fee_slot()]);
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(&mut hw, &mint, &pool.pubkey(), now);
    // A Transfer Fee item (1%, to a collector) with a 10% royalty, leased at 50% rent.
    let collector = hw.w.env.funded(SOL);
    let (lessor, item, item_mint) = hw.item(t::TRANSFER_FEE, params(&[100, 0]), 1_000);
    let rent_bps = TEST_MARKET.max_rent_bps;
    send(&mut hw.w.env, &lessor, &[offer_lease_ix(&lessor.pubkey(), &item, &item_mint, &mint, 0, rent_bps, 0, TEST_MARKET.lease_min_secs)])
        .ok();
    let payer = hw.w.env.funded(SOL);
    send(&mut hw.w.env, &payer, &[accept_lease_ix(&payer.pubkey(), &item, &lessor.pubkey())]).ok();
    let cfg = EquipConfig {
        targets: vec![collector.pubkey()],
        role: 0,
    };
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(item), cfg)).ok();
    assert_eq!(hw.slot_item(&mint, 0), item);
    let (alice, bob) = (hw.w.env.funded(SOL), hw.w.env.funded(SOL));
    hw.mint_to(&owner, &mint, &alice.pubkey(), 1_000_000);
    for o in [collector.pubkey(), lessor.pubkey()] {
        hw.w.env
            .send_paid_by(&[bordrless_token::client::create_holding(owner.pubkey(), mint, o)], &owner, &[])
            .ok();
    }
    transfer(&mut hw, &alice, &mint, &bob.pubkey(), 1_000_000).ok();
    let x = hw.w.env.holding(&mint, &pda::equip_state(&mint, 0).0);
    assert!(x > 0, "the item cut something");

    let cranker = hw.w.env.funded(SOL);
    let sol = ids::BRIDGED_SOL_MINT;
    let mut ix = settle_ix(&hw, &cranker.pubkey(), &mint, 0, &[(pda::holding(&mint, &collector.pubkey()), ids::ITEMS_ID)]);
    ix.accounts.extend([
        AccountMeta::new_readonly(ids::MARKET_ID, false),
        AccountMeta::new_readonly(ms::lease_address(&item).0, false),
        AccountMeta::new(pda::holding(&mint, &lessor.pubkey()), false),
        AccountMeta::new(pda::holding(&sol, &lessor.pubkey()), false),
    ]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let royalty = x * 1_000 / 10_000;
    let rent = royalty * u64::from(rent_bps) / 10_000;
    let bounty = (x - royalty) * u64::from(TEST_PARAMS.settle_bounty_bps) / 10_000;
    assert_eq!(hw.w.env.holding(&mint, &lessor.pubkey()), rent);
    assert_eq!(hw.w.env.holding(&mint, &pda::royalty_owner(&item).0), royalty - rent);
    assert_eq!(hw.w.env.holding(&mint, &cranker.pubkey()), bounty);
    assert_eq!(hw.w.env.holding(&mint, &collector.pubkey()), x - royalty - bounty, "the token's cut is untouched");
}

#[test]
fn merged_items_error_codes_keep_their_numbers() {
    let code = |e: ItemsError| 6000 + e as u32;
    assert_eq!(code(ItemsError::BadParams), 6000);
    assert_eq!(code(ItemsError::SoulboundTransfer), 7000);
    assert_eq!(code(ItemsError::GuestListClosed), 7100);
    assert_eq!(code(ItemsError::NotEligible), 7101);
    assert_eq!(code(ItemsError::NothingToPay), 7102);
    assert_eq!(code(ItemsError::NoLoyaltyPot), 7103);
    assert_eq!(code(ItemsError::SelfReferral), 7104);
    assert_eq!(items_code(ItemsError::WrongAccount), 6007);
}

#[test]
fn composites_refuse_modules_that_share_one_state() {
    use hookwars_common::arsenal2 as a2;
    use hookwars_common::composite::{validate_modules, CompositeError, Module, NO_READ};
    let module = |id: u16, p: &[u32]| {
        let mut params = [0u32; hookwars_common::PARAM_FIELDS];
        params[..p.len()].copy_from_slice(p);
        let data_bytes = hookwars_common::manifest(id, &params, 0).map(|m| m.data_bytes).unwrap_or(0);
        Module {
            template_id: id,
            params,
            target_start: 0,
            target_count: 0,
            data_bytes,
            reads_module: NO_READ,
            ..Default::default()
        }
    };
    let wide = |_: u16| Some(([0u32; hookwars_common::PARAM_FIELDS], [u32::MAX; hookwars_common::PARAM_FIELDS]));
    let two = [module(a2::LOYALTY_POT, &[]), module(a2::LOYALTY_POT, &[])];
    assert_eq!(validate_modules(&two, wide, 8, 64).err(), Some(CompositeError::ModuleConflict));
    let two = [module(a2::FIRST_BLOOD, &[]), module(a2::FIRST_BLOOD, &[])];
    assert_eq!(validate_modules(&two, wide, 8, 64).err(), Some(CompositeError::ModuleConflict));
    let pair = [module(t::SHIELD, &[]), module(a2::PATIENCE, &[])];
    assert_eq!(validate_modules(&pair, wide, 8, 64).err(), Some(CompositeError::ModuleConflict));
    let pair = [module(t::RAID, &[]), module(a2::MERCENARY, &[])];
    assert_eq!(validate_modules(&pair, wide, 8, 64).err(), Some(CompositeError::ModuleConflict));
}
