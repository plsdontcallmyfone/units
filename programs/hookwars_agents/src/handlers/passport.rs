// Changed by Hookwars: new file (09).
//! Passports and badges (09 sections 3 and 4).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::system_program;
use anchor_lang::InstructionData;
use bordrless_hook::{equip_rule, slot_kind};
use bordrless_token::client as token;
use bordrless_token::instructions::{CreateMintArgs, SlotInit};
use bordrless_token::state::{AuthorityKind, SlotBounds};
use hookwars_common::{ids, EquipConfig};

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::*;
use crate::handlers::common::*;
use crate::state::*;
use crate::ProfileArgs;

impl ProfileArgs {
    fn check(&self, p: &AgentsParams) -> Result<()> {
        require!(!self.name.is_empty(), AgentsError::FieldTooLong);
        bounded(&self.name, usize::from(p.name_max_len))?;
        let uri = usize::from(p.uri_max_len);
        bounded(&self.avatar_uri, uri)?;
        bounded(&self.bio_uri, uri)?;
        bounded(&self.hire_uri, uri)?;
        require!(self.kinds & !kind::ALL == 0, AgentsError::BadKinds);
        Ok(())
    }
}

/// The badge's one slot: Defense, Locked equip rule, may refuse, no cut, no data (09 4.2).
fn badge_slot() -> SlotInit {
    SlotInit {
        kind: slot_kind::DEFENSE,
        equip_rule: equip_rule::LOCKED,
        bounds: SlotBounds {
            max_cut_bps: 0,
            may_refuse: true,
            may_write_data: false,
            may_answer_touch: false,
            may_burn: false,
        },
        data_len: 0,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// Creates the badge mint of `passport` at `generation` (09 4.2 step 1).
#[allow(clippy::too_many_arguments)]
fn create_badge_mint<'info>(
    passport: &Pubkey,
    generation: u8,
    mint: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    system: &AccountInfo<'info>,
    tok: &TokenAccs<'info>,
    name: &str,
    uri: &str,
) -> Result<()> {
    let (key, bump) = pda::badge_mint(passport, generation);
    require_keys_eq!(*mint.key, key, AgentsError::WrongAccount);
    let signer = ids::AGENTS_SIGNER;
    let args = CreateMintArgs {
        decimals: 0,
        name: name.to_string(),
        symbol: BADGE_SYMBOL.to_string(),
        uri: uri.to_string(),
        max_supply: 1,
        mint_authority: Some(signer),
        freeze_authority: Some(signer),
        hook_program: None,
        hook_flags: 0,
        hook_authority: None,
        metadata_authority: Some(signer),
    };
    let authority = bordrless_hook::slot_authority(&ids::ARMORY_ID, &key).0;
    let ix = token::create_slot_mint(*payer.key, key, args, Some(authority), vec![badge_slot()]);
    cpi(
        &ix,
        &[
            payer.clone(),
            mint.clone(),
            system.clone(),
            tok.token_event_authority.to_account_info(),
            tok.token_program.to_account_info(),
        ],
        &[&[BADGE_MINT_SEED, passport.as_ref(), &[generation], &[bump]]],
    )
}

// ---- register_passport ---------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
#[instruction(index: u32)]
pub struct RegisterPassport<'info> {
    pub operator: Signer<'info>,
    pub agent_key: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    /// CHECK: the fee collector (checked against the config).
    #[account(mut, address = config.fee_collector @ AgentsError::WrongAccount)]
    pub fee_collector: UncheckedAccount<'info>,
    #[account(init_if_needed, payer = payer, space = 8 + OperatorIndex::INIT_SPACE,
        seeds = [OPERATOR_SEED, operator.key().as_ref()], bump)]
    pub operator_index: Box<Account<'info, OperatorIndex>>,
    #[account(init, payer = payer, space = 8 + Passport::INIT_SPACE,
        seeds = [PASSPORT_SEED, operator.key().as_ref(), &index.to_le_bytes()], bump)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(init, payer = payer, space = 8 + AgentKey::INIT_SPACE,
        seeds = [AGENT_KEY_SEED, agent_key.key().as_ref()], bump)]
    pub agent_key_record: Box<Account<'info, AgentKey>>,
    /// CHECK: the badge mint, created here by the token program (`["badge-mint", passport, 0]`).
    #[account(mut)]
    pub badge_mint: UncheckedAccount<'info>,
    pub token: TokenAccs<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_register(ctx: Context<RegisterPassport>, index: u32, args: ProfileArgs) -> Result<()> {
    let params = ctx.accounts.config.params;
    args.check(&params)?;
    let operator = ctx.accounts.operator.key();
    let agent_key = ctx.accounts.agent_key.key();
    require_keys_neq!(agent_key, operator, AgentsError::KeyIsOperator);
    let t = now()?;
    let oi = &mut ctx.accounts.operator_index;
    if oi.operator == Pubkey::default() {
        oi.operator = operator;
        oi.bump = ctx.bumps.operator_index;
    }
    require!(index == oi.next, AgentsError::WrongAccount);
    require!(
        params.max_passports_per_operator == 0 || oi.active < params.max_passports_per_operator,
        AgentsError::OperatorLimit
    );
    oi.next = oi.next.checked_add(1).ok_or(AgentsError::MathOverflow)?;
    oi.active = oi.active.checked_add(1).ok_or(AgentsError::MathOverflow)?;
    if params.passport_fee_lamports > 0 {
        system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.fee_collector.to_account_info(),
                },
            ),
            params.passport_fee_lamports,
        )?;
    }
    let key = ctx.accounts.passport.key();
    let badge = pda::badge_mint(&key, 0).0;
    {
        let p = &mut ctx.accounts.passport;
        p.version = VERSION;
        p.bump = ctx.bumps.passport;
        p.operator = operator;
        p.index = index;
        p.agent_key = agent_key;
        p.name = args.name.clone();
        p.avatar_uri = args.avatar_uri.clone();
        p.bio_uri = args.bio_uri.clone();
        p.hire_uri = args.hire_uri.clone();
        p.kinds = args.kinds;
        p.status = status::ACTIVE;
        p.proof = proof::DECLARED;
        p.links = 0;
        p.attested_until = 0;
        p.badge_mint = badge;
        p.badge_generation = 0;
        p.badge_issued = false;
        p.credit_agent_id = args.credit_agent_id;
        p.created_at = t;
        p.last_active_at = t;
        p.record = TrackRecord::default();
        p.link_nonce = 0;
        p.reserved = [0; 28];
    }
    let r = &mut ctx.accounts.agent_key_record;
    r.passport = key;
    r.bump = ctx.bumps.agent_key_record;
    ctx.accounts.config.passports = ctx.accounts.config.passports.saturating_add(1);
    create_badge_mint(
        &key,
        0,
        &ctx.accounts.badge_mint.to_account_info(),
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.token,
        &args.name,
        &args.avatar_uri,
    )?;
    emit_cpi!(PassportRegistered {
        passport: key,
        operator,
        agent_key,
        name: args.name,
        kinds: args.kinds,
        badge_mint: badge,
        ts: t
    });
    Ok(())
}

