// Changed by Hookwars: new file (protocol pass 4a, docs/spec/14-pass-4a.md): access modes (E-1, 11
// section 1), level gates (E-8, 11 section 3.3), the admin queue (review 1 L-1) and template
// submissions with their bond (10 section 11.5).
//! Access modes, the admin queue and template submissions.
//!
//! **Access.** An item's mode lives in `Item.access_mode` and `Item.exclusive` (mirrored in its
//! `AccessPolicy`, which also holds the licence terms), so an equip path that sees `Open` needs no
//! further account. Any other mode needs the access proof suffix `[<ARMORY_ID>, proof]` at the end
//! of the equip instruction's remaining accounts (before the agent attribution suffix): the
//! `Approval` (Gated), the market's `License` (Licensed) or the market's `Lease` (Leased). The
//! check runs at `equip_launch`, `propose` and `execute`; reverts never check it (R38), and nothing
//! is checked on transfers or swaps. `enforce_access` removes an item whose approval or licence
//! lapsed, once the slot's notice has passed since the lapse (R39).
//!
//! **Admin queue.** `queue_admin(hash)` opens `["queued", hash]` ready after
//! `admin_timelock_secs`; the gated instruction recomputes the hash from its own instruction data
//! (`sha256(data || bound keys)`) and closes the entry. The gated instructions are
//! `register_template`, `register_external_template`, `retire_template`, `set_template_economy`,
//! `set_item_protocol_bps`, `register_preset` and `set_access_params`.

use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use bordrless_token::state::Mint;
use hookwars_common::access::{self as acc, APPROVAL_SEED, ACCESS_SEED, QUEUED_SEED};
use hookwars_common::{ids, pda, seeds};

use crate::cpi::read_mint;
use crate::error::ArmoryError;
use crate::events::*;
use crate::state::*;

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

/// The hash a queue entry names: `sha256(instruction data || bound keys)`.
pub fn action_hash(data: &[u8], bound: &[Pubkey]) -> [u8; 32] {
    let mut v = data.to_vec();
    for k in bound {
        v.extend_from_slice(k.as_ref());
    }
    solana_sha256_hasher::hash(&v).to_bytes()
}

/// Client side: the hash of a gated instruction (its `data()`) with its bound keys.
pub fn action_hash_of<I: InstructionData>(ix: &I, bound: &[Pubkey]) -> [u8; 32] {
    action_hash(&ix.data(), bound)
}

/// Checks a queue entry against the action about to apply: the right address, ready, same admin.
/// The caller's context closes it to the admin.
pub fn consume_queued(q: &Account<QueuedAction>, hash: [u8; 32], admin: &Pubkey) -> Result<()> {
    require!(q.action_hash == hash, ArmoryError::NotQueued);
    require_keys_eq!(q.key(), acc::queued_address(&hash).0, ArmoryError::NotQueued);
    require_keys_eq!(q.admin, *admin, ArmoryError::NotQueued);
    let ts = now()?;
    require!(ts >= q.ready_at, ArmoryError::Timelock);
    emit!(AdminActionApplied { action_hash: hash, ts });
    Ok(())
}

