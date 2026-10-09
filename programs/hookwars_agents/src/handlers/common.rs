// Changed by Hookwars: new file (09).
//! Helpers: CPIs into the token program, lamport moves, the proof recomputation.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

use crate::constants::*;
use crate::error::AgentsError;
use crate::events::ProofChanged;
use crate::state::Passport;

/// The token program's accounts every token CPI takes.
#[derive(Accounts)]
pub struct TokenAccs<'info> {
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    #[account(address = bordrless_token::client::event_authority())]
    pub token_event_authority: UncheckedAccount<'info>,
}

/// Invokes `ix` with `infos` (any order; every account `ix` names must be among them), signing
/// with `seeds`.
pub fn cpi(ix: &Instruction, infos: &[AccountInfo], seeds: &[&[&[u8]]]) -> Result<()> {
    invoke_signed(ix, infos, seeds).map_err(Into::into)
}

/// Seeds of the agents signer.
pub fn signer_seeds(bump: &[u8; 1]) -> [&[u8]; 2] {
    [SIGNER_SEED, bump]
}

/// Moves `amount` lamports out of `from` (an account this program owns) into `to`.
pub fn move_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let f = from.lamports().checked_sub(amount).ok_or(AgentsError::InsufficientFunds)?;
    let t = to.lamports().checked_add(amount).ok_or(AgentsError::MathOverflow)?;
    **from.try_borrow_mut_lamports()? = f;
    **to.try_borrow_mut_lamports()? = t;
    Ok(())
}

/// Recomputes `passport.proof` at `now`; emits `ProofChanged` when it moves.
pub fn refresh(passport: &mut Passport, key: Pubkey, now: i64) -> Option<ProofChanged> {
    let new = passport.proof_at(now);
    if new == passport.proof {
        return None;
    }
    let old = passport.proof;
    passport.proof = new;
    Some(ProofChanged {
        passport: key,
        old,
        new,
        ts: now,
    })
}

/// Checks a string against a bound.
pub fn bounded(s: &str, max: usize) -> Result<()> {
    require!(s.len() <= max, AgentsError::FieldTooLong);
    Ok(())
}

/// The current unix time.
pub fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}
