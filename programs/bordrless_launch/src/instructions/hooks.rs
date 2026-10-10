// Changed by Hookwars: M3b forwarding to pool items on slot launches (spec 03 section 5).
// Changed by Hookwars: protocol pass 4a (Hook Lab gap 4): an external template's pool cut is recorded in the
// items program, which keeps its equip state, so settle_equip can pay it.
//! The pool hook callbacks (`docs/hooks-v2.md` §5.4): the DEX calls these, signing with its
//! signer for this program, `["hook-authority", LAUNCH_ID]` under the DEX ([`DEX_HOOK_AUTHORITY`]).
//! Only the DEX can sign for it, and only when it calls this program as a pool's hook: the signer
//! it gives any other pool hook is that hook's own, so a hook that passes its signer on in a CPI
//! here is refused (`BadHookSigner`), and the arguments a callback trusts are the DEX's.
//! `before_swap` sets the LP fee (the sniper schedule) and cuts the input: a buy's creator and
//! holder fees, a sell's burn. `after_swap` cuts the output: a buy's burn, a sell's creator and
//! holder fees. The math is `bordrless_core`'s ([`launch_before_swap`], [`launch_after_swap`]):
//! fees round up, burns round down, creator + holder is dropped when it is not below the amount,
//! a burn when it is not below the amount, and the holder fee is taken only while holders hold at
//! least the kit's threshold.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::{get_return_data, invoke_signed};
use bordrless_core::{launch_after_swap, launch_before_swap, sniper_lp_fee, HookCut};
use bordrless_hook::pool_item::{ItemPoolAnswer, ItemPoolContext};
use bordrless_hook::{hook_signer_at, Delta, HookReturn, PoolHookArgs, HOOK_AUTHORITY_SEED};

use crate::constants::hookwars::*;
use crate::constants::*;
use crate::error::LaunchError;
use crate::events::{PoolItemCuts, PoolItemPart};
use crate::instructions::slot_launch::forwards;
use crate::state::*;

/// Accounts of `before_initialize`: the prefix only. There is never a launch to find, so the
/// handler refuses.
#[derive(Accounts)]
pub struct RejectInitialize<'info> {
    /// CHECK: the DEX's signer for this program (address- and signer-checked).
    #[account(signer, address = DEX_HOOK_AUTHORITY @ LaunchError::BadHookSigner)]
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: unused.
    pub pool: UncheckedAccount<'info>,
    /// CHECK: unused.
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: unused.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: unused.
    pub actor: UncheckedAccount<'info>,
}

/// `before_initialize`.
pub fn process_before_initialize(
    _ctx: Context<RejectInitialize>,
    _args: PoolHookArgs,
) -> Result<HookReturn> {
    err!(LaunchError::OnlyLaunchpadCreatesPools)
}

/// Accounts of `before_swap` and `after_swap`: the prefix, then the registry's extras at 5 to 8:
/// the launch, its quote holding, the holder vault and the kit config.
#[derive(Accounts)]
pub struct HookCallback<'info> {
    /// CHECK: the DEX's signer for this program (address- and signer-checked).
    #[account(signer, address = DEX_HOOK_AUTHORITY @ LaunchError::BadHookSigner)]
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the pool (must be the launch's).
    pub pool: UncheckedAccount<'info>,
    /// CHECK: the base mint (must be the launch's).
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the quote mint.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the trader.
    pub actor: UncheckedAccount<'info>,
    /// The launch of the base mint. Only `create_launch` creates a `Launch`, at
    /// `["launch", mint]` with that mint, so the owner, the discriminator and the mint bind it
    /// without a derivation.
    #[account(
        mut,
        constraint = launch.mint == base_mint.key() @ LaunchError::WrongPool,
        constraint = launch.pool == pool.key() @ LaunchError::WrongPool
    )]
    pub launch: Box<Account<'info, Launch>>,
    /// CHECK: the launch's quote holding (address-checked); receives creator fees.
    #[account(mut, address = launch.quote_holding @ LaunchError::WrongHolding)]
    pub launch_quote: UncheckedAccount<'info>,
    /// CHECK: the holder vault (address-checked); receives holder fees (the DEX checks it is a
    /// writable holding of the quote before paying it).
    #[account(address = launch.holder_vault @ LaunchError::WrongHolderVault)]
    pub holder_vault: UncheckedAccount<'info>,
    /// CHECK: the kit config (address-checked); read, through the kit crate, only when holder
    /// rewards are on.
    #[account(address = launch.kit_config @ LaunchError::WrongKitAccount)]
    pub kit_config: UncheckedAccount<'info>,
}

