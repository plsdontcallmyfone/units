// Changed by Hookwars: new program (expansion, docs/spec/10-expansion.md sections 3, 6, 7).
// Integration pass 2: SpendToken through transfer_from_protocol (I-1), ItemsAuthored and
// RoyaltiesClaimed criteria (I-5), RaidPoints reads a Mercenary range too.
// Economy (11): profiles and skills (11 section 3.3).
//! `hookwars_social`: achievement badges and guild halls.
//!
//! **Badges** are units tokens that do not move. Spec 10 makes them soulbound with template 42
//! Soulbound in a Locked slot (built on the agents branch). Until that lands, a badge mint here is a
//! plain units mint whose mint and freeze authority is this program's `["badge-minter"]`, and every
//! awarded holding is frozen right after the mint: the token program refuses any transfer from or
//! to a frozen holding, so a badge cannot move (and cannot be burned) either way. The switch to the
//! Soulbound slot is an integration request in 10. Criteria are on-chain facts read in O(1):
//! a token's first siege, a wallet's raid points this season, a wallet's forge level. Badges carry
//! no money and no vote.
//!
//! **Guild halls** give a guild a member-facing treasury held by a system-owned PDA and spent only
//! by a threshold of its officers after `guild_timelock_secs`. This is custody by the officers, not
//! by the protocol (10 6.4): the protocol never routes fees here.
//!
//! **Agent leagues** (10 section 7) are a site table by default (no prize), so nothing is on chain.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::system_program;

declare_id!("CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx");

pub mod profiles;
pub use profiles::*;

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "units social",
    project_url: "https://github.com/plsdontcallmyfone/units",
    contacts: "link:https://github.com/plsdontcallmyfone/units/security/advisories/new",
    policy: "https://github.com/plsdontcallmyfone/units/blob/main/SECURITY.md",
    source_code: "https://github.com/plsdontcallmyfone/units"
}

/// Layout version.
pub const VERSION: u8 = 1;
/// Longest badge or guild name (layout constant).
pub const NAME_MAX_LEN: usize = 32;
/// Most officers a guild may list (layout bound; `guild_max_officers` must be at most this).
pub const GUILD_OFFICERS_CAP: usize = 10;

/// Seeds.
pub mod seeds {
    pub const CONFIG: &[u8] = b"social-config";
    pub const PENDING: &[u8] = b"social-pending";
    pub const BADGE: &[u8] = b"badge";
    pub const BADGE_MINT: &[u8] = b"badge-mint";
    pub const AWARD: &[u8] = b"award";
    pub const MINTER: &[u8] = b"badge-minter";
    pub const GUILD: &[u8] = b"guild";
    pub const TREASURY: &[u8] = b"guild-treasury";
    pub const ACTION: &[u8] = b"guild-action";
}

/// Parameters (10 section 13; all to set).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SocialParams {
    /// `GUILD_MAX_OFFICERS`.
    pub guild_max_officers: u8,
    /// `GUILD_TIMELOCK_SECS`: delay between a guild action's proposal and its execution.
    pub guild_timelock_secs: u32,
    /// `ADMIN_TIMELOCK_SECS` (00): delay on params changes and before a new badge can be claimed.
    pub admin_timelock_secs: u32,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct SocialConfig {
    pub version: u8,
    pub bump: u8,
    pub admin: Pubkey,
    pub params: SocialParams,
    /// Badge types created (the next id).
    pub badges: u32,
    /// Guilds created (the next id).
    pub guilds: u32,
    pub reserved: [u8; 32],
}

#[account]
#[derive(InitSpace, Debug)]
pub struct PendingSocialParams {
    pub bump: u8,
    pub params: SocialParams,
    pub ready_at: i64,
    pub active: bool,
}

/// What earns a badge: on-chain facts only, each read in O(1) (10 section 3.2). Criteria that would
/// need history the chain does not keep are not badges.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Criterion {
    /// The token has besieged a rival (its war chest has spent on a siege). Awarded to the token's
    /// war chest (shown on the token page): the chain keeps no per-wallet siege record.
    FirstSiege { mint: Pubkey },
    /// The wallet holds at least `min` raid points this season in `mint`'s Raid item range.
    RaidPoints { mint: Pubkey, min: u32 },
    /// The wallet holds an item of at least this level.
    ForgeLevel { min_level: u8 },
    /// Integration pass 2 (10 section 17 I-5): the wallet authored at least `min` items (its
    /// armory `AuthorCounter`).
    ItemsAuthored { min: u64 },
    /// Integration pass 2 (I-5): the wallet claimed at least `min_lamports` of bridged-SOL royalty
    /// (its armory `ClaimCounter`).
    RoyaltiesClaimed { min_lamports: u64 },
}

/// `BadgeType` at `["badge", id]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct BadgeType {
    pub version: u8,
    pub bump: u8,
    pub mint_bump: u8,
    pub id: u32,
    pub mint: Pubkey,
    #[max_len(NAME_MAX_LEN)]
    pub name: String,
    pub criterion: Criterion,
    /// Claims open at this time (`admin_timelock_secs` after creation).
    pub claims_open_at: i64,
    pub awarded: u64,
    pub created_at: i64,
}

