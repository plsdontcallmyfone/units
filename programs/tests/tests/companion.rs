// Changed by Hookwars: Split gains war_bps; the war chest's share is tested at the end.
//! Companions (`docs/companions.md`): a launch whose creator is the companion's creator address,
//! so its creator fees are bought back and burned, streamed to holders or paid to its launcher by
//! code, every step permissionless.

use anchor_lang::prelude::Pubkey;
use bordrless_companion::client as companion;
use bordrless_companion::error::CompanionError;
use bordrless_companion::events::{BoughtBack, CompanionWarFunded, FeesClaimed};
use bordrless_companion::instructions::CreateArgs;
use bordrless_companion::state::{Companion, Split};
use bordrless_core::policy;
use bordrless_launch::client::{self as launch, LaunchKeys};
use bordrless_launch::state::LaunchRules;
use bordrless_program_tests::env::Tx;
use bordrless_program_tests::fixture::World;
use bordrless_program_tests::launch::*;
use bordrless_token::client as token;
use bordrless_token::state::Mint;
use solana_keypair::Keypair;
use solana_signer::Signer;

const CREATOR_FEE: u16 = 200;

fn args(split: Split, vest_secs: i64) -> CreateArgs {
    CreateArgs {
        split,
        bounty_bps: 50,
        max_buyback: SOL,
        buyback_interval: 60,
        vest_secs,
        fund: SOL / 2,
    }
}

fn code(e: CompanionError) -> u32 {
    u32::from(e)
}

/// A companion for a fresh mint, then the launch through it. Answers the mint.
fn launch_through(
    w: &mut World,
    launcher: &Keypair,
    split: Split,
    vest_secs: i64,
    rules: LaunchRules,
) -> (Pubkey, Tx) {
    let mint = Keypair::new();
    let ix = companion::create(
        launcher.pubkey(),
        launcher.pubkey(),
        mint.pubkey(),
        args(split, vest_secs),
    );
    w.env.send_paid_by(&[ix], launcher, &[&mint]).ok();
    let creator = companion::creator_address(&mint.pubkey());
    let launch_args = World::launch_args("CMP", CREATOR_FEE, VQ, rules);
    let inner = launch::create_launch_with(
        creator,
        mint.pubkey(),
        w.env.treasury.pubkey(),
        w.sol,
        policy::LP_FEE_BPS,
        launch_args.clone(),
        None,
        None,
    );
    let ix = companion::launch(launcher.pubkey(), mint.pubkey(), &inner, launch_args);
    let tx = w.env.send_paid_by(&[ix], launcher, &[&mint]);
    (mint.pubkey(), tx)
}

fn keys(w: &World, mint: &Pubkey) -> LaunchKeys {
    LaunchKeys::of(&w.launch(mint))
}

fn companion_of(w: &World, mint: &Pubkey) -> Companion {
    w.env.read(&companion::companion_address(mint))
}

/// Trades enough to accrue creator fees: buys and sells by a few wallets.
fn trade(w: &mut World, mint: &Pubkey, wallets: usize, sol: u64) {
    for _ in 0..wallets {
        let t = w.wallet_with_sol(sol + SOL);
        w.buy(&t, mint, sol).ok();
        let held = w.env.holding(mint, &t.pubkey());
        w.sell(&t, mint, held / 4).ok();
    }
}

/// Honest buybacks that wait while the price is above the reference, until it has caught up and
/// the next one would buy (the reference moves 5% an interval toward the price).
fn catch_up(w: &mut World, cranker: &Keypair, mint: &Pubkey) {
    for _ in 0..80 {
        let c = companion_of(w, mint);
        let p = w.launch_pool(mint);
        let spot = bordrless_companion::state::spot_price(
            p.quote_reserve,
            p.virtual_quote,
            p.base_reserve,
            p.virtual_base,
        )
        .unwrap();
        if spot <= c.reference_price * 10_300 / 10_000 {
            return;
        }
        w.env.warp(61);
        let tx = w.env.send_paid_by(
            &[companion::buyback(cranker.pubkey(), &keys(w, mint), false)],
            cranker,
            &[],
        );
        tx.ok();
        assert_eq!(
            tx.events::<bordrless_companion::events::BoughtBack>().len(),
            0
        );
    }
    panic!("the reference never caught up");
}

