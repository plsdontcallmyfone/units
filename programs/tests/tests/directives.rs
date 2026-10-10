// Changed by Hookwars: new file, directives, commits and postage (docs/spec/11-hook-economy.md
// section 4, R41).
//! A directive transaction binds the sha256 of its memo and writes its limits into the policy
//! wallet, where `spend` enforces them even if the runtime ignores the words; a missing,
//! mismatched or unsigned memo is refused; sequences chain and supersede; `commit` and `post`
//! round trip; memo parameters wait for the timelock.

use anchor_lang::prelude::{AccountMeta, Pubkey};
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::InstructionData;
use bordrless_program_tests::agents::*;
use bordrless_token::client as token;
use hookwars_agents::constants::{kind, pda as apda, sha256, INSTRUCTIONS_SYSVAR_ID};
use hookwars_agents::error::AgentsError as A;
use hookwars_agents::handlers::directive::{
    commitment_address, directive_address, memo_config_address, Commitment, Directive, DirectiveConstraints,
    DirectiveError as D, MemoConfig, MemoParams,
};
use hookwars_agents::state::{Policy, PolicyLimits};
use hookwars_common::economy::MEMO_PROGRAM_ID;
use hookwars_common::ids;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SOL: u64 = 1_000_000_000;

/// TEST memo parameters.
const TEST_MEMO: MemoParams = MemoParams {
    memo_max_bytes: 600,
    postage_lamports: 5_000,
    min_proof: 1,
};

fn dcode(e: D) -> u32 {
    anchor_lang::error::ERROR_CODE_OFFSET + e as u32
}

fn memo_ix(signer: &Pubkey, text: &str) -> Instruction {
    Instruction {
        program_id: MEMO_PROGRAM_ID,
        accounts: vec![AccountMeta::new_readonly(*signer, true)],
        data: text.as_bytes().to_vec(),
    }
}

fn directive_text(passport: &Pubkey, seq: u64, c: &DirectiveConstraints) -> String {
    let ch = hookwars_agents::handlers::directive::constraints_hash_hex(c).unwrap();
    units_memo::directive_message(&passport.to_string(), seq, "https://example.invalid/rules.md", "00ff", &ch).encode()
}

struct Dw {
    aw: Aw,
    a: Agent,
}

/// An agent with a policy (targets: the token program) and the memo config.
fn world() -> Dw {
    let mut aw = Aw::new();
    let admin = aw.hw.admin.insecure_clone();
    let ix = agents_ix(
        hookwars_agents::accounts::InitMemoConfig {
            admin: admin.pubkey(),
            config: apda::config().0,
            memo_config: memo_config_address().0,
            system_program: anchor_lang::system_program::ID,
        },
        hookwars_agents::instruction::InitMemoConfig { params: TEST_MEMO },
    );
    aw.hw.w.env.send_paid_by(&[ix], &admin, &[]).ok();
    let a = aw.agent("director", kind::AUTHOR);
    let limits = PolicyLimits {
        per_action_lamports: SOL,
        per_day_lamports: SOL,
        tracked: vec![],
        targets: vec![bordrless_token::ID],
    };
    let ix = aw.init_policy_ix(&a, limits);
    aw.hw.w.env.send_paid_by(&[ix], &a.operator, &[]).ok();
    aw.hw.w.env.fund(apda::vault(&a.passport).0, SOL);
    Dw { aw, a }
}

