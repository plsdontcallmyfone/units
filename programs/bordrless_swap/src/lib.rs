// Changed by Hookwars: program ids and derived addresses; observation ring (obs) and swap_route; a heap
// allocator that uses a requested heap frame (a route of slot launches needs more than 32 KiB).
//! `bordrless_swap`: the Bordrless DEX.
//!
//! Constant-product pools between two Bordrless Token Standard mints. A pool has an LP fee that
//! stays in the pool, a protocol fee always taken in the quote token that the admin collects for
//! the fee collector, optional virtual reserves (a launch curve), an LP mint on the token standard,
//! and an optional hook program that runs before and after swaps and liquidity changes, can take
//! up to three deltas and a burn from a swap's input (before) and output (after), and can set the
//! LP fee of a swap (hook protocol v2, `bordrless_hook`; `docs/hooks-v2.md` §3). Every callback is
//! signed by this program's `["hook-authority", hook_program]` PDA, one per hook program, so a
//! signer one pool hook receives and passes on is never another hook's (the pool keeps the bump).
//! Each side's token-hook slice carries the token program's signer for that mint's hook.
//!
//! Amounts are measured, never assumed: the vault balance is read before and after every transfer,
//! so a token whose own hook takes a cut still prices correctly, and `min_amount_out` is checked
//! against what the recipient's holding actually gained.
//!
//! Layout: [`state`], [`instructions`] (config, pool, liquidity, swap), [`hooks`] (pool hook CPIs
//! and delta transfers), [`token`] (token standard CPIs), [`events`], [`error`], [`client`].

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

/// Changed by Hookwars: the bump allocator of `solana-program-entrypoint`, growing upward from the
/// heap's start so that a transaction without a larger heap frame behaves as before up to 32 KiB,
/// and one that requests a frame (`ComputeBudgetInstruction::RequestHeapFrame`, up to 256 KiB)
/// can use all of it. A two-hop `swap_route` through slot launches with pool items allocates more
/// than 32 KiB (each hop's CPIs carry the slices and the items' accounts).
#[cfg(all(target_os = "solana", feature = "custom-heap", not(feature = "no-entrypoint")))]
mod heap {
    use core::alloc::{GlobalAlloc, Layout};
    const START: usize = 0x300000000;
    const LEN: usize = 256 * 1024;
    struct Upward;
    unsafe impl GlobalAlloc for Upward {
        #[inline]
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let pos_ptr = START as *mut usize;
            let mut pos = *pos_ptr;
            if pos == 0 {
                pos = START + core::mem::size_of::<usize>();
            }
            let align = layout.align().max(1);
            let at = (pos + align - 1) & !(align - 1);
            let end = match at.checked_add(layout.size()) {
                Some(e) if e <= START + LEN => e,
                _ => return core::ptr::null_mut(),
            };
            *pos_ptr = end;
            at as *mut u8
        }
        #[inline]
        unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
    }
    #[global_allocator]
    static ALLOC: Upward = Upward;
}

pub mod client;
pub mod constants;
pub mod error;
pub mod events;
pub mod hooks;
pub mod instructions;
pub mod obs;
pub mod state;
pub mod token;

pub use instructions::*;

declare_id!("AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Bordrless DEX",
    project_url: "https://github.com/BordrlessDex/bordrless-programs",
    contacts: "link:https://github.com/BordrlessDex/bordrless-programs/security/advisories/new",
    policy: "https://github.com/BordrlessDex/bordrless-programs/blob/main/SECURITY.md",
    source_code: "https://github.com/BordrlessDex/bordrless-programs"
}

/// Instructions of the DEX.
#[program]
pub mod bordrless_swap {
    use super::*;

    /// Creates the config; signed by the program's upgrade authority, once.
    pub fn init_config(ctx: Context<InitConfig>, args: ConfigArgs) -> Result<()> {
        config::process_init_config(ctx, args)
    }

    /// Admin: changes the config.
    pub fn set_config(ctx: Context<SetConfig>, args: ConfigArgs) -> Result<()> {
        config::process_set_config(ctx, args)
    }

    /// Creates a pool with its first deposit. A pool with virtual reserves (a curve) can only be
    /// created by its hook program.
    pub fn create_pool<'info>(
        ctx: Context<'info, CreatePool<'info>>,
        args: CreatePoolArgs,
    ) -> Result<()> {
        pool::process_create_pool(ctx, args)
    }

    /// The pool's hook turns a curve pool into an ordinary one: reserves synced to the vaults, no
    /// virtual offsets, the first LP minted to a recipient.
    pub fn finalize_curve(ctx: Context<FinalizeCurve>, hook_caller_bump: u8) -> Result<()> {
        pool::process_finalize_curve(ctx, hook_caller_bump)
    }

    /// Admin: moves the accrued protocol fees of a pool (always in its quote token) to the fee
    /// collector's quote holding. The remaining accounts are the quote mint's token-hook slice
    /// (`quote_hook_accounts` of them, its program first).
    pub fn collect_protocol_fees<'info>(
        ctx: Context<'info, CollectProtocolFees<'info>>,
        quote_hook_accounts: u8,
    ) -> Result<()> {
        pool::process_collect_protocol_fees(ctx, quote_hook_accounts)
    }

    /// Anyone: unwraps the accrued protocol fees of a pool quoted in bridged SOL and pays them to
    /// the fee collector the config names, as SOL. Nothing else can receive them.
    pub fn collect_protocol_fees_sol(ctx: Context<CollectProtocolFeesSol>) -> Result<()> {
        pool::process_collect_protocol_fees_sol(ctx)
    }

    /// Adds liquidity at the pool's ratio and mints LP.
    pub fn add_liquidity<'info>(
        ctx: Context<'info, Liquidity<'info>>,
        args: AddLiquidityArgs,
    ) -> Result<()> {
        liquidity::process_add_liquidity(ctx, args)
    }

    /// Burns LP and withdraws the proportional reserves.
    pub fn remove_liquidity<'info>(
        ctx: Context<'info, Liquidity<'info>>,
        args: RemoveLiquidityArgs,
    ) -> Result<()> {
        liquidity::process_remove_liquidity(ctx, args)
    }

    /// Swaps an exact input for at least `min_amount_out`.
    pub fn swap<'info>(ctx: Context<'info, Swap<'info>>, args: SwapArgs) -> Result<()> {
        swap::process_swap(ctx, args)
    }

    /// Hookwars: a multi-hop swap. Hops run in order inside this instruction; each hop's input is
    /// what the previous one delivered; every hop's pool hook is told the route.
    pub fn swap_route<'info>(
        ctx: Context<'info, SwapRoute<'info>>,
        args: SwapRouteArgs,
    ) -> Result<()> {
        swap::process_swap_route(ctx, args)
    }
}