#[test]
fn the_token_that_buys_itself() {
    let mut w = World::new();
    let launcher = w.wallet_with_sol(5 * SOL);
    let split = Split {
        buyback_bps: 10_000,
        holders_bps: 0,
        beneficiary_bps: 0,
        war_bps: 0,
    };
    let (mint, tx) = launch_through(&mut w, &launcher, split, 0, LaunchRules::NONE);
    tx.ok();
    println!(
        "companion launch: CU {} size {} height {}",
        tx.cu(),
        tx.size,
        tx.max_height()
    );
    let l = w.launch(&mint);
    let creator = companion::creator_address(&mint);
    assert_eq!(l.creator, creator);
    let c = companion_of(&w, &mint);
    assert!(c.launched);

    w.env.warp(31);
    trade(&mut w, &mint, 4, 2 * SOL);
    let accrued = w.env.holding(&w.sol, &launch::launch_address(&mint));
    assert!(accrued > 0);

    // A stranger claims: paid its bounty in SOL; the rest waits for the buyback.
    let cranker = w.wallet_with_sol(SOL);
    let before = w.env.lamports(&cranker.pubkey());
    let tx = w.env.send_paid_by(
        &[companion::claim_fees(cranker.pubkey(), mint, None)],
        &cranker,
        &[],
    );
    tx.ok();
    let ev: FeesClaimed = tx.event();
    assert_eq!(ev.claimed, accrued);
    assert_eq!(ev.bounty, accrued * 50 / 10_000);
    assert_eq!(ev.to_buyback, accrued - ev.bounty);
    assert!(
        w.env.lamports(&cranker.pubkey()) > before - 10_000,
        "the bounty covers the fee"
    );
    assert_eq!(companion_of(&w, &mint).pending_buyback, accrued - ev.bounty);

    // Not before a minute after launch, and not while the price is more than 3% above the
    // reference (which the trades above moved it past): then bought and burned, the supply down by
    // what it bought.
    w.env.warp(60);
    catch_up(&mut w, &cranker, &mint);
    let pending = companion_of(&w, &mint).pending_buyback;
    let supply = w.env.read::<Mint>(&mint).supply;
    let tx = w.env.send_paid_by(
        &[companion::buyback(
            cranker.pubkey(),
            &keys(&w, &mint),
            false,
        )],
        &cranker,
        &[],
    );
    tx.ok();
    println!(
        "buyback: CU {} size {} height {}",
        tx.cu(),
        tx.size,
        tx.max_height()
    );
    let ev: BoughtBack = tx.event();
    assert!(ev.burned > 0);
    assert_eq!(w.env.read::<Mint>(&mint).supply, supply - ev.burned);
    assert_eq!(w.env.holding(&mint, &creator), 0);
    let c = companion_of(&w, &mint);
    // At most 1 SOL and 1% of the pool's quote side a buyback: what is left waits for the next.
    let pool = w.launch_pool(&mint);
    assert!(ev.spent + ev.bounty <= SOL.min((pool.quote_reserve + pool.virtual_quote) / 100) + 1);
    assert_eq!(c.pending_buyback, pending - ev.spent - ev.bounty);
    assert_eq!(c.burned_total, ev.burned);
    // The next buyback waits for the interval.
    trade(&mut w, &mint, 2, SOL);
    w.env
        .send_paid_by(
            &[companion::claim_fees(cranker.pubkey(), mint, None)],
            &cranker,
            &[],
        )
        .ok();
    let tx = w.env.send_paid_by(
        &[companion::buyback(
            cranker.pubkey(),
            &keys(&w, &mint),
            false,
        )],
        &cranker,
        &[],
    );
    tx.expect_code(code(CompanionError::BuybackNotDue));
    w.env.warp(60);
    w.env
        .send_paid_by(
            &[companion::buyback(
                cranker.pubkey(),
                &keys(&w, &mint),
                false,
            )],
            &cranker,
            &[],
        )
        .ok();
}

