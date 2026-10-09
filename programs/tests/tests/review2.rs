// Changed by Hookwars: new file, regression tests for security review 2 on the launchpad with
// `pool_item_stub` (which answers the default with no return data, as a silent item does).
//! Security review 2 regressions (launchpad side): H-A (a silent pool item must not revert the
//! swap) and L-D (`create_prepared_launch` writes the whole pool registry, so a client resolving
//! the registry right after the launch gets the items' accounts). The real-items side of H-A and
//! the items findings are in `e2e.rs`.

use anchor_lang::prelude::Pubkey;
use bordrless_hook::{AccountSource, HookAccountList};
use bordrless_launch::client as launch;
use bordrless_launch::client::slots as sl;
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::slot_launch::*;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

#[test]
fn h_a_a_silent_item_does_not_revert_a_buy_or_a_sell() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(&creator, 100, LaunchRules::NONE, vec![pool_slot(500, false)], &[(0, BOTH)]);
    w.set_stub(&creator, &items[0], ans(0, 0, 0), ans(0, 0, 0), false, false);
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, 10 * SOL).ok();
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    let got = w.env.holding(&mint, &trader.pubkey());
    assert!(got > 0, "the buy went through");
    w.slot_sell(&trader, &mint, got / 2).ok();
    let s = w.stub_script(&items[0]);
    assert!(s.before_calls >= 2 && s.after_calls >= 2, "the item was called on both sides");
    assert_eq!(w.env.holding(&w.sol.clone(), &sl::pool_cuts_owner(&mint)), 0);
}

#[test]
fn h_a_a_silent_second_item_keeps_the_first_items_cut() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(
        &creator,
        100,
        LaunchRules::NONE,
        vec![pool_slot(300, false), pool_slot(300, false)],
        &[(0, BEFORE), (1, BEFORE)],
    );
    w.set_stub(&creator, &items[0], ans(0, 10, 0), ans(0, 0, 0), false, false);
    w.set_stub(&creator, &items[1], ans(0, 0, 0), ans(0, 0, 0), false, false);
    let trader = w.env.funded(100 * SOL);
    w.wrap_sol(&trader, SOL).ok();
    let sol: Pubkey = w.sol;
    w.slot_buy(&trader, &mint, SOL / 10).ok();
    assert_eq!(w.env.holding(&sol, &sl::pool_cuts_owner(&mint)), 10, "only the first item's cut");
    assert_eq!(w.stub_script(&items[1]).collected, 0);
}

#[test]
fn l_d_the_launch_writes_the_whole_pool_registry() {
    let mut w = World::with_slot_launches();
    let creator = w.env.funded(1_000 * SOL);
    let (mint, items) = w.slot_launch(&creator, 100, LaunchRules::NONE, vec![pool_slot(500, false)], &[(0, BOTH)]);
    // No refresh_pool_registry: the registry already names the item.
    let pool = w.launch_pool_key(&mint);
    let data = w.env.svm.get_account(&launch::registry_address(&pool)).unwrap().data;
    let keys: Vec<Pubkey> = HookAccountList::decode(&data)
        .unwrap()
        .accounts
        .iter()
        .map(|a| match &a.source {
            AccountSource::Key(k) => *k,
            _ => Pubkey::default(),
        })
        .collect();
    assert_eq!(keys.len(), 5 + 3, "upstream's four, the PoolCuts holding, the item's three");
    assert_eq!(keys[5], pool_item_stub::ID);
    assert_eq!(keys[6], sl::item_signer(&pool_item_stub::ID));
    assert_eq!(keys[7], pool_item_stub::script_address(&items[0]));
}
