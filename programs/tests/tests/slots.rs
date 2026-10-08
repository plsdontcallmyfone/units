// Changed by Hookwars: new file, M1 slot tests (docs/spec/01-token-slots.md section 10).
//! Slots in the token program: merged answers, each cut into its slot's equip vault, bounds,
//! ranges and the epoch byte, the Locked slot's masking, `mint_to` calling only the Locked slot,
//! Pool and War slots, the slot authority, vote locks, `touch`, protocol sources (R16) and the
//! table rules. `slot_tester` is the scripted item, `armory_stub` signs as `["slots", mint]`,
//! `hook_tester` stands in for a Locked legacy hook.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use bordrless_hook::{
    equip_rule as rule, slot_flags as f, slot_kind as kind, token_flags, Delta, HookReturn,
    SlotReturn,
};
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::slots::*;
use bordrless_token::client as token;
use bordrless_token::constants::ARMORY_ID;
use bordrless_token::error::TokenError;
use bordrless_token::events::{HookDataWritten, SlotCut, Transferred};
use bordrless_token::instructions::{CreateMintArgs, SlotInit};
use bordrless_token::slots::SlotOp;
use bordrless_token::state::{Holding, Mint, SlotBounds};
use hook_tester::client as tester;
use slot_tester::{callback as cb, mode};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;
const SUPPLY: u64 = 1_000_000;
/// Index of a `slot_tester` item's equip vault in its callback (prefix 5, script 5, vault 6).
const VAULT: u8 = 6;

fn code(e: TokenError) -> u32 {
    u32::from(e)
}

fn cut(amount: u64) -> Vec<u8> {
    answer_bytes(&SlotReturn {
        deltas: vec![Delta {
            amount,
            account: VAULT,
        }],
        ..SlotReturn::default()
    })
}

/// A world, an owner holding `SUPPLY` of a new slot mint with `slots`, and the mint. Slot items
/// are equipped by the caller after.
fn world(slots: Vec<SlotInit>) -> (World, Keypair, Pubkey) {
    let mut w = World::with_slots();
    let owner = w.env.funded(100 * SOL);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    w.create_slot_mint(&owner, &mint_kp, slots, true).ok();
    (w, owner, mint)
}

fn supply(w: &mut World, owner: &Keypair, mint: &Pubkey) {
    let ix = w.slot_mint_ix(&owner.pubkey(), mint, &owner.pubkey(), SUPPLY);
    w.env.send_paid_by(&[ix], owner, &[]).ok();
}

/// Equips a fresh `slot_tester` item in `slot` with `flags` (creating the equip vault when it may
/// cut); answers the item.
fn new_item(w: &mut World, owner: &Keypair, mint: &Pubkey, slot: u8, flags: u16) -> Pubkey {
    let item = Pubkey::new_unique();
    w.init_item(owner, &item);
    if flags & f::TRANSFER_RETURNS_DELTA != 0 {
        w.make_equip_vault(owner, mint, slot);
    }
    w.equip(owner, mint, slot, &item, flags).ok();
    item
}

fn send(w: &mut World, ix: Instruction, signer: &Keypair) -> bordrless_program_tests::env::Tx {
    w.env.send_paid_by(&[ix], signer, &[])
}