/// `BadgeAward` at `["award", badge_id, recipient]`: one badge of a type per recipient.
#[account]
#[derive(InitSpace, Debug)]
pub struct BadgeAward {
    pub bump: u8,
    pub badge_id: u32,
    pub recipient: Pubkey,
    pub awarded_at: i64,
}

/// `Guild` at `["guild", id]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Guild {
    pub version: u8,
    pub bump: u8,
    pub treasury_bump: u8,
    pub id: u32,
    #[max_len(NAME_MAX_LEN)]
    pub name: String,
    #[max_len(GUILD_OFFICERS_CAP)]
    pub officers: Vec<Pubkey>,
    pub threshold: u8,
    /// Changes whenever the officers change; pending actions of an older version cannot execute.
    pub officers_version: u32,
    /// Actions proposed (the next nonce).
    pub actions: u64,
    pub created_at: i64,
}

/// What a guild action does.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, PartialEq, Eq)]
pub enum GuildActionKind {
    /// Pays SOL from the treasury.
    SpendSol { to: Pubkey, lamports: u64 },
    /// Moves units tokens from the treasury's holding of `mint` to `to`'s holding.
    SpendToken { mint: Pubkey, to: Pubkey, amount: u64 },
    /// Replaces the officers and the threshold.
    SetOfficers {
        #[max_len(GUILD_OFFICERS_CAP)]
        officers: Vec<Pubkey>,
        threshold: u8,
    },
}

/// `GuildAction` at `["guild-action", guild_id, nonce]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct GuildAction {
    pub version: u8,
    pub bump: u8,
    pub guild_id: u32,
    pub nonce: u64,
    pub kind: GuildActionKind,
    /// Bit i: officer i approved.
    pub approvals: u16,
    pub officers_version: u32,
    pub proposed_at: i64,
    pub eta: i64,
    pub executed: bool,
}

#[error_code]
pub enum SocialError {
    #[msg("the signer is not this program's upgrade authority")]
    NotUpgradeAuthority,
    #[msg("only the social admin may do this")]
    NotAdmin,
    #[msg("a parameter is out of bounds")]
    BadParams,
    #[msg("no params change is pending, or it is not ready yet")]
    NotReady,
    #[msg("an account is not the one expected")]
    WrongAccount,
    #[msg("a name is empty or too long")]
    BadName,
    #[msg("claims for this badge are not open yet")]
    ClaimsNotOpen,
    #[msg("the badge's criterion is not met")]
    CriterionNotMet,
    #[msg("the signer is not an officer of this guild")]
    NotOfficer,
    #[msg("officers or threshold out of bounds")]
    BadOfficers,
    #[msg("the action already executed")]
    AlreadyExecuted,
    #[msg("the action needs more approvals")]
    NotEnoughApprovals,
    #[msg("the action's delay has not passed")]
    TooEarly,
    #[msg("the officers changed since this action was proposed")]
    StaleAction,
    #[msg("amount must be above zero")]
    ZeroAmount,
    #[msg("arithmetic overflow")]
    Overflow,
    // Profiles and skills (11 section 3.3).
    #[msg("a skill is malformed (counter out of range or thresholds not ascending)")]
    BadSkill,
    #[msg("too many skills or callers")]
    TooMany,
    #[msg("the signer is not a registered caller's PDA")]
    NotCaller,
    #[msg("no pending skill change, or not ready yet")]
    SkillsNotReady,
    #[msg("unknown counter")]
    BadCounter,
    #[msg("two skills share an id")]
    DuplicateSkill,
}

#[event]
pub struct BadgeCreated {
    pub id: u32,
    pub mint: Pubkey,
    pub name: String,
    pub criterion: Criterion,
    pub claims_open_at: i64,
}

#[event]
pub struct BadgeAwarded {
    pub id: u32,
    pub recipient: Pubkey,
    pub claimant: Pubkey,
    pub ts: i64,
}

#[event]
pub struct GuildCreated {
    pub id: u32,
    pub name: String,
    pub founder: Pubkey,
    pub ts: i64,
}

#[event]
pub struct GuildDeposit {
    pub id: u32,
    pub from: Pubkey,
    pub lamports: u64,
    pub ts: i64,
}

#[event]
pub struct GuildActionProposed {
    pub id: u32,
    pub nonce: u64,
    pub kind: GuildActionKind,
    pub eta: i64,
}

#[event]
pub struct GuildActionApproved {
    pub id: u32,
    pub nonce: u64,
    pub officer: Pubkey,
}

#[event]
pub struct GuildActionExecuted {
    pub id: u32,
    pub nonce: u64,
    pub kind: GuildActionKind,
    pub ts: i64,
}

