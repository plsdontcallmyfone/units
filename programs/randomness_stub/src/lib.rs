// Changed by Hookwars: new file, a test-only randomness adapter (war suites).
//! `randomness_stub`: test-only, never deployed. A randomness adapter in the war program's
//! interface (`hookwars_war::oracle`): `request_randomness` creates `["randomness", requester]`
//! with the slot it was requested in; `fulfill` writes a value the test chooses, with the slot it
//! was written in. A real adapter (D-4) gets the value from an oracle instead.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("6ESx6cA1Hu7BiESMddfBSMwXAp1rNomvmutMHjAHs4x8");

#[program]
pub mod randomness_stub {
    use super::*;

    pub fn request_randomness(ctx: Context<RequestRandomness>, requester: Pubkey) -> Result<()> {
        require_keys_eq!(requester, ctx.accounts.requester.key(), StubError::BadCall);
        let r = &mut ctx.accounts.randomness;
        r.requester = requester;
        r.request_slot = Clock::get()?.slot;
        r.fulfilled = false;
        Ok(())
    }

    pub fn fulfill(ctx: Context<Fulfill>, value: [u8; 32]) -> Result<()> {
        let r = &mut ctx.accounts.randomness;
        r.fulfilled = true;
        r.fulfilled_slot = Clock::get()?.slot;
        r.value = value;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub requester: Signer<'info>,
    #[account(init, payer = payer, space = 8 + 32 + 8 + 1 + 8 + 32, seeds = [b"randomness", requester.key().as_ref()], bump)]
    pub randomness: Account<'info, Randomness>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Fulfill<'info> {
    pub authority: Signer<'info>,
    #[account(mut)]
    pub randomness: Account<'info, Randomness>,
}

/// The adapter's account (`hookwars_war::oracle::Randomness`).
#[account]
#[derive(Debug, Default)]
pub struct Randomness {
    pub requester: Pubkey,
    pub request_slot: u64,
    pub fulfilled: bool,
    pub fulfilled_slot: u64,
    pub value: [u8; 32],
}

#[error_code]
pub enum StubError {
    #[msg("bad call")]
    BadCall,
}
