// Changed by Hookwars: new file, rulings R20 to R24 (docs/spec/00-overview.md section 9).
//! Protocol vaults on kit tokens (R20): item cuts into equip vaults on a token with holder rewards,
//! royalty settlement out of an equip vault, payouts out of a royalty holding, normal transfers
//! into a war chest, the eligible supply and the reward vault's solvency through all of it; the
//! `may_burn` bound (R21); holder `touch` (R23); protocol payouts with items and armory seeds (R24).

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use bordrless_hook::{
    equip_rule as rule, slot_flags as f, slot_kind as kind, Delta, SlotReturn,
};
use bordrless_kit::constants::modules;
use bordrless_kit::error::KitError;
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::kit::{check_kit, refused_by_kit, DirectKit, KitSpec, Rng};
use bordrless_program_tests::program_bytes;
use bordrless_program_tests::slots::*;
use bordrless_token::client as token;
use bordrless_token::constants::{
    ARMORY_ID, ITEMS_ID, PROTOCOL_TRANSFER_MARKER, PROTOCOL_TRANSFER_SEED, WAR_ID,
};
use bordrless_token::error::TokenError;
use bordrless_token::instructions::SlotInit;
use bordrless_token::slots::SlotOp;
use bordrless_token::state::{Mint, SlotBounds};
use slot_tester::{callback as cb, mode};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;
/// The item slot of `KitSpec { in_slot: true }`: right after the kit's Locked slot.
const ITEM_SLOT: u8 = 1;
/// Index of a `slot_tester` item's equip vault in its callback (prefix 5, script 5, vault 6).
const VAULT: u8 = 6;
/// The cut the scripted item takes from every transfer it sees (below its slot's 1% bound for
/// every amount these tests move).
const CUT: u64 = 1_000_000_000;

fn cut_answer(amount: u64) -> Vec<u8> {
    answer_bytes(&SlotReturn {
        deltas: vec![Delta {
            amount,
            account: VAULT,
        }],
        ..SlotReturn::default()
    })
}

/// `ix` forwarded by `items_stub` (loaded at the items id), signing as `PDA(seeds)` there.
fn as_items(seeds: &[Vec<u8>], ix: Instruction) -> Instruction {
    let refs: Vec<&[u8]> = seeds.iter().map(Vec::as_slice).collect();
    let pda = Pubkey::find_program_address(&refs, &ITEMS_ID).0;
    let mut accounts = vec![AccountMeta::new_readonly(bordrless_token::ID, false)];
    accounts.extend(ix.accounts.into_iter().map(|mut m| {
        if m.pubkey == pda {
            m.is_signer = false;
        }
        m
    }));
    Instruction {
        program_id: ITEMS_ID,
        accounts,
        data: items_stub::instruction::AsPda {
            seeds: seeds.to_vec(),
            data: ix.data,
        }
        .data(),
    }
}

/// `ix` forwarded by `armory_stub`, signing as `PDA(seeds)` under the armory's id.
fn as_armory_pda(seeds: &[Vec<u8>], ix: Instruction) -> Instruction {
    let refs: Vec<&[u8]> = seeds.iter().map(Vec::as_slice).collect();
    let pda = Pubkey::find_program_address(&refs, &ARMORY_ID).0;
    let mut accounts = vec![AccountMeta::new_readonly(bordrless_token::ID, false)];
    accounts.extend(ix.accounts.into_iter().map(|mut m| {
        if m.pubkey == pda {
            m.is_signer = false;
        }
        m
    }));
    Instruction {
        program_id: ARMORY_ID,
        accounts,
        data: armory_stub::instruction::AsPda {
            seeds: seeds.to_vec(),
            data: ix.data,
        }
        .data(),
    }
}

/// Seeds with their bump appended, as `transfer_from_protocol` takes them.
fn with_bump(seeds: &[Vec<u8>], program: &Pubkey) -> (Pubkey, Vec<Vec<u8>>) {
    let refs: Vec<&[u8]> = seeds.iter().map(Vec::as_slice).collect();
    let (pda, bump) = Pubkey::find_program_address(&refs, program);
    let mut v = seeds.to_vec();
    v.push(vec![bump]);
    (pda, v)
}

fn equip_seeds(mint: &Pubkey, slot: u8) -> Vec<Vec<u8>> {
    vec![b"equip".to_vec(), mint.to_bytes().to_vec(), vec![slot]]
}

