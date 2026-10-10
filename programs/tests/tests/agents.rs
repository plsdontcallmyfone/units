// Changed by Hookwars: new file, the agents suite (09 section 12, A1 to A5).
//! `hookwars_agents` in LiteSVM against the real token program, armory and items program.

use anchor_lang::prelude::Pubkey;
use anchor_lang::InstructionData;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_program_tests::agents::*;
use bordrless_program_tests::armory::{params, Hw};
use bordrless_program_tests::items;
use bordrless_program_tests::slots::item_slot;
use bordrless_token::client as token;
use bordrless_token::state::{Holding, Mint};
use hookwars_agents::constants::{bond_status, kind, pda as apda, proof, record_kind, status};
use hookwars_agents::error::AgentsError as E;
use hookwars_agents::state::{AgentKey, Attestation, Bond, Link, OperatorIndex, Policy, PolicyLimits, TrackedLimit};
use hookwars_armory::state::{proposal_status, Proposal};
use hookwars_common::{ids, template_id, EquipConfig};
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

fn holding(aw: &Aw, mint: &Pubkey, owner: &Pubkey) -> Holding {
    aw.hw.w.env.read(&token::holding_address(mint, owner))
}

// ---- A1 passports and badges ------------------------------------------------------------------

#[test]
fn a_passport_issues_a_badge_that_cannot_move() {
    let mut aw = Aw::new();
    let before = aw.hw.w.env.lamports(&aw.fee_collector);
    let a = aw.agent("scout", kind::AUTHOR | kind::DIPLOMAT);
    assert_eq!(
        aw.hw.w.env.lamports(&aw.fee_collector) - before,
        TEST_AGENTS_PARAMS.passport_fee_lamports
    );
    let p = aw.passport(&a.passport);
    assert_eq!(p.operator, a.operator.pubkey());
    assert_eq!(p.agent_key, a.key.pubkey());
    assert_eq!((p.status, p.proof, p.badge_issued), (status::ACTIVE, proof::DECLARED, false));
    assert_eq!(p.badge_mint, apda::badge_mint(&a.passport, 0).0);
    let rec: AgentKey = aw.hw.w.env.read(&apda::agent_key(&a.key.pubkey()).0);
    assert_eq!(rec.passport, a.passport);
    let oi: OperatorIndex = aw.hw.w.env.read(&apda::operator(&a.operator.pubkey()).0);
    assert_eq!((oi.next, oi.active), (1, 1));

    // Not issued until the slot holds the Soulbound item.
    let ix = aw.issue_ix(&a.operator.pubkey(), &a.passport);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).expect_code(agents_code(E::BadgeNotEquipped));
    aw.badge(&a);
    let m: Mint = aw.hw.w.env.read(&a.badge);
    assert_eq!((m.supply, m.mint_authority, m.freeze_authority), (1, None, Some(ids::AGENTS_SIGNER)));
    assert_eq!(m.slots[0].item, aw.soulbound);
    assert_eq!(holding(&aw, &a.badge, &a.key.pubkey()).amount, 1);
    assert!(aw.passport(&a.passport).badge_issued);

    // A wallet send and a delegated send are refused.
    let other = aw.hw.w.env.funded(SOL).pubkey();
    items::transfer(&mut aw.hw, &a.key, &a.badge, &other, 1).expect_code(soulbound_code());
    let delegate = aw.hw.w.env.funded(SOL);
    let approve = token::approve(a.key.pubkey(), token::holding_address(&a.badge, &a.key.pubkey()), delegate.pubkey(), 1);
    aw.hw.w.env.send_paid_by(&[approve], &a.key, &[]).ok();
    let extras = items::items_extras(&aw.hw, &a.badge, bordrless_token::slots::SlotOp::Transfer);
    let ixs = [
        token::create_holding(delegate.pubkey(), a.badge, other),
        token::transfer_with(
            delegate.pubkey(),
            token::holding_address(&a.badge, &a.key.pubkey()),
            token::holding_address(&a.badge, &other),
            a.badge,
            None,
            extras,
            1,
        ),
    ];
    aw.hw.w.env.send_paid_by(&ixs, &delegate, &[]).expect_code(soulbound_code());
    assert_eq!(holding(&aw, &a.badge, &a.key.pubkey()).amount, 1);
    // Issued once.
    let ix = aw.issue_ix(&a.operator.pubkey(), &a.passport);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).expect_code(agents_code(E::BadgeIssued));
}

#[test]
fn soulbound_binds_only_on_badges() {
    let mut aw = Aw::new();
    let owner = aw.hw.w.env.funded(10 * SOL);
    let mint = aw.hw.slot_mint(&owner, vec![item_slot(slot_kind::DEFENSE, equip_rule::LOCKED, 0, 0, false)]);
    aw.hw.equip_launch(&owner, &mint, Hw::entry(0, Some(aw.soulbound), EquipConfig::default())).ok();
    aw.hw.mint_to(&owner, &mint, &owner.pubkey(), 1_000);
    let to = aw.hw.w.env.funded(SOL).pubkey();
    items::transfer(&mut aw.hw, &owner, &mint, &to, 10).ok();
    assert_eq!(holding(&aw, &mint, &to).amount, 10);
}