// ---- equip_badge -----------------------------------------------------------------------------

/// Accounts of `equip_badge`; the armory's `equip_launch` accounts follow as remaining accounts,
/// in that instruction's order, with `launch_caller` = `["armory-caller", badge_mint]` under this
/// program.
#[derive(Accounts)]
pub struct EquipBadge<'info> {
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    pub passport: Box<Account<'info, Passport>>,
    /// CHECK: the passport's current badge mint.
    #[account(address = passport.badge_mint @ AgentsError::WrongAccount)]
    pub badge_mint: UncheckedAccount<'info>,
    /// CHECK: `["armory-caller", badge_mint]` under this program; signs by CPI.
    #[account(seeds = [ARMORY_CALLER_SEED, badge_mint.key().as_ref()], bump)]
    pub caller: UncheckedAccount<'info>,
    /// CHECK: the armory.
    #[account(address = ids::ARMORY_ID)]
    pub armory_program: UncheckedAccount<'info>,
}

pub fn process_equip_badge<'info>(ctx: Context<'info, EquipBadge<'info>>) -> Result<()> {
    let item = ctx.accounts.config.soulbound_item;
    require_keys_neq!(item, Pubkey::default(), AgentsError::NoSoulboundItem);
    let p = &ctx.accounts.passport;
    require!(p.status != status::RETIRED, AgentsError::BadStatus);
    require!(!p.badge_issued, AgentsError::BadgeIssued);
    let caller = ctx.accounts.caller.key();
    require!(
        ctx.remaining_accounts.iter().any(|a| *a.key == caller),
        AgentsError::WrongAccount
    );
    let data = hookwars_armory::instruction::EquipLaunch {
        entry: hookwars_armory::LaunchEquip {
            slot: 0,
            item: Some(item),
            config: EquipConfig::default(),
            notice_secs: 0,
            rule: None,
        },
    }
    .data();
    let accounts = ctx
        .remaining_accounts
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: a.is_signer || *a.key == caller,
            is_writable: a.is_writable,
        })
        .collect();
    let ix = Instruction {
        program_id: ids::ARMORY_ID,
        accounts,
        data,
    };
    let mut infos: Vec<AccountInfo> = ctx.remaining_accounts.to_vec();
    infos.push(ctx.accounts.caller.to_account_info());
    infos.push(ctx.accounts.armory_program.to_account_info());
    let mint = ctx.accounts.badge_mint.key();
    cpi(
        &ix,
        &infos,
        &[&[ARMORY_CALLER_SEED, mint.as_ref(), &[ctx.bumps.caller]]],
    )
}