fn set_directive_ix(dw: &Dw, seq: u32, c: DirectiveConstraints) -> Instruction {
    let p = dw.a.passport;
    agents_ix(
        hookwars_agents::accounts::SetDirective {
            operator: dw.a.operator.pubkey(),
            config: apda::config().0,
            memo_config: memo_config_address().0,
            passport: p,
            policy: apda::policy(&p).0,
            directive: directive_address(&p, seq).0,
            previous: if seq == 0 { None } else { Some(directive_address(&p, seq - 1).0) },
            instructions: INSTRUCTIONS_SYSVAR_ID,
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::SetDirective { seq, constraints: c },
    )
}

fn constraints(per_action: u64) -> DirectiveConstraints {
    DirectiveConstraints {
        max_spend_per_action: per_action,
        max_spend_per_day: 10 * per_action,
        allowed_targets: vec![bordrless_token::ID],
        allowed_access_modes: 0b1,
        max_licence_price: SOL,
        frozen: false,
    }
}

fn send(dw: &mut Dw, ixs: &[Instruction]) -> bordrless_program_tests::Tx {
    let op = dw.a.operator.insecure_clone();
    dw.aw.hw.w.env.send_paid_by(ixs, &op, &[])
}

/// The vault pays a holding's rent through `spend` (a lamport outflow the policy measures).
fn spend_rent(dw: &mut Dw) -> bordrless_program_tests::Tx {
    let vault = apda::vault(&dw.a.passport).0;
    let funder = dw.aw.hw.w.env.funded(10 * SOL);
    let mint = dw.aw.hw.w.mint_to_owner(&funder, 6, 1_000, "RNT");
    let owner = Pubkey::new_unique();
    let inner = token::create_holding(vault, mint, owner);
    let ix = dw.aw.spend_ix(&dw.a, inner);
    let key = dw.a.key.insecure_clone();
    dw.aw.hw.w.env.send_paid_by(&[ix], &key, &[])
}

#[test]
fn a_directive_binds_its_memo_hash_and_writes_the_policy() {
    let mut dw = world();
    let c = constraints(1_000);
    let text = directive_text(&dw.a.passport, 0, &c);
    let op = dw.a.operator.pubkey();
    let ixs = [memo_ix(&op, &text), set_directive_ix(&dw, 0, c.clone())];
    send(&mut dw, &ixs).ok();
    let d: Directive = dw.aw.hw.w.env.read(&directive_address(&dw.a.passport, 0).0);
    assert_eq!(d.memo_hash, sha256(text.as_bytes()));
    assert_eq!((d.seq, d.superseded_by, d.constraints.clone()), (0, None, c));
    let p: Policy = dw.aw.hw.w.env.read(&apda::policy(&dw.a.passport).0);
    assert_eq!((p.per_action_lamports, p.per_day_lamports, p.frozen), (1_000, 10_000, false));
    assert_eq!(p.targets, vec![bordrless_token::ID]);
    // The memo may come after the instruction in the same transaction.
    let text = directive_text(&dw.a.passport, 1, &constraints(2_000));
    let ixs = [set_directive_ix(&dw, 1, constraints(2_000)), memo_ix(&op, &text)];
    send(&mut dw, &ixs).ok();
    let d0: Directive = dw.aw.hw.w.env.read(&directive_address(&dw.a.passport, 0).0);
    assert_eq!(d0.superseded_by, Some(1));
}

#[test]
fn missing_mismatched_and_unsigned_memos_are_refused() {
    let mut dw = world();
    let op = dw.a.operator.pubkey();
    // No memo at all.
    let ix = set_directive_ix(&dw, 0, constraints(1_000));
    send(&mut dw, &[ix]).expect_code(dcode(D::DirectiveMemoMissing));
    // A directive memo for another sequence.
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 3, &constraints(1_000))), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).expect_code(dcode(D::MemoMismatch));
    // A directive memo for another passport.
    let ixs = [memo_ix(&op, &directive_text(&Pubkey::new_unique(), 0, &constraints(1_000))), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).expect_code(dcode(D::MemoMismatch));
    // A non-canonical memo (keys out of order) is not a directive.
    let text = directive_text(&dw.a.passport, 0, &constraints(1_000)).replacen("{\"u\":1,\"k\"", "{\"k\"", 1);
    let ixs = [memo_ix(&op, &text), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).expect_code(dcode(D::DirectiveMemoMissing));
    // A memo the operator did not sign (signed by a stranger).
    let stranger = dw.aw.hw.w.env.funded(SOL);
    let ixs = [memo_ix(&stranger.pubkey(), &directive_text(&dw.a.passport, 0, &constraints(1_000))), set_directive_ix(&dw, 0, constraints(1_000))];
    let o = dw.a.operator.insecure_clone();
    dw.aw.hw.w.env.send_paid_by(&ixs, &o, &[&stranger]).expect_code(dcode(D::MemoNotSigned));
    // Over MEMO_MAX_BYTES: not read.
    let ch = hookwars_agents::handlers::directive::constraints_hash_hex(&constraints(1_000)).unwrap();
    let base = units_memo::directive_message(&dw.a.passport.to_string(), 0, "", "00", &ch).encode();
    let pad = usize::from(TEST_MEMO.memo_max_bytes) + 1 - base.len();
    let long = units_memo::directive_message(&dw.a.passport.to_string(), 0, &"x".repeat(pad), "00", &ch).encode();
    assert_eq!(long.len(), usize::from(TEST_MEMO.memo_max_bytes) + 1);
    let ixs = [memo_ix(&op, &long), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).expect_code(dcode(D::DirectiveMemoMissing));
    // Only the operator sets directives.
    let mut ix = set_directive_ix(&dw, 0, constraints(1_000));
    ix.accounts[0].pubkey = stranger.pubkey();
    let ixs = [memo_ix(&stranger.pubkey(), &directive_text(&dw.a.passport, 0, &constraints(1_000))), ix];
    dw.aw.hw.w.env.send_paid_by(&ixs, &stranger, &[]).expect_code(agents_code(A::NotOperator));
}

