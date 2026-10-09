//! Accounts of programs that are not in this branch, decoded byte for byte from the spec.
//!
//! REPLACE WITH THE OWNING CRATE'S TYPES AT INTEGRATION (05 "M4/M5 implementation notes"):
//! - [`Item`], [`Template`], [`Manifest`], [`ForgeCounter`]: `hookwars_armory` (02 sections 2.3,
//!   2.4, 2.10; manifest 04 section 2.7);
//! - [`RaidLedger`], [`RaidWindow`], [`Mark`]: `hookwars_items` (04 section 2.9);
//! - [`ObservationHeader`], [`Observation`], [`twap`]: `bordrless_swap` / `bordrless_core`
//!   (03 section 3.1).
//!
//! Each decoder checks the owner program and the Anchor discriminator
//! (`sha256("account:<Name>")[..8]`) before reading, and reads with Borsh, as Anchor does.

use anchor_lang::prelude::*;

use crate::constants::{ARMORY_ID, ITEMS_ID, SWAP_ID};
use crate::error::WarError;

/// Parameter fields per item (`PARAM_FIELDS`, 00 section 6): at least 11, the War orders
/// template's count (04 section 3.9). REPLACE with the armory's constant.
pub const PARAM_FIELDS: usize = 11;
/// Raid windows a ledger keeps (`RAID_TABLE_LEN`, 04 section 2.9). REPLACE with the items
/// program's constant.
pub const RAID_TABLE_LEN: usize = 8;
/// An observation entry's size (03 section 3.1: "An entry is 48 bytes").
pub const OBSERVATION_LEN: usize = 48;

/// Anchor account discriminators, `sha256("account:<Name>")[..8]` (precomputed, as
/// `bordrless_hook::discriminators` are; the unit tests pin them).
pub mod disc {
    pub const ITEM: [u8; 8] = [0x5c, 0x9d, 0xa3, 0x82, 0x48, 0xfe, 0x56, 0xd8];
    pub const TEMPLATE: [u8; 8] = [0x2b, 0x1a, 0x58, 0x45, 0x45, 0x60, 0x09, 0x4f];
    pub const FORGE_COUNTER: [u8; 8] = [0x61, 0x21, 0xe4, 0x18, 0x49, 0x0f, 0xd0, 0x6a];
    pub const RAID_LEDGER: [u8; 8] = [0x6e, 0x7e, 0x93, 0x3e, 0xca, 0x11, 0x7f, 0xcb];
    pub const OBSERVATIONS: [u8; 8] = [0x77, 0xcd, 0x0d, 0x06, 0x5d, 0x1d, 0xb2, 0xcb];
    pub const RANDOMNESS: [u8; 8] = [0xbc, 0x60, 0xd8, 0xf8, 0x5d, 0x5e, 0x31, 0x70];
}

/// Reads an account of `owner` named `name`: owner, discriminator, then Borsh.
pub fn decode<T: AnchorDeserialize>(info: &AccountInfo, owner: &Pubkey, disc: [u8; 8]) -> Result<T> {
    require_keys_eq!(*info.owner, *owner, WarError::WrongForeignAccount);
    let data = info.try_borrow_data()?;
    require!(
        data.len() >= 8 && data[..8] == disc,
        WarError::WrongForeignAccount
    );
    T::deserialize(&mut &data[8..]).map_err(|_| error!(WarError::WrongForeignAccount))
}

// ------------------------------------------------------------------------------- armory (02)

/// 04 section 2.7: 16 bytes, field order fixed.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub kind: u8,
    pub token_flags: u16,
    pub pool_flags: u8,
    pub max_cut_buy_bps: u16,
    pub max_cut_sell_bps: u16,
    pub max_cut_transfer_bps: u16,
    pub max_discount_bps: u16,
    pub may_refuse: bool,
    pub may_burn: bool,
    pub data_bytes: u8,
    pub reads_other_pools: u8,
}

/// `Item` at `["item", item_mint]` under the armory (02 section 2.4).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub version: u8,
    pub bump: u8,
    pub item_mint: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub manifest: Manifest,
    pub author: Pubkey,
    pub royalty_bps: u16,
    pub level: u8,
    pub source: u8,
    pub equipped_count: u32,
    pub royalty_owner_bump: u8,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

impl Item {
    pub fn read(info: &AccountInfo) -> Result<Self> {
        decode(info, &ARMORY_ID, disc::ITEM)
    }
}

