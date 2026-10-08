// Changed by Hookwars: new file, a test-only slot item.
//! `slot_tester`: test-only, never deployed. A slot item in the slot convention
//! (`bordrless_hook::TokenSlotArgs`, `SlotReturn`) whose answers a test scripts per item and per
//! callback, and which records what each callback was told. The script of an item is at
//! `["script", item]`; it is the first extra account of every callback.
//!
//! Modes: answer nothing, return the scripted bytes, fail, or (touch only) return the scripted
//! bytes only when the caller is the key the script names.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use bordrless_hook::TokenSlotArgs;

declare_id!("DxNYhg2wiqR27YtzKntfuPfxfRb7T1GaEVPaaQ52NxxP");

/// `["script", item]`.
pub const SCRIPT_SEED: &[u8] = b"script";
/// Bytes allocated for a script.
pub const SCRIPT_SPACE: usize = 4_000;
/// The token program's signer of every callback to this program.
pub const TOKEN_HOOK_SIGNER: Pubkey =
    Pubkey::from_str_const("FicD5PxPmhYm7p8HJaR9TCgDP1khtwmhBCgcnzD61zWd");

/// Callback ids.
pub mod callback {
    pub const BEFORE_TRANSFER: u8 = 0;
    pub const AFTER_TRANSFER: u8 = 1;
    pub const BEFORE_BURN: u8 = 2;
    pub const AFTER_BURN: u8 = 3;
    pub const ON_TOUCH: u8 = 4;
}

/// What a scripted callback does.
pub mod mode {
    /// Answers nothing.
    pub const NONE: u8 = 0;
    /// Returns the scripted bytes.
    pub const RETURN: u8 = 1;
    /// Fails.
    pub const FAIL: u8 = 2;
    /// Returns the bytes after the first 32 only when the caller is the key in the first 32.
    pub const RETURN_IF_CALLER: u8 = 3;
}

/// Instructions.
#[program]
pub mod slot_tester {
    use super::*;

    /// Creates the script of `item`.
    pub fn init_script(ctx: Context<InitScript>, item: Pubkey) -> Result<()> {
        let script = &mut ctx.accounts.script;
        script.authority = ctx.accounts.payer.key();
        script.item = item;
        script.bump = ctx.bumps.script;
        script.calls = 0;
        script.answers = Vec::new();
        script.told = Vec::new();
        Ok(())
    }

    /// Sets what callback `which` does.
    pub fn set_answer(ctx: Context<SetAnswer>, which: u8, action: u8, data: Vec<u8>) -> Result<()> {
        let answers = &mut ctx.accounts.script.answers;
        answers.retain(|a| a.callback != which);
        answers.push(ScriptedAnswer {
            callback: which,
            mode: action,
            data,
        });
        Ok(())
    }

    pub fn before_transfer(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        run(ctx, callback::BEFORE_TRANSFER, &args)
    }

    pub fn after_transfer(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        run(ctx, callback::AFTER_TRANSFER, &args)
    }

    pub fn before_burn(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        run(ctx, callback::BEFORE_BURN, &args)
    }

    pub fn after_burn(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        run(ctx, callback::AFTER_BURN, &args)
    }

    pub fn on_touch(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        run(ctx, callback::ON_TOUCH, &args)
    }
}

fn run(ctx: Context<SlotCallback>, which: u8, args: &TokenSlotArgs) -> Result<()> {
    let script = &mut ctx.accounts.script;
    require_keys_eq!(script.item, args.item, TesterError::BadParams);
    script.calls += 1;
    let mut told = Vec::new();
    args.serialize(&mut told)?;
    match script.told.iter_mut().find(|t| t.callback == which) {
        Some(t) => t.args = told,
        None => script.told.push(Told {
            callback: which,
            args: told,
        }),
    }
    if let Some(answer) = script.answers.iter().find(|a| a.callback == which) {
        match answer.mode {
            mode::RETURN => set_return_data(&answer.data),
            mode::FAIL => return err!(TesterError::Refused),
            mode::RETURN_IF_CALLER => {
                let caller = Pubkey::try_from(&answer.data[..32]).map_err(|_| TesterError::BadParams)?;
                if caller == args.authority {
                    set_return_data(&answer.data[32..]);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// A script.
#[account]
#[derive(Debug)]
pub struct Script {
    pub authority: Pubkey,
    pub item: Pubkey,
    pub bump: u8,
    pub calls: u64,
    pub answers: Vec<ScriptedAnswer>,
    pub told: Vec<Told>,
}

impl Script {
    /// What callback `which` was last told.
    pub fn told(&self, which: u8) -> Option<TokenSlotArgs> {
        self.told
            .iter()
            .find(|t| t.callback == which)
            .and_then(|t| TokenSlotArgs::deserialize(&mut t.args.as_slice()).ok())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ScriptedAnswer {
    pub callback: u8,
    pub mode: u8,
    pub data: Vec<u8>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Told {
    pub callback: u8,
    pub args: Vec<u8>,
}

#[derive(Accounts)]
#[instruction(item: Pubkey)]
pub struct InitScript<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = SCRIPT_SPACE, seeds = [SCRIPT_SEED, item.as_ref()], bump)]
    pub script: Account<'info, Script>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetAnswer<'info> {
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ TesterError::BadParams)]
    pub script: Account<'info, Script>,
}

/// A slot callback: the token prefix, then the extras (the script first).
#[derive(Accounts)]
pub struct SlotCallback<'info> {
    /// CHECK: the token program's signer for this program (address- and signer-checked).
    #[account(signer, address = TOKEN_HOOK_SIGNER @ TesterError::BadHookSigner)]
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the mint.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: source.
    pub source: UncheckedAccount<'info>,
    /// CHECK: destination.
    pub destination: UncheckedAccount<'info>,
    /// CHECK: authority.
    pub authority: UncheckedAccount<'info>,
    #[account(mut, seeds = [SCRIPT_SEED, script.item.as_ref()], bump = script.bump)]
    pub script: Account<'info, Script>,
}

#[error_code]
pub enum TesterError {
    #[msg("bad parameters")]
    BadParams,
    #[msg("the hook signer is not the token program's signer for this program")]
    BadHookSigner,
    #[msg("the scripted callback refused")]
    Refused,
}

/// The script address of `item`.
pub fn script_address(item: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[SCRIPT_SEED, item.as_ref()], &crate::ID).0
}
