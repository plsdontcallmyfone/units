// Changed by Hookwars: new file.
//! Loot (05 section 8): `roll` spends a ticket through `touch` and asks the randomness adapter;
//! `reveal` takes only a value fulfilled after the request and bound to it (never the slot hash),
//! draws from the season's loot table and asks the armory's `mint_loot` as the loot signer;
//! `cancel_roll` returns rent after expiry and never the ticket.

use anchor_lang::{InstructionData, ToAccountMetas};
use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_token::client as token;
use hookwars_war::client as war;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use hookwars_war::state::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn randomness_of(roll: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"randomness", roll.as_ref()], &randomness_stub::ID).0
}

fn loot_log(owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"loot-log", owner.as_ref()], &war_armory_stub::ID).0
}

fn fulfill(ww: &mut WarWorld, roll: &Pubkey, value: [u8; 32]) {
    let oracle = ww.w.env.funded(SOL);
    let ix = Instruction {
        program_id: randomness_stub::ID,
        accounts: randomness_stub::accounts::Fulfill {
            authority: oracle.pubkey(),
            randomness: randomness_of(roll),
        }
        .to_account_metas(None),
        data: randomness_stub::instruction::Fulfill { value }.data(),
    };
    ww.w.env.send(&[ix], &[&oracle]).ok();
}

/// A season open, a war token, a holder with `tickets` loot tickets and a loot log.
fn world(tickets: u16) -> (WarWorld, WarToken, Keypair) {
    let mut ww = WarWorld::new();
    let t = ww.war_token("LOOT", OrdersSpec::default());
    let s = ww.open_season(ScoreWeights::default());
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, 0, tickets);
    let log = war_armory_stub::LootLog {
        owner: holder.pubkey(),
        ..Default::default()
    };
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&log, &mut data).unwrap();
    let lamports = ww.w.env.rent(data.len());
    ww.w.env.put(
        loot_log(&holder.pubkey()),
        Account {
            lamports,
            data,
            owner: war_armory_stub::ID,
            executable: false,
            rent_epoch: 0,
        },
    );
    (ww, t, holder)
}

fn roll_ix(t: &WarToken, owner: &Pubkey, nonce: u64) -> Instruction {
    let holding = token::holding_address(&t.mint, owner);
    let roll = RollRequest::address(&holding, nonce).0;
    war::roll(*owner, t.mint, nonce, RAID_SLOT, randomness_stub::ID, randomness_of(&roll), vec![])
}

fn reveal_ix(ww: &WarWorld, t: &WarToken, owner: &Pubkey, revealer: &Pubkey, nonce: u64) -> Instruction {
    let holding = token::holding_address(&t.mint, owner);
    let roll = RollRequest::address(&holding, nonce).0;
    let season = ww.config().current_season;
    war::reveal(
        *revealer,
        *owner,
        holding,
        nonce,
        season,
        randomness_of(&roll),
        vec![AccountMeta::new(loot_log(owner), false)],
    )
}

#[test]
fn a_roll_spends_a_ticket_and_a_reveal_mints_from_the_table() {
    let (mut ww, t, holder) = world(2);
    let tx = ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 1)], &[&holder]);
    let e = tx.event::<RollRequested>();
    assert_eq!(e.oracle_program, randomness_stub::ID);
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 1);
    let roll = e.roll;
    // Not fulfilled yet.
    let revealer = ww.w.env.funded(SOL);
    let ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 1);
    ww.w.env
        .send_paid_by(&[ix.clone()], &revealer, &[])
        .expect_code(war_code(WarError::RandomnessNotReady));
    // Fulfilled in the request's own slot: refused (a value must come after the request).
    fulfill(&mut ww, &roll, [0; 32]);
    ww.w.env
        .send_paid_by(&[ix.clone()], &revealer, &[])
        .expect_code(war_code(WarError::RandomnessNotReady));
    // A later slot: revealed. Value 0 draws the first entry at its minimums.
    ww.w.env.warp(1);
    fulfill(&mut ww, &roll, [0; 32]);
    let owner_sol = ww.w.env.lamports(&holder.pubkey());
    let tx = ww.w.env.send_paid_by(&[ix], &revealer, &[]);
    let e = tx.event::<RollRevealed>();
    assert_eq!(e.template_id, RAID_TEMPLATE);
    assert_eq!(&e.params[..3], &[100, 0, 1]);
    let log: war_armory_stub::LootLog = ww.w.env.read(&loot_log(&holder.pubkey()));
    assert_eq!((log.count, log.last_template_id), (1, RAID_TEMPLATE));
    assert_eq!(log.last_params, e.params);
    // The request is closed, its rent back to the owner.
    assert!(ww.w.env.account(&roll).is_none_or(|a| a.lamports == 0));
    assert!(ww.w.env.lamports(&holder.pubkey()) > owner_sol);
}

