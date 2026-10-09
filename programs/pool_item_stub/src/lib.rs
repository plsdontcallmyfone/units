// Changed by Hookwars: new file, a test-only pool item for the launchpad's forwarding (M3b).
//! `pool_item_stub`: test-only, never deployed. A pool item with the launchpad's pool-item
//! protocol (`bordrless_hook::pool_item`): `pool_before_swap` and `pool_after_swap` answer what a
//! test scripted for the item and record what they were told. Replace with `hookwars_items`'
//! pool callbacks at integration.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::{ItemPoolAnswer, ItemPoolContext};
use bordrless_hook::PoolHookArgs;

declare_id!("BGeLKiiaGRyz7WTWh1Zf9CHgo7v92rmikCP3BARzv4HT");

/// The launchpad.
pub const LAUNCH_ID: Pubkey = Pubkey::from_str_const("fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD");

/// `["script", item]`.
pub fn script_address(item: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[b"script", item.as_ref()], &crate::ID).0
}

/// The launchpad's signer of this program's callbacks.
pub fn launch_signer() -> Pubkey {
    Pubkey::find_program_address(&[b"hook-authority", crate::ID.as_ref()], &LAUNCH_ID).0
}

/// What an item answers and what it saw.
#[account]
#[derive(InitSpace, Debug)]
pub struct Script {
    pub item: Pubkey,
    pub before: ItemPoolAnswerSpace,
    pub after: ItemPoolAnswerSpace,
    pub fail_before: bool,
    pub fail_after: bool,
    pub before_calls: u32,
    pub after_calls: u32,
    /// Sum of the cuts it answered (the `EquipState.pool_owed` it would record).
    pub collected: u64,
    pub last_slot: u8,
    pub last_item: Pubkey,
    pub last_launch_fee_bps: u16,
    pub last_launch_cut: u64,
    pub last_side_amount: u64,
    pub last_direction: u8,
    pub last_route_input: Pubkey,
    pub last_hop_count: u8,
    pub last_actor: Pubkey,
}

/// `ItemPoolAnswer` with a size.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ItemPoolAnswerSpace {
    pub discount_bps: u16,
    pub cut: u64,
    pub burn: u64,
}

impl From<ItemPoolAnswerSpace> for ItemPoolAnswer {
    fn from(a: ItemPoolAnswerSpace) -> Self {
        Self {
            discount_bps: a.discount_bps,
            cut: a.cut,
            burn: a.burn,
        }
    }
}

#[error_code]
pub enum StubError {
    #[msg("scripted failure")]
    Scripted,
    #[msg("the signer is not the launchpad's signer for this program")]
    BadSigner,
}

#[program]
pub mod pool_item_stub {
    use super::*;

    pub fn init_script(ctx: Context<InitScript>, item: Pubkey) -> Result<()> {
        ctx.accounts.script.item = item;
        Ok(())
    }

    pub fn set_script(
        ctx: Context<SetScript>,
        before: ItemPoolAnswerSpace,
        after: ItemPoolAnswerSpace,
        fail_before: bool,
        fail_after: bool,
    ) -> Result<()> {
        let s = &mut ctx.accounts.script;
        s.before = before;
        s.after = after;
        s.fail_before = fail_before;
        s.fail_after = fail_after;
        Ok(())
    }

    pub fn pool_before_swap(
        ctx: Context<Callback>,
        args: PoolHookArgs,
        item_ctx: ItemPoolContext,
    ) -> Result<ItemPoolAnswer> {
        answer(ctx, args, item_ctx, true)
    }

    pub fn pool_after_swap(
        ctx: Context<Callback>,
        args: PoolHookArgs,
        item_ctx: ItemPoolContext,
    ) -> Result<ItemPoolAnswer> {
        answer(ctx, args, item_ctx, false)
    }
}

fn answer(
    ctx: Context<Callback>,
    args: PoolHookArgs,
    item_ctx: ItemPoolContext,
    before: bool,
) -> Result<ItemPoolAnswer> {
    require_keys_eq!(ctx.accounts.signer.key(), launch_signer(), StubError::BadSigner);
    let s = &mut ctx.accounts.script;
    let (a, fail) = if before {
        s.before_calls += 1;
        (s.before, s.fail_before)
    } else {
        s.after_calls += 1;
        (s.after, s.fail_after)
    };
    require!(!fail, StubError::Scripted);
    s.collected = s.collected.saturating_add(a.cut);
    s.last_slot = item_ctx.slot;
    s.last_item = item_ctx.item;
    s.last_launch_fee_bps = item_ctx.launch_fee_bps;
    s.last_launch_cut = item_ctx.launch_cut;
    s.last_side_amount = item_ctx.side_amount;
    s.last_direction = args.direction;
    s.last_route_input = args.route.route_input_mint;
    s.last_hop_count = args.route.hop_count;
    s.last_actor = args.actor;
    Ok(a.into())
}

#[derive(Accounts)]
#[instruction(item: Pubkey)]
pub struct InitScript<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init, payer = payer, space = 8 + Script::INIT_SPACE, seeds = [b"script", item.as_ref()], bump)]
    pub script: Account<'info, Script>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetScript<'info> {
    pub payer: Signer<'info>,
    #[account(mut)]
    pub script: Account<'info, Script>,
}

#[derive(Accounts)]
pub struct Callback<'info> {
    pub signer: Signer<'info>,
    /// CHECK: the pool.
    pub pool: UncheckedAccount<'info>,
    /// CHECK: the base mint.
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the quote mint.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the trader.
    pub actor: UncheckedAccount<'info>,
    #[account(mut)]
    pub script: Account<'info, Script>,
}
