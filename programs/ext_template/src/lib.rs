// Changed by Hookwars: new file (protocol pass 4a), a test-only external template.
//! `ext_template`: test-only, never deployed. An external hook template as the Hook Lab admits
//! one (its own program, the units slot ABI): `before_transfer` cuts `params[0]` basis points of
//! every transfer into the slot's equip vault (the Hook Lab starter's rule), and
//! `pool_before_swap` / `pool_after_swap` cut `params[1]` basis points of the cut side's amount
//! into the launch's `PoolCuts` (the launchpad then records the cut in the items program).
//! Callback accounts: the caller's 5 prefix accounts, then `[Item, equip vault (token side only),
//! ...]` from the items program's registry; the rest is ignored.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use bordrless_hook::pool_item::{ItemPoolAnswer, ItemPoolContext};
use bordrless_hook::{Delta, PoolHookArgs, SlotReturn, TokenSlotArgs, TokenSlotOp};

declare_id!("DwYAoZnHU2piMTpTLbPksSbmkisfpvR99LwKs7E2vgzm");

/// The token program.
pub const TOKEN_ID: Pubkey = Pubkey::from_str_const("5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618");
/// The launchpad.
pub const LAUNCH_ID: Pubkey = Pubkey::from_str_const("fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD");
/// The armory (owner of every `Item`).
pub const ARMORY_ID: Pubkey = Pubkey::from_str_const("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");
/// Offset of `params[0]` in an armory `Item`.
pub const ITEM_PARAMS_OFFSET: usize = 44;
/// Index of the equip vault in a token callback (5 prefix accounts, then the item).
pub const EQUIP_VAULT_INDEX: u8 = 6;

fn signer_of(caller: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"hook-authority", crate::ID.as_ref()], caller).0
}

fn param(item: &AccountInfo, expect: &Pubkey, i: usize) -> Result<u32> {
    require_keys_eq!(*item.key, *expect, ExtError::WrongItem);
    require_keys_eq!(*item.owner, ARMORY_ID, ExtError::WrongItem);
    let d = item.try_borrow_data()?;
    let o = ITEM_PARAMS_OFFSET + 4 * i;
    require!(d.len() >= o + 4, ExtError::WrongItem);
    Ok(u32::from_le_bytes(d[o..o + 4].try_into().unwrap()))
}

#[error_code]
pub enum ExtError {
    #[msg("the signer is not the caller's signer for this program")]
    BadSigner,
    #[msg("the item account is not the item this slot holds")]
    WrongItem,
}

#[program]
pub mod ext_template {
    use super::*;

    /// Token slot callback: `params[0]` bps of the transfer into the equip vault.
    pub fn before_transfer(ctx: Context<TokenCallback>, args: TokenSlotArgs) -> Result<()> {
        let s = &ctx.accounts.hook_signer;
        require!(s.is_signer && *s.key == signer_of(&TOKEN_ID), ExtError::BadSigner);
        if args.op != TokenSlotOp::Transfer {
            return Ok(());
        }
        let bps = param(&ctx.accounts.item, &args.item, 0)?;
        let mut cut = (u128::from(args.amount) * u128::from(bps) / 10_000) as u64;
        if cut >= args.amount {
            cut = 0;
        }
        let answer = SlotReturn {
            deltas: if cut > 0 { vec![Delta { amount: cut, account: EQUIP_VAULT_INDEX }] } else { vec![] },
            source_data: None,
            destination_data: None,
        };
        let mut v = Vec::new();
        answer.serialize(&mut v)?;
        set_return_data(&v);
        Ok(())
    }

    pub fn pool_before_swap(ctx: Context<PoolCallback>, args: PoolHookArgs, item_ctx: ItemPoolContext) -> Result<()> {
        pool(ctx, args, item_ctx, true)
    }

    pub fn pool_after_swap(ctx: Context<PoolCallback>, args: PoolHookArgs, item_ctx: ItemPoolContext) -> Result<()> {
        pool(ctx, args, item_ctx, false)
    }
}

/// `params[1]` bps of the side amount, on the cut side only (a buy's input before, a sell's
/// output after).
fn pool(ctx: Context<PoolCallback>, args: PoolHookArgs, item_ctx: ItemPoolContext, before: bool) -> Result<()> {
    let s = &ctx.accounts.signer;
    require!(s.is_signer && *s.key == signer_of(&LAUNCH_ID), ExtError::BadSigner);
    let buy = args.direction == 1;
    let cut_side = (before && buy) || (!before && !buy);
    let bps = param(&ctx.accounts.item, &item_ctx.item, 1)?;
    let cut = if cut_side { (u128::from(item_ctx.side_amount) * u128::from(bps) / 10_000) as u64 } else { 0 };
    let a = ItemPoolAnswer { discount_bps: 0, cut, burn: 0 };
    let mut v = Vec::new();
    a.serialize(&mut v)?;
    set_return_data(&v);
    Ok(())
}

#[derive(Accounts)]
pub struct TokenCallback<'info> {
    /// CHECK: the token program's signer for this program.
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the mint.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: source holding.
    pub source: UncheckedAccount<'info>,
    /// CHECK: destination holding.
    pub destination: UncheckedAccount<'info>,
    /// CHECK: who signed.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: the armory `Item` (checked in the handler).
    pub item: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct PoolCallback<'info> {
    /// CHECK: the launchpad's signer for this program.
    pub signer: UncheckedAccount<'info>,
    /// CHECK: the pool.
    pub pool: UncheckedAccount<'info>,
    /// CHECK: the base mint.
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the quote mint.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the trader.
    pub actor: UncheckedAccount<'info>,
    /// CHECK: the armory `Item` (checked in the handler).
    pub item: UncheckedAccount<'info>,
}
