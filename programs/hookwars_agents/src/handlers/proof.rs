// Changed by Hookwars: new file (09).
//! Proof levels (09 section 5): links checked by ed25519 introspection, attestations bound to the
//! agent key and endorsed by a timelocked verifier set. The chain never says "verified AI".

use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::*;
use crate::handlers::common::*;
use crate::state::*;
use crate::AttestArgs;

/// Whether some ed25519 instruction before this one in the transaction verifies `message` signed
/// by `key`, with its signature, key and message all inside its own data.
pub fn ed25519_signed(sysvar: &AccountInfo, key: &Pubkey, message: &[u8]) -> Result<bool> {
    let current = solana_instructions_sysvar::load_current_index_checked(sysvar)?;
    for i in 0..usize::from(current) {
        let ix = solana_instructions_sysvar::load_instruction_at_checked(i, sysvar)?;
        if ix.program_id != ED25519_PROGRAM_ID {
            continue;
        }
        if ed25519_data_signs(&ix.data, key, message) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Parses the ed25519 program's data layout: a count, a padding byte, then per signature seven
/// u16 offsets (signature offset and instruction, key offset and instruction, message offset,
/// size and instruction). Only entries whose three parts live in this same instruction count.
pub fn ed25519_data_signs(data: &[u8], key: &Pubkey, message: &[u8]) -> bool {
    let u16_at = |o: usize| data.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]));
    let Some(&count) = data.first() else {
        return false;
    };
    for j in 0..usize::from(count) {
        let base = 2 + j * 14;
        let (Some(sig_ix), Some(pk_off), Some(pk_ix), Some(msg_off), Some(msg_len), Some(msg_ix)) = (
            u16_at(base + 2),
            u16_at(base + 4),
            u16_at(base + 6),
            u16_at(base + 8),
            u16_at(base + 10),
            u16_at(base + 12),
        ) else {
            return false;
        };
        if sig_ix != u16::MAX || pk_ix != u16::MAX || msg_ix != u16::MAX {
            continue;
        }
        let pk = data.get(usize::from(pk_off)..usize::from(pk_off) + 32);
        let msg = data.get(usize::from(msg_off)..usize::from(msg_off) + usize::from(msg_len));
        if pk == Some(key.as_ref()) && msg == Some(message) {
            return true;
        }
    }
    false
}

// ---- link_social -----------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
#[instruction(platform: u8)]
pub struct LinkSocial<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(init, payer = payer, space = 8 + Link::INIT_SPACE,
        seeds = [LINK_SEED, passport.key().as_ref(), &[platform]], bump)]
    pub link: Box<Account<'info, Link>>,
    /// CHECK: the instructions sysvar.
    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    pub instructions: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn process_link_social(
    ctx: Context<LinkSocial>,
    platform: u8,
    handle: String,
    post_uri: String,
) -> Result<()> {
    let params = ctx.accounts.config.params;
    require!(platform <= platform::MAX, AgentsError::BadPlatform);
    require!(!handle.is_empty(), AgentsError::FieldTooLong);
    bounded(&handle, usize::from(params.handle_max_len))?;
    bounded(&post_uri, usize::from(params.uri_max_len))?;
    let key = ctx.accounts.passport.key();
    require!(
        ctx.accounts.passport.status != status::RETIRED,
        AgentsError::BadStatus
    );
    let statement = link_statement(&key, platform, &handle);
    require!(
        ed25519_signed(
            &ctx.accounts.instructions.to_account_info(),
            &ctx.accounts.passport.agent_key,
            statement.as_bytes(),
        )?,
        AgentsError::BadLinkSignature
    );
    let t = now()?;
    let hash = sha256(statement.as_bytes());
    let l = &mut ctx.accounts.link;
    l.passport = key;
    l.platform = platform;
    l.handle = handle.clone();
    l.post_uri = post_uri.clone();
    l.statement_hash = hash;
    l.linked_at = t;
    l.bump = ctx.bumps.link;
    let p = &mut ctx.accounts.passport;
    p.links = p.links.saturating_add(1);
    let changed = refresh(p, key, t);
    emit_cpi!(LinkAdded {
        passport: key,
        platform,
        handle,
        post_uri,
        statement_hash: hash,
        ts: t
    });
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    Ok(())
}