// ---- issue_badge -----------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct IssueBadge<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    /// CHECK: the agent key (receives the badge).
    #[account(address = passport.agent_key @ AgentsError::WrongAccount)]
    pub agent_key: UncheckedAccount<'info>,
    /// CHECK: the badge mint.
    #[account(mut, address = passport.badge_mint @ AgentsError::WrongAccount)]
    pub badge_mint: UncheckedAccount<'info>,
    /// CHECK: the agent key's holding of the badge (created here).
    #[account(mut, address = token::holding_address(&passport.badge_mint, &passport.agent_key) @ AgentsError::WrongAccount)]
    pub badge_holding: UncheckedAccount<'info>,
    /// CHECK: the agents signer, the badge's mint authority.
    #[account(address = ids::AGENTS_SIGNER)]
    pub signer: UncheckedAccount<'info>,
    pub token: TokenAccs<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_issue_badge(ctx: Context<IssueBadge>) -> Result<()> {
    let item = ctx.accounts.config.soulbound_item;
    require_keys_neq!(item, Pubkey::default(), AgentsError::NoSoulboundItem);
    let p = &ctx.accounts.passport;
    require!(p.status != status::RETIRED, AgentsError::BadStatus);
    require!(!p.badge_issued, AgentsError::BadgeIssued);
    let mint_info = ctx.accounts.badge_mint.to_account_info();
    {
        let m = token::read_mint(&mint_info)?;
        require!(
            m.slot_count >= 1 && m.slots[0].item == item,
            AgentsError::BadgeNotEquipped
        );
    }
    let mint = ctx.accounts.badge_mint.key();
    let agent = ctx.accounts.agent_key.key();
    let holding = ctx.accounts.badge_holding.key();
    let signer = ctx.accounts.signer.key();
    let bump = [ctx.accounts.config.signer_bump];
    let seeds = signer_seeds(&bump);
    let infos = [
        ctx.accounts.payer.to_account_info(),
        ctx.accounts.agent_key.to_account_info(),
        mint_info.clone(),
        ctx.accounts.badge_holding.to_account_info(),
        ctx.accounts.signer.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.token.token_event_authority.to_account_info(),
        ctx.accounts.token.token_program.to_account_info(),
    ];
    cpi(&token::create_holding(ctx.accounts.payer.key(), mint, agent), &infos, &[])?;
    cpi(&token::mint_to(signer, mint, holding, None, vec![], 1), &infos, &[&seeds])?;
    cpi(
        &token::set_authority(signer, mint, AuthorityKind::Mint, None),
        &infos,
        &[&seeds],
    )?;
    let p = &mut ctx.accounts.passport;
    p.badge_issued = true;
    let generation = p.badge_generation;
    emit_cpi!(BadgeIssued {
        passport: p.key(),
        badge_mint: mint,
        agent_key: agent,
        generation,
        ts: now()?
    });
    Ok(())
}