#[test]
fn registration_refusals_and_the_operator_limit() {
    let mut aw = Aw::new();
    let op = aw.hw.w.env.funded(10 * SOL);
    // Agent key equal to the operator.
    let (ix, _) = aw.register_ix(&op.pubkey(), &op.pubkey(), &op.pubkey(), profile("self", 0));
    aw.hw.w.env.send_paid_by(&[ix], &op, &[]).expect_code(agents_code(E::KeyIsOperator));
    // Unknown kind bits; a name over the bound.
    let k = aw.hw.w.env.funded(SOL);
    let (ix, _) = aw.register_ix(&op.pubkey(), &k.pubkey(), &op.pubkey(), profile("bits", 0x80));
    aw.hw.w.env.send_paid_by(&[ix], &op, &[&k]).expect_code(agents_code(E::BadKinds));
    let long = "n".repeat(usize::from(TEST_AGENTS_PARAMS.name_max_len) + 1);
    let (ix, _) = aw.register_ix(&op.pubkey(), &k.pubkey(), &op.pubkey(), profile(&long, 0));
    assert!(aw.hw.w.env.send_paid_by(&[ix], &op, &[&k]).result.is_err());
    // Two passports, then the limit.
    let a = aw.agent_of(op.insecure_clone(), k.insecure_clone(), "one", 0);
    let k2 = aw.hw.w.env.funded(SOL);
    let _b = aw.agent_of(op.insecure_clone(), k2, "two", 0);
    let k3 = aw.hw.w.env.funded(SOL);
    let (ix, _) = aw.register_ix(&op.pubkey(), &k3.pubkey(), &op.pubkey(), profile("three", 0));
    aw.hw.w.env.send_paid_by(&[ix], &op, &[&k3]).expect_code(agents_code(E::OperatorLimit));
    // A key belongs to one passport.
    let op2 = aw.hw.w.env.funded(10 * SOL);
    let (ix, _) = aw.register_ix(&op2.pubkey(), &a.key.pubkey(), &op2.pubkey(), profile("dup", 0));
    assert!(aw.hw.w.env.send_paid_by(&[ix], &op2, &[&a.key]).result.is_err());
}

#[test]
fn pause_retire_and_rotate() {
    let mut aw = Aw::new();
    let a = aw.agent("rotor", kind::CRANKER);
    aw.badge(&a);
    // Paused: records refuse.
    let ix = aw.status_ix(&a, status::PAUSED);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    let ix = aw.record_ix(&a.passport, &a.key.pubkey(), record_kind::CRANK, 5);
    aw.hw.w.env.send(&[ix], &[]).expect_code(agents_code(E::AgentPaused));
    let ix = aw.status_ix(&a, status::ACTIVE);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();

    // Rotate to a new key: the old badge freezes, a new badge mint at generation 1.
    let new_key = aw.hw.w.env.funded(SOL);
    let p = aw.passport(&a.passport);
    let new_badge = apda::badge_mint(&a.passport, 1).0;
    let ix = agents_ix(
        hookwars_agents::accounts::RotateAgentKey {
            operator: a.operator.pubkey(),
            new_key: new_key.pubkey(),
            payer: a.operator.pubkey(),
            config: apda::config().0,
            passport: a.passport,
            old_key_record: apda::agent_key(&a.key.pubkey()).0,
            new_key_record: apda::agent_key(&new_key.pubkey()).0,
            old_badge_mint: p.badge_mint,
            old_badge_holding: token::holding_address(&p.badge_mint, &a.key.pubkey()),
            new_badge_mint: new_badge,
            signer: ids::AGENTS_SIGNER,
            token: token_accs(),
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::RotateAgentKey {},
    );
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[&new_key]).ok();
    assert!(holding(&aw, &p.badge_mint, &a.key.pubkey()).frozen);
    assert!(aw.hw.w.env.account(&apda::agent_key(&a.key.pubkey()).0).is_none_or(|x| x.data.is_empty()));
    let p = aw.passport(&a.passport);
    assert_eq!((p.agent_key, p.badge_mint, p.badge_generation, p.badge_issued), (new_key.pubkey(), new_badge, 1, false));
    let rotated = Agent {
        operator: a.operator.insecure_clone(),
        key: new_key,
        passport: a.passport,
        badge: new_badge,
    };
    aw.badge(&rotated);
    assert_eq!(holding(&aw, &new_badge, &rotated.key.pubkey()).amount, 1);

    // Retired: the badge freezes, the operator's count drops, and it stays retired.
    let ix = aw.status_ix(&rotated, status::RETIRED);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    assert!(holding(&aw, &new_badge, &rotated.key.pubkey()).frozen);
    let oi: OperatorIndex = aw.hw.w.env.read(&apda::operator(&a.operator.pubkey()).0);
    assert_eq!(oi.active, 0);
    let ix = aw.status_ix(&rotated, status::ACTIVE);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).expect_code(agents_code(E::BadStatus));
}