#[test]
fn two_cutting_slots_each_cut_into_their_equip_vault() {
    let (mut w, owner, mint) = world(vec![
        locked_slot(hook_tester::ID, token_flags::BEFORE_TRANSFER, 0, 1),
        item_slot(kind::FEE, rule::VOTE, 500, 0, false),
        item_slot(kind::REWARD, rule::VOTE, 300, 0, false),
    ]);
    w.env
        .send_paid_by(&[tester::init_script(owner.pubkey(), mint, vec![])], &owner, &[])
        .ok();
    let flags = f::BEFORE_TRANSFER | f::AFTER_TRANSFER | f::TRANSFER_RETURNS_DELTA;
    let a = new_item(&mut w, &owner, &mint, 1, flags);
    let b = new_item(&mut w, &owner, &mint, 2, flags);
    supply(&mut w, &owner, &mint);
    let bob = Pubkey::new_unique();
    w.holdings(&owner, mint, &[bob]);
    w.item_answer(&owner, &a, cb::BEFORE_TRANSFER, mode::RETURN, cut(50));
    w.item_answer(&owner, &b, cb::BEFORE_TRANSFER, mode::RETURN, cut(30));

    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob, 1_000);
    let tx = send(&mut w, ix, &owner);
    tx.ok();
    let ev: Transferred = tx.event();
    assert_eq!((ev.amount, ev.destination_post), (1_000, 920));
    assert_eq!(
        ev.slot_cuts,
        vec![
            SlotCut { slot: 1, item: a, cut: 50 },
            SlotCut { slot: 2, item: b, cut: 30 },
        ]
    );
    assert_eq!(w.env.holding(&mint, &equip_vault(&mint, 1).0), 50);
    assert_eq!(w.env.holding(&mint, &equip_vault(&mint, 2).0), 30);
    // Both saw the same pre-state; after, each its own cut and the total.
    let told = w.item_script(&b).told(cb::BEFORE_TRANSFER).unwrap();
    assert_eq!((told.amount, told.slot, told.item), (1_000, 2, b));
    let after = w.item_script(&a).told(cb::AFTER_TRANSFER).unwrap();
    assert_eq!((after.delta, after.total_delta), (50, 80));
    // The Locked legacy hook ran too.
    let locked: hook_tester::Script = w.env.read(&tester::script_address(&mint));
    assert!(locked.calls >= 1);
}

#[test]
fn a_slot_answer_is_held_to_its_rules() {
    let (mut w, owner, mint) = world(vec![
        item_slot(kind::FEE, rule::VOTE, 500, 0, false),
        item_slot(kind::FEE, rule::VOTE, 500, 0, false),
    ]);
    let flags = f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA;
    let a = new_item(&mut w, &owner, &mint, 0, flags);
    supply(&mut w, &owner, &mint);
    let bob = Pubkey::new_unique();
    w.holdings(&owner, mint, &[bob]);
    let mut attempt = |w: &mut World, answer: Vec<u8>| {
        w.item_answer(&owner, &a, cb::BEFORE_TRANSFER, mode::RETURN, answer);
        let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob, 1_000);
        w.env.send_paid_by(&[ix], &owner, &[])
    };
    // A cut to anything but the slot's equip vault (index 5 is the script).
    let wrong = answer_bytes(&SlotReturn {
        deltas: vec![Delta { amount: 10, account: 5 }],
        ..SlotReturn::default()
    });
    attempt(&mut w, wrong).expect_code(code(TokenError::WrongEquipVault));
    // Above the slot's bound: 500 bps of 1,000 is 50.
    attempt(&mut w, cut(51)).expect_code(code(TokenError::SlotCutExceeded));
    attempt(&mut w, cut(50)).ok();
    // Two deltas from one item (R1).
    let two = answer_bytes(&SlotReturn {
        deltas: vec![
            Delta { amount: 1, account: VAULT },
            Delta { amount: 1, account: 5 },
        ],
        ..SlotReturn::default()
    });
    attempt(&mut w, two).expect_code(code(TokenError::TooManyDeltas));
    // Data without WRITES_HOOK_DATA.
    let data = answer_bytes(&SlotReturn {
        source_data: Some(vec![]),
        ..SlotReturn::default()
    });
    attempt(&mut w, data).expect_code(code(TokenError::UnsupportedHookReturn));
    // A zero cut.
    attempt(&mut w, cut(0)).expect_code(code(TokenError::ZeroDelta));
}

