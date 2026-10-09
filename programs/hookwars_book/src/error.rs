// Changed by Hookwars: new file (hook economy, 11 section 6).
use anchor_lang::prelude::*;

#[error_code]
pub enum BookError {
    #[msg("the signer is not this program's upgrade authority")]
    NotUpgradeAuthority,
    #[msg("only the book admin may do this")]
    NotAdmin,
    #[msg("a book parameter is out of bounds")]
    BadParams,
    #[msg("no change is pending, or it is not ready yet")]
    NotReady,
    #[msg("an account is not the one expected")]
    WrongAccount,
    #[msg("the base is not a material of the craft program")]
    BadBase,
    #[msg("the price is zero or not a multiple of the tick")]
    BadPrice,
    #[msg("the size is zero or below the minimum")]
    BadSize,
    #[msg("a post-only order would cross")]
    WouldCross,
    #[msg("the book side is full and the order is not better than its worst")]
    BookFull,
    #[msg("no such order")]
    NoSuchOrder,
    #[msg("only the order's owner may do this")]
    NotOwner,
    #[msg("the creator's Trader level is too low")]
    LevelTooLow,
    #[msg("the item does not fit the class, or is listed, leased or equipped")]
    ClassMismatch,
    #[msg("the class bid has expired")]
    BidExpired,
    #[msg("not an item of the armory")]
    NotAnItem,
    #[msg("the signer does not hold the item")]
    NotItemHolder,
    #[msg("arithmetic overflow")]
    Overflow,
}