fn royalty_seeds(item: &Pubkey) -> Vec<Vec<u8>> {
    vec![b"royalty".to_vec(), item.to_bytes().to_vec()]
}

/// A kit token with `modules` whose kit sits in the Locked slot of a slot mint, with a scripted
/// cutting item in the item slot (cutting `CUT` from every transfer it sees), and `items_stub`
/// loaded at the items id. Answers the world, the kit and the item.
fn kit_with_item(modules_on: u8) -> (World, DirectKit, Pubkey) {
    let mut w = World::with_slots();
    w.env
        .svm
        .add_program(ITEMS_ID, &program_bytes("items_stub"))
        .expect("load items_stub");
    let d = w.direct_kit(&KitSpec {
        in_slot: true,
        ..KitSpec::new(modules_on)
    });
    let payer = w.env.funded(100 * SOL);
    let mint = d.token.mint;
    let item = Pubkey::new_unique();
    w.init_item(&payer, &item);
    w.make_equip_vault(&payer, &mint, ITEM_SLOT);
    w.equip(
        &payer,
        &mint,
        ITEM_SLOT,
        &item,
        f::BEFORE_TRANSFER | f::TRANSFER_RETURNS_DELTA,
    )
    .ok();
    w.item_answer(&payer, &item, cb::BEFORE_TRANSFER, mode::RETURN, cut_answer(CUT));
    (w, d, item)
}

/// The kit's eligible supply must equal the wallets' balances.
fn assert_eligible(w: &World, d: &DirectKit, wallets: &[Pubkey]) {
    let c = w.env.kit_config(&d.token.mint);
    let sum: u64 = wallets
        .iter()
        .map(|o| w.env.holding_state(&d.token.mint, o).0)
        .sum();
    assert_eq!(c.eligible, sum, "eligible is exactly the wallets' balances");
}

/// A `transfer_from_protocol` of `amount` from `source_owner`'s holding to `destination_owner`'s,
/// with the Locked slot's slice only (R16).
fn protocol_transfer(
    w: &World,
    mint: &Pubkey,
    source_owner: &Pubkey,
    destination_owner: &Pubkey,
    amount: u64,
    program: Pubkey,
    seeds: Vec<Vec<u8>>,
) -> Instruction {
    let source = token::holding_address(mint, source_owner);
    let destination = token::holding_address(mint, destination_owner);
    let extras = w.env.slot_mint_extras(
        mint,
        SlotOp::ProtocolTransfer,
        &source,
        &destination,
        source_owner,
        source_owner,
        destination_owner,
    );
    token::transfer_from_protocol(
        *source_owner,
        source,
        destination,
        *mint,
        extras,
        amount,
        program,
        seeds,
    )
}

#[test]
fn the_protocol_transfer_marker_is_its_derivation() {
    let (pda, _) =
        Pubkey::find_program_address(&[PROTOCOL_TRANSFER_SEED], &bordrless_token::ID);
    assert_eq!(pda, PROTOCOL_TRANSFER_MARKER);
    assert!(!PROTOCOL_TRANSFER_MARKER.is_on_curve(), "a PDA: no key signs for it");
    assert_eq!(
        bordrless_kit::constants::PROTOCOL_TRANSFER_MARKER,
        PROTOCOL_TRANSFER_MARKER
    );
}

/// R20: on a token with holder rewards, an item's cut lands in its equip vault and the kit counts
/// only what each wallet really received.
#[test]
fn item_cuts_on_a_rewards_token_keep_eligible_exact() {
    let (mut w, d, _) = kit_with_item(modules::HOLDER_REWARDS | modules::MAX_WALLET);
    let mint = d.token.mint;
    let a = w.kit_holder(&d.token, SOL);
    let b = w.kit_holder(&d.token, SOL);
    let wallets = [a.pubkey(), b.pubkey()];
    let (_, vault) = equip_vault(&mint, ITEM_SLOT);

    // Pool to wallet: the wallet receives the amount less the cut.
    let ix = w.kit_transfer_ix(mint, d.pool.pubkey(), &d.pool.pubkey(), &a.pubkey(), 10_000_000_000_000);
    w.env.send_paid_by(&[ix], &d.pool, &[]).ok();
    assert_eq!(w.env.holding_state(&mint, &a.pubkey()).0, 10_000_000_000_000 - CUT);
    assert_eligible(&w, &d, &wallets);

    // Wallet to wallet: the cut leaves the holders.
    let ix = w.kit_transfer_ix(mint, a.pubkey(), &a.pubkey(), &b.pubkey(), 4_000_000_000_000);
    w.env.send_paid_by(&[ix], &a, &[]).ok();
    assert_eq!(w.env.holding_state(&mint, &b.pubkey()).0, 4_000_000_000_000 - CUT);
    assert_eligible(&w, &d, &wallets);
    assert_eq!(w.env.holding_state(&mint, &equip_vault(&mint, ITEM_SLOT).0).0, 2 * CUT);
    let _ = vault;

    // Wallet back to the pool.
    let ix = w.kit_transfer_ix(mint, b.pubkey(), &b.pubkey(), &d.pool.pubkey(), 1_000_000_000_000);
    w.env.send_paid_by(&[ix], &b, &[]).ok();
    assert_eligible(&w, &d, &wallets);
    check_kit(&w.env, &d.token, &wallets);
}

