// Changed by Hookwars: new file (hook economy, docs/spec/11-hook-economy.md section 3.3).
//! Profiles and skills (R43). A `Profile` at `["profile", wallet]` keeps one counter per
//! `hookwars_common::economy::counter`. Counters move only through `record_wallet`, which accepts a
//! signer that is the `["social-caller"]` PDA of a protocol program named in the `SkillTable`
//! (admin set, behind `ADMIN_TIMELOCK_SECS`). A skill is a counter and its level thresholds; the
//! level is `hookwars_common::economy::level`, computed by whoever gates on it, never stored, so a
//! level cannot be bought (R43). A wallet without a profile is simply not counted: `record_wallet`
//! is a no-op for it, so callers never fail because of a missing profile.

use anchor_lang::prelude::*;
use hookwars_common::economy::{self as eco, SkillDef};

use crate::{seeds as social_seeds, SocialConfig, SocialError};

/// Seeds of this module.
pub mod pseeds {
    pub const PROFILE: &[u8] = b"profile";
    pub const SKILLS: &[u8] = b"skills";
}

/// Most skills a table holds (layout constant).
pub const SKILLS_CAP: usize = 8;
/// Most caller programs a table names (layout constant).
pub const CALLERS_CAP: usize = 8;

/// `Profile` at `["profile", wallet]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Profile {
    pub version: u8,
    pub bump: u8,
    pub wallet: Pubkey,
    pub counters: [u64; eco::counter::COUNT],
    pub created_at: i64,
    pub updated_at: i64,
    pub reserved: [u8; 32],
}

/// A pending skill table change.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, PartialEq, Eq)]
pub struct SkillChange {
    #[max_len(SKILLS_CAP)]
    pub skills: Vec<SkillDef>,
    #[max_len(CALLERS_CAP)]
    pub callers: Vec<Pubkey>,
    pub eta: i64,
}

/// `SkillTable` at `["skills"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct SkillTable {
    pub version: u8,
    pub bump: u8,
    #[max_len(SKILLS_CAP)]
    pub skills: Vec<SkillDef>,
    /// Programs whose `["social-caller"]` PDA may call `record_wallet`.
    #[max_len(CALLERS_CAP)]
    pub callers: Vec<Pubkey>,
    pub pending: Option<SkillChange>,
}

impl SkillTable {
    /// The level of `wallet_counters` in skill `id`; 0 for an unknown skill.
    pub fn level(&self, id: u8, counters: &[u64; eco::counter::COUNT]) -> u8 {
        self.skills
            .iter()
            .find(|s| s.id == id)
            .map_or(0, |s| eco::level(&s.thresholds, counters[usize::from(s.counter)]))
    }
}

/// Profile errors (offset away from `SocialError`).
#[error_code(offset = 7100)]
pub enum ProfileError {
    #[msg("a skill is malformed (counter out of range or thresholds not ascending)")]
    BadSkill,
    #[msg("too many skills or callers")]
    TooMany,
    #[msg("the signer is not a registered caller's PDA")]
    NotCaller,
    #[msg("no pending skill change, or not ready yet")]
    NotReady,
    #[msg("unknown counter")]
    BadCounter,
    #[msg("two skills share an id")]
    DuplicateSkill,
}

#[event]
pub struct ProfileOpened {
    pub wallet: Pubkey,
    pub ts: i64,
}

#[event]
pub struct WalletRecorded {
    pub wallet: Pubkey,
    pub caller_program: Pubkey,
    pub counter: u8,
    pub value: u64,
    pub total: u64,
    pub ts: i64,
}

#[event]
pub struct SkillsProposed {
    pub eta: i64,
}

#[event]
pub struct SkillsApplied {
    pub skills: u8,
    pub callers: u8,
    pub ts: i64,
}