#[test]
fn profile_updates_follow_into_the_badge_and_the_armory_accepts_the_agents_caller_for_badges() {
    let mut aw = Aw::new();
    let a = aw.agent("muse", kind::AUTHOR);
    let mut args = profile("muse-2", kind::AUTHOR | kind::RAIDER);
    args.avatar_uri = "https://example.invalid/new.png".to_string();
    let ix = agents_ix(
        hookwars_agents::accounts::UpdateProfile {
            operator: a.operator.pubkey(),
            config: apda::config().0,
            passport: a.passport,
            badge_mint: a.badge,
            signer: ids::AGENTS_SIGNER,
            token: token_accs(),
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::UpdateProfile { args },
    );
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    let m: Mint = aw.hw.w.env.read(&a.badge);
    assert_eq!((m.name.as_str(), m.uri.as_str()), ("muse-2", "https://example.invalid/new.png"));
    assert_eq!(aw.passport(&a.passport).kinds, kind::AUTHOR | kind::RAIDER);

    // Integration pass 2 (R28): `equip_badge` through the agents program equips the Soulbound item.
    let equip = aw
        .hw
        .equip_accounts(&a.operator.pubkey(), &a.badge, 0, None, Some(aw.soulbound));
    let caller = apda::armory_caller(&a.badge).0;
    let mut inner = anchor_lang::ToAccountMetas::to_account_metas(
        &hookwars_armory::accounts::EquipLaunch {
            launch_caller: caller,
            config: hookwars_common::pda::config().0,
            slot_state: hookwars_common::pda::slot_state(&a.badge, 0).0,
            equip,
            system_program: anchor_lang::system_program::ID,
            event_authority: bordrless_program_tests::armory::armory_events(),
            program: ids::ARMORY_ID,
        },
        None,
    );
    for m in inner.iter_mut() {
        if m.pubkey == caller {
            m.is_signer = false;
        }
    }
    let mut accounts = anchor_lang::ToAccountMetas::to_account_metas(
        &hookwars_agents::accounts::EquipBadge {
            config: apda::config().0,
            passport: a.passport,
            badge_mint: a.badge,
            caller,
            armory_program: ids::ARMORY_ID,
        },
        None,
    );
    accounts.extend(inner);
    let ix = anchor_lang::solana_program::instruction::Instruction {
        program_id: ids::AGENTS_ID,
        accounts,
        data: hookwars_agents::instruction::EquipBadge {}.data(),
    };
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    let m: Mint = aw.hw.w.env.read(&a.badge);
    assert_eq!(m.slots[0].item, aw.soulbound);
    // Soulbound is refused on any mint that is not a badge (09 section 21 item 2).
    let owner = aw.hw.w.env.funded(10 * SOL);
    let other = aw
        .hw
        .slot_mint(&owner, vec![item_slot(slot_kind::DEFENSE, equip_rule::VOTE, 0, 0, false)]);
    aw.hw.equip_launch(&owner, &other, Hw::entry(0, None, EquipConfig::default())).ok();
    aw.hw.mint_to(&owner, &other, &owner.pubkey(), 1_000_000);
    let soulbound = aw.soulbound;
    let (tx, _) = aw.hw.propose(&owner, &other, 0, Some(soulbound), EquipConfig::default());
    tx.expect_code(bordrless_program_tests::armory::armory_code(hookwars_armory::error::ArmoryError::NotBadge));
}

// ---- A2 proof levels ---------------------------------------------------------------------------

#[test]
fn links_need_the_agent_keys_ed25519_signature() {
    let mut aw = Aw::new();
    let a = aw.agent("linker", 0);
    let payer = aw.hw.w.env.funded(SOL);
    // No ed25519 instruction.
    let ix = aw.link_ix(&payer.pubkey(), &a.passport, 0, "linker_x");
    aw.hw.w.env.send_paid_by(&[ix], &payer, &[]).expect_code(agents_code(E::BadLinkSignature));
    // Signed by the operator, not the agent key.
    let stmt = Aw::statement(&a, 0, "linker_x");
    let ixs = [ed25519_ix(&a.operator, &stmt), aw.link_ix(&payer.pubkey(), &a.passport, 0, "linker_x")];
    aw.hw.w.env.send_paid_by(&ixs, &payer, &[]).expect_code(agents_code(E::BadLinkSignature));
    // The agent key signed another handle.
    let other = Aw::statement(&a, 0, "someone_else");
    let ixs = [ed25519_ix(&a.key, &other), aw.link_ix(&payer.pubkey(), &a.passport, 0, "linker_x")];
    aw.hw.w.env.send_paid_by(&ixs, &payer, &[]).expect_code(agents_code(E::BadLinkSignature));
    // The real statement.
    let ixs = [ed25519_ix(&a.key, &stmt), aw.link_ix(&payer.pubkey(), &a.passport, 0, "linker_x")];
    aw.hw.w.env.send_paid_by(&ixs, &payer, &[]).ok();
    let l: Link = aw.hw.w.env.read(&apda::link(&a.passport, 0).0);
    assert_eq!((l.handle.as_str(), l.statement_hash), ("linker_x", hookwars_agents::constants::sha256(&stmt)));
    assert_eq!(aw.passport(&a.passport).proof, proof::LINKED);
    // Unlink by the agent key: back to Declared.
    let ix = agents_ix(
        hookwars_agents::accounts::Unlink {
            authority: a.key.pubkey(),
            passport: a.passport,
            link: apda::link(&a.passport, 0).0,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::Unlink { platform: 0 },
    );
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).ok();
    assert_eq!(aw.passport(&a.passport).proof, proof::DECLARED);
}

#[test]
fn attestations_need_binding_quorum_and_time() {
    let mut aw = Aw::new();
    let a = aw.agent("enclave", 0);
    let ttl = 86_400;
    // Report data that does not bind this key.
    let mut bad = aw.attest_args(&a, ttl);
    bad.report_data[0] ^= 1;
    let ix = aw.attest_ix(&a, bad);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).expect_code(agents_code(E::BadReportData));
    let ix = aw.attest_ix(&a, aw.attest_args(&a, TEST_AGENTS_PARAMS.attest_max_ttl_secs + 1));
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).expect_code(agents_code(E::TtlTooLong));
    let ix = aw.attest_ix(&a, aw.attest_args(&a, ttl));
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).ok();
    // Not a verifier.
    let stranger = aw.hw.w.env.funded(SOL);
    let ix = aw.endorse_ix(&a.passport, &stranger.pubkey());
    aw.hw.w.env.send_paid_by(&[ix], &stranger, &[]).expect_code(agents_code(E::NotVerifier));
    // One of two: not yet attested; the same verifier twice still counts once.
    let v0 = aw.verifiers[0].insecure_clone();
    let v1 = aw.verifiers[1].insecure_clone();
    let ix = aw.endorse_ix(&a.passport, &v0.pubkey());
    aw.hw.w.env.send_paid_by(&[ix], &v0, &[]).ok();
    aw.hw.w.env.warp(1);
    let ix = aw.endorse_ix(&a.passport, &v0.pubkey());
    aw.hw.w.env.send_paid_by(&[ix], &v0, &[]).ok();
    assert_eq!(aw.passport(&a.passport).proof, proof::DECLARED);
    let ix = aw.endorse_ix(&a.passport, &v1.pubkey());
    aw.hw.w.env.send_paid_by(&[ix], &v1, &[]).ok();
    let att: Attestation = aw.hw.w.env.read(&apda::attestation(&a.passport).0);
    assert_eq!(att.endorsements, 2);
    assert_eq!(aw.passport(&a.passport).proof, proof::ATTESTED);
    // A revocation drops it below quorum.
    let ix = aw.revoke_ix(&a.passport, &v1.pubkey());
    aw.hw.w.env.send_paid_by(&[ix], &v1, &[]).ok();
    assert_eq!(aw.passport(&a.passport).proof, proof::DECLARED);
    let ix = aw.endorse_ix(&a.passport, &v1.pubkey());
    aw.hw.w.env.send_paid_by(&[ix], &v1, &[]).ok();
    assert_eq!(aw.passport(&a.passport).proof, proof::ATTESTED);
    // Expiry: a refresh drops it.
    aw.hw.w.env.warp(ttl + 1);
    let ix = aw.refresh_ix(&a.passport);
    aw.hw.w.env.send(&[ix], &[]).ok();
    assert_eq!(aw.passport(&a.passport).proof, proof::DECLARED);
}

