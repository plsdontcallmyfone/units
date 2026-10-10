// Changed by Hookwars: new file (M4/M5); M3b integration: the owning crates' types replace the spec
// decoders (the armory's `Item`, `Template`, `ForgeCounter`; the shared `RaidLedger`; the DEX's
// ring in the pool account); pass 4b: the items program's `EquipState` targets, config items.
//! Accounts of other Hookwars programs, read with their owners' own types:
//! - [`Item`], [`Template`], [`ForgeCounter`]: `hookwars_armory::state` (02);
//! - [`RaidLedger`], [`RaidWindow`], [`Mark`]: `hookwars_common::raid` (04 section 2.9), written by
//!   `hookwars_items`;
//! - time-weighted prices: [`pool_twap`], `bordrless_swap::obs::window_read` on the pool account
//!   (03 M3a notes: the ring lives in the pool's tail).
//!
//! Each reader checks the owner program and the Anchor discriminator before decoding.

use anchor_lang::prelude::*;

use crate::constants::{ARMORY_ID, ITEMS_ID};
use crate::error::WarError;

pub use hookwars_armory::state::{ForgeCounter, Item, Template};
pub use hookwars_common::raid::{rolling_volume, Mark, RaidLedger, RaidWindow, RAID_TABLE_LEN};
pub use hookwars_common::Manifest;
pub use hookwars_common::PARAM_FIELDS;

/// `Template.status` of a template that may still be minted and equipped.
pub const TEMPLATE_ACTIVE: u8 = hookwars_armory::state::template_status::ACTIVE;

/// Anchor account discriminators this program checks by hand.
pub mod disc {
    /// The randomness adapter's result account (`oracle.rs`).
    pub const RANDOMNESS: [u8; 8] = [0xbc, 0x60, 0xd8, 0xf8, 0x5d, 0x5e, 0x31, 0x70];
    /// Pass 4b: `hookwars_items::EquipState` (`sha256("account:EquipState")[..8]`; items depends on
    /// war, so war decodes it by hand; pinned in `programs/tests/tests/war_expansion.rs`).
    pub const EQUIP_STATE: [u8; 8] = [224, 65, 82, 232, 178, 33, 39, 28];
}

/// Pass 4b: the first target of the item equipped in `mint`'s `slot`, from the items program's
/// `EquipState` at `["equip", mint, slot]` (layout after the discriminator: version, bump, mint,
/// slot, item, template_id, then `EquipConfig { targets: Vec<Pubkey>, role }`). `None` when the
/// account is not that slot's equip state of `item` or names no target.
pub fn equip_first_target(info: &AccountInfo, mint: &Pubkey, slot: u8, item: &Pubkey) -> Option<Pubkey> {
    if *info.owner != ITEMS_ID || *info.key != hookwars_common::pda::equip_state(mint, slot).0 {
        return None;
    }
    let d = info.try_borrow_data().ok()?;
    if d.len() < 8 + 2 + 32 + 1 + 32 + 2 + 4 || d[..8] != disc::EQUIP_STATE {
        return None;
    }
    let at = |o: usize| Pubkey::try_from(&d[o..o + 32]).ok();
    if at(10)? != *mint || d[42] != slot || at(43)? != *item {
        return None;
    }
    let n = u32::from_le_bytes(d[77..81].try_into().ok()?);
    if n == 0 || d.len() < 81 + 32 {
        return None;
    }
    at(81)
}

/// Pass 4b: a config item (Coalition 43, Rivalry 45) equipped by `mint`: the slot index and the
/// item's params, when `item` is in one of `mint`'s slots, is an item of `template_id`, and
/// `template` is that template of the items program.
pub fn config_item(
    mint: &bordrless_token::state::Mint,
    item: &AccountInfo,
    template: &AccountInfo,
    template_id: u16,
) -> Result<(u8, [u32; PARAM_FIELDS])> {
    let slot = mint
        .active_slots()
        .iter()
        .position(|s| s.item == *item.key && *item.key != Pubkey::default())
        .ok_or(error!(WarError::ItemNotEquipped))?;
    let it = Item::read(item)?;
    require!(it.template_id == template_id, WarError::WrongItem);
    require_keys_eq!(*template.key, template_address(template_id), WarError::WrongItem);
    let t = Template::read(template)?;
    require!(t.program == ITEMS_ID && t.id == template_id, WarError::WrongItem);
    Ok((slot as u8, it.params))
}

