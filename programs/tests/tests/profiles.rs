// Changed by Hookwars: new file, profiles and skills (docs/spec/11-hook-economy.md section 3.3,
// R43).
//! Counters move only through `record_wallet` from a registered program's `["social-caller"]`
//! PDA; a wallet without a profile is simply not counted (never a failure for the caller); levels
//! are computed from the skill table, which changes only behind the timelock.

use anchor_lang::prelude::Pubkey;
use bordrless_program_tests::economy::*;
use bordrless_program_tests::expansion::{events, social_config, social_ix};
use hookwars_common::economy::{self as eco, counter, skill, SkillDef, MAX_LEVELS};
use hookwars_social::{profile_address, skills_address, ProfileError as P, SkillTable};
use solana_signer::Signer;

fn pcode(e: P) -> u32 {
    7_100 + e as u32
}

#[test]
fn counters_move_only_from_registered_caller_pdas() {
    let mut ew = Ew::new();
    let alice = ew.funded(SOL);
    ew.open_profile(&alice);
    let p = ew.profile(&alice.pubkey());
    assert_eq!((p.wallet, p.counters), (alice.pubkey(), [0; eco::counter::COUNT]));
    // Anyone may open a profile for any wallet, once.
    let payer = ew.funded(SOL);
    let ix = ew.open_profile_ix(&payer.pubkey(), &alice.pubkey());
    ew.send(&payer, &[ix]).expect_fail();
    // The stub is registered: its caller PDA counts.
    ew.stub_record(&alice.pubkey(), counter::ITEMS_CRAFTED, 2).ok();
    ew.stub_record(&alice.pubkey(), counter::ITEMS_CRAFTED, 1).ok();
    assert_eq!(ew.profile(&alice.pubkey()).counters[usize::from(counter::ITEMS_CRAFTED)], 3);
    // A wallet signing for itself, naming a registered program, is refused.
    let ix = ew.record_wallet_ix(&alice.pubkey(), &STUB, &alice.pubkey(), counter::ITEMS_CRAFTED, 100);
    ew.send(&alice, &[ix]).expect_code(pcode(P::NotCaller));
    // An unregistered program's PDA (here: alice naming herself as the program) is refused.
    let ix = ew.record_wallet_ix(&alice.pubkey(), &alice.pubkey(), &alice.pubkey(), counter::ITEMS_CRAFTED, 100);
    ew.send(&alice, &[ix]).expect_code(pcode(P::NotCaller));
    // Unknown counter.
    ew.stub_record(&alice.pubkey(), eco::counter::COUNT as u8, 1).expect_code(pcode(P::BadCounter));
    // No profile: a no-op that succeeds.
    let bob = Pubkey::new_unique();
    ew.stub_record(&bob, counter::ITEMS_CRAFTED, 5).ok();
    assert!(ew.hw.w.env.account(&profile_address(&bob).0).is_none());
    // Levels are computed: Crafter thresholds 1, 3, 10 (TEST).
    let t: SkillTable = ew.hw.w.env.read(&skills_address().0);
    let c = ew.profile(&alice.pubkey()).counters;
    assert_eq!(t.level(skill::CRAFTER, &c), 2);
    assert_eq!(t.level(skill::DIPLOMAT, &c), 0);
    ew.stub_record(&alice.pubkey(), counter::ITEMS_CRAFTED, 7).ok();
    let c = ew.profile(&alice.pubkey()).counters;
    assert_eq!(t.level(skill::CRAFTER, &c), 3);
}

fn skills_ix(ew: &Ew, propose: bool, skills: Vec<SkillDef>, callers: Vec<Pubkey>) -> anchor_lang::solana_program::instruction::Instruction {
    let d = ew.hw.w.env.deployer.pubkey();
    if propose {
        social_ix(
            hookwars_social::accounts::ProposeSkills {
                admin: d,
                config: social_config(),
                skills: skills_address().0,
                event_authority: events(&hookwars_social::ID),
                program: hookwars_social::ID,
            },
            hookwars_social::instruction::ProposeSkills { skills, callers },
        )
    } else {
        social_ix(
            hookwars_social::accounts::ApplySkills {
                skills: skills_address().0,
                event_authority: events(&hookwars_social::ID),
                program: hookwars_social::ID,
            },
            hookwars_social::instruction::ApplySkills {},
        )
    }
}

#[test]
fn the_skill_table_changes_only_behind_the_timelock() {
    let mut ew = Ew::new();
    let d = ew.deployer();
    let mut th = [0u64; MAX_LEVELS];
    th[0] = 2;
    let diplomat = SkillDef { id: skill::DIPLOMAT, counter: counter::TREATIES_HELD, thresholds: th };
    // Malformed tables are refused.
    let bad = SkillDef { id: 9, counter: 99, thresholds: th };
    let ix = skills_ix(&ew, true, vec![bad], vec![]);
    ew.send(&d, &[ix]).expect_code(pcode(P::BadSkill));
    let ix = skills_ix(&ew, true, vec![diplomat, diplomat], vec![]);
    ew.send(&d, &[ix]).expect_code(pcode(P::DuplicateSkill));
    // A stranger cannot propose.
    let stranger = ew.funded(SOL);
    let mut ix = skills_ix(&ew, true, vec![diplomat], vec![]);
    ix.accounts[0].pubkey = stranger.pubkey();
    ew.send(&stranger, &[ix]).expect_fail();
    // Proposed: the old table holds until the delay; the stub stays a caller until then.
    let ix = skills_ix(&ew, true, vec![diplomat], vec![]);
    ew.send(&d, &[ix]).ok();
    let ix = skills_ix(&ew, false, vec![], vec![]);
    ew.send(&d, &[ix]).expect_code(pcode(P::NotReady));
    let alice = ew.funded(SOL);
    ew.open_profile(&alice);
    ew.stub_record(&alice.pubkey(), counter::TREATIES_HELD, 2).ok();
    ew.warp(600);
    let ix = skills_ix(&ew, false, vec![], vec![]);
    ew.send(&d, &[ix]).ok();
    let t: SkillTable = ew.hw.w.env.read(&skills_address().0);
    assert_eq!(t.skills, vec![diplomat]);
    assert_eq!(t.level(skill::DIPLOMAT, &ew.profile(&alice.pubkey()).counters), 1);
    // No callers now: the stub no longer counts.
    ew.stub_record(&alice.pubkey(), counter::TREATIES_HELD, 1).expect_code(pcode(P::NotCaller));
}
