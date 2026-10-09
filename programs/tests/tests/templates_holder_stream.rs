// Changed by Hookwars: new file (arsenal wave E), Holder Stream (08 section 4.3).

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::arsenal2::*;
use bordrless_program_tests::items::{create_holding, equip, give_sol, settle_ix};
use hookwars_common::arsenal2 as a2;
use hookwars_common::{ids, pda};
use solana_signer::Signer;

#[test]
fn holder_stream_cuts_both_quote_sides_into_the_treaty_inbox() {
    let mut hw = world();
    let t = tok(&mut hw);
    let hs = item(&mut hw, a2::HOLDER_STREAM, &[100, 200], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, hs, vec![], 0).ok();
    let p = t.pool.pubkey();
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true), &[]);
    assert_eq!(a.cut, 10_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 20_000);
    let (_, a) = pool_call_with(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, false), &[]);
    assert_eq!(a.cut, 0, "a buy's after side is base");

    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&t.mint).0, 30_000);
    let inbox = Pubkey::find_program_address(&[b"treaty-inbox", t.mint.as_ref()], &ids::WAR_ID).0;
    let sol = hw.w.sol;
    create_holding(&mut hw, &funder, &sol, &inbox);
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 1, &[(ids::ITEMS_ID, pda::holding(&sol, &inbox))]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let bps = u64::from(hw.config().params.settle_bounty_bps);
    assert_eq!(bal(&hw, &sol, &inbox), 30_000 - 30_000 * bps / 10_000);
}

#[test]
fn holder_stream_forges_up() {
    let f = forge(a2::HOLDER_STREAM, 0, &[100, 50], &[50, 200]).unwrap();
    assert_eq!(f[..2], [100, 200]);
}