// ---- A3 attribution ----------------------------------------------------------------------------

#[test]
fn records_credit_only_this_agent_from_the_recorders() {
    let mut aw = Aw::new();
    let a = aw.agent("worker", kind::CRANKER | kind::AUTHOR);
    let key = a.key.pubkey();
    let vault = apda::vault(&a.passport).0;
    for (k, v) in [
        (record_kind::ITEMS_AUTHORED, 0),
        (record_kind::ITEMS_EQUIPPED, 0),
        (record_kind::ITEMS_FORGED, 0),
        (record_kind::ROYALTY_CLAIM, 700),
        (record_kind::CRANK, 1_000),
        (record_kind::BOUNTY, 250),
        (record_kind::LOOT_REVEAL, 0),
    ] {
        let ix = aw.record_ix(&a.passport, &key, k, v);
        aw.hw.w.env.send(&[ix], &[]).ok();
    }
    // The vault acts for the agent too.
    let ix = aw.record_ix(&a.passport, &vault, record_kind::CRANK, 500);
    aw.hw.w.env.send(&[ix], &[]).ok();
    let r = aw.passport(&a.passport).record;
    assert_eq!(
        (r.items_authored, r.items_equipped, r.items_forged, r.royalty_claims, r.royalties_claimed_sol),
        (1, 1, 1, 1, 700)
    );
    assert_eq!((r.cranks, r.crank_value_lamports, r.bounties_claimed_lamports, r.loot_reveals), (2, 1_500, 250, 1));
    // Another actor; an unknown kind; a caller that is not a recorder.
    let ix = aw.record_ix(&a.passport, &Pubkey::new_unique(), record_kind::CRANK, 1);
    aw.hw.w.env.send(&[ix], &[]).expect_code(agents_code(E::NotThisAgent));
    let ix = aw.record_ix(&a.passport, &key, record_kind::MAX + 1, 1);
    aw.hw.w.env.send(&[ix], &[]).expect_code(agents_code(E::BadRecordKind));
    let fake = aw.hw.w.env.funded(SOL);
    let ix = agents_ix(
        hookwars_agents::accounts::Record {
            caller: fake.pubkey(),
            passport: a.passport,
            actor: key,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::Record { kind: record_kind::CRANK, value: 1 },
    );
    aw.hw.w.env.send_paid_by(&[ix], &fake, &[]).expect_code(agents_code(E::NotRecorder));
}

// ---- A4 policy wallet --------------------------------------------------------------------------

#[test]
fn the_policy_wallet_spends_within_its_limits() {
    let mut aw = Aw::new();
    let a = aw.agent("spender", kind::RAIDER);
    let vault = apda::vault(&a.passport).0;
    let funder = aw.hw.w.env.funded(100 * SOL);
    let tracked = aw.hw.w.mint_to_owner(&funder, 6, 1_000_000, "TRK");
    let untracked = aw.hw.w.mint_to_owner(&funder, 6, 1_000_000, "UNT");
    for m in [tracked, untracked] {
        let ixs = [
            token::create_holding(funder.pubkey(), m, vault),
            token::transfer(funder.pubkey(), token::holding_address(&m, &funder.pubkey()), token::holding_address(&m, &vault), m, None, vec![], 1_000),
        ];
        aw.hw.w.env.send_paid_by(&ixs, &funder, &[]).ok();
    }
    aw.hw.w.env.fund(vault, 2 * SOL);
    let limits = PolicyLimits {
        per_action_lamports: SOL,
        per_day_lamports: SOL,
        tracked: vec![TrackedLimit { mint: tracked, per_action: 300, per_day: 500 }],
        targets: vec![bordrless_token::ID],
    };
    // A target outside the config's list is refused at creation.
    let mut bad = limits.clone();
    bad.targets = vec![ids::BRIDGE_ID];
    let ix = aw.init_policy_ix(&a, bad);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).expect_code(agents_code(E::TargetNotAllowed));
    let ix = aw.init_policy_ix(&a, limits);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();

    let to = aw.hw.w.env.funded(SOL).pubkey();
    let send = |aw: &mut Aw, mint: Pubkey, amount: u64| {
        let create = token::create_holding(a.key.pubkey(), mint, to);
        let inner = token::transfer(vault, token::holding_address(&mint, &vault), token::holding_address(&mint, &to), mint, None, vec![], amount);
        let ix = aw.spend_ix(&a, inner);
        aw.hw.w.env.send_paid_by(&[create, ix], &a.key, &[])
    };
    send(&mut aw, tracked, 200).ok();
    // Over the per-action limit.
    send(&mut aw, tracked, 301).expect_code(agents_code(E::LimitExceeded));
    // Up to the day's limit exactly, then one more unit is over it.
    send(&mut aw, tracked, 300).ok();
    send(&mut aw, tracked, 1).expect_code(agents_code(E::LimitExceeded));
    send(&mut aw, untracked, 1).expect_code(agents_code(E::UntrackedHolding));
    assert_eq!(holding(&aw, &tracked, &to).amount, 500);
    let pol: Policy = aw.hw.w.env.read(&apda::policy(&a.passport).0);
    assert_eq!(pol.tracked[0].spent_today, 500);
    // A new day resets the window.
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.policy_day_secs);
    send(&mut aw, tracked, 300).ok();

    // Frozen: spend refuses, withdraw still works.
    let ix = agents_ix(aw.operator_policy_accounts(&a), hookwars_agents::instruction::FreezePolicy { frozen: true });
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    send(&mut aw, tracked, 1).expect_code(agents_code(E::PolicyFrozen));
    let before = aw.hw.w.env.lamports(&a.operator.pubkey());
    let ix = agents_ix(aw.withdraw_accounts(&a), hookwars_agents::instruction::Withdraw { amount: SOL, mint: None });
    let fee_payer = aw.hw.w.env.funded(SOL);
    aw.hw.w.env.send_paid_by(&[ix], &fee_payer, &[&a.operator]).ok();
    assert_eq!(aw.hw.w.env.lamports(&a.operator.pubkey()) - before, SOL);
    let op = a.operator.pubkey();
    let mut ix = agents_ix(aw.withdraw_accounts(&a), hookwars_agents::instruction::Withdraw { amount: 100, mint: Some(untracked) });
    ix.accounts.extend([
        anchor_lang::prelude::AccountMeta::new_readonly(untracked, false),
        anchor_lang::prelude::AccountMeta::new(token::holding_address(&untracked, &vault), false),
        anchor_lang::prelude::AccountMeta::new(token::holding_address(&untracked, &op), false),
    ]);
    let create = token::create_holding(fee_payer.pubkey(), untracked, op);
    aw.hw.w.env.send_paid_by(&[create, ix], &fee_payer, &[&a.operator]).ok();
    assert_eq!(holding(&aw, &untracked, &op).amount, 100);
}

