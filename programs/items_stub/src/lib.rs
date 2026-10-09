// Changed by Hookwars: new file, a test-only stand-in for hookwars_items' protocol signers (R20, R24).
//! `items_stub`: test-only, never deployed. Declared at the items program's id so the LiteSVM
//! suites can move tokens out of items PDAs (equip vaults, the pool-cuts vault) the way
//! `settle_equip` will (M3b): it forwards one token-program instruction, signing as the PDA its
//! seeds give under this id.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::invoke_signed;

declare_id!("8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv");

/// Instructions of the stub.
#[program]
pub mod items_stub {
    use super::*;

    /// Calls the token program with `data` and the remaining accounts (in the token
    /// instruction's order), signing as `PDA(seeds)` under this program.
    pub fn as_pda<'info>(
        ctx: Context<'info, Forward<'info>>,
        seeds: Vec<Vec<u8>>,
        data: Vec<u8>,
    ) -> Result<()> {
        forward(&ctx.accounts.token_program, ctx.remaining_accounts, &seeds, data)
    }
}

/// Forwards `data` to the token program, signing as `PDA(seeds)` under this program.
pub fn forward<'info>(
    token_program: &UncheckedAccount<'info>,
    remaining: &[AccountInfo<'info>],
    seeds: &[Vec<u8>],
    data: Vec<u8>,
) -> Result<()> {
    let refs: Vec<&[u8]> = seeds.iter().map(Vec::as_slice).collect();
    let (pda, bump) = Pubkey::find_program_address(&refs, &crate::ID);
    let metas = remaining
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: *a.key == pda || a.is_signer,
            is_writable: a.is_writable,
        })
        .collect();
    let mut infos: Vec<AccountInfo> = remaining.to_vec();
    infos.push(token_program.to_account_info());
    let ix = Instruction {
        program_id: bordrless_token::ID,
        accounts: metas,
        data,
    };
    let bump = [bump];
    let mut signer: Vec<&[u8]> = refs.clone();
    signer.push(&bump);
    invoke_signed(&ix, &infos, &[&signer])?;
    Ok(())
}

/// Accounts of `as_pda`; the token instruction's accounts follow as remaining.
#[derive(Accounts)]
pub struct Forward<'info> {
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
}