/// Pass 5 (review 3 L-6): the memo commits to the constraints the call writes; a memo whose `c`
/// names other limits (here a looser per-action cap) is refused.
#[test]
fn p5_l6_the_memo_commits_to_the_enforced_constraints() {
    let mut dw = world();
    let op = dw.a.operator.pubkey();
    let shown = constraints(1_000);
    let written = constraints(SOL);
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 0, &shown)), set_directive_ix(&dw, 0, written.clone())];
    send(&mut dw, &ixs).expect_code(dcode(D::MemoMismatch));
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 0, &written)), set_directive_ix(&dw, 0, written)];
    send(&mut dw, &ixs).ok();
}

#[test]
fn sequences_chain_and_cannot_skip() {
    let mut dw = world();
    let op = dw.a.operator.pubkey();
    // The first must be 0.
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 1, &constraints(1_000))), set_directive_ix(&dw, 1, constraints(1_000))];
    send(&mut dw, &ixs).expect_fail();
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 0, &constraints(1_000))), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).ok();
    // Replaying 0 fails (the account exists); skipping to 2 fails (no 1).
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 0, &constraints(1_000))), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).expect_fail();
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 2, &constraints(1_000))), set_directive_ix(&dw, 2, constraints(1_000))];
    send(&mut dw, &ixs).expect_fail();
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 1, &constraints(1_000))), set_directive_ix(&dw, 1, constraints(1_000))];
    send(&mut dw, &ixs).ok();
    // A live directive reads as live; a superseded one does not.
    let d1 = dw.aw.hw.w.env.account(&directive_address(&dw.a.passport, 1).0).unwrap();
    let d0: Directive = dw.aw.hw.w.env.read(&directive_address(&dw.a.passport, 0).0);
    assert_eq!(d0.superseded_by, Some(1));
    assert_eq!(d1.owner, ids::AGENTS_ID);
}

#[test]
fn directive_limits_hold_in_the_policy_wallet() {
    let mut dw = world();
    let op = dw.a.operator.pubkey();
    // Rent of a holding is far above 1,000 lamports: refused.
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 0, &constraints(1_000))), set_directive_ix(&dw, 0, constraints(1_000))];
    send(&mut dw, &ixs).ok();
    spend_rent(&mut dw).expect_code(agents_code(A::LimitExceeded));
    // Raised by the next directive: allowed.
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 1, &constraints(SOL / 10))), set_directive_ix(&dw, 1, constraints(SOL / 10))];
    send(&mut dw, &ixs).ok();
    spend_rent(&mut dw).ok();
    // A directive with no targets: refused by target.
    let mut c = constraints(SOL / 10);
    c.allowed_targets = vec![];
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 2, &c)), set_directive_ix(&dw, 2, c)];
    send(&mut dw, &ixs).ok();
    spend_rent(&mut dw).expect_code(agents_code(A::TargetNotAllowed));
    // Frozen by directive.
    let mut c = constraints(SOL / 10);
    c.frozen = true;
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 3, &c)), set_directive_ix(&dw, 3, c)];
    send(&mut dw, &ixs).ok();
    spend_rent(&mut dw).expect_code(agents_code(A::PolicyFrozen));
    // A target the config does not allow is refused at the directive.
    let mut c = constraints(SOL / 10);
    c.allowed_targets = vec![ids::BRIDGE_ID];
    let ixs = [memo_ix(&op, &directive_text(&dw.a.passport, 4, &c)), set_directive_ix(&dw, 4, c)];
    send(&mut dw, &ixs).expect_code(agents_code(A::TargetNotAllowed));
}

