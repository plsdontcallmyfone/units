// Changed by Hookwars: new file (M3b), observation rings for suites that fake a market.
//! A pool's observation ring as bytes (`bordrless_core::observations` layout, 03 M3a notes): the
//! DEX keeps it in the tail of the pool account, from `Pool::LEN`.

use anchor_lang::prelude::Pubkey;
use bordrless_core::observations as o;

/// One entry: `(ts, price_cumulative, quote_volume, swap_count)`.
pub type RingEntry = (i64, u128, u128, u64);

/// A ring of `OBS_RING_LEN` entries holding `entries` (oldest first); the header's cumulative is
/// the newest entry's, its last price and time `last_price`, `last_ts`.
pub fn ring(pool: &Pubkey, last_price: u128, last_ts: i64, entries: &[RingEntry]) -> Vec<u8> {
    let len = bordrless_swap::constants::OBS_RING_LEN;
    let mut r = vec![0u8; o::account_len(len)];
    r[..8].copy_from_slice(&bordrless_swap::obs::OBSERVATIONS_DISCRIMINATOR);
    r[8] = o::VERSION;
    r[10..42].copy_from_slice(pool.as_ref());
    let cumulative = entries.last().map(|e| e.1).unwrap_or(0);
    r[42..58].copy_from_slice(&cumulative.to_le_bytes());
    r[58..74].copy_from_slice(&last_price.to_le_bytes());
    r[74..82].copy_from_slice(&last_ts.to_le_bytes());
    r[82..84].copy_from_slice(&((entries.len() as u16) % len).to_le_bytes());
    r[84..86].copy_from_slice(&(entries.len() as u16).to_le_bytes());
    r[86..88].copy_from_slice(&len.to_le_bytes());
    r[88..92].copy_from_slice(&bordrless_swap::constants::OBS_SPACING_SECS.to_le_bytes());
    for (i, (ts, cum, qv, sc)) in entries.iter().enumerate() {
        let at = o::HEADER_LEN + o::ENTRY_LEN * i;
        r[at..at + 8].copy_from_slice(&ts.to_le_bytes());
        r[at + 8..at + 24].copy_from_slice(&cum.to_le_bytes());
        r[at + 24..at + 40].copy_from_slice(&qv.to_le_bytes());
        r[at + 40..at + 48].copy_from_slice(&sc.to_le_bytes());
    }
    r
}

/// `pool_data` (a serialized `Pool`) with `ring` in its tail.
pub fn with_ring(mut pool_data: Vec<u8>, ring: &[u8]) -> Vec<u8> {
    let base = bordrless_swap::state::Pool::LEN;
    pool_data.resize(base, 0);
    pool_data.extend_from_slice(ring);
    pool_data
}
