// Changed by Hookwars: slot errors appended after NotHookAuthority.
//! Errors of the token standard.

use anchor_lang::prelude::*;

/// Custom errors (6000 + index).
#[error_code]
pub enum TokenError {
    #[msg("the amount is zero")]
    ZeroAmount,
    #[msg("insufficient funds")]
    InsufficientFunds,
    #[msg("insufficient delegated amount")]
    InsufficientDelegation,
    #[msg("the authority is neither the owner nor the delegate")]
    NotAuthorized,
    #[msg("the holding is frozen")]
    Frozen,
    #[msg("the holding belongs to another mint")]
    MintMismatch,
    #[msg("source and destination are the same holding")]
    SameAccount,
    #[msg("the mint has no such authority or it was revoked")]
    AuthorityRevoked,
    #[msg("the mint has a hook program but it was not passed")]
    HookProgramMissing,
    #[msg("the hook program passed is not the mint's")]
    WrongHookProgram,
    #[msg("the hook returned a delta larger than the amount")]
    DeltaTooLarge,
    #[msg("the hook named an invalid delta account")]
    InvalidDeltaAccount,
    #[msg("the max supply would be exceeded")]
    MaxSupplyExceeded,
    #[msg("invalid hook flags")]
    InvalidHookFlags,
    #[msg("invalid metadata")]
    InvalidMetadata,
    #[msg("invalid decimals")]
    InvalidDecimals,
    #[msg("the holding is not empty")]
    HoldingNotEmpty,
    #[msg("math overflow")]
    MathOverflow,
    #[msg("the holding exists for another mint or owner")]
    HoldingMismatch,
    #[msg("the hook signer is not this program's signer for the mint's hook program")]
    BadHookSigner,
    #[msg("the hook answered something this callback or the mint's flags do not allow")]
    UnsupportedHookReturn,
    #[msg("the hook's answer does not decode")]
    InvalidHookReturn,
    #[msg("the hook answered more than three deltas")]
    TooManyDeltas,
    #[msg("the hook answered a delta of zero")]
    ZeroDelta,
    #[msg("the holding still keeps hook data for its mint's hook")]
    HookDataNotEmpty,
    #[msg("the mint has no hook that writes hook data")]
    HookDataNotWritable,
    #[msg("the signer is not the hook authority of the mint's hook program")]
    NotHookAuthority,
    #[msg("the slot table breaks a rule of slot tables")]
    InvalidSlotTable,
    #[msg("the slot authority is not the armory's [\"slots\", mint] PDA")]
    InvalidSlotAuthority,
    #[msg("a mint has either one hook or a slot table, never both")]
    MixedHookModes,
    #[msg("too many slots may cut on one transfer")]
    TooManyCuttingSlots,
    #[msg("no such slot")]
    SlotIndexOutOfRange,
    #[msg("the slot is locked")]
    SlotLocked,
    #[msg("the slot is empty")]
    SlotEmpty,
    #[msg("the slot item is not acceptable")]
    InvalidSlotItem,
    #[msg("the slot's kind or bounds do not allow these flags")]
    SlotFlagsNotAllowed,
    #[msg("a slot's cut must go to its equip vault")]
    WrongEquipVault,
    #[msg("the cut exceeds the slot's bound")]
    SlotCutExceeded,
    #[msg("a slot's data answer has the wrong length")]
    SlotDataLength,
    #[msg("the slot accounts do not match the slots called")]
    SlotAccountsMismatch,
    #[msg("the slot does not answer touch")]
    TouchNotSupported,
    #[msg("the payload is too long")]
    PayloadTooLong,
    #[msg("the holding's tokens are locked for a vote")]
    VoteLocked,
    #[msg("the vote lock exceeds the balance")]
    VoteLockExceedsBalance,
    #[msg("the source is not a protocol vault")]
    NotProtocolSource,
}

/// The token program's error for a slot answer the protocol refuses.
pub fn slot_answer_error(e: bordrless_hook::SlotAnswerError) -> Error {
    use bordrless_hook::SlotAnswerError;
    match e {
        SlotAnswerError::Malformed => TokenError::InvalidHookReturn,
        SlotAnswerError::TooManyDeltas => TokenError::TooManyDeltas,
        SlotAnswerError::ZeroDelta => TokenError::ZeroDelta,
        SlotAnswerError::Unsupported => TokenError::UnsupportedHookReturn,
        SlotAnswerError::DataLength => TokenError::SlotDataLength,
    }
    .into()
}

/// The token program's error for an answer the hook protocol refuses.
pub fn answer_error(e: bordrless_hook::AnswerError) -> Error {
    use bordrless_hook::AnswerError;
    match e {
        AnswerError::Malformed => TokenError::InvalidHookReturn,
        AnswerError::TooManyDeltas => TokenError::TooManyDeltas,
        AnswerError::ZeroDelta => TokenError::ZeroDelta,
        AnswerError::DuplicateDeltaAccount => TokenError::InvalidDeltaAccount,
        AnswerError::DeltaTooLarge => TokenError::DeltaTooLarge,
        AnswerError::Unsupported => TokenError::UnsupportedHookReturn,
    }
    .into()
}