/// `Template` at `["template", id: u16 le]` under the armory (02 section 2.3).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct Template {
    pub version: u8,
    pub bump: u8,
    pub id: u16,
    pub program: Pubkey,
    pub code_hash: [u8; 32],
    pub deploy_slot: Option<u64>,
    pub kind: u8,
    pub field_count: u8,
    pub field_min: [u32; PARAM_FIELDS],
    pub field_max: [u32; PARAM_FIELDS],
    pub open_authoring: bool,
    pub loot_enabled: bool,
    pub forge_enabled: bool,
    pub max_level: u8,
    pub loot_royalty_bps: u16,
    pub status: u8,
    pub name: String,
    pub registered_by: Pubkey,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

/// `Template.status` of a template that may still be minted and equipped.
pub const TEMPLATE_ACTIVE: u8 = 0;

impl Template {
    pub fn read(info: &AccountInfo) -> Result<Self> {
        decode(info, &ARMORY_ID, disc::TEMPLATE)
    }

    /// `["template", id le]` under the armory.
    pub fn address(id: u16) -> Pubkey {
        Pubkey::find_program_address(&[b"template", &id.to_le_bytes()], &ARMORY_ID).0
    }
}

/// `ForgeCounter` at `["forges", wallet]` under the armory (02 section 2.10).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ForgeCounter {
    pub wallet: Pubkey,
    pub count: u64,
    pub bump: u8,
}

impl ForgeCounter {
    pub fn read(info: &AccountInfo) -> Result<Self> {
        decode(info, &ARMORY_ID, disc::FORGE_COUNTER)
    }

    pub fn address(wallet: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"forges", wallet.as_ref()], &ARMORY_ID).0
    }
}

// ------------------------------------------------------------------------------- items (04)

/// 04 section 2.9.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaidWindow {
    pub rival_mint: Pubkey,
    pub window_start: i64,
    pub volume: u64,
    pub prev_volume: u64,
}

/// 04 section 2.9.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mark {
    pub clock_slot: u64,
    pub recipient: Pubkey,
    pub rival: Pubkey,
    pub quote_volume: u64,
    pub stamped_slots: u8,
}

/// `RaidLedger` at `["raid-ledger", mint]` under the items program (04 section 2.9).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct RaidLedger {
    pub version: u8,
    pub bump: u8,
    pub mint: Pubkey,
    pub season_id: u32,
    pub outbound_volume_season: u64,
    pub inbound: [RaidWindow; RAID_TABLE_LEN],
    pub mark: Mark,
    pub reserved: [u8; 32],
}

impl RaidLedger {
    pub fn read(info: &AccountInfo) -> Result<Self> {
        decode(info, &ITEMS_ID, disc::RAID_LEDGER)
    }

    pub fn address(mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"raid-ledger", mint.as_ref()], &ITEMS_ID).0
    }

    /// Rolling inbound raid volume from `rival` at `now` over windows of `window` seconds
    /// (05 section 2.5: the previous window weighted by its overlap with the last `window`).
    pub fn rolling(&self, rival: &Pubkey, now: i64, window: i64) -> u64 {
        let Some(e) = self.inbound.iter().find(|e| e.rival_mint == *rival) else {
            return 0;
        };
        rolling_volume(e, now, window)
    }

    /// This season's inbound raid volume, when the ledger's season is `season`.
    pub fn season_volume(&self, season: u32) -> u64 {
        if self.season_id == season {
            self.outbound_volume_season
        } else {
            0
        }
    }
}

/// The rolling volume of one window entry at `now` (`window > 0`). The entry is stored as of its
/// last write; windows that ended since are rolled here the way 04 rolls them on write.
pub fn rolling_volume(e: &RaidWindow, now: i64, window: i64) -> u64 {
    if window <= 0 || now < e.window_start {
        return e.volume;
    }
    let passed = (now - e.window_start) / window;
    let (current, previous, start) = match passed {
        0 => (e.volume, e.prev_volume, e.window_start),
        1 => (0, e.volume, e.window_start + window),
        _ => return 0,
    };
    let overlap = (start + window - now).clamp(0, window);
    let weighted = u128::from(previous) * overlap as u128 / window as u128;
    current.saturating_add(weighted as u64)
}

// ------------------------------------------------------------------------------- DEX (03)

/// 03 section 3.1.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ObservationHeader {
    pub version: u8,
    pub bump: u8,
    pub pool: Pubkey,
    pub last_price_q64: u128,
    pub last_ts: i64,
    pub index: u16,
    pub filled: u16,
}

/// 03 section 3.1 (48 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    pub ts: i64,
    pub price_cumulative: u128,
    pub quote_volume: u128,
    pub swap_count: u64,
}

/// A pool's observations, decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Observations {
    pub header: ObservationHeader,
    pub entries: Vec<Observation>,
}

