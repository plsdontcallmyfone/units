// Changed by Hookwars: new file (arsenal wave B).
//! Guild Tag (id 39, 08 section 4.7): holders wear a guild tag they choose. Range 3 bytes: tag,
//! `guild: u16`. No extras. Only the holding's owner may set it, through `touch` (R23); emptying
//! the holding clears it. No money effect.

use anchor_lang::prelude::*;
use bordrless_hook::TokenSlotArgs;

use super::{Env, TokenOut};
use crate::ItemsError;

/// Range bytes.
pub const LEN: usize = 3;
/// Layout tag.
pub const TAG: u8 = 0x27;

/// The `touch` payload a holder sends.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildTouch {
    /// Wear `guild` (0 removes the tag).
    SetGuild { guild: u16 },
}

/// The worn guild, if any.
pub fn guild(b: &[u8]) -> Option<u16> {
    (b.len() >= LEN && b[0] == TAG).then(|| u16::from_le_bytes(b[1..3].try_into().unwrap()))
}

/// `before_transfer`: clears the tag when the holding empties.
pub fn token(_env: &Env, args: &TokenSlotArgs, src: &[u8], _dst: &[u8]) -> Result<TokenOut> {
    if args.source != args.destination && args.source_balance == args.amount && guild(src).is_some() {
        return Ok(TokenOut {
            source: Some(vec![0; LEN]),
            ..Default::default()
        });
    }
    Ok(TokenOut::default())
}

/// `on_touch`: the holder sets the tag.
pub fn touch(_env: &Env, args: &TokenSlotArgs, _src: &[u8]) -> Result<Vec<u8>> {
    require_keys_eq!(args.authority, args.source_owner, ItemsError::NotHolder);
    let GuildTouch::SetGuild { guild } =
        GuildTouch::deserialize(&mut args.payload.as_slice()).map_err(|_| error!(ItemsError::BadParams))?;
    if guild == 0 {
        return Ok(vec![0; LEN]);
    }
    let mut v = vec![TAG];
    v.extend_from_slice(&guild.to_le_bytes());
    Ok(v)
}