// ---- unlink ----------------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
#[instruction(platform: u8)]
pub struct Unlink<'info> {
    /// The operator or the agent key.
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, close = authority, seeds = [LINK_SEED, passport.key().as_ref(), &[platform]], bump = link.bump)]
    pub link: Box<Account<'info, Link>>,
}

pub fn process_unlink(ctx: Context<Unlink>, platform: u8) -> Result<()> {
    let a = ctx.accounts.authority.key();
    let key = ctx.accounts.passport.key();
    let p = &mut ctx.accounts.passport;
    require!(a == p.operator || a == p.agent_key, AgentsError::NotOperator);
    p.links = p.links.saturating_sub(1);
    let t = now()?;
    let changed = refresh(p, key, t);
    emit_cpi!(LinkRemoved {
        passport: key,
        platform,
        ts: t
    });
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    Ok(())
}

// ---- submit_attestation ----------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct SubmitAttestation<'info> {
    #[account(mut)]
    pub agent_key: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut, constraint = passport.agent_key == agent_key.key() @ AgentsError::NotThisAgent)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(init_if_needed, payer = agent_key, space = 8 + Attestation::INIT_SPACE,
        seeds = [ATTEST_SEED, passport.key().as_ref()], bump)]
    pub attestation: Box<Account<'info, Attestation>>,
    pub system_program: Program<'info, System>,
}

pub fn process_submit_attestation(ctx: Context<SubmitAttestation>, args: AttestArgs) -> Result<()> {
    let params = ctx.accounts.config.params;
    let key = ctx.accounts.passport.key();
    let agent = ctx.accounts.agent_key.key();
    require!(
        ctx.accounts.passport.status == status::ACTIVE,
        AgentsError::AgentPaused
    );
    require!(args.tee_kind <= TEE_KIND_MAX, AgentsError::BadTeeKind);
    require!(
        args.report_data == report_data(&agent, &key, &args.nonce),
        AgentsError::BadReportData
    );
    let t = now()?;
    require!(
        args.expires_at > t && args.expires_at - t <= params.attest_max_ttl_secs,
        AgentsError::TtlTooLong
    );
    bounded(&args.quote_uri, usize::from(params.uri_max_len))?;
    bounded(&args.source_uri, usize::from(params.uri_max_len))?;
    let a = &mut ctx.accounts.attestation;
    a.passport = key;
    a.agent_key = agent;
    a.tee_kind = args.tee_kind;
    a.measurement = args.measurement;
    a.report_data = args.report_data;
    a.nonce = args.nonce;
    a.quote_hash = args.quote_hash;
    a.quote_uri = args.quote_uri;
    a.source_uri = args.source_uri;
    a.submitted_at = t;
    a.expires_at = args.expires_at;
    a.endorsements = 0;
    a.round = a.round.wrapping_add(1);
    a.bump = ctx.bumps.attestation;
    let round = a.round;
    // A new submission starts over: no endorsement of an earlier quote counts.
    let p = &mut ctx.accounts.passport;
    p.attested_until = 0;
    let changed = refresh(p, key, t);
    emit_cpi!(AttestationSubmitted {
        passport: key,
        tee_kind: args.tee_kind,
        measurement: args.measurement,
        quote_hash: args.quote_hash,
        expires_at: args.expires_at,
        round,
        ts: t
    });
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    Ok(())
}

