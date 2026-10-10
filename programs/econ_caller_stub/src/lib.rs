// Changed by Hookwars: new file, a test-only stand-in (11, integration requests (economy)).
//! `econ_caller_stub`: test-only, never deployed. Lets the LiteSVM suites act as a protocol
//! program before the real items, war, market and armory make the economy calls:
//!
//! - `call(seed, data)`: calls the program in `target` with `data` and the remaining accounts,
//!   signing as `[seed]` under this program (so `["craft-caller"]` for craft `drop`/`wear` and
//!   `["social-caller"]` for social `record_wallet`).
//! - `mint_crafted(...)`: the entry point craft calls on its output program; logs what it got.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("D7wam6dwQDMApuXfgtnaEi8z3BoFh6GQ8tbnvLZDr5X1");


#[program]
pub mod econ_caller_stub {
    use super::*;

    /// Calls `target` with `data`, the remaining accounts in order, signing as `[seed]`.
    pub fn call<'info>(ctx: Context<'info, Call<'info>>, seed: Vec<u8>, data: Vec<u8>) -> Result<()> {
        let (pda, bump) = Pubkey::find_program_address(&[&seed], &crate::ID);
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
        infos.push(ctx.accounts.target.to_account_info());
        let ix = Instruction {
            program_id: ctx.accounts.target.key(),
            accounts: metas,
            data,
        };
        invoke_signed(&ix, &infos, &[&[&seed, &[bump]]])?;
        Ok(())
    }

    /// Stands in for the armory's `mint_crafted` (integration request): checks the craft signer
    /// signed and logs the request.
    pub fn mint_crafted(
        ctx: Context<MintCrafted>,
        crafter: Pubkey,
        template_id: u16,
        param_min: Vec<u32>,
        param_max: Vec<u32>,
        recipe_id: u16,
    ) -> Result<()> {
        msg!(
            "mint_crafted signer={} crafter={} template={} recipe={} min0={} max0={}",
            ctx.accounts.craft_signer.key(),
            crafter,
            template_id,
            recipe_id,
            param_min.first().copied().unwrap_or(0),
            param_max.first().copied().unwrap_or(0)
        );
        Ok(())
    }
}

/// Accounts of `call`; the target instruction's accounts follow as remaining.
#[derive(Accounts)]
pub struct Call<'info> {
    /// CHECK: the program to call (any).
    #[account(executable)]
    pub target: UncheckedAccount<'info>,
}

/// Accounts of `mint_crafted`.
#[derive(Accounts)]
pub struct MintCrafted<'info> {
    pub craft_signer: Signer<'info>,
}
