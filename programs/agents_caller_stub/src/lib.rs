// Changed by Hookwars: new file, a test-only stand-in (09 section 6.3).
//! `agents_caller_stub`: test-only, never deployed. Declared at the war program's id so the
//! LiteSVM suites can sign as `["agents-caller"]` under war and call `hookwars_agents::record`
//! before the real armory, war and items make that call (09 "Integration requests").

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2");

/// `hookwars_agents`.
pub const AGENTS_ID: Pubkey = Pubkey::from_str_const("GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9");

#[program]
pub mod agents_caller_stub {
    use super::*;

    /// Calls `hookwars_agents` with `data` and the remaining accounts (in that instruction's
    /// order), signing as `["agents-caller"]`.
    pub fn as_agents_caller<'info>(ctx: Context<'info, AsAgentsCaller<'info>>, data: Vec<u8>) -> Result<()> {
        let (pda, bump) = Pubkey::find_program_address(&[b"agents-caller"], &crate::ID);
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
        infos.push(ctx.accounts.agents_program.to_account_info());
        let ix = Instruction {
            program_id: AGENTS_ID,
            accounts: metas,
            data,
        };
        invoke_signed(&ix, &infos, &[&[b"agents-caller", &[bump]]])?;
        Ok(())
    }
}

/// Accounts of `as_agents_caller`; the agents instruction's accounts follow as remaining.
#[derive(Accounts)]
pub struct AsAgentsCaller<'info> {
    /// CHECK: the agents program.
    #[account(address = AGENTS_ID)]
    pub agents_program: UncheckedAccount<'info>,
}