/// R20 (b), R24: settling out of an equip vault into a royalty holding (a program address) works on
/// a token with holder rewards, and the royalty owner's payout to a wallet is counted exactly.
#[test]
fn royalty_settlement_and_payout_on_a_rewards_token() {
    let (mut w, d, item) = kit_with_item(modules::HOLDER_REWARDS);
    let mint = d.token.mint;
    let a = w.kit_holder(&d.token, SOL);
    let c = w.kit_holder(&d.token, SOL);
    let wallets = [a.pubkey(), c.pubkey()];
    let ix = w.kit_transfer_ix(mint, d.pool.pubkey(), &d.pool.pubkey(), &a.pubkey(), 5_000_000_000_000);
    w.env.send_paid_by(&[ix], &d.pool, &[]).ok();

    let payer = w.env.funded(10 * SOL);
    let (royalty_owner, royalty_seeds_b) = with_bump(&royalty_seeds(&item), &ARMORY_ID);
    w.holdings(&payer, mint, &[royalty_owner]);
    let (equip_owner, equip_seeds_b) = with_bump(&equip_seeds(&mint, ITEM_SLOT), &ITEMS_ID);

    // Equip vault to the royalty holding: a protocol transfer to a program address.
    let ix = protocol_transfer(&w, &mint, &equip_owner, &royalty_owner, CUT, ITEMS_ID, equip_seeds_b);
    w.env
        .send_paid_by(&[as_items(&equip_seeds(&mint, ITEM_SLOT), ix)], &payer, &[])
        .ok();
    assert_eq!(w.env.holding_state(&mint, &royalty_owner).0, CUT);
    assert_eq!(w.env.holding_state(&mint, &royalty_owner).1, [0; 64], "never stamped");
    assert_eligible(&w, &d, &wallets);

    // The royalty owner pays a wallet: counted as received, no item cut (R16).
    let ix = protocol_transfer(&w, &mint, &royalty_owner, &c.pubkey(), CUT, ARMORY_ID, royalty_seeds_b);
    w.env
        .send_paid_by(&[as_armory_pda(&royalty_seeds(&item), ix)], &payer, &[])
        .ok();
    assert_eq!(w.env.holding_state(&mint, &c.pubkey()).0, CUT);
    assert_eligible(&w, &d, &wallets);
    check_kit(&w.env, &d.token, &wallets);
}

/// R20 (a): a war chest (derived from the mint) may receive by an ordinary transfer, is excluded
/// and never capped by max wallet; any other program address is still refused (no second pool).
#[test]
fn derived_vaults_are_excluded_other_program_addresses_are_refused() {
    let (mut w, d, _) = kit_with_item(modules::HOLDER_REWARDS | modules::MAX_WALLET);
    let mint = d.token.mint;
    let payer = w.env.funded(10 * SOL);
    let chest = Pubkey::find_program_address(&[b"war-chest", mint.as_ref()], &WAR_ID).0;
    let stranger = Pubkey::find_program_address(&[b"somewhere", mint.as_ref()], &WAR_ID).0;
    w.holdings(&payer, mint, &[chest, stranger]);
    let cap = w.env.kit_config(&mint).max_wallet_amount;

    // More than max wallet into the chest: allowed, not capped, not counted.
    let ix = w.kit_transfer_ix(mint, d.pool.pubkey(), &d.pool.pubkey(), &chest, cap * 2);
    w.env.send_paid_by(&[ix], &d.pool, &[]).ok();
    assert_eq!(w.env.holding_state(&mint, &chest).0, cap * 2 - CUT);
    assert_eq!(w.env.kit_config(&mint).eligible, 0);

    // Any other program address: refused as before.
    let ix = w.kit_transfer_ix(mint, d.pool.pubkey(), &d.pool.pubkey(), &stranger, 1_000_000_000_000);
    let tx = w.env.send_paid_by(&[ix], &d.pool, &[]);
    assert!(refused_by_kit(&tx, KitError::DestinationNotAllowed));
}