#[test]
fn the_rug_proof_dev() {
    let mut w = World::new();
    let launcher = w.wallet_with_sol(10 * SOL);
    let split = Split {
        buyback_bps: 0,
        holders_bps: 5_000,
        beneficiary_bps: 5_000,
        war_bps: 0,
    };
    let rules = presets::burn(); // holder rewards 0.5%, burn 0.5%
    let vest = 30 * 86_400;
    let (mint, tx) = launch_through(&mut w, &launcher, split, vest, rules);
    tx.ok();
    let k = keys(&w, &mint);
    let creator = companion::creator_address(&mint);

    // The dev's buy is the companion's, vesting to them.
    let tx = w.env.send_paid_by(
        &[companion::dev_buy(launcher.pubkey(), &k, SOL / 2, 1)],
        &launcher,
        &[],
    );
    tx.ok();
    let bag = w.env.holding(&mint, &creator);
    assert!(bag > 0);
    assert_eq!(companion_of(&w, &mint).dev_tokens, bag);
    // Only the beneficiary buys for the bag.
    let stranger = w.wallet_with_sol(2 * SOL);
    let tx = w.env.send_paid_by(
        &[companion::dev_buy(stranger.pubkey(), &k, SOL / 2, 1)],
        &stranger,
        &[],
    );
    tx.expect_fail();

    // Half vests in half the time; release is permissionless and pays only the beneficiary.
    w.env.warp(vest / 2);
    w.env
        .send_paid_by(
            &[companion::release(
                stranger.pubkey(),
                &k,
                true,
                launcher.pubkey(),
            )],
            &stranger,
            &[],
        )
        .ok();
    let released = w.env.holding(&mint, &launcher.pubkey());
    assert!(released.abs_diff(bag / 2) <= 1, "{released} of {bag}");
    let tx = w.env.send_paid_by(
        &[companion::release(
            stranger.pubkey(),
            &k,
            true,
            launcher.pubkey(),
        )],
        &stranger,
        &[],
    );
    tx.expect_code(code(CompanionError::NothingToDo));

    // Fees: half to holders through the kit, half to the dev as SOL.
    trade(&mut w, &mint, 6, 3 * SOL);
    let tx = w.env.send_paid_by(
        &[companion::claim_fees(stranger.pubkey(), mint, None)],
        &stranger,
        &[],
    );
    tx.ok();
    let ev: FeesClaimed = tx.event();
    assert_eq!(ev.to_buyback, 0);
    assert!(ev.to_holders > 0 && ev.to_beneficiary > 0);
    let vault = launch::holder_vault_address(&mint, &w.sol);
    let in_vault = w.env.read::<bordrless_token::state::Holding>(&vault).amount;
    w.env
        .send_paid_by(&[companion::share(stranger.pubkey(), mint)], &stranger, &[])
        .ok();
    let shared = companion_of(&w, &mint).shared_total;
    assert!(shared > 0);
    assert_eq!(
        w.env.read::<bordrless_token::state::Holding>(&vault).amount,
        in_vault + shared
    );
    let before = w.env.lamports(&launcher.pubkey());
    w.env
        .send_paid_by(
            &[companion::withdraw(
                stranger.pubkey(),
                mint,
                launcher.pubkey(),
            )],
            &stranger,
            &[],
        )
        .ok();
    assert!(w.env.lamports(&launcher.pubkey()) >= before + ev.to_beneficiary);
}