#[test]
fn ranges_are_separate_stamped_with_the_epoch_and_go_stale_on_equip() {
    let (mut w, owner, mint) = world(vec![
        item_slot(kind::REWARD, rule::VOTE, 0, 9, false),
        item_slot(kind::DEFENSE, rule::VOTE, 0, 5, false),
    ]);
    let flags = f::BEFORE_TRANSFER | f::WRITES_HOOK_DATA;
    let a = new_item(&mut w, &owner, &mint, 0, flags);
    let d = new_item(&mut w, &owner, &mint, 1, flags);
    supply(&mut w, &owner, &mint);
    let carol = Keypair::new();
    w.env.fund(carol.pubkey(), SOL);
    w.holdings(&owner, mint, &[carol.pubkey()]);
    w.item_answer(
        &owner,
        &a,
        cb::BEFORE_TRANSFER,
        mode::RETURN,
        answer_bytes(&SlotReturn {
            destination_data: Some(vec![1, 2, 3, 4, 5, 6, 7, 8]),
            ..SlotReturn::default()
        }),
    );
    w.item_answer(
        &owner,
        &d,
        cb::BEFORE_TRANSFER,
        mode::RETURN,
        answer_bytes(&SlotReturn {
            destination_data: Some(vec![9, 9, 9, 9]),
            ..SlotReturn::default()
        }),
    );
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &carol.pubkey(), 100);
    send(&mut w, ix, &owner).ok();
    let data = w.env.hook_data(&mint, &carol.pubkey());
    let m: Mint = w.env.read(&mint);
    let (e0, e1) = (m.slots[0].data_epoch, m.slots[1].data_epoch);
    assert_eq!(&data[0..9], &[e0, 1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(&data[9..14], &[e1, 9, 9, 9, 9]);
    assert!(data[14..].iter().all(|b| *b == 0));
    assert_eq!((m.slots[0].data_offset, m.slots[1].data_offset), (0, 9));

    // A data answer of the wrong length.
    w.item_answer(
        &owner,
        &d,
        cb::BEFORE_TRANSFER,
        mode::RETURN,
        answer_bytes(&SlotReturn {
            destination_data: Some(vec![9; 5]),
            ..SlotReturn::default()
        }),
    );
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &carol.pubkey(), 1);
    send(&mut w, ix, &owner).expect_code(code(TokenError::SlotDataLength));
    w.item_answer(&owner, &d, cb::BEFORE_TRANSFER, mode::NONE, vec![]);
    w.item_answer(&owner, &a, cb::BEFORE_TRANSFER, mode::NONE, vec![]);

    // Carol sends everything back; live data blocks her close.
    let ix = w.slot_transfer_ix(&carol.pubkey(), &mint, &owner.pubkey(), 100);
    send(&mut w, ix, &carol).ok();
    let close = token::close_holding(
        carol.pubkey(),
        mint,
        token::holding_address(&mint, &carol.pubkey()),
        carol.pubkey(),
    );
    w.env
        .send_paid_by(&[close.clone()], &carol, &[])
        .expect_code(code(TokenError::HookDataNotEmpty));

    // A new item in each slot: every previous range reads as zeros and no longer blocks.
    let a2 = new_item(&mut w, &owner, &mint, 0, flags);
    let d2 = new_item(&mut w, &owner, &mint, 1, flags);
    let m: Mint = w.env.read(&mint);
    assert_eq!((m.slots[0].data_epoch, m.slots[1].data_epoch), (e0 + 1, e1 + 1));
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &Pubkey::new_unique(), 0);
    let _ = ix; // a zero transfer is refused; the stale read is shown with a real one below.
    let dave = Pubkey::new_unique();
    w.holdings(&owner, mint, &[dave]);
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &dave, 1);
    send(&mut w, ix, &owner).ok();
    let told = w.item_script(&a2).told(cb::BEFORE_TRANSFER).unwrap();
    assert_eq!(told.source_data, vec![0; 8]);
    let _ = d2;
    // Carol's raw bytes are still there, but stale: the close goes through.
    assert_eq!(w.env.hook_data(&mint, &carol.pubkey())[0], e0);
    w.env.send_paid_by(&[close], &carol, &[]).ok();
}

