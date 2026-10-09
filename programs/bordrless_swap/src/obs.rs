//! Hookwars: the pool's observation ring (spec 03 section 3.1, as built: see the M3a
//! implementation notes there).
//!
//! The ring lives in the **tail of the pool account**, from offset [`Pool::LEN`]: `create_pool`
//! allocates the pool with room for it, and every instruction that can move the price writes it at
//! its start and end. Keeping it in the pool adds no account to any transaction (a separate
//! `["obs", pool]` account made upstream's launch with a custom hook 1,233 bytes, over Solana's
//! 1,232), and a reader needs the pool's counters anyway, so one account answers a read. Anchor
//! writes back only the `Pool` struct's own bytes, so the tail is untouched by it.
//!
//! The byte layout and the math are `bordrless_core::observations`.

use anchor_lang::prelude::*;
use bordrless_core::observations as core_obs;

use crate::constants::*;
use crate::error::SwapError;
use crate::state::Pool;

/// `sha256("account:Observations")[..8]`, Anchor's discriminator for the name; the ring's own tag
/// inside the pool account.
pub const OBSERVATIONS_DISCRIMINATOR: [u8; 8] = [119, 205, 13, 6, 93, 29, 178, 203];

/// The pool account's size with its ring.
pub const POOL_ACCOUNT_LEN: usize = Pool::LEN + core_obs::account_len(OBS_RING_LEN);

/// The ring inside a pool account's data (empty when the account has none).
pub fn ring_of(pool_data: &[u8]) -> &[u8] {
    pool_data.get(Pool::LEN..).unwrap_or(&[])
}

/// The pool's price as the ring stores it.
pub fn price_of(pool: &Pool) -> u128 {
    core_obs::pool_price_q64(
        pool.base_reserve,
        pool.quote_reserve,
        pool.virtual_base,
        pool.virtual_quote,
    )
}

/// A time-weighted read of a pool account (the ring and the pool's current counters), for
/// readers in other programs: `window` at least `min_window` (`MIN_TWAP_SECS`). Any error means
/// no signal.
pub fn window_read(
    pool_info: &AccountInfo,
    now: i64,
    window: i64,
    min_window: i64,
) -> core::result::Result<core_obs::WindowRead, core_obs::ReadError> {
    if *pool_info.owner != crate::ID {
        return Err(core_obs::ReadError::Malformed);
    }
    let data = pool_info
        .try_borrow_data()
        .map_err(|_| core_obs::ReadError::Malformed)?;
    let pool = Pool::try_deserialize(&mut &data[..]).map_err(|_| core_obs::ReadError::Malformed)?;
    core_obs::window_read(
        ring_of(&data),
        &OBSERVATIONS_DISCRIMINATOR,
        now,
        window,
        min_window,
        pool.quote_volume,
        pool.swap_count,
    )
}

/// The start of a price-moving instruction, before any reserve changes: accrue the last price and
/// write an entry when one is due.
pub fn begin(pool_info: &AccountInfo, pool: &Pool, now: i64) -> Result<()> {
    let mut data = pool_info.try_borrow_mut_data()?;
    let ring = data
        .get_mut(Pool::LEN..)
        .ok_or(SwapError::WrongObservations)?;
    core_obs::accumulate(
        ring,
        &OBSERVATIONS_DISCRIMINATOR,
        now,
        pool.quote_volume,
        pool.swap_count,
    )
    .map_err(|_| SwapError::WrongObservations)?;
    Ok(())
}

/// The end of a price-moving instruction: the price it left.
pub fn end(pool_info: &AccountInfo, pool: &Pool) -> Result<()> {
    let mut data = pool_info.try_borrow_mut_data()?;
    let ring = data
        .get_mut(Pool::LEN..)
        .ok_or(SwapError::WrongObservations)?;
    core_obs::set_price(ring, &OBSERVATIONS_DISCRIMINATOR, price_of(pool))
        .map_err(|_| SwapError::WrongObservations)?;
    Ok(())
}

/// Fills the ring of a new pool (`create_pool`, after the first deposit), with one entry at `now`
/// at the pool's opening price.
pub fn create(pool_info: &AccountInfo, pool_key: &Pubkey, pool: &Pool, now: i64) -> Result<()> {
    let mut data = pool_info.try_borrow_mut_data()?;
    let ring = data
        .get_mut(Pool::LEN..)
        .ok_or(SwapError::WrongObservations)?;
    core_obs::init(
        ring,
        &OBSERVATIONS_DISCRIMINATOR,
        0,
        pool_key.to_bytes(),
        OBS_RING_LEN,
        OBS_SPACING_SECS,
        now,
        price_of(pool),
        pool.quote_volume,
        pool.swap_count,
    )
    .map_err(|_| SwapError::WrongObservations)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only for its discriminator: Anchor's for the name `Observations`.
    #[account]
    pub struct Observations {}

    #[test]
    fn the_discriminator_is_anchors() {
        use anchor_lang::Discriminator;
        assert_eq!(Observations::DISCRIMINATOR, &OBSERVATIONS_DISCRIMINATOR[..]);
    }
}