#[test]
fn what_a_companion_refuses() {
    let mut w = World::new();
    let launcher = w.wallet_with_sol(10 * SOL);
    let mint_kp = Keypair::new();
    let mint = mint_kp.pubkey();
    let bad = |split, bounty_bps| CreateArgs {
        bounty_bps,
        ..args(split, 0)
    };
    let send = |w: &mut World, a: CreateArgs| {
        w.env.send_paid_by(
            &[companion::create(
                launcher.pubkey(),
                launcher.pubkey(),
                mint,
                a,
            )],
            &launcher,
            &[&mint_kp],
        )
    };
    send(
        &mut w,
        bad(
            Split {
                buyback_bps: 9_000,
                holders_bps: 0,
                beneficiary_bps: 0,
                war_bps: 0,
            },
            50,
        ),
    )
    .expect_code(code(CompanionError::BadSplit));
    send(
        &mut w,
        bad(
            Split {
                buyback_bps: 10_000,
                holders_bps: 0,
                beneficiary_bps: 0,
                war_bps: 0,
            },
            101,
        ),
    )
    .expect_code(code(CompanionError::BountyTooHigh));
    send(
        &mut w,
        CreateArgs {
            buyback_interval: 10,
            ..args(
                Split {
                    buyback_bps: 10_000,
                    holders_bps: 0,
                    beneficiary_bps: 0,
                    war_bps: 0,
                },
                0,
            )
        },
    )
    .expect_code(code(CompanionError::BadBuybackLimits));

    // Holders without holder rewards, or the creator wallet lock, are refused at launch.
    let (_, tx) = launch_through(
        &mut w,
        &launcher,
        Split {
            buyback_bps: 0,
            holders_bps: 10_000,
            beneficiary_bps: 0,
            war_bps: 0,
        },
        0,
        LaunchRules::NONE,
    );
    tx.expect_code(code(CompanionError::HolderRewardsOff));
    let (_, tx) = launch_through(
        &mut w,
        &launcher,
        Split {
            buyback_bps: 10_000,
            holders_bps: 0,
            beneficiary_bps: 0,
            war_bps: 0,
        },
        0,
        presets::rewards_and_cap(),
    );
    tx.expect_code(code(CompanionError::CreatorLockUnsupported));

    // A step pointed at other accounts: the swap's recipient swapped for the cranker's holding.
    let (mint, tx) = launch_through(
        &mut w,
        &launcher,
        Split {
            buyback_bps: 10_000,
            holders_bps: 0,
            beneficiary_bps: 0,
            war_bps: 0,
        },
        0,
        LaunchRules::NONE,
    );
    tx.ok();
    w.env.warp(31);
    trade(&mut w, &mint, 3, 2 * SOL);
    let thief = w.wallet_with_sol(SOL);
    w.env
        .send_paid_by(
            &[companion::claim_fees(thief.pubkey(), mint, None)],
            &thief,
            &[],
        )
        .ok();
    w.env.warp(61);
    catch_up(&mut w, &thief, &mint);
    let mut ix = companion::buyback(thief.pubkey(), &keys(&w, &mint), false);
    let creator_holding = token::holding_address(&mint, &companion::creator_address(&mint));
    let theirs = token::holding_address(&mint, &thief.pubkey());
    for m in ix
        .accounts
        .iter_mut()
        .filter(|m| m.pubkey == creator_holding)
    {
        m.pubkey = theirs;
    }
    w.env
        .send_paid_by(&[ix], &thief, &[])
        .expect_code(code(CompanionError::MissingAccount));
    // Withdraw pays only the companion's beneficiary.
    w.env
        .send_paid_by(
            &[companion::withdraw(thief.pubkey(), mint, thief.pubkey())],
            &thief,
            &[],
        )
        .expect_fail();
}