#[test]
fn the_locked_slot_writes_only_inside_its_range() {
    let (mut w, owner, mint) = world(vec![
        locked_slot(
            hook_tester::ID,
            token_flags::BEFORE_TRANSFER | token_flags::WRITES_HOOK_DATA,
            32,
            1,
        ),
        item_slot(kind::REWARD, rule::VOTE, 0, 9, false),
    ]);
    w.env
        .send_paid_by(&[tester::init_script(owner.pubkey(), mint, vec![])], &owner, &[])
        .ok();
    let a = new_item(&mut w, &owner, &mint, 1, f::BEFORE_TRANSFER | f::WRITES_HOOK_DATA);
    supply(&mut w, &owner, &mint);
    let bob = Pubkey::new_unique();
    w.holdings(&owner, mint, &[bob]);
    let m: Mint = w.env.read(&mint);
    assert_eq!((m.slots[0].data_offset, m.slots[1].data_offset), (0, 32));
    // The legacy hook answers all 64 bytes; only 0..32 are written.
    w.env
        .send_paid_by(
            &[tester::answer(
                owner.pubkey(),
                mint,
                hook_tester::callback::BEFORE_TRANSFER,
                &HookReturn {
                    destination_hook_data: Some([0xab; 64]),
                    ..HookReturn::default()
                },
            )],
            &owner,
            &[],
        )
        .ok();
    w.item_answer(
        &owner,
        &a,
        cb::BEFORE_TRANSFER,
        mode::RETURN,
        answer_bytes(&SlotReturn {
            destination_data: Some(vec![7; 8]),
            ..SlotReturn::default()
        }),
    );
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob, 10);
    send(&mut w, ix, &owner).ok();
    let data = w.env.hook_data(&mint, &bob);
    let e = w.env.read::<Mint>(&mint).slots[1].data_epoch;
    assert!(data[..32].iter().all(|b| *b == 0xab));
    assert_eq!(&data[32..41], &[e, 7, 7, 7, 7, 7, 7, 7, 7]);
    assert!(data[41..].iter().all(|b| *b == 0));
    // write_hook_data from the Locked program: only 0..32.
    let (signer, bump) = tester::hook_authority();
    let ix = tester::write_hook_data_as(
        signer,
        vec![bordrless_hook::HOOK_AUTHORITY_SEED.to_vec(), vec![bump]],
        mint,
        token::holding_address(&mint, &bob),
        [0xcd; 64],
    );
    send(&mut w, ix, &owner).ok();
    let data = w.env.hook_data(&mint, &bob);
    assert!(data[..32].iter().all(|b| *b == 0xcd));
    assert_eq!(&data[32..41], &[e, 7, 7, 7, 7, 7, 7, 7, 7]);
}

#[test]
fn mint_to_calls_only_the_locked_slot_pool_halves_stamp_and_war_never_runs() {
    let (mut w, owner, mint) = world(vec![
        locked_slot(hook_tester::ID, token_flags::BEFORE_MINT, 0, 1),
        item_slot(kind::POOL, rule::VOTE, 0, 4, false),
        item_slot(kind::WAR, rule::VOTE, 0, 0, false),
    ]);
    w.env
        .send_paid_by(&[tester::init_script(owner.pubkey(), mint, vec![])], &owner, &[])
        .ok();
    let pool_item = new_item(&mut w, &owner, &mint, 1, f::BEFORE_TRANSFER | f::WRITES_HOOK_DATA);
    let war_item = new_item(&mut w, &owner, &mint, 2, 0);
    supply(&mut w, &owner, &mint);
    let locked: hook_tester::Script = w.env.read(&tester::script_address(&mint));
    assert_eq!(locked.calls, 1);
    assert_eq!(w.item_script(&pool_item).calls, 0);

    w.item_answer(
        &owner,
        &pool_item,
        cb::BEFORE_TRANSFER,
        mode::RETURN,
        answer_bytes(&SlotReturn {
            destination_data: Some(vec![4, 3, 2]),
            ..SlotReturn::default()
        }),
    );
    let bob = Pubkey::new_unique();
    w.holdings(&owner, mint, &[bob]);
    // The War slot brings no accounts and is never called; the Locked slot has no transfer flags.
    assert_eq!(w.slot_extras(&mint, SlotOp::Transfer).len(), 3);
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob, 5);
    send(&mut w, ix, &owner).ok();
    let e = w.env.read::<Mint>(&mint).slots[1].data_epoch;
    assert_eq!(&w.env.hook_data(&mint, &bob)[0..4], &[e, 4, 3, 2]);
    assert_eq!(w.item_script(&war_item).calls, 0);
    // A War slot accepts no token flags.
    let ix = token::set_slot_item(
        authority_of(&mint),
        mint,
        slot_tester::ID,
        None,
        2,
        war_item,
        f::BEFORE_TRANSFER,
        0,
        1,
    );
    send(&mut w, as_armory(&mint, ix), &owner).expect_code(code(TokenError::SlotFlagsNotAllowed));
    // Item slots never take mint callbacks.
    let ix = token::set_slot_item(
        authority_of(&mint),
        mint,
        slot_tester::ID,
        None,
        1,
        pool_item,
        f::BEFORE_MINT,
        0,
        1,
    );
    send(&mut w, as_armory(&mint, ix), &owner).expect_code(code(TokenError::SlotFlagsNotAllowed));
}

