// Changed by Hookwars: new file (09); directive errors (11 section 4).
//! Errors of `hookwars_agents` (09 section 16).

use anchor_lang::prelude::*;

#[error_code]
pub enum AgentsError {
    #[msg("a required signature is missing")]
    MissingSignature,
    #[msg("the agent key must differ from the operator")]
    KeyIsOperator,
    #[msg("a field is longer than allowed")]
    FieldTooLong,
    #[msg("unknown kind bits")]
    BadKinds,
    #[msg("this agent key already has a passport")]
    KeyTaken,
    #[msg("the operator runs as many passports as allowed")]
    OperatorLimit,
    #[msg("insufficient funds")]
    InsufficientFunds,
    #[msg("the agent is not active")]
    AgentPaused,
    #[msg("no ed25519 instruction signs the link statement with the agent key")]
    BadLinkSignature,
    #[msg("the report data does not bind this agent key and passport")]
    BadReportData,
    #[msg("the attestation lasts longer than allowed")]
    TtlTooLong,
    #[msg("not a registered verifier")]
    NotVerifier,
    #[msg("only the armory, war or items may record")]
    NotRecorder,
    #[msg("the actor is not this agent")]
    NotThisAgent,
    #[msg("unknown record kind")]
    BadRecordKind,
    #[msg("the policy vault is frozen")]
    PolicyFrozen,
    #[msg("this program is not an allowed target")]
    TargetNotAllowed,
    #[msg("a vault holding of an untracked mint is writable")]
    UntrackedHolding,
    #[msg("the spend is over a policy limit")]
    LimitExceeded,
    #[msg("the passport is not a diplomat")]
    NotDiplomat,
    #[msg("the passport is too young to bond")]
    PassportTooYoung,
    #[msg("the proposals are not one treaty on two tokens by this agent")]
    BadProposalPair,
    #[msg("a proposal is already bonded")]
    AlreadyBonded,
    #[msg("the bond's proposals are not final")]
    BondNotFinal,
    #[msg("too early")]
    TooEarly,
    // ---- added by the build
    #[msg("only the operator may do this")]
    NotOperator,
    #[msg("only the admin may do this")]
    NotAdmin,
    #[msg("only this program's upgrade authority may initialize it")]
    NotUpgradeAuthority,
    #[msg("parameters out of range")]
    InvalidParams,
    #[msg("nothing is pending")]
    NothingPending,
    #[msg("the timelock has not passed")]
    TimelockActive,
    #[msg("an account is not the expected one")]
    WrongAccount,
    #[msg("unknown platform")]
    BadPlatform,
    #[msg("unknown status or a retired passport")]
    BadStatus,
    #[msg("unknown TEE kind")]
    BadTeeKind,
    #[msg("the badge's slot does not hold the Soulbound item")]
    BadgeNotEquipped,
    #[msg("the badge is already issued")]
    BadgeIssued,
    #[msg("the Soulbound item is not set")]
    NoSoulboundItem,
    #[msg("the attestation names another key")]
    StaleAttestation,
    #[msg("the bond is not in the right state")]
    BadBondStatus,
    #[msg("arithmetic overflow")]
    MathOverflow,
    // Directives, commits and postage (11 section 4).
    #[msg("no memo instruction with this directive in the transaction")]
    DirectiveMemoMissing,
    #[msg("the memo is not the directive this instruction names")]
    MemoMismatch,
    #[msg("the sequence is not the next one")]
    BadSeq,
    #[msg("the operator did not sign the memo")]
    MemoNotSigned,
    #[msg("memo parameters out of range")]
    BadMemoParams,
    #[msg("the signer is not the passport's agent key")]
    NotAgent,
    // Security review 3 (H-1).
    #[msg("the token program instruction is not one spend may call")]
    InstructionNotAllowed,
    #[msg("the call changed a vault holding's delegate, owner or freeze state")]
    VaultHoldingChanged,
}