/// The equip gate's access row (11 section 1.3): `item` may go into `slot` of `token_mint`.
pub fn check_access(item: &Item, item_key: &Pubkey, token_mint: &Pubkey, slot: u8, proof: Option<&AccountInfo>, ts: i64) -> Result<()> {
    if item.exclusive {
        require!(item.equipped_count == 0, ArmoryError::ExclusiveInUse);
    }
    match item.access_mode {
        acc::OPEN | acc::EXCLUSIVE => Ok(()),
        acc::GATED => {
            let p = proof.ok_or(ArmoryError::AccessDenied)?;
            require_keys_eq!(p.key(), acc::approval_address(item_key, token_mint).0, ArmoryError::AccessDenied);
            require!(*p.owner == crate::ID && !p.data_is_empty(), ArmoryError::AccessDenied);
            let a = Approval::try_deserialize(&mut &p.try_borrow_data()?[..]).map_err(|_| error!(ArmoryError::AccessDenied))?;
            require!(a.item == *item_key && a.token_mint == *token_mint && a.live_at(ts), ArmoryError::AccessDenied);
            Ok(())
        }
        acc::LICENSED => {
            let p = proof.ok_or(ArmoryError::AccessDenied)?;
            let l = acc::read_license(p, item_key, token_mint).ok_or(ArmoryError::AccessDenied)?;
            require!(l.live_at(ts), ArmoryError::AccessDenied);
            Ok(())
        }
        acc::LEASED => {
            let p = proof.ok_or(ArmoryError::AccessDenied)?;
            require_keys_eq!(p.key(), hookwars_common::market::lease(item_key), ArmoryError::AccessDenied);
            let l = hookwars_common::market::read_lease(p).ok_or(ArmoryError::AccessDenied)?;
            require!(
                l.state == hookwars_common::market::LEASE_ACTIVE && l.token_mint == *token_mint && l.slot == slot,
                ArmoryError::AccessDenied
            );
            Ok(())
        }
        _ => err!(ArmoryError::AccessDenied),
    }
}

/// The signer's standing toward an item: its holder, or the operator or key of the agent whose
/// vault holds it (then the agent's live directive constrains what it may set).
fn holder_standing(item: &Item, holding: &AccountInfo, signer: &Pubkey, agent: Option<&[AccountInfo]>) -> Result<Option<acc::DirectiveView>> {
    require_keys_eq!(*holding.owner, bordrless_token::ID, ArmoryError::NotItemHolder);
    let h = bordrless_token::client::read_holding(holding).map_err(|_| error!(ArmoryError::NotItemHolder))?;
    require!(h.mint == item.item_mint && h.amount == 1, ArmoryError::NotItemHolder);
    require_keys_eq!(holding.key(), pda::holding(&item.item_mint, &h.owner), ArmoryError::NotItemHolder);
    if h.owner == *signer {
        return Ok(None);
    }
    // `[<AGENTS_ID>, passport, directive]`: the agent's vault holds the item.
    let s = agent.ok_or(ArmoryError::NotItemHolder)?;
    let (passport, directive) = (&s[1], &s[2]);
    require_keys_eq!(h.owner, acc::agent_vault(passport.key), ArmoryError::NotItemHolder);
    require_keys_eq!(*passport.owner, ids::AGENTS_ID, ArmoryError::NotItemHolder);
    let d = passport.try_borrow_data()?;
    // Passport: discriminator, version, bump, operator (32), index (4), agent_key (32).
    require!(d.len() >= 78, ArmoryError::NotItemHolder);
    let operator = Pubkey::try_from(&d[10..42]).map_err(|_| error!(ArmoryError::NotItemHolder))?;
    let agent_key = Pubkey::try_from(&d[46..78]).map_err(|_| error!(ArmoryError::NotItemHolder))?;
    require!(*signer == operator || *signer == agent_key, ArmoryError::NotItemHolder);
    let v = acc::read_directive(directive, passport.key).ok_or(ArmoryError::DirectiveForbids)?;
    require!(!v.frozen, ArmoryError::DirectiveForbids);
    Ok(Some(v))
}

/// Splits `[<AGENTS_ID>, passport, directive]` and `[<SOCIAL_ID>, skills, profile]` off `rem`.
fn split_gates<'a, 'info>(rem: &'a [AccountInfo<'info>]) -> (Option<&'a [AccountInfo<'info>]>, Option<&'a [AccountInfo<'info>]>) {
    let (rem, social) = hookwars_common::eco_cpi::split_tagged(rem, &hookwars_common::economy::SOCIAL_ID, 3);
    let (_, agent) = hookwars_common::eco_cpi::split_tagged(rem, &ids::AGENTS_ID, 3);
    (agent, social)
}

fn builder_level(social: Option<&[AccountInfo]>, wallet: &Pubkey) -> u8 {
    social
        .and_then(|s| acc::wallet_level(&s[2], &s[1], wallet, hookwars_common::economy::skill::BUILDER))
        .unwrap_or(0)
}

