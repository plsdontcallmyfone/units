// Changed by Hookwars: new file (arsenal wave E), Target Burn (08 section 4.6).

use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::equip;
use hookwars_common::arsenal2 as a2;
use solana_signer::Signer;

#[test]
fn target_burn_burns_the_base_side_until_the_target() {
    let mut hw = world();
    let t = tok(&mut hw);
    let holder = hw.w.env.funded(SOL);
    let tb = item(&mut hw, a2::TARGET_BURN, &[100, 9_000], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, tb, vec![], 0).ok();
    hw.mint_to(&t.owner, &t.mint, &holder.pubkey(), 1_000_000_000);
    set_launch_supply(&mut hw, &t.mint, 750_000_000, 250_000_000);
    let p = t.pool.pubkey();

    // A sell's input and a buy's output are base: 1% burns.
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.burn, 10_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.burn, 10_000);
    // The quote sides never burn.
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.burn, 0);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.burn, 0);

    // Close to the target the burn stops at it; at the target, nothing.
    let holder_burn = 1_000_000_000 - 900_000_005;
    let ix = hw.w.slot_burn_ix(&holder.pubkey(), &t.mint, holder_burn);
    hw.w.env.send_paid_by(&[ix], &holder, &[]).ok();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.burn, 5);
    let ix = hw.w.slot_burn_ix(&holder.pubkey(), &t.mint, 5);
    hw.w.env.send_paid_by(&[ix], &holder, &[]).ok();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.burn, 0);
}

#[test]
fn target_burn_forges_the_rate_up_and_keeps_the_target() {
    let f = forge(a2::TARGET_BURN, 5_000, &[100, 9_000], &[200, 9_000]).unwrap();
    assert_eq!(f[..2], [250, 9_000]);
    assert!(forge(a2::TARGET_BURN, 5_000, &[100, 9_000], &[200, 8_000]).is_err());
}