/// R20: a seeded walk of wallet trades with item cuts, shares, claims, settlements into a royalty
/// holding and royalty payouts; after every step eligible is exact and the reward vault covers
/// every claim.
#[test]
fn a_seeded_walk_with_protocol_vaults_stays_solvent() {
    let (mut w, d, item) = kit_with_item(modules::HOLDER_REWARDS);
    let mint = d.token.mint;
    let payer = w.env.funded(100 * SOL);
    let wallets: Vec<Keypair> = (0..4).map(|_| w.kit_holder(&d.token, 50 * SOL)).collect();
    let keys: Vec<Pubkey> = wallets.iter().map(|k| k.pubkey()).collect();
    for k in &wallets {
        let ix = w.kit_transfer_ix(mint, d.pool.pubkey(), &d.pool.pubkey(), &k.pubkey(), 10_000_000_000_000);
        w.env.send_paid_by(&[ix], &d.pool, &[]).ok();
    }
    let (royalty_owner, royalty_b) = with_bump(&royalty_seeds(&item), &ARMORY_ID);
    w.holdings(&payer, mint, &[royalty_owner]);
    let (equip_owner, equip_b) = with_bump(&equip_seeds(&mint, ITEM_SLOT), &ITEMS_ID);
    let mut rng = Rng::new(20);
    for step in 0..60 {
        match rng.below(5) {
            0 | 1 => {
                let i = rng.below(4) as usize;
                let j = (i + 1 + rng.below(3) as usize) % 4;
                let have = w.env.holding_state(&mint, &keys[i]).0;
                // The item's cut must stay within its slot's 1%: move at least 100 cuts.
                if have > CUT * 200 {
                    let amount = rng.range(CUT * 100, have / 2);
                    let ix = w.kit_transfer_ix(mint, keys[i], &keys[i], &keys[j], amount);
                    w.env.send_paid_by(&[ix], &wallets[i], &[]).ok();
                }
            }
            2 => {
                let i = rng.below(4) as usize;
                w.kit_share(&wallets[i], &d.token, rng.range(bordrless_kit::constants::MIN_SHARE_LAMPORTS, SOL)).ok();
            }
            3 => {
                let i = rng.below(4) as usize;
                if w.env.claimable(&d.token, &keys[i]) > 0 {
                    w.kit_claim(&wallets[i], &d.token).ok();
                }
            }
            _ => {
                let in_vault = w.env.holding_state(&mint, &equip_owner).0;
                if in_vault > 0 {
                    let ix = protocol_transfer(&w, &mint, &equip_owner, &royalty_owner, in_vault, ITEMS_ID, equip_b.clone());
                    w.env
                        .send_paid_by(&[as_items(&equip_seeds(&mint, ITEM_SLOT), ix)], &payer, &[])
                        .ok();
                }
                let held = w.env.holding_state(&mint, &royalty_owner).0;
                if held > 0 {
                    let to = keys[rng.below(4) as usize];
                    let ix = protocol_transfer(&w, &mint, &royalty_owner, &to, held, ARMORY_ID, royalty_b.clone());
                    w.env
                        .send_paid_by(&[as_armory_pda(&royalty_seeds(&item), ix)], &payer, &[])
                        .ok();
                }
            }
        }
        w.env.warp(37);
        assert_eligible(&w, &d, &keys);
        check_kit(&w.env, &d.token, &keys);
        let _ = step;
    }
}

/// R21: `may_burn` is a Pool slot's bound only.
#[test]
fn may_burn_is_a_pool_slot_bound_only() {
    let burn = |k: u8| SlotInit {
        bounds: SlotBounds {
            may_burn: true,
            ..item_slot(k, rule::VOTE, 0, 0, false).bounds
        },
        ..item_slot(k, rule::VOTE, 0, 0, false)
    };
    for k in [kind::FEE, kind::REWARD, kind::DEFENSE, kind::RELATION, kind::WAR] {
        let mut w = World::with_slots();
        let owner = w.env.funded(10 * SOL);
        let mint = Keypair::new();
        w.create_slot_mint(&owner, &mint, vec![burn(k)], true)
            .expect_code(u32::from(TokenError::InvalidSlotTable));
    }
    let mut w = World::with_slots();
    let owner = w.env.funded(10 * SOL);
    let mint = Keypair::new();
    w.create_slot_mint(&owner, &mint, vec![burn(kind::POOL)], true).ok();
    let m: Mint = w.env.read(&mint.pubkey());
    assert!(m.slots[0].bounds.may_burn);
}