// ---- A5 bonds ----------------------------------------------------------------------------------

/// Two tokens with one Relation slot (vote), each minted to `holder`.
fn treaty_world(aw: &mut Aw, holder: &Keypair, proposers: &[Pubkey]) -> (Pubkey, Pubkey) {
    let mut mints = Vec::new();
    for _ in 0..2 {
        let owner = aw.hw.w.env.funded(10 * SOL);
        let m = aw.hw.slot_mint(&owner, vec![item_slot(slot_kind::RELATION, equip_rule::VOTE, 0, 0, false)]);
        aw.hw.equip_launch(&owner, &m, Hw::entry(0, None, EquipConfig::default())).ok();
        aw.hw.mint_to(&owner, &m, &holder.pubkey(), 1_000_000);
        // Security review 1 M-2: a proposer must hold the proposal threshold of the supply.
        for p in proposers {
            aw.hw.mint_to(&owner, &m, p, 100_000);
        }
        mints.push(m);
    }
    (mints[0], mints[1])
}

fn post_bond_ix(aw: &Aw, a: &Agent, pa: &Pubkey, pb: &Pubkey, item: &Pubkey) -> anchor_lang::solana_program::instruction::Instruction {
    agents_ix(
        hookwars_agents::accounts::PostBond {
            agent_key: a.key.pubkey(),
            config: apda::config().0,
            passport: a.passport,
            proposal_a: *pa,
            proposal_b: *pb,
            treaty_item: *item,
            bond: apda::bond(&a.passport, pa).0,
            mark_a: apda::bond_mark(pa).0,
            mark_b: apda::bond_mark(pb).0,
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::PostBond {},
    )
}

fn resolve_ix(aw: &Aw, a: &Agent, bond: &Pubkey) -> anchor_lang::solana_program::instruction::Instruction {
    let b: Bond = aw.hw.w.env.read(bond);
    agents_ix(
        hookwars_agents::accounts::ResolveBond {
            config: apda::config().0,
            bond: *bond,
            passport: a.passport,
            proposal_a: b.proposal_a,
            proposal_b: b.proposal_b,
            mint_a: b.mint_a,
            mint_b: b.mint_b,
            armory_config: hookwars_common::pda::config().0,
            agent_key: a.key.pubkey(),
            inbox_a: apda::treaty_inbox(&b.mint_a),
            inbox_b: apda::treaty_inbox(&b.mint_b),
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::ResolveBond {},
    )
}

fn mark_ix(aw: &Aw, a: &Agent, bond: &Pubkey) -> anchor_lang::solana_program::instruction::Instruction {
    let b: Bond = aw.hw.w.env.read(bond);
    agents_ix(
        hookwars_agents::accounts::MarkTreatyOutcome {
            config: apda::config().0,
            bond: *bond,
            passport: a.passport,
            mint_a: b.mint_a,
            mint_b: b.mint_b,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::MarkTreatyOutcome {},
    )
}

/// Sets a proposal's status (and votes) directly: the armory's own transitions are its suites'
/// subject; here only what a bond reads matters.
fn set_proposal(aw: &mut Aw, proposal: &Pubkey, status: u8, votes_for: u64, votes_against: u64) {
    let mut acc = aw.hw.w.env.account(proposal).expect("proposal");
    let mut p: Proposal = anchor_lang::AccountDeserialize::try_deserialize(&mut &acc.data[..]).unwrap();
    p.status = status;
    p.votes_for = votes_for;
    p.votes_against = votes_against;
    let mut data = Vec::new();
    anchor_lang::AccountSerialize::try_serialize(&p, &mut data).unwrap();
    acc.data[..data.len()].copy_from_slice(&data);
    aw.hw.w.env.put(*proposal, acc);
}

/// A diplomat old enough to bond, with an open treaty proposal on each token.
fn bonded_pair(aw: &mut Aw) -> (Agent, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
    let a = aw.agent("envoy", kind::DIPLOMAT);
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.bond_min_passport_age_secs);
    let holder = aw.hw.w.env.funded(10 * SOL);
    let (ma, mb) = treaty_world(aw, &holder, &[a.key.pubkey()]);
    let (_, item, _) = aw.hw.item(template_id::TREATY, params(&[100, 100, 0]), 0);
    let (tx, pa) = aw.hw.propose(&a.key, &ma, 0, Some(item), EquipConfig { targets: vec![mb], role: 0 });
    tx.ok();
    let (tx, pb) = aw.hw.propose(&a.key, &mb, 0, Some(item), EquipConfig { targets: vec![ma], role: 0 });
    tx.ok();
    (a, ma, mb, item, pa, pb)
}

#[test]
fn bond_refusals() {
    let mut aw = Aw::new();
    // Too young, and not a diplomat.
    let young = aw.agent("young", kind::DIPLOMAT);
    let holder = aw.hw.w.env.funded(10 * SOL);
    let (ma, mb) = treaty_world(&mut aw, &holder, &[young.key.pubkey()]);
    let (_, item, _) = aw.hw.item(template_id::TREATY, params(&[100, 100, 0]), 0);
    let (_, pa) = aw.hw.propose(&young.key, &ma, 0, Some(item), EquipConfig { targets: vec![mb], role: 0 });
    let (_, pb) = aw.hw.propose(&young.key, &mb, 0, Some(item), EquipConfig { targets: vec![ma], role: 0 });
    let ix = post_bond_ix(&aw, &young, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &young.key, &[]).expect_code(agents_code(E::PassportTooYoung));
    let author = aw.agent("author", kind::AUTHOR);
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.bond_min_passport_age_secs);
    let ix = post_bond_ix(&aw, &author, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &author.key, &[]).expect_code(agents_code(E::NotDiplomat));
    // Same proposal twice; proposals by someone else; another item.
    let ix = post_bond_ix(&aw, &young, &pa, &pa, &item);
    assert!(aw.hw.w.env.send_paid_by(&[ix], &young.key, &[]).result.is_err());
    let other = aw.agent("other", kind::DIPLOMAT);
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.bond_min_passport_age_secs);
    let ix = post_bond_ix(&aw, &other, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &other.key, &[]).expect_code(agents_code(E::BadProposalPair));
    let (_, item2, _) = aw.hw.item(template_id::TREATY, params(&[50, 50, 0]), 0);
    let ix = post_bond_ix(&aw, &young, &pa, &pb, &item2);
    aw.hw.w.env.send_paid_by(&[ix], &young.key, &[]).expect_code(agents_code(E::BadProposalPair));
    // Posted once; a second bond on either proposal fails.
    let ix = post_bond_ix(&aw, &young, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &young.key, &[]).ok();
    let ix = post_bond_ix(&aw, &young, &pb, &pa, &item);
    assert!(aw.hw.w.env.send_paid_by(&[ix], &young.key, &[]).result.is_err());
}

