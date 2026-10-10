// Changed by Hookwars: new file (expansion, 10); licence errors (11 section 1.4).
use anchor_lang::prelude::*;

#[error_code]
pub enum MarketError {
    #[msg("the signer is not this program's upgrade authority")]
    NotUpgradeAuthority,
    #[msg("only the market admin may do this")]
    NotAdmin,
    #[msg("a market parameter is out of bounds")]
    BadParams,
    #[msg("no params change is pending, or it is not ready yet")]
    NotReady,
    #[msg("an account is not the one expected")]
    WrongAccount,
    #[msg("not an item of the armory")]
    NotAnItem,
    #[msg("the signer does not hold the item")]
    NotItemHolder,
    #[msg("a price must be above zero")]
    ZeroPrice,
    #[msg("only the seller may do this")]
    NotSeller,
    #[msg("the listing has expired")]
    ListingExpired,
    #[msg("the listing has not expired")]
    NotExpired,
    #[msg("the price is above the buyer's maximum")]
    PriceMoved,
    #[msg("a recipient is not the one the listing or item names")]
    WrongRecipient,
    #[msg("a collection name is empty or too long")]
    BadName,
    #[msg("too many templates for a collection")]
    TooManyTemplates,
    #[msg("a template is not an active armory template")]
    UnknownTemplate,
    #[msg("a template appears twice")]
    DuplicateTemplate,
    #[msg("the lease term is outside the allowed range")]
    BadTerm,
    #[msg("the rent share is above the ceiling")]
    RentTooHigh,
    #[msg("the lease is not in the state this needs")]
    WrongLeaseState,
    #[msg("only the lessor may do this")]
    NotLessor,
    #[msg("the lease term is not over")]
    LeaseNotOver,
    #[msg("the slot does not exist on this token")]
    NoSuchSlot,
    #[msg("the slot is not chosen by holder vote")]
    SlotNotVotable,
    #[msg("the bounty is below the minimum")]
    BountyTooSmall,
    #[msg("the brief URI is too long")]
    BadUri,
    #[msg("the commission is not open")]
    CommissionNotOpen,
    #[msg("the commission is closed for submissions")]
    CommissionClosed,
    #[msg("the item's kind does not fit the slot")]
    DoesNotFit,
    #[msg("the submitted item is not equipped in the slot (or was already there)")]
    NotEquipped,
    #[msg("the refund window has not opened")]
    RefundTooEarly,
    #[msg("arithmetic overflow")]
    Overflow,
    // Licences (11 section 1.4).
    #[msg("licence terms are out of bounds")]
    BadLicenceTerms,
    #[msg("the item is not offered for licence")]
    NotLicensable,
    #[msg("every live licence of this item is taken")]
    LicenceSoldOut,
    #[msg("the licence is not live")]
    NotLive,
    #[msg("per-token licences cannot be revoked")]
    NotRevocable,
    #[msg("the licence has not ended yet")]
    NotEnded,
    #[msg("licence parameters out of bounds")]
    BadLicenceParams,
    // Security review 3.
    #[msg("only the payer of a live licence may renew it")]
    NotLicencePayer,
    #[msg("the item sits in a market escrow; licence it when it is back with its holder")]
    HolderIsEscrow,
    #[msg("end_lease needs the slot revert accounts, or the token mint when the slot no longer holds the item")]
    RevertAccountsMissing,
}
