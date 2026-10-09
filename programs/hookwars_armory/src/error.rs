// Changed by Hookwars: new file (M2); security review 1 (H-2, M-2).
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
}