// ------------------------------------------------------------------------------ admin queue

#[event_cpi]
#[derive(Accounts)]
#[instruction(action_hash: [u8; 32])]
pub struct QueueAdmin<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init, payer = admin, space = 8 + QueuedAction::INIT_SPACE, seeds = [QUEUED_SEED, action_hash.as_ref()], bump)]
    pub queued: Box<Account<'info, QueuedAction>>,
    pub system_program: Program<'info, System>,
}

pub fn process_queue_admin(ctx: Context<QueueAdmin>, action_hash: [u8; 32]) -> Result<()> {
    let ready_at = now()? + i64::from(ctx.accounts.config.params.admin_timelock_secs);
    let q = &mut ctx.accounts.queued;
    q.action_hash = action_hash;
    q.admin = ctx.accounts.admin.key();
    q.ready_at = ready_at;
    q.bump = ctx.bumps.queued;
    emit_cpi!(AdminActionQueued { action_hash, ready_at });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct CancelAdmin<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, close = admin, seeds = [QUEUED_SEED, queued.action_hash.as_ref()], bump = queued.bump)]
    pub queued: Box<Account<'info, QueuedAction>>,
}

pub fn process_cancel_admin(ctx: Context<CancelAdmin>) -> Result<()> {
    emit_cpi!(AdminActionCancelled { action_hash: ctx.accounts.queued.action_hash });
    Ok(())
}

/// A gated admin setter on the config.
#[event_cpi]
#[derive(Accounts)]
pub struct AdminQueued<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, close = admin)]
    pub queued: Box<Account<'info, QueuedAction>>,
}

pub fn process_set_access_params(ctx: Context<AdminQueued>, params: AccessParams) -> Result<()> {
    let hash = action_hash_of(&crate::instruction::SetAccessParams { params }, &[]);
    consume_queued(&ctx.accounts.queued, hash, &ctx.accounts.admin.key())?;
    require!(
        params.licence_min_secs <= params.licence_max_secs && params.lab_bond_discount_bps <= 10_000,
        ArmoryError::InvalidParams
    );
    ctx.accounts.config.access = params;
    emit_cpi!(AccessParamsSet { params });
    Ok(())
}

// ------------------------------------------------------------------------------ access

#[event_cpi]
#[derive(Accounts)]
pub struct SetAccess<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, seeds = [seeds::ITEM, item.item_mint.as_ref()], bump = item.bump)]
    pub item: Box<Account<'info, Item>>,
    #[account(seeds = [seeds::TEMPLATE, &item.template_id.to_le_bytes()], bump = template.bump)]
    pub template: Box<Account<'info, Template>>,
    /// CHECK: the holding of the item mint that holds it (read in the handler).
    pub item_holding: UncheckedAccount<'info>,
    #[account(init_if_needed, payer = signer, space = 8 + AccessPolicy::INIT_SPACE,
        seeds = [ACCESS_SEED, item.key().as_ref()], bump)]
    pub policy: Box<Account<'info, AccessPolicy>>,
    /// CHECK: `["lease", item]` under the market: a lease that exists refuses the change.
    #[account(address = hookwars_common::market::lease(&item.key()) @ ArmoryError::WrongAccount)]
    pub lease: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `set_access(mode, exclusive, licence_terms)` (11 section 1.3, E-8). Remaining accounts: the
