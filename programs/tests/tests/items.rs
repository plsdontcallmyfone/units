// Changed by Hookwars: new file (M3b), the items program: callbacks, templates, composites,
// settlement (docs/spec/04-templates.md, 08-arsenal.md).

use anchor_lang::prelude::Pubkey;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::armory::{armory_code, items_code, params, Hw};
use bordrless_program_tests::items::*;
use bordrless_token::instructions::SlotInit;
use bordrless_token::state::{Mint, SlotBounds};
use hookwars_armory::error::ArmoryError;
use hookwars_common::{ids, pda, template_id as T};
use hookwars_items::ItemsError as E;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SUPPLY: u64 = 1_000_000_000;
const SOL: u64 = 1_000_000_000;

fn slot(kind: u8, max_cut_bps: u16, data_len: u8, touch: bool, burn: bool) -> SlotInit {
    SlotInit {
        kind,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: touch,
            may_burn: burn,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// 0 Fee (cuts, 6 bytes), 1 Pool (24 bytes, touch, burns), 2 Defense, 3 Relation.
fn slots() -> Vec<SlotInit> {
    vec![
        slot(slot_kind::FEE, 5_000, 6, false, false),
        slot(slot_kind::POOL, 5_000, 24, true, true),
        slot(slot_kind::DEFENSE, 0, 0, false, false),
        slot(slot_kind::RELATION, 0, 0, false, false),
    ]
}

struct Token {
    owner: Keypair,
    mint: Pubkey,
    pool: Keypair,
}

fn token(hw: &mut Hw) -> Token {
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, slots());
    let pool = Keypair::new();
    let now = hw.w.env.now;
    put_launch(hw, &mint, &pool.pubkey(), now);
    Token { owner, mint, pool }
}

fn item(hw: &mut Hw, template: u16, p: &[u32], royalty_bps: u16) -> Pubkey {
    hw.item(template, params(p), royalty_bps).1
}

fn supply(hw: &Hw, mint: &Pubkey) -> u64 {
    let m: Mint = hw.w.env.read(mint);
    m.supply
}

fn bal(hw: &Hw, mint: &Pubkey, owner: &Pubkey) -> u64 {
    hw.w.env.holding(mint, owner)
}

/// Supply equals the sum of the holdings named (07 invariant 1, for the holdings a suite uses).
fn assert_supply(hw: &Hw, mint: &Pubkey, owners: &[Pubkey]) {
    let sum: u64 = owners.iter().map(|o| bal(hw, mint, o)).sum();
    assert_eq!(sum, supply(hw, mint));
}

// ------------------------------------------------------------------------------------ Half-Life

#[test]
fn half_life_cuts_by_age_and_settles_to_a_burn() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let hl = item(&mut hw, T::HALF_LIFE, &[200_000, 3_600, 4], 1_000);
    equip(&mut hw, &t.owner, &t.mint, 0, hl, vec![], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 1_000_000);

    // Never stamped: the full fee (20%).
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 100_000).ok();
    assert_eq!(bal(&hw, &t.mint, &bob.pubkey()), 80_000);
    assert_eq!(hw.w.env.holding(&t.mint, &pda::equip_state(&t.mint, 0).0), 20_000);
    let st = equip_state(&hw, &t.mint, 0);
    assert_eq!((st.token_unsettled[0], st.collected_token), (20_000, 20_000));

    // Bob's tokens are stamped now: one half-life later the fee is half (10%).
    hw.w.env.warp(3_600);
    let carol = hw.w.env.funded(SOL);
    transfer(&mut hw, &bob, &t.mint, &carol.pubkey(), 10_000).ok();
    assert_eq!(bal(&hw, &t.mint, &carol.pubkey()), 9_000);
    // Four halvings: nothing.
    hw.w.env.warp(4 * 3_600);
    transfer(&mut hw, &bob, &t.mint, &carol.pubkey(), 10_000).ok();
    assert_eq!(bal(&hw, &t.mint, &carol.pubkey()), 19_000);

    // Settle: royalty 10%, bounty 0.5% of the rest, the rest burned.
    let cranker = hw.w.env.funded(SOL);
    let before = supply(&hw, &t.mint);
    let x = 21_000u64;
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 0, &[(t.mint, ids::ITEMS_ID)]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let royalty = x * 1_000 / 10_000;
    let bounty = (x - royalty) * 50 / 10_000;
    let burned = x - royalty - bounty;
    assert_eq!(bal(&hw, &t.mint, &pda::royalty_owner(&hl).0), royalty);
    assert_eq!(bal(&hw, &t.mint, &cranker.pubkey()), bounty);
    assert_eq!(supply(&hw, &t.mint), before - burned);
    assert_eq!(hw.w.env.holding(&t.mint, &pda::equip_state(&t.mint, 0).0), 0);
    assert_eq!(equip_state(&hw, &t.mint, 0).token_unsettled[0], 0);
    assert_supply(
        &hw,
        &t.mint,
        &[
            alice.pubkey(),
            bob.pubkey(),
            carol.pubkey(),
            cranker.pubkey(),
            pda::royalty_owner(&hl).0,
            pda::equip_state(&t.mint, 0).0,
        ],
    );
}

