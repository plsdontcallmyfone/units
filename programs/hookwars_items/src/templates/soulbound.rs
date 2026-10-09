// Changed by Hookwars: new file (09 section 4.3, agents branch); integration pass 2: error in ItemsError.
//! Soulbound (id 42): the agent badge's Defense item. It refuses every transfer of a badge, so a
//! badge never leaves the agent key it was minted to; mints and burns pass. Extras: the mint.
//!
//! It binds only on a badge: a mint whose freeze authority is the agents signer
//! (`["agents-signer"]` under `hookwars_agents`). Equipped anywhere else it answers nothing, so it
//! can never be used to make an ordinary token untransferable.

use anchor_lang::prelude::*;
use bordrless_hook::{TokenSlotArgs, TokenSlotOp};
use hookwars_common::ids::AGENTS_SIGNER;

use super::{Env, TokenOut};

/// Errors of this template: `ItemsError::SoulboundTransfer` (code 7000, integration pass 2).
pub use crate::ItemsError as SoulboundError;

/// Whether `info` is `mint` and a badge.
fn is_badge(info: &AccountInfo, mint: &Pubkey) -> bool {
    if info.key != mint {
        return false;
    }
    bordrless_token::client::read_mint(info)
        .map(|m| m.freeze_authority == Some(AGENTS_SIGNER))
        .unwrap_or(false)
}

/// `before_transfer`: refuse a badge's transfers.
pub fn token(env: &Env, args: &TokenSlotArgs) -> Result<TokenOut> {
    if args.op == TokenSlotOp::Transfer && env.extras.first().is_some_and(|m| is_badge(m, &env.mint)) {
        return err!(SoulboundError::SoulboundTransfer);
    }
    Ok(TokenOut::default())
}