/// R23: a holder may `touch` its own holding; the item is told the caller and the owner, and
/// decides. Here the item acts only for the owner.
#[test]
fn a_holder_touches_its_own_holding_and_the_item_decides() {
    let mut w = World::with_slots();
    let owner = w.env.funded(10 * SOL);
    let stranger = w.env.funded(10 * SOL);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    w.create_slot_mint(
        &owner,
        &mint_kp,
        vec![item_slot(kind::REWARD, rule::VOTE, 0, 5, true)],
        true,
    )
    .ok();
    let item = Pubkey::new_unique();
    w.init_item(&owner, &item);
    w.equip(&owner, &mint, 0, &item, f::ANSWERS_TOUCH | f::WRITES_HOOK_DATA)
        .ok();
    let stamp = answer_bytes(&SlotReturn {
        source_data: Some(vec![7, 7, 7, 7]),
        ..SlotReturn::default()
    });
    let mut scripted = owner.pubkey().to_bytes().to_vec();
    scripted.extend(stamp);
    w.item_answer(&owner, &item, cb::ON_TOUCH, mode::RETURN_IF_CALLER, scripted);
    let holding = token::holding_address(&mint, &owner.pubkey());
    let touch = |caller: &Pubkey| {
        token::touch(
            *caller,
            mint,
            holding,
            slot_tester::ID,
            0,
            vec![1],
            item_extras(&mint, 0, &item, f::ANSWERS_TOUCH | f::WRITES_HOOK_DATA),
        )
    };

    // A stranger: told, the item answers nothing, nothing is written.
    w.env.send_paid_by(&[touch(&stranger.pubkey())], &stranger, &[]).ok();
    let told = w.item_script(&item).told(cb::ON_TOUCH).expect("told");
    assert_eq!((told.authority, told.source_owner), (stranger.pubkey(), owner.pubkey()));
    assert_eq!(w.env.holding_state(&mint, &owner.pubkey()).1[..5], [0; 5]);

    // The owner: the item stamps the range.
    w.env.send_paid_by(&[touch(&owner.pubkey())], &owner, &[]).ok();
    let told = w.item_script(&item).told(cb::ON_TOUCH).expect("told");
    assert_eq!(told.authority, told.source_owner);
    let data = w.env.holding_state(&mint, &owner.pubkey()).1;
    assert_eq!(data[1..5], [7, 7, 7, 7]);
}

/// R24: protocol payouts work with items seeds, and an items PDA cannot pass another program's
/// seeds.
#[test]
fn protocol_payouts_with_items_seeds_and_no_borrowed_seeds() {
    let (mut w, d, _) = kit_with_item(modules::HOLDER_REWARDS);
    let mint = d.token.mint;
    let a = w.kit_holder(&d.token, SOL);
    let ix = w.kit_transfer_ix(mint, d.pool.pubkey(), &d.pool.pubkey(), &a.pubkey(), 3_000_000_000_000);
    w.env.send_paid_by(&[ix], &d.pool, &[]).ok();
    let payer = w.env.funded(10 * SOL);
    let (equip_owner, equip_b) = with_bump(&equip_seeds(&mint, ITEM_SLOT), &ITEMS_ID);

    // The equip vault pays a wallet straight away (a referral or loyalty payout, R24).
    let ix = protocol_transfer(&w, &mint, &equip_owner, &a.pubkey(), CUT / 2, ITEMS_ID, equip_b.clone());
    w.env
        .send_paid_by(&[as_items(&equip_seeds(&mint, ITEM_SLOT), ix)], &payer, &[])
        .ok();
    assert_eligible(&w, &d, &[a.pubkey()]);

    // The same seeds named under the war program: not the source's owner.
    let ix = protocol_transfer(&w, &mint, &equip_owner, &a.pubkey(), CUT / 2, WAR_ID, equip_b);
    w.env
        .send_paid_by(&[as_items(&equip_seeds(&mint, ITEM_SLOT), ix)], &payer, &[])
        .expect_code(u32::from(TokenError::NotProtocolSource));
}