#[test]
fn half_life_buys_are_free_and_stamp() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let hl = item(&mut hw, T::HALF_LIFE, &[200_000, 3_600, 4], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, hl, vec![], 0).ok();
    let pool = t.pool.insecure_clone();
    hw.w.env.fund(pool.pubkey(), SOL);
    hw.mint_to(&t.owner, &t.mint, &pool.pubkey(), 1_000_000);
    let buyer = hw.w.env.funded(SOL);
    transfer(&mut hw, &pool, &t.mint, &buyer.pubkey(), 50_000).ok();
    assert_eq!(bal(&hw, &t.mint, &buyer.pubkey()), 50_000);
    assert_eq!(equip_state(&hw, &t.mint, 0).token_unsettled[0], 0);
}

// ---------------------------------------------------------------------------------- Transfer Fee

#[test]
fn transfer_fee_pays_its_collector_and_caps_wallets() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let collector = hw.w.env.funded(SOL);
    let tf = item(&mut hw, T::TRANSFER_FEE, &[100, 5_000], 0);
    equip(&mut hw, &t.owner, &t.mint, 0, tf, vec![collector.pubkey()], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), SUPPLY);
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 1_000_000).ok();
    assert_eq!(bal(&hw, &t.mint, &bob.pubkey()), 990_000);
    // Over half of the supply in one wallet: refused.
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), SUPPLY * 6 / 10)
        .expect_code(items_code(E::WalletTooLarge));
    // Settle to the collector's holding.
    create_holding(&mut hw, &collector, &t.mint, &collector.pubkey());
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(
        &hw,
        &cranker.pubkey(),
        &t.mint,
        0,
        &[(pda::holding(&t.mint, &collector.pubkey()), ids::ITEMS_ID)],
    );
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let bounty = 10_000 * 50 / 10_000;
    assert_eq!(bal(&hw, &t.mint, &collector.pubkey()), 10_000 - bounty);
    assert_eq!(bal(&hw, &t.mint, &cranker.pubkey()), bounty);
}

// -------------------------------------------------------------------------- Wall, Max Tx, Dust

#[test]
fn wall_caps_wallets_only_under_siege() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let wall = item(&mut hw, T::WALL, &[1_000], 0);
    equip(&mut hw, &t.owner, &t.mint, 2, wall, vec![], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), SUPPLY);
    // No war state: no siege.
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), SUPPLY / 5).ok();
    let now = hw.w.env.now;
    put_war_state(&mut hw, &t.mint, now + 3_600, Pubkey::new_unique());
    let carol = hw.w.env.funded(SOL);
    transfer(&mut hw, &alice, &t.mint, &carol.pubkey(), SUPPLY / 10 + 1)
        .expect_code(items_code(E::WallHolds));
    transfer(&mut hw, &alice, &t.mint, &carol.pubkey(), SUPPLY / 10).ok();
    put_war_state(&mut hw, &t.mint, now - 1, Pubkey::new_unique());
    transfer(&mut hw, &alice, &t.mint, &carol.pubkey(), SUPPLY / 10).ok();
    assert_eq!(bal(&hw, &t.mint, &carol.pubkey()), SUPPLY / 5);
}