#[test]
fn only_the_armory_changes_slots_and_vote_locks() {
    let (mut w, owner, mint) = world(vec![item_slot(kind::FEE, rule::VOTE, 100, 0, false)]);
    let item = Pubkey::new_unique();
    w.init_item(&owner, &item);
    let ix = token::set_slot_item(owner.pubkey(), mint, slot_tester::ID, None, 0, item, f::BEFORE_TRANSFER, 0, 1);
    send(&mut w, ix, &owner).expect_code(code(TokenError::InvalidSlotAuthority));
    let ix = token::set_vote_lock(
        owner.pubkey(),
        mint,
        token::holding_address(&mint, &owner.pubkey()),
        1,
        i64::MAX,
    );
    send(&mut w, ix, &owner).expect_code(code(TokenError::InvalidSlotAuthority));
    // The armory may not put a protocol program in a slot.
    let ix = token::set_slot_item(authority_of(&mint), mint, bordrless_token::constants::SWAP_ID, None, 0, item, f::BEFORE_TRANSFER, 0, 1);
    send(&mut w, as_armory(&mint, ix), &owner).expect_fail();
    // A cutting item needs the slot's existing equip vault.
    let ix = token::set_slot_item(
        authority_of(&mint),
        mint,
        slot_tester::ID,
        Some(Pubkey::new_unique()),
        0,
        item,
        f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA,
        0,
        2,
    );
    send(&mut w, as_armory(&mint, ix), &owner).expect_code(code(TokenError::WrongEquipVault));
}

#[test]
fn a_vote_lock_holds_tokens_in_place_until_it_ends() {
    let (mut w, owner, mint) = world(vec![item_slot(kind::FEE, rule::VOTE, 100, 0, false)]);
    supply(&mut w, &owner, &mint);
    let bob = Keypair::new();
    w.env.fund(bob.pubkey(), SOL);
    w.holdings(&owner, mint, &[bob.pubkey()]);
    let mine = token::holding_address(&mint, &owner.pubkey());
    let until = w.env.now + 100;
    let lock = |amount| as_armory(&mint, token::set_vote_lock(authority_of(&mint), mint, mine, amount, until));
    send(&mut w, lock(SUPPLY + 1), &owner).expect_code(code(TokenError::VoteLockExceedsBalance));
    send(&mut w, lock(SUPPLY - 100), &owner).ok();
    let h: Holding = w.env.read(&mine);
    assert_eq!((h.vote_locked, h.vote_lock_until), (SUPPLY - 100, until));
    // 100 are free.
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob.pubkey(), 101);
    send(&mut w, ix, &owner).expect_code(code(TokenError::VoteLocked));
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob.pubkey(), 60);
    send(&mut w, ix, &owner).ok();
    let ix = w.slot_burn_ix(&owner.pubkey(), &mint, 41);
    send(&mut w, ix, &owner).expect_code(code(TokenError::VoteLocked));
    // A delegate is bound the same way.
    send(&mut w, token::approve(owner.pubkey(), mine, bob.pubkey(), 1_000), &owner).ok();
    let ix = w.slot_transfer_ix(&bob.pubkey(), &mint, &bob.pubkey(), 1);
    let _ = ix;
    let ix = token::transfer_with(
        bob.pubkey(),
        mine,
        token::holding_address(&mint, &bob.pubkey()),
        mint,
        None,
        w.slot_extras(&mint, SlotOp::Transfer),
        41,
    );
    send(&mut w, ix, &bob).expect_code(code(TokenError::VoteLocked));
    // After `until`, everything moves.
    w.env.warp(101);
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &bob.pubkey(), SUPPLY - 60);
    send(&mut w, ix, &owner).ok();
}

