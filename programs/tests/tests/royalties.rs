// Changed by Hookwars: new file (M2), royalty holdings and claims by the item's current holder
// (docs/spec/02-armory.md sections 2.6 and 5; R16).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::armory::*;
use bordrless_token::client as token;
use hookwars_armory::error::ArmoryError as E;
use hookwars_common::{ids, pda, template_id as T, EquipConfig};
use solana_keypair::Keypair;
use solana_signer::Signer;

fn claim_ix(claimant: &Keypair, item: &Pubkey, item_mint: &Pubkey, cut_mint: &Pubkey, amount: u64)
    -> anchor_lang::solana_program::instruction::Instruction {
    let owner = pda::royalty_owner(item).0;
    armory_ix(
        hookwars_armory::accounts::ClaimRoyalty {
            claimant: claimant.pubkey(),
            item: *item,
            item_holding: token::holding_address(item_mint, &claimant.pubkey()),
            royalty_owner: owner,
            cut_mint: *cut_mint,
            royalty_holding: pda::holding(cut_mint, &owner),
            destination: pda::holding(cut_mint, &claimant.pubkey()),
            token: token_accounts(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::ClaimRoyalty { amount },
    )
}

#[test]
fn royalty_goes_to_whoever_holds_the_item_and_skips_the_items() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let collector = Pubkey::new_unique();
    // A Transfer Fee item in the Fee slot: it cuts on the token side, so its royalty owner gets a
    // holding of the token when it is equipped.
    let (author, fee, fee_mint) = hw.item(T::TRANSFER_FEE, params(&[100, 0]), 1_000);
    hw.equip_launch(
        &owner,
        &mint,
        Hw::entry(
            2,
            Some(fee),
            EquipConfig {
                targets: vec![collector],
                role: 0,
            },
        ),
    )
    .ok();
    let royalty_owner = pda::royalty_owner(&fee).0;
    assert!(hw.w.env.account(&pda::holding(&mint, &royalty_owner)).is_some());
    // Royalty accrues there (settle_equip is M3; mint straight into the holding).
    hw.mint_to(&owner, &mint, &royalty_owner, 50);
    assert_eq!(hw.w.env.holding(&mint, &royalty_owner), 50);

    // A transfer of this token runs the Fee item, whose callbacks are M3: it fails today...
    let o = owner.pubkey();
    hw.mint_to(&owner, &mint, &o, 10);
    let send = hw.w.slot_transfer_ix(&o, &mint, &author.pubkey(), 1);
    hw.w.env.send_paid_by(&[send], &owner, &[]).expect_fail();
    // ... while a royalty claim does not run it (R16: protocol payouts skip item slots).
    hw.w.env
        .send_paid_by(&[claim_ix(&author, &fee, &fee_mint, &mint, 20)], &author, &[])
        .ok();
    assert_eq!(hw.w.env.holding(&mint, &author.pubkey()), 20);
    assert_eq!(hw.w.env.holding(&mint, &royalty_owner), 30);
    hw.w.env
        .send_paid_by(&[claim_ix(&author, &fee, &fee_mint, &mint, 31)], &author, &[])
        .expect_code(armory_code(E::InsufficientRoyalty));

    // The item is sold: the buyer takes everything accrued, the author nothing.
    let buyer = hw.w.env.funded(1_000_000_000);
    let move_item = [
        token::create_holding(author.pubkey(), fee_mint, buyer.pubkey()),
        token::transfer(
            author.pubkey(),
            token::holding_address(&fee_mint, &author.pubkey()),
            token::holding_address(&fee_mint, &buyer.pubkey()),
            fee_mint,
            None,
            vec![],
            1,
        ),
    ];
    hw.w.env.send_paid_by(&move_item, &author, &[]).ok();
    hw.w.env
        .send_paid_by(&[claim_ix(&author, &fee, &fee_mint, &mint, 1)], &author, &[])
        .expect_code(armory_code(E::NotItemOwner));
    hw.w.env
        .send_paid_by(&[claim_ix(&buyer, &fee, &fee_mint, &mint, u64::MAX)], &buyer, &[])
        .ok();
    assert_eq!(hw.w.env.holding(&mint, &buyer.pubkey()), 30);
    assert_eq!(hw.w.env.holding(&mint, &royalty_owner), 0);
}

#[test]
fn pool_items_get_a_bridged_sol_royalty_holding() {
    let mut hw = Hw::new();
    let owner = hw.w.env.funded(10_000_000_000);
    let mint = hw.slot_mint(&owner, test_slots());
    let (_, treaty, _) = hw.item(T::TREATY, params(&[50, 50, 0]), 300);
    let mut e = Hw::entry(
        3,
        Some(treaty),
        EquipConfig {
            targets: vec![Pubkey::new_unique()],
            role: 0,
        },
    );
    e.rule = Some(hookwars_common::PerformanceRule {
        metric: 0,
        window_secs: 60,
        base_window_secs: 600,
        op: 0,
        ratio_bps: 5_000,
        hold_secs: 60,
    });
    hw.equip_launch(&owner, &mint, e).ok();
    let r = pda::royalty_owner(&treaty).0;
    assert!(hw.w.env.account(&pda::holding(&hw.w.sol, &r)).is_some());
    // No token-side cut: no holding of the token.
    assert!(hw.w.env.account(&pda::holding(&mint, &r)).is_none());
}