#[test]
fn max_transaction_refuses_large_transfers() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let mx = item(&mut hw, T::MAX_TRANSACTION, &[500], 0);
    equip(&mut hw, &t.owner, &t.mint, 2, mx, vec![], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), SUPPLY);
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), SUPPLY / 20 + 1)
        .expect_code(items_code(E::TransferTooLarge));
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), SUPPLY / 20).ok();
}

#[test]
fn dust_guard_refuses_small_wallet_sends_but_not_an_empty_out() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let dg = item(&mut hw, T::DUST_GUARD, &[1_000], 0);
    equip(&mut hw, &t.owner, &t.mint, 2, dg, vec![], 0).ok();
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 5_000);
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 999).expect_code(items_code(E::DustRefused));
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 1_000).ok();
    // Bob empties out under the minimum: allowed.
    let carol = hw.w.env.funded(SOL);
    transfer(&mut hw, &bob, &t.mint, &carol.pubkey(), 1_000).ok();
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 4_000).ok();
    transfer(&mut hw, &bob, &t.mint, &carol.pubkey(), 4_000).ok();
    assert_eq!(bal(&hw, &t.mint, &carol.pubkey()), 5_000);
}

// --------------------------------------------------------------------------- pool templates

fn buy(side: u64, mint: &Pubkey, pool: &Pubkey, before: bool) -> PoolCall {
    PoolCall {
        before,
        direction: 1,
        actor: Pubkey::new_unique(),
        recipient: Pubkey::new_unique(),
        amount_in: side,
        side_amount: side,
        route: plain_route(mint, pool, side),
    }
}

fn sell(side: u64, mint: &Pubkey, pool: &Pubkey, before: bool) -> PoolCall {
    PoolCall {
        direction: 0,
        ..buy(side, mint, pool, before)
    }
}

#[test]
fn side_skew_cuts_the_quote_side_and_settles_to_the_war_chest() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let ss = item(&mut hw, T::SIDE_SKEW, &[100, 200], 1_000);
    equip(&mut hw, &t.owner, &t.mint, 1, ss, vec![], 0).ok();
    let p = t.pool.pubkey();
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true));
    assert_eq!((a.cut, a.discount_bps, a.burn), (10_000, 0, 0));
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, false));
    assert_eq!(a.cut, 0, "a buy's after side is base");
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, false));
    assert_eq!(a.cut, 20_000);
    let st = equip_state(&hw, &t.mint, 1);
    assert_eq!((st.pool_unsettled[0], st.pool_owed), (30_000, 30_000));

    // The launchpad's merged delta reaches PoolCuts; settlement pays royalty, bounty, chest.
    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&t.mint).0, 30_000);
    let chest = Pubkey::find_program_address(&[b"war-chest", t.mint.as_ref()], &ids::WAR_ID).0;
    let sol = hw.w.sol;
    create_holding(&mut hw, &funder, &sol, &chest);
    let cranker = hw.w.env.funded(SOL);
    let ix = settle_ix(&hw, &cranker.pubkey(), &t.mint, 1, &[(ids::ITEMS_ID, pda::holding(&sol, &chest))]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let royalty = 3_000;
    let bounty = (30_000 - royalty) * 50 / 10_000;
    assert_eq!(hw.w.env.holding(&sol, &pda::royalty_owner(&ss).0), royalty);
    assert_eq!(hw.w.env.holding(&sol, &cranker.pubkey()), bounty);
    assert_eq!(hw.w.env.holding(&sol, &chest), 30_000 - royalty - bounty);
    assert_eq!(hw.w.env.holding(&sol, &pda::pool_cuts(&t.mint).0), 0);
    let st = equip_state(&hw, &t.mint, 1);
    assert_eq!((st.pool_unsettled[0], st.pool_settled), (0, 30_000));
}