/// optional agent suffix `[<AGENTS_ID>, passport, directive]` when an agent's vault holds the item,
/// then the optional level suffix `[<SOCIAL_ID>, skills, profile]` (required for a licence priced
/// above `licence_tier_1_lamports` while `licence_tier_1_level` is above 0).
pub fn process_set_access<'info>(
    ctx: Context<'info, SetAccess<'info>>,
    mode: u8,
    exclusive: bool,
    licence_terms: Option<LicenceTerms>,
) -> Result<()> {
    let a = &ctx.accounts;
    let signer = a.signer.key();
    let (agent, social) = split_gates(ctx.remaining_accounts);
    let directive = holder_standing(&a.item, &a.item_holding.to_account_info(), &signer, agent)?;
    // A lease that exists (offered, active or ended but not closed) keeps the mode as it is.
    require!(
        hookwars_common::market::read_lease(&a.lease.to_account_info()).is_none(),
        ArmoryError::ItemLeasedElsewhere
    );
    let t = &a.template;
    require!(acc::mode_allowed(t.allowed_access, mode), ArmoryError::AccessNotAllowed);
    let exclusive = exclusive || mode == acc::EXCLUSIVE;
    if exclusive {
        require!(acc::mode_allowed(t.allowed_access, acc::EXCLUSIVE), ArmoryError::AccessNotAllowed);
    }
    // Leased is the market's flow (10 section 4); it is not set here.
    require!(mode != acc::LEASED, ArmoryError::AccessNotAllowed);
    let p = &a.config.access;
    match (mode, licence_terms) {
        (acc::LICENSED, Some(lt)) => {
            require!(
                lt.price_lamports > 0
                    && lt.term_secs >= p.licence_min_secs
                    && lt.term_secs <= p.licence_max_secs
                    && p.licence_max_secs > 0
                    && lt.per <= 1
                    && lt.max_live >= 1
                    && (!exclusive || lt.max_live == 1),
                ArmoryError::BadLicenceTerms
            );
            // E-8: a licence above tier 1 needs the Builder level.
            if lt.price_lamports > p.licence_tier_1_lamports && p.licence_tier_1_level > 0 {
                require!(builder_level(social, &signer) >= p.licence_tier_1_level, ArmoryError::LevelTooLow);
            }
        }
        (acc::LICENSED, None) => return err!(ArmoryError::BadLicenceTerms),
        (_, Some(_)) => return err!(ArmoryError::BadLicenceTerms),
        (_, None) => {}
    }
    if let Some(d) = directive {
        require!(d.allowed_access_modes & (1u8 << mode) != 0, ArmoryError::DirectiveForbids);
        if exclusive {
            require!(d.allowed_access_modes & (1u8 << acc::EXCLUSIVE) != 0, ArmoryError::DirectiveForbids);
        }
        if let Some(lt) = licence_terms {
            require!(lt.price_lamports <= d.max_licence_price, ArmoryError::DirectiveForbids);
        }
    }
    let ts = now()?;
    let item_key = a.item.key();
    let pol = &mut ctx.accounts.policy;
    pol.item = item_key;
    pol.mode = mode;
    pol.exclusive = exclusive;
    pol.licence_terms = licence_terms;
    pol.holder_at_set = signer;
    pol.updated_at = ts;
    pol.bump = ctx.bumps.policy;
    let it = &mut ctx.accounts.item;
    it.access_mode = mode;
    it.exclusive = exclusive;
    emit_cpi!(AccessSet { item: item_key, holder: signer, mode, exclusive, licence_terms, ts });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(token_mint: Pubkey)]
pub struct Approve<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(seeds = [seeds::ITEM, item.item_mint.as_ref()], bump = item.bump)]
    pub item: Box<Account<'info, Item>>,
    /// CHECK: the holding of the item mint that holds it.
    pub item_holding: UncheckedAccount<'info>,
    #[account(init_if_needed, payer = signer, space = 8 + Approval::INIT_SPACE,
        seeds = [APPROVAL_SEED, item.key().as_ref(), token_mint.as_ref()], bump)]
    pub approval: Box<Account<'info, Approval>>,
    pub system_program: Program<'info, System>,
}