/// The kit's eligible supply and threshold, read when this side takes a holder fee (which holder
/// rewards being on implies); zeros otherwise, as nothing reads them then.
fn eligibility(ctx: &Context<'_, HookCallback<'_>>, holder_fee_bps: u16) -> Result<(u64, u64)> {
    if holder_fee_bps == 0 || !ctx.accounts.launch.rewards_on() {
        return Ok((0, 0));
    }
    let kit = bordrless_kit::client::read_kit_config(&ctx.accounts.kit_config)?;
    require_keys_eq!(
        kit.mint,
        ctx.accounts.launch.mint,
        LaunchError::WrongKitAccount
    );
    Ok((kit.eligible, kit.min_eligible))
}

/// The answer that takes `cut`: the creator fee to the launch's quote holding, the holder fee to
/// the holder vault, each only when above zero, and the burn; the launch counts each. The
/// counters are running totals for display (`claim_creator_fees` pays the quote holding's
/// balance, not a counter), so they saturate: a counter never blocks a trade.
fn answer(launch: &mut Launch, cut: &HookCut, lp_fee_bps: Option<u16>) -> HookReturn {
    let mut ret = HookReturn {
        lp_fee_bps,
        burn: cut.burn,
        ..HookReturn::default()
    };
    if cut.creator_fee > 0 {
        ret.deltas.push(Delta {
            amount: cut.creator_fee,
            account: QUOTE_HOLDING_INDEX,
        });
        launch.creator_fees_accrued = launch.creator_fees_accrued.saturating_add(cut.creator_fee);
    }
    if cut.holder_fee > 0 {
        ret.deltas.push(Delta {
            amount: cut.holder_fee,
            account: HOLDER_VAULT_INDEX,
        });
        launch.holder_fees_accrued = launch.holder_fees_accrued.saturating_add(cut.holder_fee);
    }
    launch.burned_on_trades = launch.burned_on_trades.saturating_add(cut.burn);
    ret
}

/// Hookwars M3b: what the pool items of a slot launch answered on one side (03 section 5.3).
#[derive(Default)]
struct ItemsAnswer {
    discount_bps: u16,
    cut: u64,
    burn: u64,
}

