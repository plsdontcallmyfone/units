// Changed by Hookwars: new file (hook economy, 11 section 5).
use anchor_lang::prelude::*;

#[error_code]
pub enum CraftError {
    #[msg("the signer is not this program's upgrade authority")]
    NotUpgradeAuthority,
    #[msg("only the craft admin may do this")]
    NotAdmin,
    #[msg("a craft parameter is out of bounds")]
    BadParams,
    #[msg("no change is pending, or it is not ready yet")]
    NotReady,
    #[msg("an account is not the one expected")]
    WrongAccount,
    #[msg("the signer is not a registered caller's PDA")]
    NotCaller,
    #[msg("a material name or symbol is too long or empty")]
    BadName,
    #[msg("a drop rule is malformed")]
    BadDropRule,
    #[msg("a recipe is malformed")]
    BadRecipe,
    #[msg("the recipe is closed or not yet in effect")]
    RecipeClosed,
    #[msg("the recipe is not of this kind, or not for this item's template")]
    WrongRecipe,
    #[msg("the crafter's level is too low for this recipe")]
    LevelTooLow,
    #[msg("the signer does not hold the item")]
    NotItemHolder,
    #[msg("not an item of the armory")]
    NotAnItem,
    #[msg("the item never wears")]
    NeverWears,
    #[msg("arithmetic overflow")]
    Overflow,
}