#[test]
fn size_tiers_launch_decay_and_sell_burn_answer_by_their_rules() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let p = t.pool.pubkey();
    let st = item(&mut hw, T::SIZE_TIERS, &[1_000_000, 100_000_000, 50, 100, 200], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, st, vec![], 0).ok();
    for (side, cut) in [(500_000u64, 2_500u64), (5_000_000, 50_000), (200_000_000, 4_000_000)] {
        let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(side, &t.mint, &p, true));
        assert_eq!(a.cut, cut);
    }

    let mut hw = arsenal();
    let t = token(&mut hw);
    let p = t.pool.pubkey();
    let ld = item(&mut hw, T::LAUNCH_DECAY, &[300, 100, 3_600], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, ld, vec![], 0).ok();
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true));
    assert_eq!(a.cut, 30_000);
    hw.w.env.warp(1_800);
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true));
    assert_eq!(a.cut, 20_000);
    hw.w.env.warp(3_600);
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true));
    assert_eq!(a.cut, 10_000);

    let mut hw = arsenal();
    let t = token(&mut hw);
    let p = t.pool.pubkey();
    let sb = item(&mut hw, T::SELL_BURN, &[250], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, sb, vec![], 0).ok();
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &sell(1_000_000, &t.mint, &p, true));
    assert_eq!((a.burn, a.cut), (25_000, 0));
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true));
    assert_eq!(a.burn, 0);
}

#[test]
fn a_burning_item_needs_a_slot_that_allows_burns() {
    let mut hw = arsenal();
    let owner = hw.w.env.funded(100 * SOL);
    let no_burn = vec![slot(slot_kind::POOL, 0, 24, true, false)];
    let mint = hw.slot_mint(&owner, no_burn);
    let sb = item(&mut hw, T::SELL_BURN, &[250], 0);
    equip(&mut hw, &owner, &mint, 0, sb, vec![], 0).expect_code(armory_code(ArmoryError::OverBounds));
}

#[test]
fn only_the_launchpads_items_signer_may_call_a_pool_item() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let ss = item(&mut hw, T::SIDE_SKEW, &[100, 200], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, ss, vec![], 0).ok();
    let p = t.pool.pubkey();
    let mut ix = pool_call_ix(&hw, &t.mint, &p, 1, &buy(1_000_000, &t.mint, &p, true));
    // Call the items program directly, with the signer slot unsigned.
    ix.program_id = ids::ITEMS_ID;
    ix.accounts.remove(0);
    ix.data = ix.data[12..].to_vec();
    let payer = hw.w.env.payer.insecure_clone();
    hw.w.env.send(&[ix], &[&payer]).expect_code(items_code(E::BadHookSigner));
}

// ------------------------------------------------------------------------------------- Raid