#[test]
fn a_ratified_treaty_returns_the_bond_and_is_marked_held_or_broken() {
    let mut aw = Aw::new();
    let (a, ma, _mb, item, pa, pb) = bonded_pair(&mut aw);
    let before = aw.hw.w.env.lamports(&a.key.pubkey());
    let ix = post_bond_ix(&aw, &a, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).ok();
    let bond = apda::bond(&a.passport, &pa).0;
    assert_eq!(aw.passport(&a.passport).record.treaties_proposed, 1);
    // Not final while open.
    let ix = resolve_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).expect_code(agents_code(E::BondNotFinal));
    set_proposal(&mut aw, &pa, proposal_status::EXECUTED, 10, 0);
    set_proposal(&mut aw, &pb, proposal_status::EXECUTED, 10, 0);
    let ix = resolve_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).ok();
    let b: Bond = aw.hw.w.env.read(&bond);
    assert_eq!(b.status, bond_status::RATIFIED);
    // Only fees and rent of the bond, marks and nothing else left the key.
    let rent = aw.hw.w.env.rent(8 + <Bond as anchor_lang::Space>::INIT_SPACE)
        + 2 * aw.hw.w.env.rent(8 + <hookwars_agents::state::BondMark as anchor_lang::Space>::INIT_SPACE);
    assert!(before - aw.hw.w.env.lamports(&a.key.pubkey()) < rent + 100_000);
    assert_eq!(aw.passport(&a.passport).record.treaties_ratified, 1);
    // The items are not in either slot here (the proposals were set, not executed): broken.
    let ix = mark_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).ok();
    assert_eq!(aw.hw.w.env.read::<Bond>(&bond).status, bond_status::BROKEN);
    assert_eq!(aw.passport(&a.passport).record.treaties_broken, 1);
    let _ = ma;
}