#[event]
pub struct SocialParamsProposed {
    pub ready_at: i64,
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn upgrade_authority(program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    require_keys_eq!(
        *program_data.key,
        hookwars_common::programdata_address(&crate::ID),
        SocialError::NotUpgradeAuthority
    );
    require_keys_eq!(
        *program_data.owner,
        hookwars_common::ids::BPF_LOADER_UPGRADEABLE_ID,
        SocialError::NotUpgradeAuthority
    );
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        SocialError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    Ok(Some(Pubkey::new_from_array(data[13..45].try_into().unwrap())))
}

fn check_params(p: &SocialParams) -> Result<()> {
    require!(
        p.guild_max_officers > 0 && usize::from(p.guild_max_officers) <= GUILD_OFFICERS_CAP,
        SocialError::BadParams
    );
    Ok(())
}

fn check_officers(officers: &[Pubkey], threshold: u8, max: u8) -> Result<()> {
    require!(
        !officers.is_empty() && officers.len() <= usize::from(max),
        SocialError::BadOfficers
    );
    require!(
        threshold > 0 && usize::from(threshold) <= officers.len(),
        SocialError::BadOfficers
    );
    for (i, o) in officers.iter().enumerate() {
        require!(!officers[..i].contains(o), SocialError::BadOfficers);
    }
    Ok(())
}

/// `["badge-minter"]`.
pub fn minter_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::MINTER], &crate::ID)
}

/// `["badge-mint", id]`.
pub fn badge_mint_address(id: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::BADGE_MINT, &id.to_le_bytes()], &crate::ID)
}

/// `["guild-treasury", id]`.
pub fn treasury_address(id: u32) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::TREASURY, &id.to_le_bytes()], &crate::ID)
}

/// `["war-chest", mint]` under war.
pub fn war_chest(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"war-chest", mint.as_ref()], &hookwars_common::ids::WAR_ID).0
}

fn call<'info>(ix: &Instruction, infos: &[AccountInfo<'info>], seeds: &[&[&[u8]]]) -> Result<()> {
    if seeds.is_empty() {
        invoke(ix, infos)?;
    } else {
        invoke_signed(ix, infos, seeds)?;
    }
    Ok(())
}

fn check_token(token_program: &AccountInfo, event_authority: &AccountInfo) -> Result<()> {
    require_keys_eq!(*token_program.key, bordrless_token::ID, SocialError::WrongAccount);
    require_keys_eq!(
        *event_authority.key,
        bordrless_token::client::event_authority(),
        SocialError::WrongAccount
    );
    Ok(())
}