#[test]
fn touch_lets_one_slot_rewrite_one_holding_for_the_caller_it_trusts() {
    let (mut w, owner, mint) = world(vec![
        item_slot(kind::REWARD, rule::VOTE, 0, 5, true),
        item_slot(kind::DEFENSE, rule::VOTE, 0, 3, false),
    ]);
    let item = new_item(&mut w, &owner, &mint, 0, f::WRITES_HOOK_DATA | f::ANSWERS_TOUCH);
    let other = new_item(&mut w, &owner, &mint, 1, f::WRITES_HOOK_DATA);
    supply(&mut w, &owner, &mint);
    let war = Keypair::new();
    w.env.fund(war.pubkey(), SOL);
    let mut data = war.pubkey().to_bytes().to_vec();
    data.extend(answer_bytes(&SlotReturn {
        source_data: Some(vec![9, 8, 7, 6]),
        ..SlotReturn::default()
    }));
    w.item_answer(&owner, &item, cb::ON_TOUCH, mode::RETURN_IF_CALLER, data);
    let holding = token::holding_address(&mint, &owner.pubkey());
    let touch = |caller: &Pubkey, slot: u8, item: &Pubkey, payload: Vec<u8>| {
        token::touch(
            *caller,
            mint,
            holding,
            slot_tester::ID,
            slot,
            payload,
            vec![AccountMeta::new(slot_tester::script_address(item), false)],
        )
    };
    // Anyone may call; the item ignores callers it does not trust.
    let stranger = w.env.funded(SOL);
    let tx = send(&mut w, touch(&stranger.pubkey(), 0, &item, vec![1]), &stranger);
    tx.ok();
    assert!(tx.events::<HookDataWritten>().is_empty());
    assert_eq!(w.env.hook_data(&mint, &owner.pubkey())[0..5], [0; 5]);
    let tx = send(&mut w, touch(&war.pubkey(), 0, &item, vec![1]), &war);
    tx.ok();
    let ev: HookDataWritten = tx.event();
    let e = w.env.read::<Mint>(&mint).slots[0].data_epoch;
    assert_eq!(&ev.data[0..5], &[e, 9, 8, 7, 6]);
    let told = w.item_script(&item).told(cb::ON_TOUCH).unwrap();
    assert_eq!((told.authority, told.payload.clone()), (war.pubkey(), vec![1]));
    // A slot without ANSWERS_TOUCH, and a payload too long.
    send(&mut w, touch(&war.pubkey(), 1, &other, vec![]), &war)
        .expect_code(code(TokenError::TouchNotSupported));
    send(&mut w, touch(&war.pubkey(), 0, &item, vec![0; 257]), &war)
        .expect_code(code(TokenError::PayloadTooLong));
}

#[test]
fn a_forged_callback_is_refused_by_the_item() {
    let (mut w, owner, mint) = world(vec![item_slot(kind::FEE, rule::VOTE, 100, 0, false)]);
    let item = new_item(&mut w, &owner, &mint, 0, f::BEFORE_TRANSFER);
    let forger = w.env.funded(SOL);
    let args = bordrless_hook::TokenSlotArgs {
        op: bordrless_hook::TokenSlotOp::Transfer,
        phase: bordrless_hook::Phase::Before,
        slot: 0,
        item,
        mint,
        source: Pubkey::new_unique(),
        destination: Pubkey::new_unique(),
        source_owner: Pubkey::default(),
        destination_owner: Pubkey::default(),
        authority: forger.pubkey(),
        authority_is_delegate: false,
        amount: 1,
        delta: 0,
        total_delta: 0,
        source_balance: 0,
        destination_balance: 0,
        decimals: 6,
        supply: 0,
        source_data: vec![],
        destination_data: vec![],
        payload: vec![],
    };
    let ix = Instruction {
        program_id: slot_tester::ID,
        accounts: vec![
            AccountMeta::new_readonly(forger.pubkey(), true),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(args.source, false),
            AccountMeta::new_readonly(args.destination, false),
            AccountMeta::new_readonly(forger.pubkey(), false),
            AccountMeta::new(slot_tester::script_address(&item), false),
        ],
        data: slot_tester::instruction::BeforeTransfer { args }.data(),
    };
    let tx = send(&mut w, ix, &forger);
    tx.expect_fail();
    assert!(tx.logs().join("\n").contains("BadHookSigner"));
}

