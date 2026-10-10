// Changed by Hookwars: new file (M2); security review 1 (H-2, M-2); integration pass 3: WearAccountsMissing, NotCraftSigner, NotItemHolder.; protocol pass 4a: access, queue, submission and forge errors.
//! Errors of the armory (docs/spec/02-armory.md section 12).

use anchor_lang::prelude::*;

#[error_code]
pub enum ArmoryError {
    #[msg("not the admin")]
    NotAdmin,
    #[msg("the template program is upgradeable by someone not allowed")]
    TemplateUpgradeable,
    #[msg("the template program's ProgramData is missing or malformed")]
    ProgramDataMissing,
    #[msg("the template program is a protocol program")]
    InvalidTemplateProgram,
    #[msg("not a template kind")]
    InvalidKind,
    #[msg("the template's schema is invalid")]
    InvalidSchema,
    #[msg("royalty above the maximum")]
    RoyaltyTooHigh,
    #[msg("the template is retired or closed to this path")]
    TemplateClosed,
    #[msg("the template program changed since registration")]
    TemplateChanged,
    #[msg("the items are of different templates")]
    TemplateMismatch,
    #[msg("a param is outside its floor and ceiling")]
    ParamOutOfRange,
    #[msg("the item's kind is not the slot's")]
    KindMismatch,
    #[msg("not the war program's loot signer")]
    NotLootSigner,
    #[msg("not the launchpad's caller for this mint")]
    NotLaunchCaller,
    #[msg("the mint is not fresh")]
    NotFreshMint,
    #[msg("invalid performance rule")]
    InvalidRule,
    #[msg("not the item's holder")]
    NotItemOwner,
    #[msg("not enough royalty")]
    InsufficientRoyalty,
    #[msg("the slot is locked")]
    SlotLocked,
    #[msg("the item exceeds the slot's bounds")]
    OverBounds,
    #[msg("the slot's data range is too small")]
    DataRangeTooSmall,
    #[msg("the item is already in another slot of this token")]
    AlreadyEquipped,
    #[msg("a proposal is open for this slot")]
    ProposalOpen,
    #[msg("voting is closed")]
    VotingClosed,
    #[msg("invalid amount")]
    InvalidAmount,
    #[msg("the proposal cannot be executed")]
    NotExecutable,
    #[msg("the item is equipped")]
    ItemEquipped,
    #[msg("the forge would pass the template's maximum level")]
    MaxLevel,
    #[msg("the proposal is not final")]
    ProposalNotFinal,
    #[msg("vote locks of this proposal are still open")]
    VotesOpen,
    #[msg("the proposal has votes")]
    VotesExist,
    #[msg("voting has not ended")]
    VotingNotEnded,
    #[msg("the proposal still fits")]
    NotStale,
    #[msg("the slot already runs this item")]
    NoChange,
    #[msg("notice outside its bounds")]
    InvalidNotice,
    #[msg("invalid params")]
    InvalidParams,
    #[msg("the timelock has not passed")]
    Timelock,
    #[msg("not the pending admin")]
    NotPendingAdmin,
    #[msg("not the upgrade authority")]
    NotUpgradeAuthority,
    #[msg("a wrong account")]
    WrongAccount,
    #[msg("the items program answered nothing usable")]
    BadReturn,
    #[msg("the slot index is out of range")]
    SlotIndexOutOfRange,
    #[msg("the condition does not hold long enough")]
    ConditionNotMet,
    #[msg("royalties of a kit token go to a wallet (a key on the curve)")]
    RecipientOffCurve,
    #[msg("the proposer holds less than the proposal threshold")]
    BelowProposalThreshold,
    #[msg("only an agent badge mint may hold a Soulbound item")]
    NotBadge,
    #[msg("a bond on this proposal is still posted: resolve it first")]
    BondStillPosted,
    #[msg("the item is listed on the market: its royalties go with the sale")]
    ItemListed,
    #[msg("the item is leased to another token or slot")]
    ItemLeasedElsewhere,
    #[msg("only the market may end a lease")]
    NotMarketCaller,
    #[msg("this template's items wear: pass the craft init-wear accounts")]
    WearAccountsMissing,
    #[msg("only craft's [\"craft-signer\"] may mint a crafted item")]
    NotCraftSigner,
    #[msg("the signer does not hold this item")]
    NotItemHolder,
    // Protocol pass 4a (E-1, E-8, L-1, external templates, forge of composites).
    #[msg("this token may not equip the item under its access mode")]
    AccessDenied,
    #[msg("the item is exclusive and already equipped on another token")]
    ExclusiveInUse,
    #[msg("the template does not allow this access mode")]
    AccessNotAllowed,
    #[msg("licence terms out of bounds, or given for a mode that takes none")]
    BadLicenceTerms,
    #[msg("the item's access mode does not take this instruction")]
    WrongAccessMode,
    #[msg("the item's access to this token is still valid")]
    AccessStillValid,
    #[msg("the agent's live directive forbids this")]
    DirectiveForbids,
    #[msg("the wallet's level is too low for this")]
    LevelTooLow,
    #[msg("this admin action was not queued, or the queue entry is for another action")]
    NotQueued,
    #[msg("the approval is not live")]
    ApprovalNotLive,
    #[msg("the submission is closed or does not match")]
    SubmissionClosed,
    #[msg("composites forge only with the same module sequence")]
    ModulesMismatch,
}
