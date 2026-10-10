// Changed by Hookwars: new file.
//! D-4 (17-randomness): loot rolls on Switchboard On-Demand randomness, read directly by the war
//! program. The Switchboard account is a fixture with the layout of `switchboard-on-demand` 0.13.0
//! `RandomnessAccountData`, owned by the devnet On-Demand program; Switchboard's own commit and
//! reveal instructions are what the fixture's slots stand for (the client puts them first in the
//! same transaction).

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::AccountSerialize;
use bordrless_program_tests::kit::SOL;
use bordrless_program_tests::war::*;
use bordrless_token::client as token;
use hookwars_war::client as war;
use hookwars_war::error::WarError;
use hookwars_war::events::*;
use hookwars_war::oracle::{SbRandomness, SWITCHBOARD_DEVNET, SWITCHBOARD_MAINNET};
use hookwars_war::state::*;
use solana_account::Account;
use solana_keypair::Keypair;
use solana_signer::Signer;

fn loot_log(owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"loot-log", owner.as_ref()], &war_armory_stub::ID).0
}

/// A season open, a war token, a holder with `tickets` loot tickets and a loot log, with the war
/// config naming the Switchboard devnet program (as the admin would through the timelock).
fn world(tickets: u16) -> (WarWorld, WarToken, Keypair) {
    let mut ww = WarWorld::new();
    let t = ww.war_token("SBLOOT", OrdersSpec::default());
    let s = ww.open_season(ScoreWeights::default());
    let holder = ww.buyer(&t.mint, SOL);
    ww.set_raid(&t.mint, &holder.pubkey(), s.number, 0, tickets);
    let log = war_armory_stub::LootLog {
        owner: holder.pubkey(),
        ..Default::default()
    };
    put_anchor(&mut ww, loot_log(&holder.pubkey()), &log, war_armory_stub::ID);
    let mut config = ww.config();
    config.randomness_program = SWITCHBOARD_DEVNET;
    put_anchor(&mut ww, war::config_address(), &config, hookwars_war::ID);
    (ww, t, holder)
}

fn put_anchor<T: AccountSerialize>(ww: &mut WarWorld, key: Pubkey, value: &T, owner: Pubkey) {
    let mut data = Vec::new();
    value.try_serialize(&mut data).unwrap();
    let lamports = ww.w.env.rent(data.len()).max(ww.w.env.account(&key).map_or(0, |a| a.lamports));
    ww.w.env.put(key, Account { lamports, data, owner, executable: false, rent_epoch: 0 });
}

/// Writes the Switchboard randomness account at `key`, owned by `owner`.
fn put_sb(ww: &mut WarWorld, key: Pubkey, sb: SbRandomness, owner: Pubkey) {
    let data = sb.to_bytes();
    let lamports = ww.w.env.rent(data.len());
    ww.w.env.put(key, Account { lamports, data, owner, executable: false, rent_epoch: 0 });
}

/// A fresh commit by `authority`: seeded on the previous slot, not revealed.
fn committed(ww: &WarWorld, authority: Pubkey) -> SbRandomness {
    SbRandomness { authority, seed_slot: ww.w.env.slot - 1, reveal_slot: 0, value: [0; 32] }
}

fn roll_ix(t: &WarToken, owner: &Pubkey, nonce: u64, rng: Pubkey) -> Instruction {
    war::roll(*owner, t.mint, nonce, RAID_SLOT, SWITCHBOARD_DEVNET, rng, vec![])
}

fn reveal_ix(ww: &WarWorld, t: &WarToken, owner: &Pubkey, revealer: &Pubkey, nonce: u64, rng: Pubkey) -> Instruction {
    let holding = token::holding_address(&t.mint, owner);
    let season = ww.config().current_season;
    war::reveal(*revealer, *owner, holding, nonce, season, rng, vec![AccountMeta::new(loot_log(owner), false)])
}