#[test]
fn protocol_payouts_skip_item_slots() {
    let (mut w, owner, mint) = world(vec![
        locked_slot(hook_tester::ID, token_flags::BEFORE_TRANSFER, 0, 1),
        item_slot(kind::FEE, rule::VOTE, 500, 0, false),
    ]);
    w.env
        .send_paid_by(&[tester::init_script(owner.pubkey(), mint, vec![])], &owner, &[])
        .ok();
    let item = new_item(&mut w, &owner, &mint, 1, f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA);
    supply(&mut w, &owner, &mint);
    w.item_answer(&owner, &item, cb::BEFORE_TRANSFER, mode::RETURN, cut(25));
    // A protocol vault: a holding owned by a PDA of an allowed program (here the armory's
    // ["slots", mint], which armory_stub can sign for).
    let (pda, bump) = bordrless_hook::slot_authority(&ARMORY_ID, &mint);
    w.holdings(&owner, mint, &[pda]);
    let ix = w.slot_transfer_ix(&owner.pubkey(), &mint, &pda, 1_000);
    send(&mut w, ix, &owner).ok();
    assert_eq!(w.env.holding(&mint, &pda), 975);
    let calls = w.item_script(&item).calls;
    let payout = |seeds: Vec<Vec<u8>>, program: Pubkey, w: &World| {
        as_armory(
            &mint,
            token::transfer_from_protocol(
                pda,
                token::holding_address(&mint, &pda),
                token::holding_address(&mint, &owner.pubkey()),
                mint,
                w.slot_extras(&mint, SlotOp::ProtocolTransfer),
                500,
                program,
                seeds,
            ),
        )
    };
    let good = vec![b"slots".to_vec(), mint.to_bytes().to_vec(), vec![bump]];
    let bad = vec![b"slots".to_vec(), Pubkey::new_unique().to_bytes().to_vec(), vec![bump]];
    let ix = payout(bad, ARMORY_ID, &w);
    send(&mut w, ix, &owner).expect_code(code(TokenError::NotProtocolSource));
    let ix = payout(good.clone(), hook_tester::ID, &w);
    send(&mut w, ix, &owner).expect_code(code(TokenError::NotProtocolSource));
    let ix = payout(good, ARMORY_ID, &w);
    let tx = send(&mut w, ix, &owner);
    tx.ok();
    let ev: Transferred = tx.event();
    assert_eq!((ev.amount, ev.slot_cuts.len()), (500, 0));
    assert_eq!(w.item_script(&item).calls, calls);
    assert_eq!(w.env.holding(&mint, &pda), 475);
}

/// `create_slot_mint` with `slots`, expected to fail with `err`.
fn refused(slots: Vec<SlotInit>, with_authority: bool, err: TokenError) {
    let mut w = World::with_slots();
    let owner = w.env.funded(10 * SOL);
    let mint = Keypair::new();
    w.create_slot_mint(&owner, &mint, slots, with_authority)
        .expect_code(code(err));
}