/// `approve(token_mint)`: the holder approves a token for a Gated item (a revoked approval is
/// renewed). Remaining accounts: the optional agent suffix.
pub fn process_approve<'info>(ctx: Context<'info, Approve<'info>>, token_mint: Pubkey) -> Result<()> {
    let a = &ctx.accounts;
    let signer = a.signer.key();
    let (agent, _) = split_gates(ctx.remaining_accounts);
    if let Some(d) = holder_standing(&a.item, &a.item_holding.to_account_info(), &signer, agent)? {
        require!(d.allowed_access_modes & (1u8 << acc::GATED) != 0, ArmoryError::DirectiveForbids);
    }
    require!(a.item.access_mode == acc::GATED, ArmoryError::WrongAccessMode);
    let ts = now()?;
    let item = a.item.key();
    let ap = &mut ctx.accounts.approval;
    ap.item = item;
    ap.token_mint = token_mint;
    ap.approved_by = signer;
    ap.approved_at = ts;
    ap.revoke_after = 0;
    ap.bump = ctx.bumps.approval;
    emit_cpi!(Approved { item, token_mint, by: signer, ts });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct RevokeApproval<'info> {
    pub signer: Signer<'info>,
    #[account(seeds = [seeds::ITEM, item.item_mint.as_ref()], bump = item.bump)]
    pub item: Box<Account<'info, Item>>,
    /// CHECK: the holding of the item mint that holds it.
    pub item_holding: UncheckedAccount<'info>,
    #[account(mut, seeds = [APPROVAL_SEED, item.key().as_ref(), approval.token_mint.as_ref()], bump = approval.bump)]
    pub approval: Box<Account<'info, Approval>>,
    /// CHECK: the approved token (read with `read_mint`).
    #[account(address = approval.token_mint @ ArmoryError::WrongAccount)]
    pub token_mint: UncheckedAccount<'info>,
}

/// `revoke_approval`: removal becomes possible after the longest notice of the token's slots that
/// hold the item (R39); remaining accounts: the agent suffix (optional), then each such slot's
/// `SlotState`, in slot order.
pub fn process_revoke_approval<'info>(ctx: Context<'info, RevokeApproval<'info>>) -> Result<()> {
    let a = &ctx.accounts;
    let signer = a.signer.key();
    let rem = ctx.remaining_accounts;
    let (rem, agent) = hookwars_common::eco_cpi::split_tagged(rem, &ids::AGENTS_ID, 3);
    holder_standing(&a.item, &a.item_holding.to_account_info(), &signer, agent)?;
    let ts = now()?;
    require!(a.approval.revoke_after == 0, ArmoryError::ApprovalNotLive);
    let item = a.item.key();
    let mint_key = a.token_mint.key();
    let notice = if *a.token_mint.owner == bordrless_token::ID {
        let mint = read_mint(&a.token_mint.to_account_info())?;
        max_notice(&mint, &mint_key, &item, rem)?
    } else {
        0
    };
    let revoke_after = ts + i64::from(notice);
    ctx.accounts.approval.revoke_after = revoke_after.max(ts);
    emit_cpi!(ApprovalRevoked { item, token_mint: mint_key, revoke_after, ts });
    Ok(())
}

/// The longest `notice_secs` among the slots of `mint` holding `item`; every such slot's
/// `SlotState` must be in `states`, in slot order.
fn max_notice(mint: &Mint, mint_key: &Pubkey, item: &Pubkey, states: &[AccountInfo]) -> Result<u32> {
    let mut it = states.iter();
    let mut notice = 0u32;
    for i in 0..mint.slot_count {
        if mint.slots[usize::from(i)].item != *item {
            continue;
        }
        let s = it.next().ok_or(ArmoryError::WrongAccount)?;
        require_keys_eq!(s.key(), pda::slot_state(mint_key, i).0, ArmoryError::WrongAccount);
        require_keys_eq!(*s.owner, crate::ID, ArmoryError::WrongAccount);
        let st = SlotState::try_deserialize(&mut &s.try_borrow_data()?[..])?;
        notice = notice.max(st.notice_secs);
    }
    Ok(notice)
}

// ------------------------------------------------------------------------------ submissions

/// `["submission-tpl", program]`.
pub const SUBMISSION_SEED: &[u8] = b"submission-tpl";