#[test]
fn raid_discounts_raid_buys_marks_them_and_stamps_points_at_delivery() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    put_war_config(&mut hw, 1);
    let rival = Pubkey::new_unique();
    let rival_pool = Pubkey::new_unique();
    let now = hw.w.env.now;
    put_launch(&mut hw, &rival, &rival_pool, now);
    let raid = item(&mut hw, T::RAID, &[2_000, 100, 3], 0);
    equip(&mut hw, &t.owner, &t.mint, 1, raid, vec![rival], 0).ok();
    let payer = t.owner.insecure_clone();
    init_ledger(&mut hw, &payer, &t.mint).ok();
    let p = t.pool.pubkey();
    let buyer = hw.w.env.funded(SOL);
    let amount = 20 * 1_000_000u64;

    // A plain buy, a route from elsewhere, a forged first pool: nothing.
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(amount, &t.mint, &p, true));
    assert_eq!((a.discount_bps, a.cut), (0, 0));
    let mut forged = buy(amount, &t.mint, &p, true);
    forged.route = raid_route(&rival, &Pubkey::new_unique(), &t.mint, amount);
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &forged);
    assert_eq!((a.discount_bps, a.cut), (0, 0));

    // A raid buy: the discount and the toll, then the mark.
    let mut call = buy(amount, &t.mint, &p, true);
    call.recipient = buyer.pubkey();
    call.route = raid_route(&rival, &rival_pool, &t.mint, amount);
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &call);
    assert_eq!((a.discount_bps, a.cut), (2_000, 200_000));
    call.before = false;
    pool_call(&mut hw, &t.mint, &p, 1, &call).0.ok();
    let l = hookwars_common::raid::RaidLedger::decode(&hw.w.env.account(&pda::raid_ledger(&t.mint).0).unwrap().data).unwrap();
    assert_eq!((l.inbound[0].rival_mint, l.inbound[0].volume, l.outbound_volume_season), (rival, amount, amount));
    assert_eq!((l.mark.recipient, l.mark.quote_volume, l.mark.rival), (buyer.pubkey(), amount, rival));

    // The delivery in the same slot stamps 20 units x 3 points and one ticket.
    let pool = t.pool.insecure_clone();
    hw.w.env.fund(pool.pubkey(), SOL);
    hw.mint_to(&t.owner, &t.mint, &pool.pubkey(), 1_000_000);
    transfer(&mut hw, &pool, &t.mint, &buyer.pubkey(), 100_000).ok();
    let raid_bytes = |hw: &Hw, owner: &Pubkey| {
        let h: bordrless_token::state::Holding = hw.w.env.read(&pda::holding(&t.mint, owner));
        let m: Mint = hw.w.env.read(&t.mint);
        let r = bordrless_token::slots::read_range(&h.hook_data, &m.slots[1]);
        hookwars_common::raid::RaidRange::read(&r, 1)
    };
    let r = raid_bytes(&hw, &buyer.pubkey());
    assert_eq!((r.raid_points, r.tickets), (60, 1));
    // A second delivery in the slot does not stamp again.
    transfer(&mut hw, &pool, &t.mint, &buyer.pubkey(), 100_000).ok();
    assert_eq!(raid_bytes(&hw, &buyer.pubkey()).raid_points, 60);
    // Half to a friend: half the points move.
    let friend = hw.w.env.funded(SOL);
    transfer(&mut hw, &buyer, &t.mint, &friend.pubkey(), 100_000).ok();
    assert_eq!(raid_bytes(&hw, &buyer.pubkey()).raid_points, 30);
    assert_eq!(raid_bytes(&hw, &friend.pubkey()).raid_points, 30);
    // A sell into the pool destroys the seller's share.
    transfer(&mut hw, &friend, &t.mint, &pool.pubkey(), 50_000).ok();
    assert_eq!(raid_bytes(&hw, &friend.pubkey()).raid_points, 15);
}

// ------------------------------------------------------------------------------------ Treaty

#[test]
fn a_treaty_pays_only_when_both_sides_equip_it() {
    let mut hw = arsenal();
    let a = token(&mut hw);
    let b = token(&mut hw);
    let treaty = item(&mut hw, T::TREATY, &[100, 200, 0], 0);
    equip(&mut hw, &a.owner, &a.mint, 3, treaty, vec![b.mint], 0).ok();
    let p = a.pool.pubkey();
    let (_, ans) = pool_call(&mut hw, &a.mint, &p, 3, &buy(1_000_000, &a.mint, &p, true));
    assert_eq!(ans.cut, 0, "the partner has not equipped it");
    equip(&mut hw, &b.owner, &b.mint, 3, treaty, vec![a.mint], 0).ok();
    let (_, ans) = pool_call(&mut hw, &a.mint, &p, 3, &buy(1_000_000, &a.mint, &p, true));
    let bps = if a.mint.to_bytes() < b.mint.to_bytes() { 100 } else { 200 };
    assert_eq!(ans.cut, 1_000_000 * bps / 10_000);
}

// --------------------------------------------------------------------------------- composites