/// Whether `recipient` meets `c`. `extra` are the criterion's accounts:
/// FirstSiege: `[war_state]`; RaidPoints: `[mint, recipient's holding, raid item, war_config]`;
/// ForgeLevel: `[item, recipient's holding of the item]`; ItemsAuthored: `[AuthorCounter]`;
/// RoyaltiesClaimed: `[ClaimCounter]`.
fn criterion_met(c: &Criterion, recipient: &Pubkey, extra: &[AccountInfo]) -> Result<bool> {
    use hookwars_common::{ids, pda};
    match *c {
        Criterion::FirstSiege { mint } => {
            let ws = extra.first().ok_or(SocialError::WrongAccount)?;
            require_keys_eq!(*ws.owner, ids::WAR_ID, SocialError::WrongAccount);
            require_keys_eq!(*ws.key, pda::war_state(&mint).0, SocialError::WrongAccount);
            require_keys_eq!(*recipient, war_chest(&mint), SocialError::WrongAccount);
            let data = ws.try_borrow_data()?;
            let s = Box::new(
                hookwars_war::state::WarState::try_deserialize(&mut &data[..])
                    .map_err(|_| error!(SocialError::WrongAccount))?,
            );
            Ok(s.spent_siege > 0)
        }
        Criterion::RaidPoints { mint, min } => {
            require!(extra.len() >= 4, SocialError::WrongAccount);
            let (mint_info, holding_info, item_info, war_info) =
                (&extra[0], &extra[1], &extra[2], &extra[3]);
            require_keys_eq!(*mint_info.key, mint, SocialError::WrongAccount);
            let m = Box::new(bordrless_token::client::read_mint(mint_info)?);
            require_keys_eq!(
                *holding_info.key,
                bordrless_token::client::holding_address(&mint, recipient),
                SocialError::WrongAccount
            );
            let h = bordrless_token::client::read_holding(holding_info)?;
            require!(h.owner == *recipient && h.mint == mint, SocialError::WrongAccount);
            require_keys_eq!(*item_info.owner, ids::ARMORY_ID, SocialError::WrongAccount);
            let item = {
                let d = item_info.try_borrow_data()?;
                hookwars_armory::state::Item::try_deserialize(&mut &d[..])
                    .map_err(|_| error!(SocialError::WrongAccount))?
            };
            require_keys_eq!(*item_info.key, pda::item(&item.item_mint).0, SocialError::WrongAccount);
            // Integration pass 2 (08 arsenal 2 request 3): a Mercenary keeps the same Raid range.
            if item.template_id != hookwars_common::template_id::RAID
                && item.template_id != hookwars_common::arsenal2::MERCENARY
            {
                return Ok(false);
            }
            let Some(slot) = m.slots.iter().find(|s| s.item == *item_info.key) else {
                return Ok(false);
            };
            let off = usize::from(slot.data_offset);
            let len = usize::from(slot.data_len);
            let Some(range) = h.hook_data.get(off..off + len) else {
                return Ok(false);
            };
            if range.is_empty() || range[0] != slot.data_epoch {
                return Ok(false);
            }
            let season = if *war_info.owner == ids::WAR_ID && *war_info.key == pda::war_config().0 {
                let d = war_info.try_borrow_data()?;
                hookwars_war::state::WarConfig::try_deserialize(&mut &d[..])
                    .map(|c| c.current_season)
                    .unwrap_or(0)
            } else {
                return Err(error!(SocialError::WrongAccount));
            };
            let r = hookwars_common::raid::RaidRange::read(&range[1..], season);
            Ok(r.raid_points >= min)
        }
        Criterion::ForgeLevel { min_level } => {
            require!(extra.len() >= 2, SocialError::WrongAccount);
            let (item_info, holding_info) = (&extra[0], &extra[1]);
            require_keys_eq!(*item_info.owner, ids::ARMORY_ID, SocialError::WrongAccount);
            let item = {
                let d = item_info.try_borrow_data()?;
                hookwars_armory::state::Item::try_deserialize(&mut &d[..])
                    .map_err(|_| error!(SocialError::WrongAccount))?
            };
            require_keys_eq!(*item_info.key, pda::item(&item.item_mint).0, SocialError::WrongAccount);
            require_keys_eq!(
                *holding_info.key,
                bordrless_token::client::holding_address(&item.item_mint, recipient),
                SocialError::WrongAccount
            );
            let h = bordrless_token::client::read_holding(holding_info)?;
            Ok(h.owner == *recipient && h.mint == item.item_mint && h.amount == 1 && item.level >= min_level)
        }
        Criterion::ItemsAuthored { min } => {
            let c = extra.first().ok_or(SocialError::WrongAccount)?;
            require_keys_eq!(*c.owner, ids::ARMORY_ID, SocialError::WrongAccount);
            require_keys_eq!(*c.key, pda::author_counter(recipient).0, SocialError::WrongAccount);
            let v = hookwars_armory::state::AuthorCounter::try_deserialize(&mut &c.try_borrow_data()?[..])
                .map_err(|_| error!(SocialError::WrongAccount))?;
            Ok(v.items >= min)
        }
        Criterion::RoyaltiesClaimed { min_lamports } => {
            let c = extra.first().ok_or(SocialError::WrongAccount)?;
            require_keys_eq!(*c.owner, ids::ARMORY_ID, SocialError::WrongAccount);
            require_keys_eq!(*c.key, pda::claim_counter(recipient).0, SocialError::WrongAccount);
            let v = hookwars_armory::state::ClaimCounter::try_deserialize(&mut &c.try_borrow_data()?[..])
                .map_err(|_| error!(SocialError::WrongAccount))?;
            Ok(v.lamports >= min_lamports)
        }
    }
}

#[program]
pub mod hookwars_social {
    use super::*;

    /// Creates the config; the program's upgrade authority signs.
    pub fn init(ctx: Context<Init>, admin: Pubkey, params: SocialParams) -> Result<()> {
        let up = upgrade_authority(&ctx.accounts.program_data)?;
        require!(
            up == Some(ctx.accounts.authority.key()),
            SocialError::NotUpgradeAuthority
        );
        check_params(&params)?;
        let c = &mut ctx.accounts.config;
        c.version = VERSION;
        c.bump = ctx.bumps.config;
        c.admin = admin;
        c.params = params;
        c.badges = 0;
        c.guilds = 0;
        c.reserved = [0; 32];
        Ok(())
    }

    /// The admin proposes new params; they apply after `admin_timelock_secs`.
    pub fn propose_params(ctx: Context<ProposeParams>, params: SocialParams) -> Result<()> {
        check_params(&params)?;
        let ready_at = now()?
            .checked_add(i64::from(ctx.accounts.config.params.admin_timelock_secs))
            .ok_or(SocialError::Overflow)?;
        let p = &mut ctx.accounts.pending;
        p.bump = ctx.bumps.pending;
        p.params = params;
        p.ready_at = ready_at;
        p.active = true;
        emit_cpi!(SocialParamsProposed { ready_at });
        Ok(())
    }

    /// Anyone applies a ready params change.
    pub fn apply_params(ctx: Context<ApplyParams>) -> Result<()> {
        let p = &mut ctx.accounts.pending;
        require!(p.active && now()? >= p.ready_at, SocialError::NotReady);
        ctx.accounts.config.params = p.params;
        p.active = false;
        Ok(())
    }