/// Hookwars M3b: calls each forwarded pool slot's item (03 section 5.1 step 2), in slot order, with
/// the DEX's `args` and an [`ItemPoolContext`] for this side, signing as this program's
/// `["hook-authority", program]`, and checks every answer (03 section 5.2). The remaining accounts
/// are the `PoolCuts` holding, then per forwarded slot `[program, signer, extras]`. `launch_cut`
/// is what the launch's own rules take on this side, `side_amount` what is left of it.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn forward_items<'info>(
    ctx: &Context<'info, HookCallback<'info>>,
    args: &PoolHookArgs,
    before: bool,
    launch_fee_bps: u16,
    launch_cut: u64,
    side_amount: u64,
) -> Result<ItemsAnswer> {
    let buy = args.direction == 1;
    let cut_side = (before && buy) || (!before && !buy);
    let remaining = ctx.remaining_accounts;
    require!(!remaining.is_empty(), LaunchError::ItemAccountsMissing);
    let launch = &ctx.accounts.launch;
    let (pool_cuts_owner, _) =
        Pubkey::find_program_address(&[POOL_CUTS_SEED, launch.mint.as_ref()], &ITEMS_ID);
    require_keys_eq!(
        *remaining[0].key,
        bordrless_token::client::holding_address(&launch.quote_mint, &pool_cuts_owner),
        LaunchError::WrongPoolCuts
    );
    let mint = Box::new(bordrless_token::client::read_mint(
        &ctx.accounts.base_mint.to_account_info(),
    )?);
    let (bit, discriminator) = if before {
        (ITEM_POOL_BEFORE, POOL_BEFORE_SWAP_DISCRIMINATOR)
    } else {
        (ITEM_POOL_AFTER, POOL_AFTER_SWAP_DISCRIMINATOR)
    };
    let mut out = ItemsAnswer::default();
    let mut discount: u32 = 0;
    let mut parts = Vec::new();
    let mut r = 1usize;
    for (index, s) in mint.active_slots().iter().enumerate() {
        if !forwards(s) {
            continue;
        }
        let n = 2 + usize::from(s.extra_count);
        require!(r + n <= remaining.len(), LaunchError::ItemAccountsMissing);
        let accounts = &remaining[r..r + n];
        r += n;
        if s.pool_flags & bit == 0 {
            continue;
        }
        require_keys_eq!(*accounts[0].key, s.program, LaunchError::WrongItemProgram);
        let signer = hook_signer_at(&crate::ID, &s.program, s.launch_signer_bump)
            .ok_or(LaunchError::WrongItemProgram)?;
        require_keys_eq!(*accounts[1].key, signer, LaunchError::WrongItemProgram);
        let item_ctx = ItemPoolContext {
            slot: index as u8,
            item: s.item,
            launch_fee_bps,
            launch_cut,
            side_amount,
        };
        let mut data = discriminator.to_vec();
        args.serialize(&mut data)?;
        item_ctx.serialize(&mut data)?;
        let mut metas = vec![
            AccountMeta::new_readonly(signer, true),
            AccountMeta::new_readonly(ctx.accounts.pool.key(), false),
            AccountMeta::new_readonly(ctx.accounts.base_mint.key(), false),
            AccountMeta::new_readonly(ctx.accounts.quote_mint.key(), false),
            AccountMeta::new_readonly(ctx.accounts.actor.key(), false),
        ];
        let mut infos = vec![
            accounts[1].clone(),
            ctx.accounts.pool.to_account_info(),
            ctx.accounts.base_mint.to_account_info(),
            ctx.accounts.quote_mint.to_account_info(),
            ctx.accounts.actor.to_account_info(),
        ];
        for a in &accounts[2..] {
            metas.push(AccountMeta {
                pubkey: *a.key,
                is_signer: false,
                is_writable: a.is_writable,
            });
            infos.push(a.clone());
        }
        infos.push(accounts[0].clone());
        let ix = Instruction {
            program_id: s.program,
            accounts: metas,
            data,
        };
        let bump = [s.launch_signer_bump];
        invoke_signed(
            &ix,
            &infos,
            &[&[HOOK_AUTHORITY_SEED, s.program.as_ref(), &bump]],
        )?;
        // Changed by Hookwars (security review 2 H-A): no return data, or empty data from the
        // item's program, is the default answer, as the token program and the DEX read hooks
        // (the runtime resets return data to (callee, []) at every invocation, so nothing stale
        // survives). Data from any other program is still refused.
        let answer = match get_return_data() {
            None => ItemPoolAnswer::default(),
            Some((from, ret)) => {
                require_keys_eq!(from, s.program, LaunchError::ForeignAnswer);
                if ret.is_empty() {
                    ItemPoolAnswer::default()
                } else {
                    ItemPoolAnswer::try_from_slice(&ret)
                        .map_err(|_| error!(LaunchError::ForeignAnswer))?
                }
            }
        };
        require!(
            answer.discount_bps <= 10_000,
            LaunchError::ItemDiscountTooHigh
        );
        if answer.cut > 0 {
            require!(cut_side, LaunchError::ItemCutWrongSide);
            // Protocol pass 4a: an external template's program cannot write the items program's
            // equip state; its registry ends `[EquipState, our items signer, items program]` and the cut is recorded
            // there, signed by this program's `["hook-authority", items]`.
            if s.program != ITEMS_ID {
                record_external_cut(accounts, &launch.mint, index as u8, &s.item, answer.cut, u8::from(!buy) + 1)?;
            }
        }
        if answer.burn > 0 {
            require!(!cut_side && s.bounds.may_burn, LaunchError::ItemBurnWrongSide);
        }
        // The per-item pool cut ceiling is the armory's (`max_pool_item_cut_bps`, checked against
        // the item's manifest at every equip, 02 M2 note 4): a Pool slot carries no token-side cut
        // bound. Here: no item takes more than what is left of the side.
        require!(
            u128::from(answer.cut) + u128::from(answer.burn) <= u128::from(side_amount),
            LaunchError::ItemCutOutOfBounds
        );
        discount += u32::from(answer.discount_bps);
        out.cut = out
            .cut
            .checked_add(answer.cut)
            .ok_or(LaunchError::MathOverflow)?;
        out.burn = out
            .burn
            .checked_add(answer.burn)
            .ok_or(LaunchError::MathOverflow)?;
        if answer.cut > 0 || answer.burn > 0 || answer.discount_bps > 0 {
            parts.push(PoolItemPart {
                slot: index as u8,
                item: s.item,
                discount_bps: answer.discount_bps,
                cut: answer.cut,
                burn: answer.burn,
            });
        }
    }
    require!(r == remaining.len(), LaunchError::StaleRegistry);
    out.discount_bps = discount.min(10_000) as u16;
    if !parts.is_empty() {
        let clock = Clock::get()?;
        emit!(PoolItemCuts {
            launch: launch.key(),
            pool: launch.pool,
            mint: launch.mint,
            side: u8::from(!before),
            discount_bps: out.discount_bps,
            parts,
            pool_cuts_delta: out.cut,
            burned: out.burn,
            slot: clock.slot,
            ts: clock.unix_timestamp,
        });
    }
    Ok(out)
}