/// Header size after the discriminator.
pub const OBS_HEADER_LEN: usize = 1 + 1 + 32 + 16 + 8 + 2 + 2;

impl Observations {
    /// `["obs", pool]` under the DEX.
    pub fn address(pool: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"obs", pool.as_ref()], &SWAP_ID).0
    }

    /// Reads the account: owner, discriminator, header, then the filled entries of the ring.
    pub fn read(info: &AccountInfo, pool: &Pubkey) -> Result<Self> {
        require_keys_eq!(*info.key, Self::address(pool), WarError::WrongForeignAccount);
        require_keys_eq!(*info.owner, SWAP_ID, WarError::WrongForeignAccount);
        let data = info.try_borrow_data()?;
        require!(
            data.len() >= 8 + OBS_HEADER_LEN && data[..8] == disc::OBSERVATIONS,
            WarError::WrongForeignAccount
        );
        let header = ObservationHeader::deserialize(&mut &data[8..8 + OBS_HEADER_LEN])
            .map_err(|_| error!(WarError::WrongForeignAccount))?;
        require_keys_eq!(header.pool, *pool, WarError::WrongForeignAccount);
        let ring = &data[8 + OBS_HEADER_LEN..];
        let capacity = ring.len() / OBSERVATION_LEN;
        let filled = usize::from(header.filled).min(capacity);
        let mut entries = Vec::with_capacity(filled);
        for i in 0..filled {
            let at = i * OBSERVATION_LEN;
            entries.push(
                Observation::deserialize(&mut &ring[at..at + OBSERVATION_LEN])
                    .map_err(|_| error!(WarError::WrongForeignAccount))?,
            );
        }
        Ok(Self { header, entries })
    }

    /// The time-weighted price (Q64.64) over the last `window` seconds at `now`, or `None` (no
    /// signal) when the ring does not reach back that far (03 section 3.1, `window_read`).
    pub fn twap(&self, now: i64, window: i64) -> Option<u128> {
        if window <= 0 || self.entries.is_empty() {
            return None;
        }
        let newest = self.entries.iter().max_by_key(|e| e.ts)?;
        let elapsed = u128::try_from(now.checked_sub(self.header.last_ts)?).ok()?;
        let cum_now = newest
            .price_cumulative
            .wrapping_add(self.header.last_price_q64.wrapping_mul(elapsed));
        let target = now.checked_sub(window)?;
        let e = self
            .entries
            .iter()
            .filter(|e| e.ts <= target)
            .max_by_key(|e| e.ts)?;
        let span = u128::try_from(now - e.ts).ok().filter(|s| *s > 0)?;
        Some(cum_now.wrapping_sub(e.price_cumulative) / span)
    }
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
        // Inside the window: all of the current, the previous by its remaining overlap.
        assert_eq!(rolling_volume(&e, 1_000, 100), 100 + 1_000);
        assert_eq!(rolling_volume(&e, 1_050, 100), 100 + 500);
        // One window later the current becomes the previous.
        assert_eq!(rolling_volume(&e, 1_100, 100), 100);
        assert_eq!(rolling_volume(&e, 1_150, 100), 50);
        // Two windows later nothing is left.
        assert_eq!(rolling_volume(&e, 1_200, 100), 0);
    }

    #[test]
    fn manifest_is_sixteen_bytes() {
        let mut v = Vec::new();
        Manifest::default().serialize(&mut v).unwrap();
        assert_eq!(v.len(), 16);
    }

    #[test]
    fn an_observation_is_forty_eight_bytes() {
        let mut v = Vec::new();
        Observation::default().serialize(&mut v).unwrap();
        assert_eq!(v.len(), OBSERVATION_LEN);
        let mut h = Vec::new();
        ObservationHeader::default().serialize(&mut h).unwrap();
        assert_eq!(h.len(), OBS_HEADER_LEN);
    }

    #[test]
    fn twap_is_none_until_the_ring_reaches_back() {
        let price = 5u128 << 64;
        let obs = Observations {
            header: ObservationHeader {
                last_price_q64: price,
                last_ts: 2_000,
                filled: 2,
                ..Default::default()
            },
            entries: vec![
                Observation {
                    ts: 1_000,
                    price_cumulative: 0,
                    ..Default::default()
                },
                Observation {
                    ts: 2_000,
                    price_cumulative: price * 1_000,
                    ..Default::default()
                },
            ],
        };
        assert_eq!(obs.twap(2_500, 1_000), Some(price));
        assert_eq!(obs.twap(2_500, 2_000), None);
    }
}