    /// The admin creates a badge type and its mint; claims open after `admin_timelock_secs`, so a
    /// new badge is public before anyone can earn it.
    pub fn create_badge(ctx: Context<CreateBadge>, name: String, criterion: Criterion) -> Result<()> {
        require!(!name.is_empty() && name.len() <= NAME_MAX_LEN, SocialError::BadName);
        check_token(&ctx.accounts.token_program, &ctx.accounts.token_event_authority)?;
        let id = ctx.accounts.config.badges;
        let id_bytes = id.to_le_bytes();
        let mint_bump = [ctx.bumps.badge_mint];
        let mint_seeds: &[&[u8]] = &[seeds::BADGE_MINT, &id_bytes, &mint_bump];
        let (minter, _) = minter_address();
        let args = bordrless_token::CreateMintArgs {
            decimals: 0,
            name: name.clone(),
            symbol: "BADGE".to_string(),
            uri: String::new(),
            max_supply: 0,
            mint_authority: Some(minter),
            freeze_authority: Some(minter),
            hook_program: None,
            hook_flags: 0,
            hook_authority: None,
            metadata_authority: None,
        };
        let ix = bordrless_token::client::create_mint(
            ctx.accounts.admin.key(),
            ctx.accounts.badge_mint.key(),
            args,
        );
        call(
            &ix,
            &[
                ctx.accounts.admin.to_account_info(),
                ctx.accounts.badge_mint.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ctx.accounts.token_event_authority.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
            &[mint_seeds],
        )?;
        let ts = now()?;
        let claims_open_at = ts
            .checked_add(i64::from(ctx.accounts.config.params.admin_timelock_secs))
            .ok_or(SocialError::Overflow)?;
        let config = &mut ctx.accounts.config;
        config.badges = id.checked_add(1).ok_or(SocialError::Overflow)?;
        let b = &mut ctx.accounts.badge;
        b.version = VERSION;
        b.bump = ctx.bumps.badge;
        b.mint_bump = ctx.bumps.badge_mint;
        b.id = id;
        b.mint = ctx.accounts.badge_mint.key();
        b.name = name.clone();
        b.criterion = criterion;
        b.claims_open_at = claims_open_at;
        b.awarded = 0;
        b.created_at = ts;
        emit_cpi!(BadgeCreated {
            id,
            mint: b.mint,
            name,
            criterion,
            claims_open_at
        });
        Ok(())
    }

    /// Anyone claims a badge for a recipient that meets its criterion (the recipient is the
    /// claimant's wallet, or a token's war chest for `FirstSiege`). The badge is minted into the
    /// recipient's holding, which is then frozen: it cannot move.
    pub fn claim_badge<'info>(
        ctx: Context<'info, ClaimBadge<'info>>,
        badge_id: u32,
        recipient_key: Pubkey,
    ) -> Result<()> {
        let recipient = recipient_key;
        let b = &ctx.accounts.badge;
        require!(b.id == badge_id, SocialError::WrongAccount);
        let ts = now()?;
        require!(ts >= b.claims_open_at, SocialError::ClaimsNotOpen);
        if !matches!(b.criterion, Criterion::FirstSiege { .. }) {
            require_keys_eq!(recipient, ctx.accounts.claimant.key(), SocialError::WrongAccount);
        }
        require_keys_eq!(ctx.accounts.recipient.key(), recipient, SocialError::WrongAccount);
        require!(
            criterion_met(&b.criterion, &recipient, ctx.remaining_accounts)?,
            SocialError::CriterionNotMet
        );
        check_token(&ctx.accounts.token_program, &ctx.accounts.token_event_authority)?;
        let mint = ctx.accounts.badge_mint.key();
        require_keys_eq!(mint, b.mint, SocialError::WrongAccount);
        require_keys_eq!(
            ctx.accounts.recipient_holding.key(),
            bordrless_token::client::holding_address(&mint, &recipient),
            SocialError::WrongAccount
        );
        let tp = ctx.accounts.token_program.to_account_info();
        let ea = ctx.accounts.token_event_authority.to_account_info();
        let ix = bordrless_token::client::create_holding(ctx.accounts.claimant.key(), mint, recipient);
        call(
            &ix,
            &[
                ctx.accounts.claimant.to_account_info(),
                ctx.accounts.badge_mint.to_account_info(),
                ctx.accounts.recipient.to_account_info(),
                ctx.accounts.recipient_holding.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
                ea.clone(),
                tp.clone(),
            ],
            &[],
        )?;
        let (minter, minter_bump) = minter_address();
        require_keys_eq!(ctx.accounts.minter.key(), minter, SocialError::WrongAccount);
        let mb = [minter_bump];
        let minter_seeds: &[&[u8]] = &[seeds::MINTER, &mb];
        let ix = bordrless_token::client::mint_to(minter, mint, ctx.accounts.recipient_holding.key(), None, vec![], 1);
        call(
            &ix,
            &[
                ctx.accounts.minter.to_account_info(),
                ctx.accounts.badge_mint.to_account_info(),
                ctx.accounts.recipient_holding.to_account_info(),
                ea.clone(),
                tp.clone(),
            ],
            &[minter_seeds],
        )?;
        let ix = bordrless_token::client::set_frozen(minter, mint, ctx.accounts.recipient_holding.key(), true);
        call(
            &ix,
            &[
                ctx.accounts.minter.to_account_info(),
                ctx.accounts.badge_mint.to_account_info(),
                ctx.accounts.recipient_holding.to_account_info(),
                ea,
                tp,
            ],
            &[minter_seeds],
        )?;
        let award = &mut ctx.accounts.award;
        award.bump = ctx.bumps.award;
        award.badge_id = badge_id;
        award.recipient = recipient;
        award.awarded_at = ts;
        let b = &mut ctx.accounts.badge;
        b.awarded = b.awarded.checked_add(1).ok_or(SocialError::Overflow)?;
        emit_cpi!(BadgeAwarded {
            id: badge_id,
            recipient,
            claimant: ctx.accounts.claimant.key(),
            ts
        });
        Ok(())
    }

