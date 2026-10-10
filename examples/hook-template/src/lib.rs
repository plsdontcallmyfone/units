//! units Hook Lab starter template: "Transfer Cut".
//!
//! An external hook template: a program of its own that implements the units slot ABI
//! (`bordrless_hook::TokenSlotArgs` in, `bordrless_hook::SlotReturn` as return data), so the token
//! program can call it from a slot of any units token. What it does:
//!
//! - `before_transfer`: takes `cut_bps` (the item's `params[0]`) of every wallet transfer as one
//!   delta into the slot's equip vault, and counts the sender's sends in its range of the
//!   sender's holding (`[tag = 1][count u32 le]`, 5 bytes). When a transfer empties the holding it
//!   writes zeros, so the holding can always close.
//!
//! Callback accounts: the token program's 5 prefix accounts (hook signer, mint, source,
//! destination, authority), then the extras its manifest names (`hooklab.json`): the armory
//! `Item` it reads its parameters from, then the equip vault it cuts into.
//!
//! The features `overcut`, `refuse`, `badwrite` and `cuburn` exist only so the Hook Lab can prove
//! it rejects broken templates; a real template has none of them.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use bordrless_hook::{Delta, SlotReturn, TokenSlotArgs, TokenSlotOp};

declare_id!("4NZsFq4R1UgcAzRKCeqcW8iyNvhcuLCfhGQ34Ey4gSce");

/// The units token program (`bordrless_token`).
pub const TOKEN_ID: Pubkey = Pubkey::from_str_const("5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618");
/// The units armory: owner of every `Item` account.
pub const ARMORY_ID: Pubkey = Pubkey::from_str_const("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");
/// `["hook-authority", this program]` under the token program: the only signer of our callbacks
/// (precomputed so no callback pays for a PDA search).
pub const TOKEN_HOOK_SIGNER: Pubkey =
    Pubkey::from_str_const("5evZ2Ca5VAuhT3s3MVgsCHLYrT7FmkasABKzv3weComA");
/// Byte offset of `params[0]` in an armory `Item` account (discriminator 8, version 1, bump 1,
/// item_mint 32, template_id 2).
pub const ITEM_PARAMS_OFFSET: usize = 44;
/// Index of the equip vault in the callback's account list (5 prefix accounts, then `item`).
pub const EQUIP_VAULT_INDEX: u8 = 6;
/// The range layout tag.
pub const TAG: u8 = 1;
/// Bytes of our range (without the token program's epoch byte).
pub const DATA_BYTES: usize = 5;

#[program]
pub mod hook_template_example {
    use super::*;

    /// Token slot callback: the cut and the send counter.
    pub fn before_transfer(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        require!(args.op == TokenSlotOp::Transfer, TemplateError::WrongOp);
        let item = &ctx.accounts.item;
        require_keys_eq!(item.key(), args.item, TemplateError::WrongItem);
        require_keys_eq!(*item.owner, ARMORY_ID, TemplateError::WrongItem);
        let cut_bps = {
            let data = item.try_borrow_data()?;
            require!(data.len() >= ITEM_PARAMS_OFFSET + 4, TemplateError::WrongItem);
            u32::from_le_bytes(data[ITEM_PARAMS_OFFSET..ITEM_PARAMS_OFFSET + 4].try_into().unwrap())
        };

        #[cfg(feature = "refuse")]
        if args.amount % 3 == 0 {
            return err!(TemplateError::Refused);
        }
        #[cfg(feature = "cuburn")]
        {
            let mut x: u64 = args.amount | 1;
            for i in 0..40_000u64 {
                x = x.rotate_left(7) ^ i.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            }
            msg!("burn {}", x & 1);
        }

        let mut cut = (u128::from(args.amount) * u128::from(cut_bps) / 10_000) as u64;
        #[cfg(feature = "overcut")]
        {
            cut = (u128::from(args.amount) * (u128::from(cut_bps) + 100) / 10_000) as u64;
        }
        if cut >= args.amount {
            cut = 0;
        }

        // The sender's send counter; zeros when this transfer empties the holding.
        let mut source = vec![0u8; DATA_BYTES];
        if args.source_balance > args.amount {
            let count = if args.source_data.first() == Some(&TAG) && args.source_data.len() >= DATA_BYTES {
                u32::from_le_bytes(args.source_data[1..5].try_into().unwrap())
            } else {
                0
            };
            source[0] = TAG;
            source[1..5].copy_from_slice(&count.saturating_add(1).to_le_bytes());
        }
        #[cfg(feature = "badwrite")]
        source.push(0xFF);

        let answer = SlotReturn {
            deltas: if cut > 0 {
                vec![Delta {
                    amount: cut,
                    account: EQUIP_VAULT_INDEX,
                }]
            } else {
                vec![]
            },
            source_data: Some(source),
            destination_data: None,
        };
        let mut out = Vec::new();
        answer.serialize(&mut out)?;
        set_return_data(&out);
        Ok(())
    }
}

/// A slot callback: the token program's prefix, then our extras.
#[derive(Accounts)]
pub struct SlotCallback<'info> {
    /// CHECK: the token program's signer for this program (address- and signer-checked).
    #[account(signer, address = TOKEN_HOOK_SIGNER @ TemplateError::BadHookSigner)]
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the mint.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: source holding.
    pub source: UncheckedAccount<'info>,
    /// CHECK: destination holding.
    pub destination: UncheckedAccount<'info>,
    /// CHECK: who signed.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: the armory `Item` (owner- and key-checked in the handler).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the slot's equip vault; the token program checks a cut goes there.
    pub equip_vault: UncheckedAccount<'info>,
}

#[error_code]
pub enum TemplateError {
    #[msg("the hook signer is not the token program's signer for this program")]
    BadHookSigner,
    #[msg("the item account is not the armory item this slot holds")]
    WrongItem,
    #[msg("this template only answers transfers")]
    WrongOp,
    #[msg("refused (Hook Lab fixture)")]
    Refused,
}