/// The launch through a companion fits mainnet's limits as the site sends it: v0 with the protocol
/// lookup table (extended with the companion program and its event authority), the longest
/// metadata the site sends, holder rewards and a burn, at stack height 5 (Solana's limit).
#[test]
fn a_companion_launch_fits_mainnet_limits() {
    use bordrless_program_tests::env::{compute_unit_limit, compute_unit_price};
    let mut w = World::new();
    let mut addresses = protocol_lookup_table(&w);
    let base_table = w.env.put_lookup_table(Pubkey::new_unique(), &addresses);
    addresses.extend([
        bordrless_companion::ID,
        companion::event_authority(),
        bordrless_launch::ID,
        bordrless_swap::ID,
        bordrless_token::ID,
    ]);
    let table = w.env.put_lookup_table(Pubkey::new_unique(), &addresses);
    let launcher = w.wallet_with_sol(5 * SOL);
    let mint = Keypair::new();
    let split = Split {
        buyback_bps: 5_000,
        holders_bps: 5_000,
        beneficiary_bps: 0,
        war_bps: 0,
    };
    w.env
        .send_paid_by(
            &[companion::create(
                launcher.pubkey(),
                launcher.pubkey(),
                mint.pubkey(),
                args(split, 0),
            )],
            &launcher,
            &[&mint],
        )
        .ok();
    let creator = companion::creator_address(&mint.pubkey());
    let mut launch_args = World::launch_args("TENCHARSXX", CREATOR_FEE, VQ, presets::burn());
    launch_args.name = "N".repeat(32);
    launch_args.uri = format!("https://gateway.pinata.cloud/ipfs/{}", "b".repeat(94));
    let inner = launch::create_launch_with(
        creator,
        mint.pubkey(),
        w.env.treasury.pubkey(),
        w.sol,
        policy::LP_FEE_BPS,
        launch_args.clone(),
        None,
        None,
    );
    let ix = companion::launch(launcher.pubkey(), mint.pubkey(), &inner, launch_args);
    let ixs = [
        compute_unit_limit(1_400_000),
        compute_unit_price(20_000),
        ix,
    ];
    let base = w.env.v0_size(&ixs, &launcher, &[&mint], &[base_table]);
    let tx = w.env.send_v0(&ixs, &launcher, &[&mint], &[table]);
    tx.ok();
    println!("companion launch v0: {} bytes with the protocol table, {} with it extended; {} CU; height {}", base, tx.size, tx.cu(), tx.max_height());
    assert!(tx.size <= 1_232, "{} bytes", tx.size);
    assert!(tx.max_height() <= 5);
    assert_eq!(w.launch(&mint.pubkey()).creator, creator);
}