    /// Creates a guild; the founder is its only officer with threshold 1.
    pub fn create_guild(ctx: Context<CreateGuild>, name: String) -> Result<()> {
        require!(!name.is_empty() && name.len() <= NAME_MAX_LEN, SocialError::BadName);
        let id = ctx.accounts.config.guilds;
        ctx.accounts.config.guilds = id.checked_add(1).ok_or(SocialError::Overflow)?;
        let ts = now()?;
        let g = &mut ctx.accounts.guild;
        g.version = VERSION;
        g.bump = ctx.bumps.guild;
        g.treasury_bump = treasury_address(id).1;
        g.id = id;
        g.name = name.clone();
        g.officers = vec![ctx.accounts.founder.key()];
        g.threshold = 1;
        g.officers_version = 0;
        g.actions = 0;
        g.created_at = ts;
        emit_cpi!(GuildCreated {
            id,
            name,
            founder: ctx.accounts.founder.key(),
            ts
        });
        Ok(())
    }

    /// Anyone deposits SOL into a guild treasury. (Units tokens are deposited by an ordinary
    /// transfer to the treasury's holding.)
    pub fn deposit_sol(ctx: Context<DepositSol>, lamports: u64) -> Result<()> {
        require!(lamports > 0, SocialError::ZeroAmount);
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                system_program::Transfer {
                    from: ctx.accounts.from.to_account_info(),
                    to: ctx.accounts.treasury.to_account_info(),
                },
            ),
            lamports,
        )?;
        emit_cpi!(GuildDeposit {
            id: ctx.accounts.guild.id,
            from: ctx.accounts.from.key(),
            lamports,
            ts: now()?
        });
        Ok(())
    }

    /// An officer proposes a guild action; it can execute after `guild_timelock_secs` with the
    /// threshold of approvals. The proposer's approval is counted.
    pub fn propose_action(ctx: Context<ProposeAction>, kind: GuildActionKind) -> Result<()> {
        let g = &ctx.accounts.guild;
        let idx = g
            .officers
            .iter()
            .position(|o| *o == ctx.accounts.officer.key())
            .ok_or(SocialError::NotOfficer)?;
        match &kind {
            GuildActionKind::SpendSol { lamports, .. } => require!(*lamports > 0, SocialError::ZeroAmount),
            GuildActionKind::SpendToken { amount, .. } => require!(*amount > 0, SocialError::ZeroAmount),
            GuildActionKind::SetOfficers { officers, threshold } => check_officers(
                officers,
                *threshold,
                ctx.accounts.config.params.guild_max_officers,
            )?,
        }
        let ts = now()?;
        let eta = ts
            .checked_add(i64::from(ctx.accounts.config.params.guild_timelock_secs))
            .ok_or(SocialError::Overflow)?;
        let (id, nonce, version) = (g.id, g.actions, g.officers_version);
        ctx.accounts.guild.actions = nonce.checked_add(1).ok_or(SocialError::Overflow)?;
        let a = &mut ctx.accounts.action;
        a.version = VERSION;
        a.bump = ctx.bumps.action;
        a.guild_id = id;
        a.nonce = nonce;
        a.kind = kind.clone();
        a.approvals = 1u16 << idx;
        a.officers_version = version;
        a.proposed_at = ts;
        a.eta = eta;
        a.executed = false;
        emit_cpi!(GuildActionProposed { id, nonce, kind, eta });
        Ok(())
    }

    /// An officer approves a pending action.
    pub fn approve_action(ctx: Context<ApproveAction>) -> Result<()> {
        let g = &ctx.accounts.guild;
        let a = &ctx.accounts.action;
        require!(!a.executed, SocialError::AlreadyExecuted);
        require!(a.officers_version == g.officers_version, SocialError::StaleAction);
        let idx = g
            .officers
            .iter()
            .position(|o| *o == ctx.accounts.officer.key())
            .ok_or(SocialError::NotOfficer)?;
        let (id, nonce) = (a.guild_id, a.nonce);
        ctx.accounts.action.approvals |= 1u16 << idx;
        emit_cpi!(GuildActionApproved {
            id,
            nonce,
            officer: ctx.accounts.officer.key()
        });
        Ok(())
    }

    /// Anyone executes an approved action after its delay. `SpendToken` passes the mint's slot
    /// slices (if any) as remaining accounts, as a token transfer of that mint takes them.
    pub fn execute_action<'info>(ctx: Context<'info, ExecuteAction<'info>>) -> Result<()> {
        let g = &ctx.accounts.guild;
        let a = &ctx.accounts.action;
        require!(!a.executed, SocialError::AlreadyExecuted);
        require!(a.officers_version == g.officers_version, SocialError::StaleAction);
        require!(
            a.approvals.count_ones() >= u32::from(g.threshold),
            SocialError::NotEnoughApprovals
        );
        let ts = now()?;
        require!(ts >= a.eta, SocialError::TooEarly);
        let id_bytes = g.id.to_le_bytes();
        let tb = [g.treasury_bump];
        let treasury_seeds: &[&[u8]] = &[seeds::TREASURY, &id_bytes, &tb];
        let kind = a.kind.clone();
        match &kind {
            GuildActionKind::SpendSol { to, lamports } => {
                require_keys_eq!(ctx.accounts.to.key(), *to, SocialError::WrongAccount);
                system_program::transfer(
                    CpiContext::new_with_signer(
                        ctx.accounts.system_program.key(),
                        system_program::Transfer {
                            from: ctx.accounts.treasury.to_account_info(),
                            to: ctx.accounts.to.to_account_info(),
                        },
                        &[treasury_seeds],
                    ),
                    *lamports,
                )?;
            }
            GuildActionKind::SpendToken { mint, to, amount } => {
                require_keys_eq!(ctx.accounts.to.key(), *to, SocialError::WrongAccount);
                let tp = ctx.accounts.token_program.to_account_info();
                let ea = ctx.accounts.token_event_authority.to_account_info();
                check_token(&tp, &ea)?;
                let rest = ctx.remaining_accounts;
                require!(rest.len() >= 3, SocialError::WrongAccount);
                let (mint_info, from_h, to_h) = (&rest[0], &rest[1], &rest[2]);
                require_keys_eq!(*mint_info.key, *mint, SocialError::WrongAccount);
                let treasury = ctx.accounts.treasury.key();
                require_keys_eq!(
                    *from_h.key,
                    bordrless_token::client::holding_address(mint, &treasury),
                    SocialError::WrongAccount
                );
                require_keys_eq!(
                    *to_h.key,
                    bordrless_token::client::holding_address(mint, to),
                    SocialError::WrongAccount
                );
                let extras: Vec<AccountMeta> = rest[3..]
                    .iter()
                    .map(|i| {
                        if i.is_writable {
                            AccountMeta::new(*i.key, false)
                        } else {
                            AccountMeta::new_readonly(*i.key, false)
                        }
                    })
                    .collect();
                // Integration pass 2 (10 section 17 I-1, R16, R24): a guild treasury pays out as a protocol
                // source, so the token's own items do not cut the guild's spend.
                let ix = bordrless_token::client::transfer_from_protocol(
                    treasury,
                    *from_h.key,
                    *to_h.key,
                    *mint,
                    extras,
                    *amount,
                    crate::ID,
                    treasury_seeds.iter().map(|x| x.to_vec()).collect(),
                );
                let mut infos = vec![
                    ctx.accounts.treasury.to_account_info(),
                    from_h.clone(),
                    to_h.clone(),
                    mint_info.clone(),
                    tp,
                    ea,
                ];
                infos.extend(rest[3..].iter().cloned());
                call(&ix, &infos, &[treasury_seeds])?;
            }
            GuildActionKind::SetOfficers { officers, threshold } => {
                let g = &mut ctx.accounts.guild;
                g.officers = officers.clone();
                g.threshold = *threshold;
                g.officers_version = g.officers_version.checked_add(1).ok_or(SocialError::Overflow)?;
            }
        }
        let (id, nonce) = (ctx.accounts.action.guild_id, ctx.accounts.action.nonce);
        ctx.accounts.action.executed = true;
        emit_cpi!(GuildActionExecuted { id, nonce, kind, ts });
        Ok(())
    }

    /// Opens `wallet`'s profile (anyone pays the rent) (11 3.3).
    pub fn open_profile(ctx: Context<OpenProfile>, wallet: Pubkey) -> Result<()> {
        profiles::process_open_profile(ctx, wallet)
    }

    /// Creates the skill table (admin, once).
    pub fn init_skills(
        ctx: Context<InitSkills>,
        skills: Vec<hookwars_common::economy::SkillDef>,
        callers: Vec<Pubkey>,
    ) -> Result<()> {
        profiles::process_init_skills(ctx, skills, callers)
    }

    /// Proposes a new skill table and caller list (admin; applies after the admin timelock).
    pub fn propose_skills(
        ctx: Context<ProposeSkills>,
        skills: Vec<hookwars_common::economy::SkillDef>,
        callers: Vec<Pubkey>,
    ) -> Result<()> {
        profiles::process_propose_skills(ctx, skills, callers)
    }

    /// Applies a ready skill change (anyone).
    pub fn apply_skills(ctx: Context<ApplySkills>) -> Result<()> {
        profiles::process_apply_skills(ctx)
    }

    /// Adds `value` to a wallet counter; signed by a registered caller program's PDA (R43).
    pub fn record_wallet(
        ctx: Context<RecordWallet>,
        caller_program: Pubkey,
        counter: u8,
        value: u64,
    ) -> Result<()> {
        profiles::process_record_wallet(ctx, caller_program, counter, value)
    }
}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + SocialConfig::INIT_SPACE, seeds = [seeds::CONFIG], bump)]
    pub config: Box<Account<'info, SocialConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeParams<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ SocialError::NotAdmin)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + PendingSocialParams::INIT_SPACE, seeds = [seeds::PENDING], bump)]
    pub pending: Box<Account<'info, PendingSocialParams>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApplyParams<'info> {
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(mut, seeds = [seeds::PENDING], bump = pending.bump)]
    pub pending: Box<Account<'info, PendingSocialParams>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CreateBadge<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ SocialError::NotAdmin)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(
        init,
        payer = admin,
        space = 8 + BadgeType::INIT_SPACE,
        seeds = [seeds::BADGE, &config.badges.to_le_bytes()],
        bump
    )]
    pub badge: Box<Account<'info, BadgeType>>,
    /// CHECK: `["badge-mint", id]`, created by the token program here.
    #[account(mut, seeds = [seeds::BADGE_MINT, &config.badges.to_le_bytes()], bump)]
    pub badge_mint: UncheckedAccount<'info>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(badge_id: u32, recipient_key: Pubkey)]
