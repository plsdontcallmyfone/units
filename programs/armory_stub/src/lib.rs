// Changed by Hookwars: new file, a test-only stand-in for hookwars_armory.
//! `armory_stub`: test-only, never deployed. It is declared at the armory's program id
//! (`bordrless_token::constants::ARMORY_ID`) so the LiteSVM suites can sign as a mint's slot
//! authority, `["slots", mint]` under the armory, before the real armory exists (M2). It does one
//! thing: forward a token-program instruction, signing as that PDA.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");

/// Instructions of the stub.
#[program]
pub mod armory_stub {
    use super::*;

    /// Calls the token program with `data` and the remaining accounts (in the token
    /// instruction's order), signing as `["slots", mint]`.
    pub fn as_slot_authority<'info>(
        ctx: Context<'info, AsSlotAuthority<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        let mint = ctx.accounts.mint.key();
        let (pda, bump) =
            Pubkey::find_program_address(&[b"slots", mint.as_ref()], &crate::ID);
        let metas = ctx
            .remaining_accounts
            .iter()
            .map(|a| AccountMeta {
                pubkey: *a.key,
                is_signer: *a.key == pda,
                is_writable: a.is_writable,
            })
            .collect();
        let mut infos: Vec<AccountInfo> = ctx.remaining_accounts.to_vec();
        infos.push(ctx.accounts.token_program.to_account_info());
        let ix = Instruction {
            program_id: bordrless_token::ID,
            accounts: metas,
            data,
        };
        invoke_signed(&ix, &infos, &[&[b"slots", mint.as_ref(), &[bump]]])?;
        Ok(())
    }
}

/// Accounts of `as_slot_authority`; the token instruction's accounts follow as remaining.
#[derive(Accounts)]
pub struct AsSlotAuthority<'info> {
    /// CHECK: the mint whose slot authority signs.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
}