#[test]
fn a_composite_runs_its_modules_as_one_item_and_settles_each() {
    let mut hw = arsenal();
    let t = token(&mut hw);
    let author = hw.w.env.funded(10 * SOL);
    let (tx, comp) = create_composite(
        &mut hw,
        &author,
        vec![
            module(T::SIDE_SKEW, params(&[100, 0]), 0, 0),
            module(T::SIZE_TIERS, params(&[1_000_000, 100_000_000, 50, 100, 200]), 0, 0),
            module(T::HALF_LIFE, params(&[200_000, 3_600, 4]), 0, 0),
        ],
        0,
    );
    tx.ok();
    let it: hookwars_armory::state::Item = hw.read_item(&comp);
    assert_eq!((it.template_id, it.manifest.kind, it.manifest.data_bytes), (T::COMPOSITE, slot_kind::POOL, 5));
    equip(&mut hw, &t.owner, &t.mint, 1, comp, vec![], 0).ok();

    // Pool side: both fee modules cut the same base amount.
    let p = t.pool.pubkey();
    let (_, a) = pool_call(&mut hw, &t.mint, &p, 1, &buy(5_000_000, &t.mint, &p, true));
    assert_eq!(a.cut, 50_000 + 50_000);
    let st = equip_state(&hw, &t.mint, 1);
    assert_eq!(&st.pool_unsettled[..3], &[50_000, 50_000, 0]);

    // Token side: the Half-Life module cuts into the vault.
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&t.owner, &t.mint, &alice.pubkey(), 100_000);
    transfer(&mut hw, &alice, &t.mint, &bob.pubkey(), 100_000).ok();
    assert_eq!(bal(&hw, &t.mint, &bob.pubkey()), 80_000);
    assert_eq!(equip_state(&hw, &t.mint, 1).token_unsettled[2], 20_000);

    // Settlement pays each module its own destination.
    let funder = hw.w.env.funded(10 * SOL);
    give_sol(&mut hw, &funder, &pda::pool_cuts(&t.mint).0, 100_000);
    let sol = hw.w.sol;
    let chest = Pubkey::find_program_address(&[b"war-chest", t.mint.as_ref()], &ids::WAR_ID).0;
    create_holding(&mut hw, &funder, &sol, &chest);
    let chest_h = pda::holding(&sol, &chest);
    let cranker = hw.w.env.funded(SOL);
    let supply_before = supply(&hw, &t.mint);
    let ix = settle_ix(
        &hw,
        &cranker.pubkey(),
        &t.mint,
        1,
        &[(ids::ITEMS_ID, chest_h), (ids::ITEMS_ID, chest_h), (t.mint, ids::ITEMS_ID)],
    );
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let b = |x: u64| x * 50 / 10_000;
    assert_eq!(hw.w.env.holding(&sol, &chest), 100_000 - b(50_000) * 2);
    assert_eq!(supply(&hw, &t.mint), supply_before - (20_000 - b(20_000)));
}

#[test]
fn composites_refuse_what_cannot_be_composed() {
    let mut hw = arsenal();
    let author = hw.w.env.funded(10 * SOL);
    // War orders are not a module.
    let (tx, _) = create_composite(
        &mut hw,
        &author,
        vec![module(T::WAR_ORDERS, params(&[10, 100, 600, 500, 600, 3_600, 600, 100, 0, 1, 10]), 0, 0)],
        0,
    );
    tx.expect_code(armory_code(ArmoryError::InvalidSchema));
    // A Relation module and a Pool module have no common host.
    let (tx, _) = create_composite(
        &mut hw,
        &author,
        vec![
            module(T::TREATY, params(&[100, 100, 0]), 0, 1),
            module(T::SIDE_SKEW, params(&[100, 0]), 0, 0),
        ],
        0,
    );
    tx.expect_code(armory_code(ArmoryError::KindMismatch));
    // No module.
    let (tx, _) = create_composite(&mut hw, &author, vec![], 0);
    tx.expect_code(armory_code(ArmoryError::InvalidSchema));
}