pub struct ClaimBadge<'info> {
    #[account(mut)]
    pub claimant: Signer<'info>,
    #[account(mut, seeds = [seeds::BADGE, &badge_id.to_le_bytes()], bump = badge.bump)]
    pub badge: Box<Account<'info, BadgeType>>,
    #[account(
        init,
        payer = claimant,
        space = 8 + BadgeAward::INIT_SPACE,
        seeds = [seeds::AWARD, &badge_id.to_le_bytes(), recipient_key.as_ref()],
        bump
    )]
    pub award: Box<Account<'info, BadgeAward>>,
    /// CHECK: the recipient (checked against the argument).
    pub recipient: UncheckedAccount<'info>,
    /// CHECK: the badge mint (checked against the badge).
    #[account(mut)]
    pub badge_mint: UncheckedAccount<'info>,
    /// CHECK: the recipient's holding of the badge (address checked; created here).
    #[account(mut)]
    pub recipient_holding: UncheckedAccount<'info>,
    /// CHECK: `["badge-minter"]` (checked in the handler).
    pub minter: UncheckedAccount<'info>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CreateGuild<'info> {
    #[account(mut)]
    pub founder: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(
        init,
        payer = founder,
        space = 8 + Guild::INIT_SPACE,
        seeds = [seeds::GUILD, &config.guilds.to_le_bytes()],
        bump
    )]
    pub guild: Box<Account<'info, Guild>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct DepositSol<'info> {
    #[account(mut)]
    pub from: Signer<'info>,
    #[account(seeds = [seeds::GUILD, &guild.id.to_le_bytes()], bump = guild.bump)]
    pub guild: Box<Account<'info, Guild>>,
    /// CHECK: `["guild-treasury", id]`, a system-owned PDA.
    #[account(mut, seeds = [seeds::TREASURY, &guild.id.to_le_bytes()], bump = guild.treasury_bump)]
    pub treasury: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeAction<'info> {
    #[account(mut)]
    pub officer: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, SocialConfig>>,
    #[account(mut, seeds = [seeds::GUILD, &guild.id.to_le_bytes()], bump = guild.bump)]
    pub guild: Box<Account<'info, Guild>>,
    #[account(
        init,
        payer = officer,
        space = 8 + GuildAction::INIT_SPACE,
        seeds = [seeds::ACTION, &guild.id.to_le_bytes(), &guild.actions.to_le_bytes()],
        bump
    )]
    pub action: Box<Account<'info, GuildAction>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ApproveAction<'info> {
    pub officer: Signer<'info>,
    #[account(seeds = [seeds::GUILD, &guild.id.to_le_bytes()], bump = guild.bump)]
    pub guild: Box<Account<'info, Guild>>,
    #[account(
        mut,
        seeds = [seeds::ACTION, &guild.id.to_le_bytes(), &action.nonce.to_le_bytes()],
        bump = action.bump
    )]
    pub action: Box<Account<'info, GuildAction>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ExecuteAction<'info> {
    #[account(mut, seeds = [seeds::GUILD, &guild.id.to_le_bytes()], bump = guild.bump)]
    pub guild: Box<Account<'info, Guild>>,
    #[account(
        mut,
        seeds = [seeds::ACTION, &guild.id.to_le_bytes(), &action.nonce.to_le_bytes()],
        bump = action.bump
    )]
    pub action: Box<Account<'info, GuildAction>>,
    /// CHECK: `["guild-treasury", id]`.
    #[account(mut, seeds = [seeds::TREASURY, &guild.id.to_le_bytes()], bump = guild.treasury_bump)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the action's recipient (checked against it; for `SetOfficers` any account).
    #[account(mut)]
    pub to: UncheckedAccount<'info>,
    /// CHECK: the token program (checked for `SpendToken`).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked for `SpendToken`).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