/// Hookwars: a companion whose split gives the war chest a share. Every claim pays it at once, as
/// bridged SOL, into the holding of `PDA(["war-chest", mint], WAR_ID)` (created by the claim), with
/// `CompanionWarFunded`; the other parts split what is left exactly as before, and the shares still
/// add up to 10,000.
#[test]
fn a_claim_pays_the_war_chest_its_share() {
    let mut w = World::new();
    let launcher = w.wallet_with_sol(5 * SOL);
    let split = Split {
        buyback_bps: 5_000,
        holders_bps: 0,
        beneficiary_bps: 3_000,
        war_bps: 2_000,
    };
    let (mint, tx) = launch_through(&mut w, &launcher, split, 0, LaunchRules::NONE);
    tx.ok();
    w.env.warp(31);
    trade(&mut w, &mint, 4, 2 * SOL);
    let accrued = w.env.holding(&w.sol, &launch::launch_address(&mint));
    assert!(accrued > 0);

    let war_chest = bordrless_companion::instructions::war_chest_address(&mint);
    assert_eq!(
        war_chest,
        Pubkey::find_program_address(
            &[b"war-chest", mint.as_ref()],
            &bordrless_companion::constants::WAR_ID
        )
        .0
    );
    let war_holding = token::holding_address(&w.sol, &war_chest);
    assert!(w.env.account(&war_holding).is_none(), "no war holding before the first claim");

    let cranker = w.wallet_with_sol(SOL);
    let tx = w.env.send_paid_by(
        &[companion::claim_fees(cranker.pubkey(), mint, None)],
        &cranker,
        &[],
    );
    tx.ok();
    println!(
        "claim_fees with a war share: CU {} size {} height {}",
        tx.cu(),
        tx.size,
        tx.max_height()
    );
    let ev: FeesClaimed = tx.event();
    let war: CompanionWarFunded = tx.event();
    assert_eq!(ev.claimed, accrued);
    let bounty = accrued * 50 / 10_000;
    let rest = accrued - bounty;
    let to_war = rest * 2_000 / 10_000;
    let to_beneficiary = rest * 3_000 / 10_000;
    assert_eq!(ev.bounty, bounty);
    assert_eq!(war.amount, to_war);
    assert_eq!(war.war_chest, war_chest);
    assert_eq!(war.mint, mint);
    assert_eq!(war.war_total, to_war);
    assert_eq!(ev.to_beneficiary, to_beneficiary);
    assert_eq!(ev.to_buyback, rest - to_war - to_beneficiary, "rounding stays with buybacks");
    assert_eq!(ev.to_holders, 0);
    assert_eq!(w.env.holding(&w.sol, &war_chest), to_war);
    let c = companion_of(&w, &mint);
    assert_eq!(c.war_total, to_war);
    assert_eq!(c.pending_beneficiary, to_beneficiary);
    // The creator's holding keeps exactly what is set aside: the war share left it.
    let creator = companion::creator_address(&mint);
    assert_eq!(
        w.env.holding(&w.sol, &creator),
        c.pending_buyback + c.pending_holders + c.pending_beneficiary
    );

    // A second claim pays into the same holding and the total grows.
    trade(&mut w, &mint, 2, SOL);
    let accrued2 = w.env.holding(&w.sol, &launch::launch_address(&mint));
    let tx = w.env.send_paid_by(
        &[companion::claim_fees(cranker.pubkey(), mint, None)],
        &cranker,
        &[],
    );
    tx.ok();
    let war2: CompanionWarFunded = tx.event();
    let rest2 = accrued2 - accrued2 * 50 / 10_000;
    assert_eq!(war2.amount, rest2 * 2_000 / 10_000);
    assert_eq!(war2.war_total, to_war + war2.amount);
    assert_eq!(w.env.holding(&w.sol, &war_chest), to_war + war2.amount);
}

/// Hookwars: without a war share a claim emits no `CompanionWarFunded` and creates no war holding;
/// a split whose four parts do not add up to 10,000 is refused.
#[test]
fn no_war_share_no_war_payment_and_the_split_still_adds_up() {
    let mut w = World::new();
    let launcher = w.wallet_with_sol(5 * SOL);
    let split = Split {
        buyback_bps: 10_000,
        holders_bps: 0,
        beneficiary_bps: 0,
        war_bps: 0,
    };
    let (mint, tx) = launch_through(&mut w, &launcher, split, 0, LaunchRules::NONE);
    tx.ok();
    w.env.warp(31);
    trade(&mut w, &mint, 2, SOL);
    let cranker = w.wallet_with_sol(SOL);
    let tx = w.env.send_paid_by(
        &[companion::claim_fees(cranker.pubkey(), mint, None)],
        &cranker,
        &[],
    );
    tx.ok();
    assert!(tx.events::<CompanionWarFunded>().is_empty());
    let war_chest = bordrless_companion::instructions::war_chest_address(&mint);
    assert!(w.env.account(&token::holding_address(&w.sol, &war_chest)).is_none());

    for bad in [
        Split {
            buyback_bps: 5_000,
            holders_bps: 0,
            beneficiary_bps: 3_000,
            war_bps: 1_000,
        },
        Split {
            buyback_bps: 5_000,
            holders_bps: 0,
            beneficiary_bps: 3_000,
            war_bps: 3_000,
        },
    ] {
        let mint = Keypair::new();
        let ix = companion::create(
            launcher.pubkey(),
            launcher.pubkey(),
            mint.pubkey(),
            args(bad, 0),
        );
        w.env
            .send_paid_by(&[ix], &launcher, &[&mint])
            .expect_code(code(CompanionError::BadSplit));
    }
}
