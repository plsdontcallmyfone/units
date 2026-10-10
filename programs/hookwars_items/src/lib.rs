// Changed by Hookwars: integration pass 2: one items error enum (Soulbound and arsenal codes kept); LeaseRentPaid; integration pass 3: EquipState.runs_at_settle, init_equip wear flag, ProtocolFee.
// Changed by Hookwars: new file (M2), the template program's armory-facing entry points; M3b: the
// token and pool callbacks, every base template and arsenal wave A, composites, the raid ledger
// and settle_equip; arsenal waves D and E (templates, `payouts`).
//! `hookwars_items`: one program implementing every template (docs/spec/04-templates.md,
//! 08-arsenal.md, 00 D-2).
//!
//! - Armory-facing (signed by the armory's `["armory"]` PDA, M2): `validate_params`, `manifest`,
//!   `combine_params`, `init_equip`, `close_equip`.
//! - Token slot callbacks (signed by the token program's `["hook-authority", items]`):
//!   `before_transfer`, `after_transfer`, `before_burn`, `after_burn`, `on_touch`.
//! - Pool callbacks (signed by the launchpad's `["hook-authority", items]`, 03 section 5):
//!   `pool_before_swap`, `pool_after_swap`.
//! - Permissionless: `init_raid_ledger`, `settle_equip`.
//!
//! Callbacks are leaves (R3): they make no CPI. Each template lives in `templates/<name>.rs`.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use bordrless_hook::pool_item::ItemPoolContext;
use bordrless_hook::{PoolHookArgs, TokenSlotArgs};
use hookwars_common::{EquipConfig, ParamsError, MAX_MODULES, PARAM_FIELDS};

pub mod engine;
pub mod equip;
pub mod payouts;
pub mod settle;
pub mod templates;

pub use payouts::*;

declare_id!("8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "Hookwars items",
    project_url: "https://github.com/BordrlessDex/bordrless-programs",
    contacts: "link:https://github.com/BordrlessDex/bordrless-programs/security/advisories/new",
    policy: "https://github.com/BordrlessDex/bordrless-programs/blob/main/SECURITY.md",
    source_code: "https://github.com/BordrlessDex/bordrless-programs"
}

/// The armory's signer of every call here: `["armory"]` under the armory (02 section 2.2).
pub const ARMORY_SIGNER: Pubkey =
    Pubkey::from_str_const("2NSNJ4W51G5ZZSc4wVqkjyR8yr5yzuEjquKv7iprzSHP");
/// The token program's signer of every slot callback here: `["hook-authority", items]` under the
/// token program (04 section 2.1).
pub const TOKEN_ITEMS_SIGNER: Pubkey =
    Pubkey::from_str_const("GtuzTUqRkfbWLarc8hhn7WWTkkuPXavZBQGb8MH43kwN");
/// The launchpad's signer of every pool-item callback: `["hook-authority", items]` under the
/// launchpad (04 section 2.1, 03 section 5.1).
pub const LAUNCH_ITEMS_SIGNER: Pubkey =
    Pubkey::from_str_const("7WNwXG1tgUs2Srx4wPxBiRfhtMvMkZUZCDuyvrGAYLnX");
/// The war program's `["war-signer"]`: the only `authority` a war touch is accepted from.
pub const WAR_SIGNER: Pubkey =
    Pubkey::from_str_const("HCBAfMMN6JMUfnJ64H4Kxs6r3gaTpJCeFr5dyrDMENoe");
/// `EquipState` layout version.
pub const VERSION: u8 = 2;
/// Accounts before a token callback's extras: the signer, mint, source, destination, authority
/// (`bordrless_hook::TOKEN_PREFIX_ACCOUNTS`).
pub const TOKEN_PREFIX: usize = bordrless_hook::TOKEN_PREFIX_ACCOUNTS;
/// Index of the equip vault in a token callback's accounts (prefix, `Item`, `EquipState`, vault).
pub const VAULT_INDEX: u8 = (TOKEN_PREFIX + 2) as u8;