#[test]
fn the_rules_of_a_slot_table() {
    let fee = |cut| item_slot(kind::FEE, rule::VOTE, cut, 0, false);
    use TokenError::*;
    refused(vec![], true, InvalidSlotTable);
    refused(vec![fee(1), fee(1), fee(1), fee(0), fee(0)], true, InvalidSlotTable);
    refused(
        vec![
            locked_slot(hook_tester::ID, 0, 0, 0),
            locked_slot(hook_tester::ID, 0, 0, 0),
        ],
        false,
        InvalidSlotTable,
    );
    refused(vec![item_slot(kind::WAR, rule::VOTE, 0, 4, false)], true, InvalidSlotTable);
    refused(vec![item_slot(kind::REWARD, rule::VOTE, 0, 1, false)], true, InvalidSlotTable);
    refused(vec![item_slot(kind::DEFENSE, rule::VOTE, 10, 0, false)], true, InvalidSlotTable);
    refused(vec![item_slot(kind::POOL, rule::VOTE, 10, 0, false)], true, InvalidSlotTable);
    refused(vec![fee(6_000), fee(6_000)], true, InvalidSlotTable);
    refused(
        vec![
            item_slot(kind::REWARD, rule::VOTE, 0, 30, false),
            item_slot(kind::REWARD, rule::VOTE, 0, 30, false),
            item_slot(kind::REWARD, rule::VOTE, 0, 30, false),
        ],
        true,
        InvalidSlotTable,
    );
    refused(vec![item_slot(kind::FEE, rule::VOTE, 0, 0, true)], true, InvalidSlotTable);
    refused(vec![item_slot(9, rule::VOTE, 0, 0, false)], true, InvalidSlotTable);
    refused(vec![fee(1), fee(1), fee(1), fee(1)], true, TooManyCuttingSlots);
    let mut cutting_locked = locked_slot(
        hook_tester::ID,
        token_flags::BEFORE_TRANSFER | token_flags::TRANSFER_RETURNS_DELTA,
        0,
        0,
    );
    cutting_locked.bounds.max_cut_bps = 10;
    refused(vec![cutting_locked, fee(1), fee(1), fee(1)], true, TooManyCuttingSlots);
    refused(vec![fee(1)], false, InvalidSlotAuthority);

    // An authority that is not the armory's PDA, and a single hook beside slots.
    let mut w = World::with_slots();
    let owner = w.env.funded(10 * SOL);
    let mint = Keypair::new();
    let args = CreateMintArgs {
        decimals: 6,
        name: "X".into(),
        symbol: "X".into(),
        uri: String::new(),
        max_supply: 0,
        mint_authority: Some(owner.pubkey()),
        freeze_authority: None,
        hook_program: None,
        hook_flags: 0,
        hook_authority: None,
        metadata_authority: None,
    };
    let ix = token::create_slot_mint(owner.pubkey(), mint.pubkey(), args.clone(), Some(owner.pubkey()), vec![fee(1)]);
    w.env
        .send_paid_by(&[ix], &owner, &[&mint])
        .expect_code(code(InvalidSlotAuthority));
    let hooked = CreateMintArgs {
        hook_program: Some(hook_tester::ID),
        hook_flags: token_flags::BEFORE_TRANSFER,
        ..args
    };
    let ix = token::create_slot_mint(
        owner.pubkey(),
        mint.pubkey(),
        hooked,
        Some(authority_of(&mint.pubkey())),
        vec![fee(1)],
    );
    w.env
        .send_paid_by(&[ix], &owner, &[&mint])
        .expect_code(code(MixedHookModes));

    // A valid table, and what it records.
    let (w, _, mint) = world(vec![
        locked_slot(hook_tester::ID, token_flags::BEFORE_TRANSFER, 0, 1),
        fee(500),
        item_slot(kind::REWARD, rule::PERFORMANCE, 0, 9, true),
        item_slot(kind::WAR, rule::LOCKED, 0, 0, false),
    ]);
    let m: Mint = w.env.read(&mint);
    assert_eq!(m.slot_count, 4);
    assert_eq!(m.slot_authority, Some(authority_of(&mint)));
    assert_eq!(m.slots[1].equip_vault, equip_vault(&mint, 1).1);
    assert_eq!(m.slots[2].data_offset, 0);
    assert!(m.slots.iter().all(|s| s.bounds != SlotBounds { max_cut_bps: 1, ..SlotBounds::default() }));
    let _ = (m.hook_program, Mint::LEN);
}