// ---- update_profile --------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct UpdateProfile<'info> {
    pub operator: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    /// CHECK: the badge mint (its metadata follows the profile).
    #[account(mut, address = passport.badge_mint @ AgentsError::WrongAccount)]
    pub badge_mint: UncheckedAccount<'info>,
    /// CHECK: the agents signer, the badge's metadata authority.
    #[account(address = ids::AGENTS_SIGNER)]
    pub signer: UncheckedAccount<'info>,
    pub token: TokenAccs<'info>,
}

pub fn process_update_profile(ctx: Context<UpdateProfile>, args: ProfileArgs) -> Result<()> {
    args.check(&ctx.accounts.config.params)?;
    require!(
        ctx.accounts.passport.status != status::RETIRED,
        AgentsError::BadStatus
    );
    let bump = [ctx.accounts.config.signer_bump];
    let seeds = signer_seeds(&bump);
    let ix = token::update_metadata(
        ids::AGENTS_SIGNER,
        ctx.accounts.badge_mint.key(),
        Some(args.name.clone()),
        None,
        Some(args.avatar_uri.clone()),
    );
    cpi(
        &ix,
        &[
            ctx.accounts.signer.to_account_info(),
            ctx.accounts.badge_mint.to_account_info(),
            ctx.accounts.token.token_event_authority.to_account_info(),
            ctx.accounts.token.token_program.to_account_info(),
        ],
        &[&seeds],
    )?;
    let p = &mut ctx.accounts.passport;
    p.name = args.name.clone();
    p.avatar_uri = args.avatar_uri;
    p.bio_uri = args.bio_uri;
    p.hire_uri = args.hire_uri;
    p.kinds = args.kinds;
    p.credit_agent_id = args.credit_agent_id;
    let key = p.key();
    emit_cpi!(ProfileUpdated {
        passport: key,
        name: args.name,
        kinds: args.kinds,
        ts: now()?
    });
    Ok(())
}

// ---- set_status ------------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct SetStatus<'info> {
    pub operator: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, seeds = [OPERATOR_SEED, operator.key().as_ref()], bump = operator_index.bump)]
    pub operator_index: Box<Account<'info, OperatorIndex>>,
    /// CHECK: the badge mint.
    #[account(address = passport.badge_mint @ AgentsError::WrongAccount)]
    pub badge_mint: UncheckedAccount<'info>,
    /// CHECK: the agent key's badge holding (frozen on retirement once issued).
    #[account(mut, address = token::holding_address(&passport.badge_mint, &passport.agent_key) @ AgentsError::WrongAccount)]
    pub badge_holding: UncheckedAccount<'info>,
    /// CHECK: the agents signer, the badge's freeze authority.
    #[account(address = ids::AGENTS_SIGNER)]
    pub signer: UncheckedAccount<'info>,
    pub token: TokenAccs<'info>,
}

pub fn process_set_status(ctx: Context<SetStatus>, new: u8) -> Result<()> {
    require!(new <= status::RETIRED, AgentsError::BadStatus);
    let p = &ctx.accounts.passport;
    require!(p.status != status::RETIRED && p.status != new, AgentsError::BadStatus);
    if new == status::RETIRED {
        if p.badge_issued {
            let bump = [ctx.accounts.config.signer_bump];
            let seeds = signer_seeds(&bump);
            let ix = token::set_frozen(
                ids::AGENTS_SIGNER,
                ctx.accounts.badge_mint.key(),
                ctx.accounts.badge_holding.key(),
                true,
            );
            cpi(
                &ix,
                &[
                    ctx.accounts.signer.to_account_info(),
                    ctx.accounts.badge_mint.to_account_info(),
                    ctx.accounts.badge_holding.to_account_info(),
                    ctx.accounts.token.token_event_authority.to_account_info(),
                    ctx.accounts.token.token_program.to_account_info(),
                ],
                &[&seeds],
            )?;
        }
        let oi = &mut ctx.accounts.operator_index;
        oi.active = oi.active.saturating_sub(1);
    }
    let p = &mut ctx.accounts.passport;
    p.status = new;
    let key = p.key();
    emit_cpi!(PassportStatus {
        passport: key,
        status: new,
        ts: now()?
    });
    Ok(())
}