#[test]
fn a_draw_follows_the_weights_and_stays_in_range() {
    let (mut ww, t, holder) = world(1);
    ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 7)], &[&holder]).ok();
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let roll = RollRequest::address(&holding, 7).0;
    ww.w.env.warp(1);
    // Weights 3 and 1: a pick of 3 is the second entry; every field at its top bytes.
    let mut value = [0xffu8; 32];
    value[..8].copy_from_slice(&3u64.to_le_bytes());
    fulfill(&mut ww, &roll, value);
    let revealer = ww.w.env.funded(SOL);
    let ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 7);
    let e = ww.w.env.send_paid_by(&[ix], &revealer, &[]).event::<RollRevealed>();
    assert_eq!(e.template_id, WAR_ORDERS_TEMPLATE);
    let entries = WarWorld::loot_entries();
    for (i, p) in e.params.iter().enumerate() {
        let r = entries[1].ranges[i];
        assert!(r.min <= *p && *p <= r.max, "field {i}: {p} outside {r:?}");
    }
}

#[test]
fn a_reveal_takes_only_its_own_randomness() {
    let (mut ww, t, holder) = world(2);
    ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 1)], &[&holder]).ok();
    ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 2)], &[&holder]).ok();
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let other = RollRequest::address(&holding, 2).0;
    ww.w.env.warp(1);
    fulfill(&mut ww, &other, [9; 32]);
    let revealer = ww.w.env.funded(SOL);
    let mut ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 1);
    // Point roll 1's reveal at roll 2's randomness.
    for m in ix.accounts.iter_mut() {
        if m.pubkey == randomness_of(&RollRequest::address(&holding, 1).0) {
            m.pubkey = randomness_of(&other);
        }
    }
    ww.w.env
        .send_paid_by(&[ix], &revealer, &[])
        .expect_code(war_code(WarError::WrongRandomness));
}

#[test]
fn a_roll_needs_a_ticket_an_open_season_and_the_configured_oracle() {
    let (mut ww, t, holder) = world(0);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1)], &[&holder])
        .expect_code(war_code(WarError::NothingToDo));
    ww.set_raid(&t.mint, &holder.pubkey(), 1, 0, 1);
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let roll = RollRequest::address(&holding, 1).0;
    let ix = war::roll(holder.pubkey(), t.mint, 1, RAID_SLOT, hookwars_war::ID, randomness_of(&roll), vec![]);
    ww.w.env
        .send(&[ix], &[&holder])
        .expect_code(war_code(WarError::WrongRandomness));

    let mut ww = WarWorld::new();
    let t = ww.war_token("NOSZN", OrdersSpec::default());
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), 0, 0, 1);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1)], &[&holder])
        .expect_code(war_code(WarError::SeasonNotOpen));
}

#[test]
fn a_cancelled_roll_returns_rent_and_never_the_ticket() {
    let (mut ww, t, holder) = world(1);
    ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 1)], &[&holder]).ok();
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let cancel = war::cancel_roll(holder.pubkey(), holding, 1);
    ww.w.env
        .send(&[cancel.clone()], &[&holder])
        .expect_code(war_code(WarError::RollNotExpired));
    ww.w.env.warp(TEST_PARAMS.roll_expiry_secs);
    let e = ww.w.env.send(&[cancel], &[&holder]).event::<RollCancelled>();
    assert_eq!(e.owner, holder.pubkey());
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 0);
}

#[test]
fn only_the_loot_signer_mints_loot() {
    let (mut ww, _t, holder) = world(0);
    let ix = Instruction {
        program_id: war_armory_stub::ID,
        accounts: vec![
            AccountMeta::new_readonly(holder.pubkey(), true),
            AccountMeta::new(holder.pubkey(), true),
            AccountMeta::new_readonly(holder.pubkey(), false),
            AccountMeta::new(loot_log(&holder.pubkey()), false),
        ],
        data: war_armory_stub::instruction::MintLoot {
            owner: holder.pubkey(),
            template_id: RAID_TEMPLATE,
            params: [0; hookwars_war::foreign::PARAM_FIELDS],
        }
        .data(),
    };
    ww.w.env.send(&[ix], &[&holder]).expect_fail();
}