#[test]
fn commit_and_post_round_trip() {
    let mut dw = world();
    let reference = [4u8; 32];
    let hash = sha256(b"accepted offer");
    let key = dw.a.key.insecure_clone();
    let commit = |dw: &Dw, signer: &Pubkey| {
        agents_ix(
            hookwars_agents::accounts::Commit {
                agent: *signer,
                passport: dw.a.passport,
                commitment: commitment_address(&dw.a.passport, &reference).0,
                system_program: anchor_lang::system_program::ID,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::Commit { reference, hash },
        )
    };
    // Only the agent key commits.
    let op = dw.a.operator.insecure_clone();
    let ix = commit(&dw, &op.pubkey());
    dw.aw.hw.w.env.send_paid_by(&[ix], &op, &[]).expect_code(dcode(D::NotAgent));
    let ix = commit(&dw, &key.pubkey());
    dw.aw.hw.w.env.send_paid_by(&[ix], &key, &[]).ok();
    let c: Commitment = dw.aw.hw.w.env.read(&commitment_address(&dw.a.passport, &reference).0);
    assert_eq!((c.passport, c.reference, c.hash), (dw.a.passport, reference, hash));
    // Postage goes to the fee collector.
    let fee = dw.aw.fee_collector;
    let f0 = dw.aw.hw.w.env.lamports(&fee);
    let ix = agents_ix(
        hookwars_agents::accounts::Post {
            agent: key.pubkey(),
            passport: dw.a.passport,
            config: apda::config().0,
            memo_config: memo_config_address().0,
            fee_collector: fee,
            system_program: anchor_lang::system_program::ID,
            event_authority: agents_events(),
            program: ids::AGENTS_ID,
        },
        hookwars_agents::instruction::Post { reference },
    );
    dw.aw.hw.w.env.send_paid_by(&[ix], &key, &[]).ok();
    assert_eq!(dw.aw.hw.w.env.lamports(&fee) - f0, TEST_MEMO.postage_lamports);
}

#[test]
fn memo_parameters_wait_for_the_timelock() {
    let mut dw = world();
    let admin = dw.aw.hw.admin.insecure_clone();
    let mut p = TEST_MEMO;
    p.postage_lamports = 9_000;
    let propose = |p: MemoParams| {
        agents_ix(
            hookwars_agents::accounts::ProposeMemoConfig {
                admin: admin.pubkey(),
                config: apda::config().0,
                memo_config: memo_config_address().0,
                event_authority: agents_events(),
                program: ids::AGENTS_ID,
            },
            hookwars_agents::instruction::ProposeMemoConfig { params: p },
        )
    };
    let mut bad = p;
    bad.memo_max_bytes = 0;
    dw.aw.hw.w.env.send_paid_by(&[propose(bad)], &admin, &[]).expect_code(dcode(D::BadMemoParams));
    dw.aw.hw.w.env.send_paid_by(&[propose(p)], &admin, &[]).ok();
    let apply = Instruction {
        program_id: ids::AGENTS_ID,
        accounts: anchor_lang::ToAccountMetas::to_account_metas(
            &hookwars_agents::accounts::ApplyMemoConfig { memo_config: memo_config_address().0 },
            None,
        ),
        data: hookwars_agents::instruction::ApplyMemoConfig {}.data(),
    };
    dw.aw.hw.w.env.send_paid_by(std::slice::from_ref(&apply), &admin, &[]).expect_code(agents_code(A::TimelockActive));
    dw.aw.hw.w.env.warp(600);
    dw.aw.hw.w.env.send_paid_by(&[apply], &admin, &[]).ok();
    let c: MemoConfig = dw.aw.hw.w.env.read(&memo_config_address().0);
    assert_eq!(c.params.postage_lamports, 9_000);
    let _ = Keypair::new();
}