// ---- rotate_agent_key ------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct RotateAgentKey<'info> {
    pub operator: Signer<'info>,
    pub new_key: Signer<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, has_one = operator @ AgentsError::NotOperator)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, close = payer, seeds = [AGENT_KEY_SEED, passport.agent_key.as_ref()], bump = old_key_record.bump)]
    pub old_key_record: Box<Account<'info, AgentKey>>,
    #[account(init, payer = payer, space = 8 + AgentKey::INIT_SPACE,
        seeds = [AGENT_KEY_SEED, new_key.key().as_ref()], bump)]
    pub new_key_record: Box<Account<'info, AgentKey>>,
    /// CHECK: the current badge mint.
    #[account(address = passport.badge_mint @ AgentsError::WrongAccount)]
    pub old_badge_mint: UncheckedAccount<'info>,
    /// CHECK: the old key's badge holding (frozen when issued).
    #[account(mut, address = token::holding_address(&passport.badge_mint, &passport.agent_key) @ AgentsError::WrongAccount)]
    pub old_badge_holding: UncheckedAccount<'info>,
    /// CHECK: the next generation's badge mint (created here).
    #[account(mut)]
    pub new_badge_mint: UncheckedAccount<'info>,
    /// CHECK: the agents signer.
    #[account(address = ids::AGENTS_SIGNER)]
    pub signer: UncheckedAccount<'info>,
    pub token: TokenAccs<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_rotate(ctx: Context<RotateAgentKey>) -> Result<()> {
    let p = &ctx.accounts.passport;
    require!(p.status != status::RETIRED, AgentsError::BadStatus);
    let new_key = ctx.accounts.new_key.key();
    require_keys_neq!(new_key, p.operator, AgentsError::KeyIsOperator);
    let old_key = p.agent_key;
    let generation = p.badge_generation.checked_add(1).ok_or(AgentsError::MathOverflow)?;
    if p.badge_issued {
        let bump = [ctx.accounts.config.signer_bump];
        let seeds = signer_seeds(&bump);
        let ix = token::set_frozen(
            ids::AGENTS_SIGNER,
            ctx.accounts.old_badge_mint.key(),
            ctx.accounts.old_badge_holding.key(),
            true,
        );
        cpi(
            &ix,
            &[
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.old_badge_mint.to_account_info(),
                ctx.accounts.old_badge_holding.to_account_info(),
                ctx.accounts.token.token_event_authority.to_account_info(),
                ctx.accounts.token.token_program.to_account_info(),
            ],
            &[&seeds],
        )?;
    }
    let key = ctx.accounts.passport.key();
    let (name, uri) = (p.name.clone(), p.avatar_uri.clone());
    create_badge_mint(
        &key,
        generation,
        &ctx.accounts.new_badge_mint.to_account_info(),
        &ctx.accounts.payer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        &ctx.accounts.token,
        &name,
        &uri,
    )?;
    let r = &mut ctx.accounts.new_key_record;
    r.passport = key;
    r.bump = ctx.bumps.new_key_record;
    let t = now()?;
    let p = &mut ctx.accounts.passport;
    p.agent_key = new_key;
    p.badge_mint = ctx.accounts.new_badge_mint.key();
    p.badge_generation = generation;
    p.badge_issued = false;
    // The attestation's report data names the old key: it no longer counts (09 3.4).
    p.attested_until = 0;
    let changed = refresh(p, key, t);
    let badge = p.badge_mint;
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    emit_cpi!(AgentKeyRotated {
        passport: key,
        old_key,
        new_key,
        badge_mint: badge,
        generation,
        ts: t
    });
    Ok(())
}