/// Reads an armory account of type `T` (owner and discriminator checked by `try_deserialize`).
fn armory<T: AccountDeserialize>(info: &AccountInfo) -> Result<T> {
    require_keys_eq!(*info.owner, ARMORY_ID, WarError::WrongForeignAccount);
    let data = info.try_borrow_data()?;
    T::try_deserialize(&mut &data[..]).map_err(|_| error!(WarError::WrongForeignAccount))
}

/// Readers and addresses of foreign accounts, as associated functions on their types.
pub trait Foreign: Sized {
    /// Reads the account.
    fn read(info: &AccountInfo) -> Result<Self>;
}

impl Foreign for Item {
    fn read(info: &AccountInfo) -> Result<Self> {
        armory(info)
    }
}

impl Foreign for Template {
    fn read(info: &AccountInfo) -> Result<Self> {
        armory(info)
    }
}

impl Foreign for ForgeCounter {
    fn read(info: &AccountInfo) -> Result<Self> {
        armory(info)
    }
}

impl Foreign for RaidLedger {
    fn read(info: &AccountInfo) -> Result<Self> {
        require_keys_eq!(*info.owner, ITEMS_ID, WarError::WrongForeignAccount);
        RaidLedger::decode(&info.try_borrow_data()?).ok_or(error!(WarError::WrongForeignAccount))
    }
}

/// `["template", id le]` under the armory.
pub fn template_address(id: u16) -> Pubkey {
    hookwars_common::pda::template(id).0
}

/// `["forges", wallet]` under the armory.
pub fn forge_counter_address(wallet: &Pubkey) -> Pubkey {
    hookwars_common::pda::forge_counter(wallet).0
}

/// `["raid-ledger", mint]` under the items program.
pub fn raid_ledger_address(mint: &Pubkey) -> Pubkey {
    hookwars_common::pda::raid_ledger(mint).0
}

/// The time-weighted price (Q64.64) of the pool `info` over the last `window` seconds at `now`,
/// or `None` (no signal) when its ring does not reach back that far.
pub fn pool_twap(info: &AccountInfo, now: i64, window: i64) -> Option<u128> {
    bordrless_swap::obs::window_read(info, now, window, 1)
        .ok()
        .map(|r| r.twap_q64)
}

/// A pool's price, quote per base unit as Q64.64, virtual reserves included; `None` for an empty
/// base side or a quote side too large for the format.
pub fn spot_q64(quote: u64, virtual_quote: u64, base: u64, virtual_base: u64) -> Option<u128> {
    let q = u128::from(quote).checked_add(u128::from(virtual_quote))?;
    let b = u128::from(base).checked_add(u128::from(virtual_base))?;
    if b == 0 || q >= 1u128 << 63 {
        return None;
    }
    Some((q << 64) / b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_volume_weights_the_previous_window() {
        let e = RaidWindow {
            rival_mint: Pubkey::new_unique(),
            window_start: 1_000,
            volume: 100,
            prev_volume: 1_000,
        };
        assert_eq!(rolling_volume(&e, 1_000, 100), 100 + 1_000);
        assert_eq!(rolling_volume(&e, 1_050, 100), 100 + 500);
        assert_eq!(rolling_volume(&e, 1_100, 100), 100);
        assert_eq!(rolling_volume(&e, 1_150, 100), 50);
        assert_eq!(rolling_volume(&e, 1_200, 100), 0);
    }

    #[test]
    fn manifest_is_sixteen_bytes() {
        let mut v = Vec::new();
        Manifest::default().serialize(&mut v).unwrap();
        assert_eq!(v.len(), 16);
    }
}
