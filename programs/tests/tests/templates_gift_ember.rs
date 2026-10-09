// Changed by Hookwars: new file (arsenal wave E), Gift Ember (08 section 4.6).

use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{equip, equip_state, settle_ix, transfer};
use hookwars_common::arsenal2 as a2;
use hookwars_common::pda;
use solana_signer::Signer;

#[test]
fn gift_ember_cuts_sends_only_and_burns_them_at_settlement() {
    let mut hw = world();
    let t = tok(&mut hw);
    let ge = item(&mut hw, a2::GIFT_EMBER, &[500], 1_000);
    equip(&mut hw, &t.owner, &t.mint, 0, ge, vec![], 0).ok();
    let pool = t.pool.insecure_clone();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &pool.pubkey(), 1_000_000);

    // A buy and a sell pay nothing.
    transfer(&mut hw, &pool, &t.mint, &alice.pubkey(), 200_000).ok();
    assert_eq!(bal(&hw, &t.mint, &alice.pubkey()), 200_000);
    transfer(&mut hw, &alice, &t.mint, &pool.pubkey(), 50_000).ok();
    assert_eq!(bal(&hw, &t.mint, &alice.pubkey()), 150_000);
    // A send pays 5% into the vault.
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 100_000).ok();
    assert_eq!(bal(&hw, &t.mint, &bob.pubkey()), 95_000);
    let vault = pda::equip_state(&t.mint, 0).0;
    assert_eq!(bal(&hw, &t.mint, &vault), 5_000);
    assert_eq!(equip_state(&hw, &t.mint, 0).token_unsettled[0], 5_000);

    // Settlement: royalty and bounty, the rest burned.
    let cranker = hw.w.env.funded(SOL);
    let before = supply(&hw, &t.mint);
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 0, &[(t.mint, hookwars_common::ids::ITEMS_ID)]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let bps = u64::from(hw.config().params.settle_bounty_bps);
    let royalty = 500u64;
    let bounty = (5_000 - royalty) * bps / 10_000;
    assert_eq!(supply(&hw, &t.mint), before - (5_000 - royalty - bounty));
    assert_supply(
        &hw,
        &t.mint,
        &[
            pool.pubkey(),
            alice.pubkey(),
            bob.pubkey(),
            cranker.pubkey(),
            vault,
            pda::royalty_owner(&ge).0,
        ],
    );
}

#[test]
fn gift_ember_forges_up() {
    assert_eq!(forge(a2::GIFT_EMBER, 5_000, &[200], &[100]).unwrap()[0], 600);
}
