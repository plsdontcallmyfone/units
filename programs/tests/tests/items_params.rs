// Changed by Hookwars: new file (M2), template rules in the items program: manifests, forging
// (docs/spec/02-armory.md section 9, 04 section 2.7).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::*;
use bordrless_program_tests::env::Tx;
use bordrless_token::state::Mint;
use hookwars_armory::error::ArmoryError as E;
use hookwars_armory::state::{source, ForgeCounter, Item};
use hookwars_common::{ids, manifest, pda, template_id as T, EquipConfig, Params};
use hookwars_items::ItemsError;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn give(hw: &mut Hw, from: &Keypair, item_mint: &Pubkey, to: &Pubkey) {
    hw.give_item(from, item_mint, to);
}

fn forge(hw: &mut Hw, forger: &Keypair, a: &Pubkey, b: &Pubkey) -> (Tx, Pubkey) {
    let (ix, item) = hw.forge_ix(&forger.pubkey(), a, b);
    (hw.w.env.send_paid_by(&[ix], forger, &[]), item)
}

#[test]
fn forging_moves_each_field_toward_its_ceiling() {
    let mut hw = Hw::new();
    let (forger, a, _) = hw.item(T::RAID, params(&[100, 100, 10]), 100);
    let (other, b, b_mint) = hw.item(T::RAID, params(&[300, 50, 20]), 200);
    give(&mut hw, &other, &b_mint, &forger.pubkey());
    let ia = hw.read_item(&a);
    let (tx, c) = forge(&mut hw, &forger, &a, &b);
    tx.ok();
    let ic: Item = hw.read_item(&c);
    // TEST ceilings 5,000 / 300 / 1,000 and FORGE_GAIN 50%: max(a, b) + half the distance.
    assert_eq!(ic.params[..3], [2_650, 200, 510]);
    assert_eq!((ic.level, ic.royalty_bps, ic.source), (2, 200, source::FORGED));
    assert_eq!(ic.author, forger.pubkey());
    assert_eq!(ic.manifest, manifest(T::RAID, &ic.params, 3).unwrap());
    // Both inputs are gone: their Items closed, their tokens burned.
    assert!(hw.w.env.account(&a).is_none_or(|x| x.lamports == 0));
    assert!(hw.w.env.account(&b).is_none_or(|x| x.lamports == 0));
    let m: Mint = hw.w.env.read(&ia.item_mint);
    assert_eq!(m.supply, 0);
    let fc: ForgeCounter = hw.w.env.read(&pda::forge_counter(&forger.pubkey()).0);
    assert_eq!(fc.count, 1);
    assert_eq!(hw.w.env.holding(&ic.item_mint, &forger.pubkey()), 1);
}

#[test]
fn forging_refuses_what_it_must() {
    let mut hw = Hw::new();
    let (forger, a, _) = hw.item(T::RAID, params(&[100, 100, 10]), 0);
    // Not holding b.
    let (_, b, b_mint) = hw.item(T::RAID, params(&[300, 50, 20]), 0);
    let (tx, _) = forge(&mut hw, &forger, &a, &b);
    tx.expect_code(armory_code(E::NotItemOwner));
    // Different templates.
    let (o2, wall, wall_mint) = hw.item(T::WALL, params(&[500]), 0);
    give(&mut hw, &o2, &wall_mint, &forger.pubkey());
    let (tx, _) = forge(&mut hw, &forger, &a, &wall);
    tx.expect_code(armory_code(E::TemplateMismatch));
    // A Keep field that differs: Shield only_under_siege.
    let (f2, s1, _) = hw.item(T::SHIELD, params(&[100, 60, 0]), 0);
    let (o3, s2, s2_mint) = hw.item(T::SHIELD, params(&[100, 60, 1]), 0);
    give(&mut hw, &o3, &s2_mint, &f2.pubkey());
    let (tx, _) = forge(&mut hw, &f2, &s1, &s2);
    tx.expect_code(items_code(ItemsError::NotForgeable));
    // Unforgeable templates.
    let (f3, t1, _) = hw.item(T::TREATY, params(&[10, 10, 0]), 0);
    let (o4, t2, t2_mint) = hw.item(T::TREATY, params(&[10, 10, 0]), 0);
    give(&mut hw, &o4, &t2_mint, &f3.pubkey());
    let (tx, _) = forge(&mut hw, &f3, &t1, &t2);
    tx.expect_code(armory_code(E::TemplateClosed));
    // An equipped item.
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (o5, b2, b2_mint) = hw.item(T::RAID, params(&[300, 50, 20]), 0);
    hw.equip_launch(
        &owner,
        &mint,
        Hw::entry(
            1,
            Some(b2),
            EquipConfig {
                targets: vec![Pubkey::new_unique()],
                role: 0,
            },
        ),
    )
    .ok();
    give(&mut hw, &o5, &b2_mint, &forger.pubkey());
    let (tx, _) = forge(&mut hw, &forger, &a, &b2);
    tx.expect_code(armory_code(E::ItemEquipped));
    let _ = b_mint;
}

#[test]
fn forging_stops_at_the_template_maximum_level() {
    let mut hw = Hw::bare();
    hw.init().ok();
    let admin = hw.admin.insecure_clone();
    let mut args = Hw::template_args(T::RAID);
    args.max_level = 1;
    hw.register_with(&admin, ids::ITEMS_ID, args).ok();
    let (forger, a, _) = hw.item(T::RAID, params(&[100, 100, 10]), 0);
    let (o, b, b_mint) = hw.item(T::RAID, params(&[300, 50, 20]), 0);
    give(&mut hw, &o, &b_mint, &forger.pubkey());
    let (tx, _) = forge(&mut hw, &forger, &a, &b);
    tx.expect_code(armory_code(E::MaxLevel));
}

#[test]
fn manifests_follow_the_templates() {
    let mut hw = Hw::new();
    let cases: [(u16, Params); 9] = [
        (T::RAID, params(&[1_000, 250, 5])),
        (T::SHIELD, params(&[200, 600, 1])),
        (T::WALL, params(&[300])),
        (T::SPY, params(&[1, 600, 100, 50])),
        (T::TREATY, params(&[20, 80, 1])),
        (T::TRIBUTE, params(&[40])),
        (T::HALF_LIFE, params(&[199_999, 21_600, 8])),
        (T::TRANSFER_FEE, params(&[30, 200])),
        (
            T::WAR_ORDERS,
            params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 1, 1, 10]),
        ),
    ];
    for (id, p) in cases {
        let (_, item, _) = hw.item(id, p, 0);
        let i: Item = hw.read_item(&item);
        let max_targets = test_schema(id).2;
        assert_eq!(i.manifest, manifest(id, &p, max_targets).unwrap(), "template {id}");
    }
    let m = manifest(T::HALF_LIFE, &params(&[199_999, 21_600, 8]), 0).unwrap();
    assert_eq!(m.max_cut_transfer_bps, 2_000);
    let m = manifest(T::TRANSFER_FEE, &params(&[30, 200]), 1).unwrap();
    assert!(m.may_refuse);
    let m = manifest(T::SPY, &params(&[2, 600, 100, 50]), 1).unwrap();
    assert_eq!((m.max_cut_sell_bps, m.max_discount_bps), (0, 50));
    let m = manifest(T::TREATY, &params(&[20, 80, 1]), 1).unwrap();
    assert_eq!(m.max_cut_buy_bps, 80);
}
