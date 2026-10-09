// Changed by Hookwars: new file, a test-only stand-in for the war program's loot signer.
//! `war_stub`: test-only, never deployed. Declared at the war program's id so the LiteSVM suites
//! can call the armory's `mint_loot` signed as `["loot-signer"]` before the war program exists (M4).

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2");

/// The armory.
pub const ARMORY_ID: Pubkey = Pubkey::from_str_const("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");

#[program]
pub mod war_stub {
    use super::*;

    /// Calls the armory with `data` and the remaining accounts, signing as `["loot-signer"]`.
    pub fn as_loot_signer<'info>(ctx: Context<'info, Forward<'info>>, data: Vec<u8>) -> Result<()> {
        let (pda, bump) = Pubkey::find_program_address(&[b"loot-signer"], &crate::ID);
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
        invoke_signed(&ix, &infos, &[&[b"loot-signer", &[bump]]])?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Forward<'info> {
    /// CHECK: the armory.
    #[account(address = ARMORY_ID)]
    pub armory: UncheckedAccount<'info>,
}