/// Errors (04 section 6, 08 section 2.10).
#[error_code]
pub enum ItemsError {
    /// The params break a template rule.
    #[msg("the params break a template rule")]
    BadParams,
    /// Unknown template.
    #[msg("unknown template")]
    UnknownTemplate,
    /// The two items cannot be forged together.
    #[msg("the two items cannot be forged together")]
    NotForgeable,
    /// The targets or role do not fit the template.
    #[msg("the targets or role do not fit the template")]
    BadTargets,
    /// A vault of the slot still holds something unsettled.
    #[msg("a vault of the slot still holds something unsettled")]
    VaultNotSettled,
    /// The slot already holds an item.
    #[msg("the slot already holds an item")]
    SlotNotEmpty,
    /// The slot holds no item.
    #[msg("the slot holds no item")]
    NotEquipped,
    /// A wrong account.
    #[msg("a wrong account")]
    WrongAccount,
    /// The caller is not the token program's or the launchpad's signer for this program.
    #[msg("the hook signer is not the caller's signer for this program")]
    BadHookSigner,
    /// The item in the arguments is not the one its accounts name.
    #[msg("the item in the arguments is not the one its accounts name")]
    WrongItem,
    /// A Wall refuses the transfer.
    #[msg("the wall holds: the wallet would hold too much while under siege")]
    WallHolds,
    /// A wallet cap refuses the transfer.
    #[msg("the wallet would hold more than the cap")]
    WalletTooLarge,
    /// A war touch from another caller.
    #[msg("only the war program may touch raid points and tickets")]
    NotWarSigner,
    /// Not enough raid points this season.
    #[msg("not enough raid points this season")]
    NotEnoughPoints,
    /// No loot ticket.
    #[msg("no loot ticket")]
    NoTicket,
    /// Arithmetic overflow.
    #[msg("arithmetic overflow")]
    Overflow,
    /// A transfer larger than Max Transaction allows.
    #[msg("the transfer moves more of the supply than one transfer may")]
    TransferTooLarge,
    /// A wallet send under Dust Guard's minimum.
    #[msg("the send is under the minimum")]
    DustRefused,
    /// The composite's module list is unreadable or does not match the item.
    #[msg("the composite's module list does not match the item")]
    BadComposite,
    /// The range is shorter than the item's bytes.
    #[msg("the slot's range is shorter than the item's bytes")]
    RangeTooShort,
    // Arsenal waves B and C.
    /// Cooldown: the wallet bought too recently to sell or send.
    #[msg("the wallet bought too recently to sell or send")]
    CooldownActive,
    /// Flash Guard: a sell too soon after the buy.
    #[msg("the tokens were bought too few slots ago to sell")]
    FlashSellTooSoon,
    /// Daily Sell Cap: the day's cap is spent.
    #[msg("the wallet has sold or sent its daily cap")]
    DailyCapExceeded,
    /// A holder payload from someone other than the holder.
    #[msg("only the holding's owner may set this")]
    NotHolder,
    // Integration pass 2 (app/INTEGRATION.md section 4): the Soulbound (7000) and arsenal waves D
    // and E (7100) errors joined this one enum so the IDL builder sees a single error enum; their
    // explicit discriminants keep every code number unchanged.
    /// Soulbound: a badge cannot be sent or sold.
    #[msg("this badge cannot be sent or sold")]
    SoulboundTransfer = 1000,
    /// Guest List: the opening is for holders of the target only.
    #[msg("the opening is for holders of the target mint only")]
    GuestListClosed = 1100,
    /// The holder joined during the claimed epoch, or claimed it already.
    #[msg("not eligible for this epoch's loyalty claim")]
    NotEligible,
    /// Nothing to pay.
    #[msg("nothing to pay")]
    NothingToPay,
    /// The slot does not hold a Loyalty Pot.
    #[msg("the slot does not hold a Loyalty Pot")]
    NoLoyaltyPot,
    /// A referrer cannot be the buyer.
    #[msg("a buyer cannot refer itself")]
    SelfReferral,
}

/// Maps a shared-rule error.
pub fn map(e: ParamsError) -> Error {
    match e {
        ParamsError::UnknownTemplate => ItemsError::UnknownTemplate.into(),
        ParamsError::NotForgeable => ItemsError::NotForgeable.into(),
        ParamsError::OutOfRange | ParamsError::BadParams => ItemsError::BadParams.into(),
    }
}

/// One equip of a mint's slot (04 section 4, 08 section 2.11). Also the owner of the slot's
/// equip vault.
#[account]
#[derive(Debug)]
pub struct EquipState {
    /// Layout version.
    pub version: u8,
    /// Bump of `["equip", mint, slot]`.
    pub bump: u8,
    /// The token.
    pub mint: Pubkey,
    /// The slot.
    pub slot: u8,
    /// The equipped item (default when empty).
    pub item: Pubkey,
    /// Its template.
    pub template_id: u16,
    /// Targets and role.
    pub config: EquipConfig,
    /// When equipped.
    pub equipped_at: i64,
    /// Callbacks run (informational).
    pub runs: u64,
    /// Token-side cuts taken, all time.
    pub collected_token: u64,
    /// Pool-side cuts recorded, all time.
    pub pool_owed: u64,
    /// Pool-side cuts settled, all time.
    pub pool_settled: u64,
    /// Unsettled token-side cuts per module (index 0 for a plain item).
    pub token_unsettled: [u64; MAX_MODULES],
    /// Unsettled pool-side cuts per module.
    pub pool_unsettled: [u64; MAX_MODULES],
    /// Integration pass 3 (E-3): `runs` at the last `settle_equip`; the difference is the wear.
    pub runs_at_settle: u64,
    /// Reserved.
    pub reserved: [u8; 24],
}