#[test]
fn a_real_rejection_forfeits_into_both_inboxes_and_a_failed_quorum_returns() {
    let mut aw = Aw::new();
    let (a, ma, mb, item, pa, pb) = bonded_pair(&mut aw);
    let ix = post_bond_ix(&aw, &a, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).ok();
    let bond = apda::bond(&a.passport, &pa).0;
    // Failed with votes against above for, but under quorum: returned, no forfeit.
    set_proposal(&mut aw, &pa, proposal_status::FAILED, 0, 1);
    let before = aw.hw.w.env.lamports(&a.key.pubkey());
    let ix = resolve_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).ok();
    assert_eq!(aw.hw.w.env.read::<Bond>(&bond).status, bond_status::RETURNED);
    assert_eq!(aw.hw.w.env.lamports(&a.key.pubkey()) - before, TEST_AGENTS_PARAMS.bond_lamports);

    // A second pair: a real rejection on one side forfeits half into each inbox.
    let (b, ma2, mb2, item2, pa2, pb2) = bonded_pair(&mut aw);
    let ix = post_bond_ix(&aw, &b, &pa2, &pb2, &item2);
    aw.hw.w.env.send_paid_by(&[ix], &b.key, &[]).ok();
    let bond2 = apda::bond(&b.passport, &pa2).0;
    let supply: Mint = aw.hw.w.env.read(&mb2);
    set_proposal(&mut aw, &pb2, proposal_status::FAILED, 1, supply.supply);
    let (ia, ib) = (apda::treaty_inbox(&ma2), apda::treaty_inbox(&mb2));
    let (a0, b0) = (aw.hw.w.env.lamports(&ia), aw.hw.w.env.lamports(&ib));
    let ix = resolve_ix(&aw, &b, &bond2);
    aw.hw.w.env.send(&[ix], &[]).ok();
    assert_eq!(aw.hw.w.env.read::<Bond>(&bond2).status, bond_status::REJECTED);
    let half = TEST_AGENTS_PARAMS.bond_lamports / 2;
    assert_eq!(aw.hw.w.env.lamports(&ia) - a0, half);
    assert_eq!(aw.hw.w.env.lamports(&ib) - b0, TEST_AGENTS_PARAMS.bond_lamports - half);
    assert_eq!(aw.passport(&b.passport).record.bonds_forfeited, 1);
    // Resolved once.
    let ix = resolve_ix(&aw, &b, &bond2);
    aw.hw.w.env.send(&[ix], &[]).expect_code(agents_code(E::BadBondStatus));
    let _ = (ma, mb);
}