/// Protocol pass 4a: `items::record_pool_cut(mint, slot, item, cut, side)` with the last three
/// accounts of an external slot's slice (`EquipState`, this program's `["hook-authority", items]`,
/// the items program).
fn record_external_cut<'info>(accounts: &[AccountInfo<'info>], mint: &Pubkey, slot: u8, item: &Pubkey, cut: u64, side: u8) -> Result<()> {
    let n = accounts.len();
    let (signer, bump) = Pubkey::find_program_address(&[HOOK_AUTHORITY_SEED, ITEMS_ID.as_ref()], &crate::ID);
    require!(
        n >= 5 && *accounts[n - 1].key == ITEMS_ID && *accounts[n - 2].key == signer,
        LaunchError::ItemAccountsMissing
    );
    let state = &accounts[n - 3];
    // `sha256("global:record_pool_cut")[..8]`.
    let mut data = vec![119, 109, 14, 167, 96, 253, 125, 130];
    (*mint, slot, *item, cut, side).serialize(&mut data)?;
    let ix = Instruction {
        program_id: ITEMS_ID,
        accounts: vec![
            anchor_lang::solana_program::instruction::AccountMeta::new_readonly(signer, true),
            anchor_lang::solana_program::instruction::AccountMeta::new(*state.key, false),
        ],
        data,
    };
    invoke_signed(
        &ix,
        &[accounts[n - 2].clone(), state.clone(), accounts[n - 1].clone()],
        &[&[HOOK_AUTHORITY_SEED, ITEMS_ID.as_ref(), &[bump]]],
    )?;
    Ok(())
}

/// Hookwars M3b: applies the items' merged answer to the launch's own cut `L` (03 section 5.3):
/// the discount comes out of the creator and holder fees only (never the LP fee or the burn), the
/// items' burns add to the burn. Answers the cut the launch keeps.
fn merge(mut cut: HookCut, items: &ItemsAnswer) -> HookCut {
    if items.discount_bps > 0 {
        let keep = u128::from(10_000 - items.discount_bps);
        cut.creator_fee = ((u128::from(cut.creator_fee) * keep) / 10_000) as u64;
        cut.holder_fee = ((u128::from(cut.holder_fee) * keep) / 10_000) as u64;
    }
    cut.burn = cut.burn.saturating_add(items.burn);
    cut
}