impl EquipState {
    /// Space for an `EquipState` with `targets` targets.
    pub fn space(targets: usize) -> usize {
        8 + 1 + 1 + 32 + 1 + 32 + 2 + (4 + 32 * targets + 1) + 8 + 8 + 8 + 8 + 8
            + 16 * MAX_MODULES
            + 32
    }
}

/// Emitted by `init_equip` (program log: called only by CPI).
#[event]
pub struct EquipInitialized {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub config: EquipConfig,
}

/// Emitted by `close_equip`.
#[event]
pub struct EquipClosed {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
}

/// A raid buy stamped on its buyer's holding (04 section 5; 06's name and fields).
#[event]
pub struct RaidMarked {
    pub mint: Pubkey,
    pub rival: Pubkey,
    pub trader: Pubkey,
    pub volume: u64,
    pub points: u32,
    pub loot_ticket: bool,
}

/// A Shield's sell cut.
#[event]
pub struct ShieldTaken {
    pub mint: Pubkey,
    pub owner: Pubkey,
    pub cut: u64,
}

/// Any item cut. `side`: 0 token, 1 pool on a buy, 2 pool on a sell.
#[event]
pub struct ItemCut {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub module: u8,
    pub side: u8,
    pub amount: u64,
}

/// `settle_equip` (04 section 5).
#[event]
pub struct EquipSettled {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub royalty_token: u64,
    pub royalty_quote: u64,
    pub amount_token: u64,
    pub amount_quote: u64,
    pub burned: u64,
    pub bounty_token: u64,
    pub bounty_quote: u64,
}

/// Integration pass 3 (E-2, R37): the protocol's share of a settle.
#[event]
pub struct ProtocolFee {
    pub source: u8,
    pub mint: Pubkey,
    pub amount: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

/// Integration pass 3 (R34, spec 10 I-2): the template author's share of a settle's royalty.
#[event]
pub struct AuthorSharePaid {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub template_id: u16,
    pub author_token: u64,
    pub author_quote: u64,
}

/// Integration pass 2 (10 section 17 I-7, R32): the lessor's share of a leased item's royalty,
/// paid by `settle_equip` out of the royalty (never on top of it).
#[event]
pub struct LeaseRentPaid {
    pub mint: Pubkey,
    pub slot: u8,
    pub item: Pubkey,
    pub lessor: Pubkey,
    pub rent_token: u64,
    pub rent_quote: u64,
}

/// Instructions.
#[program]
pub mod hookwars_items {
    use super::*;

    /// Template rules beyond floor and ceiling (also re-checks floor and ceiling).
    pub fn validate_params(
        _ctx: Context<ArmoryOnly>,
        template_id: u16,
        field_min: [u32; PARAM_FIELDS],
        field_max: [u32; PARAM_FIELDS],
        params: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        equip::validate_params(template_id, field_min, field_max, params)
    }

    /// The item's manifest, as return data (16 bytes).
    pub fn manifest(
        _ctx: Context<ArmoryOnly>,
        template_id: u16,
        params: [u32; PARAM_FIELDS],
        max_targets: u8,
    ) -> Result<()> {
        equip::manifest(template_id, params, max_targets)
    }

    /// The forged params, as return data.
    pub fn combine_params(
        _ctx: Context<ArmoryOnly>,
        template_id: u16,
        field_min: [u32; PARAM_FIELDS],
        field_max: [u32; PARAM_FIELDS],
        gain_bps: u16,
        a: [u32; PARAM_FIELDS],
        b: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        equip::combine_params(template_id, field_min, field_max, gain_bps, a, b)
    }

    /// Creates or resets the slot's `EquipState`, writes the item's registry, creates the equip
    /// vault for an item that may cut on the token side. Return data: the registry length (`u8`).
    /// For a composite, the module list is the first remaining account.
    pub fn init_equip<'info>(
        ctx: Context<'info, InitEquip<'info>>,
        slot: u8,
        item: Pubkey,
        template_id: u16,
        manifest: hookwars_common::Manifest,
        config: EquipConfig,
        max_targets: u8,
        wear: bool,
    ) -> Result<()> {
        equip::process_init_equip(ctx, slot, item, template_id, manifest, config, max_targets, wear)
    }