/// Checks a skill list and a caller list.
pub fn check_skills(skills: &[SkillDef], callers: &[Pubkey]) -> Result<()> {
    require!(
        skills.len() <= SKILLS_CAP && callers.len() <= CALLERS_CAP,
        ProfileError::TooMany
    );
    for (i, s) in skills.iter().enumerate() {
        require!(s.valid(), ProfileError::BadSkill);
        require!(
            !skills[..i].iter().any(|o| o.id == s.id),
            ProfileError::DuplicateSkill
        );
    }
    Ok(())
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

pub fn process_open_profile(ctx: Context<OpenProfile>, wallet: Pubkey) -> Result<()> {
    let t = now()?;
    let p = &mut ctx.accounts.profile;
    p.version = crate::VERSION;
    p.bump = ctx.bumps.profile;
    p.wallet = wallet;
    p.counters = [0; eco::counter::COUNT];
    p.created_at = t;
    p.updated_at = t;
    p.reserved = [0; 32];
    emit_cpi!(ProfileOpened { wallet, ts: t });
    Ok(())
}

pub fn process_init_skills(ctx: Context<InitSkills>, skills: Vec<SkillDef>, callers: Vec<Pubkey>) -> Result<()> {
    check_skills(&skills, &callers)?;
    let s = &mut ctx.accounts.skills;
    s.version = crate::VERSION;
    s.bump = ctx.bumps.skills;
    s.skills = skills;
    s.callers = callers;
    s.pending = None;
    Ok(())
}

pub fn process_propose_skills(
    ctx: Context<ProposeSkills>,
    skills: Vec<SkillDef>,
    callers: Vec<Pubkey>,
) -> Result<()> {
    check_skills(&skills, &callers)?;
    let eta = now()?
        .checked_add(i64::from(ctx.accounts.config.params.admin_timelock_secs))
        .ok_or(SocialError::Overflow)?;
    ctx.accounts.skills.pending = Some(SkillChange { skills, callers, eta });
    emit_cpi!(SkillsProposed { eta });
    Ok(())
}

pub fn process_apply_skills(ctx: Context<ApplySkills>) -> Result<()> {
    let t = now()?;
    let s = &mut ctx.accounts.skills;
    let p = s.pending.clone().ok_or(ProfileError::NotReady)?;
    require!(t >= p.eta, ProfileError::NotReady);
    s.skills = p.skills;
    s.callers = p.callers;
    s.pending = None;
    let (n, c) = (s.skills.len() as u8, s.callers.len() as u8);
    emit_cpi!(SkillsApplied { skills: n, callers: c, ts: t });
    Ok(())
}

pub fn process_record_wallet(
    ctx: Context<RecordWallet>,
    caller_program: Pubkey,
    counter: u8,
    value: u64,
) -> Result<()> {
    require!(usize::from(counter) < eco::counter::COUNT, ProfileError::BadCounter);
    require!(
        ctx.accounts.skills.callers.contains(&caller_program)
            && ctx.accounts.caller.key()
                == eco::caller_pda(eco::SOCIAL_CALLER_SEED, &caller_program).0,
        ProfileError::NotCaller
    );
    let info = ctx.accounts.profile.to_account_info();
    // A wallet without a profile is not counted (never a failure for the caller).
    if *info.owner != crate::ID || info.data_is_empty() {
        return Ok(());
    }
    let mut p = {
        let data = info.try_borrow_data()?;
        match Profile::try_deserialize(&mut &data[..]) {
            Ok(p) => p,
            Err(_) => return Ok(()),
        }
    };
    require_keys_eq!(
        *info.key,
        Pubkey::create_program_address(&[pseeds::PROFILE, p.wallet.as_ref(), &[p.bump]], &crate::ID)
            .map_err(|_| error!(SocialError::WrongAccount))?,
        SocialError::WrongAccount
    );
    let i = usize::from(counter);
    p.counters[i] = p.counters[i].saturating_add(value);
    let t = now()?;
    p.updated_at = t;
    let total = p.counters[i];
    let wallet = p.wallet;
    {
        let mut data = info.try_borrow_mut_data()?;
        let mut w: &mut [u8] = &mut data[..];
        p.try_serialize(&mut w)?;
    }
    emit_cpi!(WalletRecorded {
        wallet,
        caller_program,
        counter,
        value,
        total,
        ts: t
    });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(wallet: Pubkey)]
pub struct OpenProfile<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + Profile::INIT_SPACE,
        seeds = [pseeds::PROFILE, wallet.as_ref()], bump)]
    pub profile: Box<Account<'info, Profile>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitSkills<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [social_seeds::CONFIG], bump = config.bump, has_one = admin @ SocialError::NotAdmin)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(init, payer = admin, space = 8 + SkillTable::INIT_SPACE, seeds = [pseeds::SKILLS], bump)]
    pub skills: Box<Account<'info, SkillTable>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeSkills<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [social_seeds::CONFIG], bump = config.bump, has_one = admin @ SocialError::NotAdmin)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(mut, seeds = [pseeds::SKILLS], bump = skills.bump)]
    pub skills: Box<Account<'info, SkillTable>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ApplySkills<'info> {
    #[account(mut, seeds = [pseeds::SKILLS], bump = skills.bump)]
    pub skills: Box<Account<'info, SkillTable>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct RecordWallet<'info> {
    /// The `["social-caller"]` PDA of a registered protocol program.
    pub caller: Signer<'info>,
    #[account(seeds = [pseeds::SKILLS], bump = skills.bump)]
    pub skills: Box<Account<'info, SkillTable>>,
    /// CHECK: the wallet's profile, or any account when the wallet has none (then a no-op).
    #[account(mut)]
    pub profile: UncheckedAccount<'info>,
}

/// `["profile", wallet]`.
pub fn profile_address(wallet: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[pseeds::PROFILE, wallet.as_ref()], &crate::ID)
}

/// `["skills"]`.
pub fn skills_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[pseeds::SKILLS], &crate::ID)
}

/// Reads a profile owned by this program at the address of `wallet`; `None` when absent.
pub fn read_profile(info: &AccountInfo, wallet: &Pubkey) -> Result<Option<Profile>> {
    require_keys_eq!(*info.key, profile_address(wallet).0, SocialError::WrongAccount);
    if *info.owner != crate::ID || info.data_is_empty() {
        return Ok(None);
    }
    let data = info.try_borrow_data()?;
    Ok(Some(Profile::try_deserialize(&mut &data[..])?))
}

/// Reads the skill table (owner and address checked).
pub fn read_skills(info: &AccountInfo) -> Result<SkillTable> {
    require_keys_eq!(*info.key, skills_address().0, SocialError::WrongAccount);
    require_keys_eq!(*info.owner, crate::ID, SocialError::WrongAccount);
    let data = info.try_borrow_data()?;
    SkillTable::try_deserialize(&mut &data[..])
}

/// The level of `wallet` in skill `skill` from a profile account (or 0 without a profile).
pub fn wallet_level(profile: &AccountInfo, skills: &AccountInfo, wallet: &Pubkey, skill: u8) -> Result<u8> {
    let table = read_skills(skills)?;
    Ok(match read_profile(profile, wallet)? {
        Some(p) => table.level(skill, &p.counters),
        None => 0,
    })
}