// ---- endorse_attestation and revoke_endorsement ----------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct EndorseAttestation<'info> {
    #[account(mut)]
    pub verifier: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, seeds = [ATTEST_SEED, passport.key().as_ref()], bump = attestation.bump)]
    pub attestation: Box<Account<'info, Attestation>>,
    #[account(init_if_needed, payer = verifier, space = 8 + Endorsement::INIT_SPACE,
        seeds = [ENDORSE_SEED, attestation.key().as_ref(), verifier.key().as_ref()], bump)]
    pub endorsement: Box<Account<'info, Endorsement>>,
    pub system_program: Program<'info, System>,
}

pub fn process_endorse(ctx: Context<EndorseAttestation>) -> Result<()> {
    let v = ctx.accounts.verifier.key();
    let params = ctx.accounts.config.params;
    require!(
        ctx.accounts.config.verifiers.contains(&v),
        AgentsError::NotVerifier
    );
    let key = ctx.accounts.passport.key();
    require!(
        ctx.accounts.attestation.agent_key == ctx.accounts.passport.agent_key,
        AgentsError::StaleAttestation
    );
    let t = now()?;
    require!(ctx.accounts.attestation.expires_at > t, AgentsError::TtlTooLong);
    let round = ctx.accounts.attestation.round;
    let quote = ctx.accounts.attestation.quote_hash;
    let e = &mut ctx.accounts.endorsement;
    let fresh = e.attestation == Pubkey::default() || e.round != round;
    e.attestation = ctx.accounts.attestation.key();
    e.verifier = v;
    e.quote_hash = quote;
    e.round = round;
    e.endorsed_at = t;
    e.bump = ctx.bumps.endorsement;
    let a = &mut ctx.accounts.attestation;
    if fresh {
        a.endorsements = a.endorsements.saturating_add(1);
    }
    let count = a.endorsements;
    let expires = a.expires_at;
    let p = &mut ctx.accounts.passport;
    if count >= params.attest_quorum {
        p.attested_until = expires;
    }
    let changed = refresh(p, key, t);
    emit_cpi!(AttestationEndorsed {
        passport: key,
        verifier: v,
        quote_hash: quote,
        endorsements: count,
        ts: t
    });
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct RevokeEndorsement<'info> {
    #[account(mut)]
    pub verifier: Signer<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Box<Account<'info, AgentsConfig>>,
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
    #[account(mut, seeds = [ATTEST_SEED, passport.key().as_ref()], bump = attestation.bump)]
    pub attestation: Box<Account<'info, Attestation>>,
    #[account(mut, close = verifier,
        seeds = [ENDORSE_SEED, attestation.key().as_ref(), verifier.key().as_ref()], bump = endorsement.bump)]
    pub endorsement: Box<Account<'info, Endorsement>>,
}

pub fn process_revoke(ctx: Context<RevokeEndorsement>) -> Result<()> {
    let v = ctx.accounts.verifier.key();
    let key = ctx.accounts.passport.key();
    let t = now()?;
    let a = &mut ctx.accounts.attestation;
    if ctx.accounts.endorsement.round == a.round {
        a.endorsements = a.endorsements.saturating_sub(1);
    }
    let count = a.endorsements;
    let p = &mut ctx.accounts.passport;
    if count < ctx.accounts.config.params.attest_quorum {
        p.attested_until = 0;
    }
    let changed = refresh(p, key, t);
    emit_cpi!(EndorsementRevoked {
        passport: key,
        verifier: v,
        endorsements: count,
        ts: t
    });
    if let Some(e) = changed {
        emit_cpi!(e);
    }
    Ok(())
}

// ---- refresh_proof ---------------------------------------------------------------------------

#[event_cpi]
#[derive(Accounts)]
pub struct RefreshProof<'info> {
    #[account(mut)]
    pub passport: Box<Account<'info, Passport>>,
}

pub fn process_refresh(ctx: Context<RefreshProof>) -> Result<()> {
    let key = ctx.accounts.passport.key();
    if let Some(e) = refresh(&mut ctx.accounts.passport, key, now()?) {
        emit_cpi!(e);
    }
    Ok(())
}