    /// Empties the slot's `EquipState`; refused while its vaults hold anything unsettled.
    pub fn close_equip(ctx: Context<CloseEquip>, slot: u8) -> Result<()> {
        equip::process_close_equip(ctx, slot)
    }

    /// Creates a mint's `RaidLedger` (permissionless; the payer pays rent).
    pub fn init_raid_ledger(ctx: Context<InitRaidLedger>) -> Result<()> {
        engine::process_init_raid_ledger(ctx)
    }

    /// Token slot callback.
    pub fn before_transfer<'info>(
        ctx: Context<'info, TokenCallback<'info>>,
        args: TokenSlotArgs,
    ) -> Result<()> {
        engine::token_before(ctx, args)
    }

    /// Token slot callback (answers nothing).
    pub fn after_transfer<'info>(
        ctx: Context<'info, TokenCallback<'info>>,
        _args: TokenSlotArgs,
    ) -> Result<()> {
        engine::check_token_signer(&ctx.accounts.hook_signer)
    }

    /// Token slot callback (answers nothing).
    pub fn before_burn<'info>(
        ctx: Context<'info, TokenCallback<'info>>,
        _args: TokenSlotArgs,
    ) -> Result<()> {
        engine::check_token_signer(&ctx.accounts.hook_signer)
    }

    /// Token slot callback (answers nothing).
    pub fn after_burn<'info>(
        ctx: Context<'info, TokenCallback<'info>>,
        _args: TokenSlotArgs,
    ) -> Result<()> {
        engine::check_token_signer(&ctx.accounts.hook_signer)
    }

    /// `touch` of one holding (war payloads, 04 section 2.10).
    pub fn on_touch<'info>(
        ctx: Context<'info, TokenCallback<'info>>,
        args: TokenSlotArgs,
    ) -> Result<()> {
        engine::touch(ctx, args)
    }

    /// Pool-item callback before a launch-pool swap.
    pub fn pool_before_swap<'info>(
        ctx: Context<'info, PoolCallback<'info>>,
        args: PoolHookArgs,
        item_ctx: ItemPoolContext,
    ) -> Result<()> {
        engine::pool(ctx, args, item_ctx, true)
    }

    /// Pool-item callback after a launch-pool swap.
    pub fn pool_after_swap<'info>(
        ctx: Context<'info, PoolCallback<'info>>,
        args: PoolHookArgs,
        item_ctx: ItemPoolContext,
    ) -> Result<()> {
        engine::pool(ctx, args, item_ctx, false)
    }

    /// Pays what a slot's item collected (04 section 2.5, 08 section 2.11): the royalty, the
    /// sender's bounty, the rest to each module's destination. Permissionless.
    pub fn settle_equip<'info>(ctx: Context<'info, SettleEquip<'info>>, slot: u8) -> Result<()> {
        settle::process(ctx, slot)
    }

    /// Referral: the buyer names its referrer, once (08 4.7).
    pub fn set_referrer(ctx: Context<SetReferrer>, referrer: Pubkey) -> Result<()> {
        payouts::process_set_referrer(ctx, referrer)
    }

    /// Referral: pays a buyer's owed referral from the Referral vault (permissionless).
    pub fn settle_referral(ctx: Context<SettleReferral>, slot: u8) -> Result<()> {
        payouts::process_settle_referral(ctx, slot)
    }

    /// First Blood: creates its state (permissionless).
    pub fn init_first_blood(ctx: Context<InitFirstBlood>) -> Result<()> {
        payouts::process_init_first_blood(ctx)
    }

    /// Loyalty Pot: creates its state for the item in `slot` (permissionless).
    pub fn init_loyalty(ctx: Context<InitLoyalty>, slot: u8) -> Result<()> {
        payouts::process_init_loyalty(ctx, slot)
    }

    /// Loyalty Pot: moves the pot to `slot` when its slot no longer holds the pot and `slot` does
    /// (integration pass 2, 08 arsenal 2 request 7; permissionless).
    pub fn reslot_loyalty(ctx: Context<ReslotLoyalty>, slot: u8) -> Result<()> {
        payouts::process_reslot_loyalty(ctx, slot)
    }

    /// Loyalty Pot: the holder's claim for the epoch (08 4.3).
    pub fn claim_loyalty(ctx: Context<ClaimLoyalty>) -> Result<()> {
        payouts::process_claim_loyalty(ctx)
    }
}

