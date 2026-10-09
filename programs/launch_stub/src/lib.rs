// Changed by Hookwars: new file, a test-only stand-in for the launchpad's armory caller.
//! `launch_stub`: test-only, never deployed. Declared at the launchpad's id so the LiteSVM suites
//! can call the armory's `equip_launch` signed as `["armory-caller", mint]` before the launchpad
//! does it itself (M3). It forwards one armory instruction.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD");

/// The armory.
pub const ARMORY_ID: Pubkey = Pubkey::from_str_const("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");

#[program]
pub mod launch_stub {
    use super::*;

    /// Calls the armory with `data` and the remaining accounts, signing as `["armory-caller", mint]`.
    pub fn as_armory_caller<'info>(
        ctx: Context<'info, Forward<'info>>,
        data: Vec<u8>,
    ) -> Result<()> {
        let mint = ctx.accounts.mint.key();
        let (pda, bump) = Pubkey::find_program_address(&[b"armory-caller", mint.as_ref()], &crate::ID);
        let metas = ctx
            .remaining_accounts
            .iter()
            .map(|a| AccountMeta {
                pubkey: *a.key,
                is_signer: *a.key == pda || a.is_signer,
                is_writable: a.is_writable,
            })
            .collect();
        let mut infos: Vec<AccountInfo> = ctx.remaining_accounts.to_vec();
        infos.push(ctx.accounts.armory.to_account_info());
        let ix = Instruction {
            program_id: ARMORY_ID,
            accounts: metas,
            data,
        };
        invoke_signed(&ix, &infos, &[&[b"armory-caller", mint.as_ref(), &[bump]]])?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Forward<'info> {
    /// CHECK: the mint whose caller signs.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: the armory.
    #[account(address = ARMORY_ID)]
    pub armory: UncheckedAccount<'info>,
}