#[test]
fn the_switchboard_layout_reads_the_documented_offsets() {
    // Built field by field from `RandomnessAccountData` (repr(C)): discriminator, authority, queue,
    // seed_slothash, seed_slot, oracle, reveal_slot, value, 224 reserved.
    let authority = Pubkey::new_unique();
    let mut d = Vec::new();
    d.extend_from_slice(&[10, 66, 229, 135, 220, 239, 217, 114]);
    d.extend_from_slice(authority.as_ref());
    d.extend_from_slice(&[7; 32]); // queue
    d.extend_from_slice(&[8; 32]); // seed_slothash
    d.extend_from_slice(&41u64.to_le_bytes());
    d.extend_from_slice(&[9; 32]); // oracle
    d.extend_from_slice(&42u64.to_le_bytes());
    d.extend_from_slice(&[5; 32]);
    d.extend_from_slice(&[0; 224]);
    assert_eq!(d.len(), SbRandomness::LEN);
    let sb = SbRandomness::parse(&d).unwrap();
    assert_eq!(sb, SbRandomness { authority, seed_slot: 41, reveal_slot: 42, value: [5; 32] });
    let mut fixture = sb.to_bytes();
    fixture[40..104].copy_from_slice(&d[40..104]);
    fixture[112..144].copy_from_slice(&d[112..144]);
    assert_eq!(fixture, d);
    // A short account or another discriminator is refused.
    assert!(SbRandomness::parse(&d[..400]).is_err());
    let mut other = d.clone();
    other[0] ^= 1;
    assert!(SbRandomness::parse(&other).is_err());
    assert_ne!(SWITCHBOARD_MAINNET, SWITCHBOARD_DEVNET);
}

#[test]
fn an_honest_roll_commits_then_reveals_in_the_reveal_slot() {
    let (mut ww, t, holder) = world(2);
    let rng = Pubkey::new_unique();
    let sb = committed(&ww, holder.pubkey());
    put_sb(&mut ww, rng, sb, SWITCHBOARD_DEVNET);
    let tx = ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder]);
    let e = tx.event::<RollRequested>();
    assert_eq!((e.oracle_program, e.oracle_account), (SWITCHBOARD_DEVNET, rng));
    assert_eq!(e.requested_slot, sb.seed_slot);
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 1);
    let roll = e.roll;

    // Not revealed yet.
    ww.w.env.warp(1);
    let revealer = ww.w.env.funded(SOL);
    let ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 1, rng);
    ww.w.env
        .send_paid_by(std::slice::from_ref(&ix), &revealer, &[])
        .expect_code(war_code(WarError::RandomnessNotReady));

    // Revealed in an earlier slot: Switchboard's freshness rule refuses it.
    let sb_value = SbRandomness { reveal_slot: ww.w.env.slot, ..sb };
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    ww.w.env.warp(1);
    let ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 1, rng);
    ww.w.env
        .send_paid_by(std::slice::from_ref(&ix), &revealer, &[])
        .expect_code(war_code(WarError::RandomnessNotReady));

    // Revealed in this slot (Switchboard's reveal first in the same transaction): minted.
    let sb_value = SbRandomness { reveal_slot: ww.w.env.slot, ..sb };
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    let owner_sol = ww.w.env.lamports(&holder.pubkey());
    let e = ww.w.env.send_paid_by(&[ix], &revealer, &[]).event::<RollRevealed>();
    assert_eq!(e.template_id, RAID_TEMPLATE);
    assert_eq!(&e.params[..3], &[100, 0, 1]);
    let log: war_armory_stub::LootLog = ww.w.env.read(&loot_log(&holder.pubkey()));
    assert_eq!((log.count, log.last_template_id), (1, RAID_TEMPLATE));
    assert!(ww.w.env.account(&roll).is_none_or(|a| a.lamports == 0));
    assert!(ww.w.env.lamports(&holder.pubkey()) > owner_sol);
}

#[test]
fn a_roll_refuses_a_randomness_account_of_another_owner_or_authority() {
    let (mut ww, t, holder) = world(1);
    // The Switchboard layout, owned by another program.
    let rng = Pubkey::new_unique();
    let sb_value = committed(&ww, holder.pubkey());
    put_sb(&mut ww, rng, sb_value, randomness_stub::ID);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder])
        .expect_code(war_code(WarError::WrongRandomness));
    // The mainnet program while the config names devnet: the oracle program check refuses it.
    let ix = war::roll(holder.pubkey(), t.mint, 1, RAID_SLOT, SWITCHBOARD_MAINNET, rng, vec![]);
    ww.w.env.send(&[ix], &[&holder]).expect_code(war_code(WarError::WrongRandomness));
    // Someone else's randomness account (they could re-commit it and strand the roll).
    let other = Pubkey::new_unique();
    let sb_value = committed(&ww, other);
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder])
        .expect_code(war_code(WarError::RandomnessAuthority));
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 1);
}