#[event_cpi]
#[derive(Accounts)]
#[instruction(template_program: Pubkey)]
pub struct SubmitTemplate<'info> {
    #[account(mut)]
    pub submitter: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(init, payer = submitter, space = 8 + TemplateSubmission::INIT_SPACE,
        seeds = [SUBMISSION_SEED, template_program.as_ref()], bump)]
    pub submission: Box<Account<'info, TemplateSubmission>>,
    pub system_program: Program<'info, System>,
}

/// `submit_template(program, code_hash, uri_hash)` (10 section 11.5, E-8): posts the bond
/// (`lab_bond_lamports`, less `lab_bond_discount_bps` for a submitter at Builder level
/// `lab_bond_discount_level` or above, read from the optional level suffix `[<SOCIAL_ID>, skills,
/// profile]`). The lab service runs the suite; the admin then registers the template through the
/// queue and closes the submission with `settle_submission`.
pub fn process_submit_template<'info>(
    ctx: Context<'info, SubmitTemplate<'info>>,
    template_program: Pubkey,
    code_hash: [u8; 32],
    uri_hash: [u8; 32],
) -> Result<()> {
    let program = template_program;
    let p = ctx.accounts.config.access;
    let submitter = ctx.accounts.submitter.key();
    let (_, social) = split_gates(ctx.remaining_accounts);
    let mut bond = p.lab_bond_lamports;
    if p.lab_bond_discount_level > 0 && builder_level(social, &submitter) >= p.lab_bond_discount_level {
        bond -= hookwars_common::economy::bps(bond, p.lab_bond_discount_bps);
    }
    require!(code_hash != [0; 32] && program != Pubkey::default(), ArmoryError::InvalidSchema);
    if bond > 0 {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.key(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.submitter.to_account_info(),
                    to: ctx.accounts.submission.to_account_info(),
                },
            ),
            bond,
        )?;
    }
    let ts = now()?;
    let key = ctx.accounts.submission.key();
    let s = &mut ctx.accounts.submission;
    s.program = program;
    s.code_hash = code_hash;
    s.uri_hash = uri_hash;
    s.submitter = submitter;
    s.bond = bond;
    s.submitted_at = ts;
    s.bump = ctx.bumps.submission;
    emit_cpi!(TemplateSubmitted { submission: key, program, code_hash, uri_hash, submitter, bond, ts });
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct SettleSubmission<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ ArmoryError::NotAdmin)]
    pub config: Box<Account<'info, ArmoryConfig>>,
    #[account(mut, close = submitter, seeds = [SUBMISSION_SEED, submission.program.as_ref()], bump = submission.bump)]
    pub submission: Box<Account<'info, TemplateSubmission>>,
    /// CHECK: receives the rent, and the bond unless forfeited.
    #[account(mut, address = submission.submitter @ ArmoryError::WrongAccount)]
    pub submitter: UncheckedAccount<'info>,
    /// The registered template, when approving (its program and code hash must match).
    pub template: Option<Box<Account<'info, Template>>>,
}

/// `settle_submission(approved, forfeit)`: approving needs the registered `Template` for the
/// submitted program and code hash; the bond goes back. Rejecting refunds it, unless `forfeit`
/// (the submission failed a check the lab published), when it goes to the admin.
pub fn process_settle_submission(ctx: Context<SettleSubmission>, approved: bool, forfeit: bool) -> Result<()> {
    let s = &ctx.accounts.submission;
    if approved {
        let t = ctx.accounts.template.as_ref().ok_or(ArmoryError::SubmissionClosed)?;
        require!(t.program == s.program && t.code_hash == s.code_hash && !forfeit, ArmoryError::SubmissionClosed);
    }
    let bond = s.bond;
    let forfeited = !approved && forfeit;
    if forfeited && bond > 0 {
        let from = ctx.accounts.submission.to_account_info();
        let to = ctx.accounts.admin.to_account_info();
        **from.try_borrow_mut_lamports()? -= bond;
        **to.try_borrow_mut_lamports()? += bond;
    }
    emit_cpi!(SubmissionSettled {
        submission: ctx.accounts.submission.key(),
        program: ctx.accounts.submission.program,
        approved,
        forfeited,
        bond,
        ts: now()?,
    });
    Ok(())
}