#[test]
fn a_held_treaty_counts_after_the_hold_time() {
    let mut aw = Aw::new();
    let (a, ma, mb, item, pa, pb) = bonded_pair(&mut aw);
    let ix = post_bond_ix(&aw, &a, &pa, &pb, &item);
    aw.hw.w.env.send_paid_by(&[ix], &a.key, &[]).ok();
    let bond = apda::bond(&a.passport, &pa).0;
    set_proposal(&mut aw, &pa, proposal_status::EXECUTED, 10, 0);
    set_proposal(&mut aw, &pb, proposal_status::EXECUTED, 10, 0);
    let ix = resolve_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).ok();
    // Put the treaty item in both slot tables (what the two executes would have done).
    for m in [ma, mb] {
        let mut acc = aw.hw.w.env.account(&m).expect("mint");
        let mut mint: Mint = anchor_lang::AccountDeserialize::try_deserialize(&mut &acc.data[..]).unwrap();
        mint.slots[0].item = item;
        let mut data = Vec::new();
        anchor_lang::AccountSerialize::try_serialize(&mint, &mut data).unwrap();
        acc.data[..data.len()].copy_from_slice(&data);
        aw.hw.w.env.put(m, acc);
    }
    let ix = mark_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).expect_code(agents_code(E::TooEarly));
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.treaty_hold_secs);
    let ix = mark_ix(&aw, &a, &bond);
    aw.hw.w.env.send(&[ix], &[]).ok();
    assert_eq!(aw.hw.w.env.read::<Bond>(&bond).status, bond_status::HELD);
    assert_eq!(aw.passport(&a.passport).record.treaties_held, 1);
}

// ---- config ------------------------------------------------------------------------------------

#[test]
fn config_changes_wait_for_the_timelock() {
    let mut aw = Aw::new();
    let mut args = aw.config_args();
    args.params.bond_lamports = 2 * SOL;
    let admin = aw.hw.admin.insecure_clone();
    let propose = |args| {
        agents_ix(
            hookwars_agents::accounts::ProposeConfig {
                admin: admin.pubkey(),
                config: apda::config().0,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::ProposeConfig { args },
        )
    };
    let stranger = aw.hw.w.env.funded(SOL);
    let mut ix = propose(args.clone());
    ix.accounts[0].pubkey = stranger.pubkey();
    aw.hw.w.env.send_paid_by(&[ix], &stranger, &[]).expect_code(agents_code(E::NotAdmin));
    aw.hw.w.env.send_paid_by(&[propose(args)], &admin, &[]).ok();
    let apply = agents_ix(
        hookwars_agents::accounts::ApplyConfig {
            config: apda::config().0,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::ApplyConfig {},
    );
    aw.hw.w.env.send(&[apply.clone()], &[]).expect_code(agents_code(E::TimelockActive));
    aw.hw.w.env.warp(TEST_AGENTS_PARAMS.admin_timelock_secs);
    aw.hw.w.env.send(&[apply], &[]).ok();
    let c: hookwars_agents::state::AgentsConfig = aw.hw.w.env.read(&apda::config().0);
    assert_eq!(c.params.bond_lamports, 2 * SOL);
}