#[test]
fn a_roll_refuses_a_stale_or_already_revealed_commit() {
    let (mut ww, t, holder) = world(1);
    let rng = Pubkey::new_unique();
    let slot = ww.w.env.slot;
    // Committed two slots ago: not this transaction's commit.
    let sb_value = SbRandomness { seed_slot: slot - 2, ..committed(&ww, holder.pubkey()) };
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder])
        .expect_code(war_code(WarError::RandomnessStale));
    // Reused: the account still holds an earlier commit that was revealed (its value is public).
    let sb_value = SbRandomness { seed_slot: slot - 1, reveal_slot: slot - 1, value: [9; 32], ..committed(&ww, holder.pubkey()) };
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder])
        .expect_code(war_code(WarError::RandomnessStale));
    // A commit in the future slot cannot exist; a seed slot equal to this slot is refused too.
    let sb_value = SbRandomness { seed_slot: slot, ..committed(&ww, holder.pubkey()) };
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    ww.w.env
        .send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder])
        .expect_code(war_code(WarError::RandomnessStale));
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 1);
}

#[test]
fn a_reveal_refuses_a_recommitted_account() {
    let (mut ww, t, holder) = world(1);
    let rng = Pubkey::new_unique();
    let sb = committed(&ww, holder.pubkey());
    put_sb(&mut ww, rng, sb, SWITCHBOARD_DEVNET);
    ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder]).ok();
    // The authority commits again later and reveals: the value is not the roll's commit.
    ww.w.env.warp(2);
    let slot = ww.w.env.slot;
    put_sb(&mut ww, rng, SbRandomness { seed_slot: slot - 1, reveal_slot: slot, value: [3; 32], ..sb }, SWITCHBOARD_DEVNET);
    let revealer = ww.w.env.funded(SOL);
    let ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 1, rng);
    ww.w.env
        .send_paid_by(&[ix], &revealer, &[])
        .expect_code(war_code(WarError::RandomnessStale));
    // A reveal naming another account than the roll's is refused by its address.
    let other = Pubkey::new_unique();
    put_sb(&mut ww, other, SbRandomness { reveal_slot: slot, ..sb }, SWITCHBOARD_DEVNET);
    let ix = reveal_ix(&ww, &t, &holder.pubkey(), &revealer.pubkey(), 1, other);
    ww.w.env
        .send_paid_by(&[ix], &revealer, &[])
        .expect_code(war_code(WarError::WrongRandomness));
}

#[test]
fn an_unrevealed_switchboard_roll_expires_without_returning_the_ticket() {
    let (mut ww, t, holder) = world(1);
    let rng = Pubkey::new_unique();
    let sb_value = committed(&ww, holder.pubkey());
    put_sb(&mut ww, rng, sb_value, SWITCHBOARD_DEVNET);
    ww.w.env.send(&[roll_ix(&t, &holder.pubkey(), 1, rng)], &[&holder]).ok();
    let holding = token::holding_address(&t.mint, &holder.pubkey());
    let cancel = war::cancel_roll(holder.pubkey(), holding, 1);
    ww.w.env
        .send(std::slice::from_ref(&cancel), &[&holder])
        .expect_code(war_code(WarError::RollNotExpired));
    ww.w.env.warp(TEST_PARAMS.roll_expiry_secs);
    let e = ww.w.env.send(&[cancel], &[&holder]).event::<RollCancelled>();
    assert_eq!(e.owner, holder.pubkey());
    assert_eq!(ww.raid(&t.mint, &holder.pubkey()).2, 0);
    assert!(ww.w.env.account(&RollRequest::address(&holding, 1).0).is_none_or(|a| a.lamports == 0));
}