/// The items' single cut delta to the `PoolCuts` holding (R2), after the launch's own.
fn with_items(mut ret: HookReturn, items: &ItemsAnswer) -> HookReturn {
    if items.cut > 0 {
        ret.deltas.push(Delta {
            amount: items.cut,
            account: POOL_CUTS_INDEX,
        });
    }
    ret
}

/// `before_swap`: the LP fee (the sniper schedule; the creator's own first buy into its own
/// wallet inside the window pays the normal fee, once), then the cut from the input. On a slot
/// launch, then the pool items (Hookwars M3b).
pub fn process_before_swap<'info>(
    ctx: Context<'info, HookCallback<'info>>,
    args: PoolHookArgs,
) -> Result<HookReturn> {
    require_keys_eq!(args.pool, ctx.accounts.launch.pool, LaunchError::WrongPool);
    let now = Clock::get()?.unix_timestamp;
    let buy = args.direction == 1;
    let rates = {
        let l = &ctx.accounts.launch;
        l.rules.fee_rates(l.creator_fee_bps)
    };
    let (eligible, min_eligible) =
        eligibility(&ctx, if buy { rates.holder_fee_buy_bps } else { 0 })?;
    let (lp_fee, cut) = {
        let launch = &mut ctx.accounts.launch;
        let own_first = buy
            && args.actor == launch.creator
            && args.recipient == launch.creator
            && !launch.creator_bought
            && now < launch.created_at.saturating_add(launch.sniper_window_secs);
        let lp_fee = if own_first {
            launch.creator_bought = true;
            launch.lp_fee_bps
        } else {
            sniper_lp_fee(
                now,
                launch.created_at,
                launch.sniper_window_secs,
                launch.sniper_start_bps,
                launch.lp_fee_bps,
            )
        };
        (
            lp_fee,
            launch_before_swap(buy, args.amount_in, &rates, eligible, min_eligible),
        )
    };
    if !ctx.accounts.launch.is_slot_launch() {
        return Ok(answer(&mut ctx.accounts.launch, &cut, Some(lp_fee)));
    }
    let launch_cut = cut.creator_fee + cut.holder_fee + cut.burn;
    let fee_bps = if buy {
        rates.creator_fee_bps.saturating_add(rates.holder_fee_buy_bps)
    } else {
        0
    };
    let items = forward_items(
        &ctx,
        &args,
        true,
        fee_bps,
        launch_cut,
        args.amount_in.saturating_sub(launch_cut),
    )?;
    let ret = answer(&mut ctx.accounts.launch, &merge(cut, &items), Some(lp_fee));
    Ok(with_items(ret, &items))
}

/// `after_swap`: the cut from the output the DEX hands on (a buy: the curve's output; a sell: the
/// curve's output less the DEX's protocol fee, which is always in the quote). On a slot launch,
/// then the pool items (Hookwars M3b).
pub fn process_after_swap<'info>(
    ctx: Context<'info, HookCallback<'info>>,
    args: PoolHookArgs,
) -> Result<HookReturn> {
    require_keys_eq!(args.pool, ctx.accounts.launch.pool, LaunchError::WrongPool);
    let buy = args.direction == 1;
    let rates = {
        let l = &ctx.accounts.launch;
        l.rules.fee_rates(l.creator_fee_bps)
    };
    // A sell's input has left the seller by now: the kit counts it out of the eligible supply.
    let (eligible, min_eligible) =
        eligibility(&ctx, if buy { 0 } else { rates.holder_fee_sell_bps })?;
    let cut = launch_after_swap(buy, args.amount_out, &rates, eligible, min_eligible);
    if !ctx.accounts.launch.is_slot_launch() {
        return Ok(answer(&mut ctx.accounts.launch, &cut, None));
    }
    let launch_cut = cut.creator_fee + cut.holder_fee + cut.burn;
    let fee_bps = if buy {
        0
    } else {
        rates.creator_fee_bps.saturating_add(rates.holder_fee_sell_bps)
    };
    let items = forward_items(
        &ctx,
        &args,
        false,
        fee_bps,
        launch_cut,
        args.amount_out.saturating_sub(launch_cut),
    )?;
    let ret = answer(&mut ctx.accounts.launch, &merge(cut, &items), None);
    Ok(with_items(ret, &items))
}