/// Accounts of the pure entry points: only the armory's signer.
#[derive(Accounts)]
pub struct ArmoryOnly<'info> {
    /// The armory's `["armory"]` PDA.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: Signer<'info>,
}

/// Accounts of `init_equip`.
#[derive(Accounts)]
pub struct InitEquip<'info> {
    /// The armory's `["armory"]` PDA.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: Signer<'info>,
    /// Pays rent.
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the token (a mint of the token program).
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: `["equip", mint, slot]` under this program, created or reset here.
    #[account(mut)]
    pub equip_state: UncheckedAccount<'info>,
    /// CHECK: `["bordrless-hook-accounts", mint, item]` under this program, written here.
    #[account(mut)]
    pub registry: UncheckedAccount<'info>,
    /// CHECK: the equip vault, created here when the item may cut on the token side.
    #[account(mut)]
    pub equip_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Accounts of `close_equip`.
#[derive(Accounts)]
pub struct CloseEquip<'info> {
    /// The armory's `["armory"]` PDA.
    #[account(address = ARMORY_SIGNER)]
    pub armory_signer: Signer<'info>,
    /// CHECK: the token.
    pub mint: UncheckedAccount<'info>,
    /// The slot's `EquipState`.
    #[account(mut)]
    pub equip_state: Account<'info, EquipState>,
    /// CHECK: the equip vault, when the equipped item may cut on the token side.
    pub equip_vault: Option<UncheckedAccount<'info>>,
}

/// Accounts of `init_raid_ledger`.
#[derive(Accounts)]
pub struct InitRaidLedger<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the token (a mint of the token program).
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: `["raid-ledger", mint]`, created here.
    #[account(mut)]
    pub raid_ledger: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// A token slot callback: the token prefix, then the item's extras (remaining accounts).
#[derive(Accounts)]
pub struct TokenCallback<'info> {
    /// CHECK: the token program's signer for this program (checked in the engine).
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the mint.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: source.
    pub source: UncheckedAccount<'info>,
    /// CHECK: destination.
    pub destination: UncheckedAccount<'info>,
    /// CHECK: authority.
    pub authority: UncheckedAccount<'info>,
}

/// A pool-item callback: the launchpad's signer and the pool prefix, then the item's extras.
#[derive(Accounts)]
pub struct PoolCallback<'info> {
    /// CHECK: the launchpad's signer for this program (checked in the engine).
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the pool.
    pub pool: UncheckedAccount<'info>,
    /// CHECK: the base mint (the token).
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the quote mint.
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the trader.
    pub actor: UncheckedAccount<'info>,
}

/// Accounts of `settle_equip`. Remaining accounts: per module, its token-side destination and
/// its pool-side destination (a holding, or the mint itself for a burn; the program id when the
/// module has none), then the mint's `Locked` slot slice when it has one.
#[derive(Accounts)]
pub struct SettleEquip<'info> {
    /// Sends the instruction, pays any holding it creates, receives the bounty.
    #[account(mut)]
    pub cranker: Signer<'info>,
    /// CHECK: the token (writable: burns).
    #[account(mut, owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// CHECK: the item (armory `Item`).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the composite's module list, or this program's id.
    pub composite: UncheckedAccount<'info>,
    /// The slot's `EquipState`.
    #[account(mut)]
    pub equip_state: Account<'info, EquipState>,
    /// CHECK: the equip vault, when the item cuts on the token side.
    #[account(mut)]
    pub equip_vault: Option<UncheckedAccount<'info>>,
    /// CHECK: `["pool-cuts", mint]` (system-owned signer).
    pub pool_cuts: UncheckedAccount<'info>,
    /// CHECK: its holding of the quote.
    #[account(mut)]
    pub pool_cuts_holding: UncheckedAccount<'info>,
    /// CHECK: the quote mint (bridged SOL).
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: `["royalty", item]` under the armory.
    pub royalty_owner: UncheckedAccount<'info>,
    /// CHECK: its holding of the token.
    #[account(mut)]
    pub royalty_token: UncheckedAccount<'info>,
    /// CHECK: its holding of the quote.
    #[account(mut)]
    pub royalty_quote: UncheckedAccount<'info>,
    /// CHECK: the cranker's holding of the token (created when missing).
    #[account(mut)]
    pub cranker_token: UncheckedAccount<'info>,
    /// CHECK: the cranker's holding of the quote (created when missing).
    #[account(mut)]
    pub cranker_quote: UncheckedAccount<'info>,
    /// CHECK: the armory's config (the settle bounty rate).
    pub armory_config: UncheckedAccount<'info>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
