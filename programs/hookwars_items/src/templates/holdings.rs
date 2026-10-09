// Changed by Hookwars: new file (arsenal waves D and E).
//! Reads of a holder's holding passed as a derived extra (R22): the account must be exactly
//! `["holding", mint, owner]` under the token program, or the read gives nothing.

use anchor_lang::prelude::*;

/// The balance of `owner`'s holding of `mint`, when `info` is that holding.
pub fn held(info: &AccountInfo, mint: &Pubkey, owner: &Pubkey) -> Option<u64> {
    if *info.owner != bordrless_token::ID || *info.key != hookwars_common::pda::holding(mint, owner) {
        return None;
    }
    bordrless_token::client::read_holding(info).ok().map(|h| h.amount)
}
