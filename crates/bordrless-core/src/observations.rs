//! Hookwars: the price observation ring of a pool (spec 03 section 3.1), as bytes.
//!
//! The DEX keeps the ring in the tail of every pool account, right after the `Pool` fields
//! (`bordrless_swap::obs`). Its layout is fixed here so every reader (relation items, the armory's
//! `Performance` rule, the war program, the SDK) parses the same bytes. Offsets below are from the
//! start of the ring. Little-endian throughout.
//!
//! ```text
//! 0..8     discriminator (Anchor style, sha256("account:Observations")[..8])
//! 8        version
//! 9        reserved (0)
//! 10..42   pool
//! 42..58   cumulative: u128     sum over time of last_price_q64 * seconds, wrapping
//! 58..74   last_price_q64: u128 price at the end of the last write (quote per base, Q64.64)
//! 74..82   last_ts: i64         time of the last accumulation
//! 82..84   index: u16           next ring entry to write
//! 84..86   filled: u16          entries written (at most len)
//! 86..88   len: u16             ring length, fixed at creation
//! 88..92   spacing: u32         least seconds between two entries, fixed at creation
//! 92..96   reserved
//! 96..     len entries of 48 bytes:
//!            ts: i64, price_cumulative: u128, quote_volume: u128, swap_count: u64
//! ```
//!
//! Writing: at the start of every instruction that can move the price, [`accumulate`] adds
//! `last_price_q64 * (now - last_ts)` to `cumulative` and, when the newest entry is at least
//! `spacing` seconds old, writes `{ now, cumulative, quote_volume, swap_count }`; at the end,
//! [`set_price`] stores the price the instruction left. So the price that accrues for a second is
//! the one the previous instruction ended at, and an instruction never accrues at its own price.
//!
//! Reading: [`window_read`] returns the time-weighted price, the quote volume and the swap count
//! over at least `window` seconds, or a [`ReadError`] that callers treat as "no signal".

/// Discriminator length.
pub const DISCRIMINATOR_LEN: usize = 8;
/// Header length (discriminator included).
pub const HEADER_LEN: usize = 96;
/// One ring entry.
pub const ENTRY_LEN: usize = 48;
/// Layout version.
pub const VERSION: u8 = 1;

const O_VERSION: usize = 8;
const O_BUMP: usize = 9;
const O_POOL: usize = 10;
const O_CUMULATIVE: usize = 42;
const O_LAST_PRICE: usize = 58;
const O_LAST_TS: usize = 74;
const O_INDEX: usize = 82;
const O_FILLED: usize = 84;
const O_LEN: usize = 86;
const O_SPACING: usize = 88;

/// Account size for a ring of `len` entries.
pub const fn account_len(len: u16) -> usize {
    HEADER_LEN + ENTRY_LEN * len as usize
}

/// Why a read gives no signal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadError {
    /// The window asked for is shorter than the reader's minimum.
    WindowTooShort,
    /// No entry is old enough: the pool's history is shorter than the window, or the ring has
    /// wrapped past it.
    TooOld,
    /// The bytes are not an observation ring (too short, wrong discriminator or version).
    Malformed,
}

/// One ring entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Entry {
    /// When it was written.
    pub ts: i64,
    /// `cumulative` at `ts`.
    pub price_cumulative: u128,
    /// The pool's quote volume at `ts`, before the instruction that wrote it.
    pub quote_volume: u128,
    /// The pool's swap count at `ts`, before the instruction that wrote it.
    pub swap_count: u64,
}

/// The header fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Header {
    /// Layout version.
    pub version: u8,
    /// Reserved (0).
    pub bump: u8,
    /// The pool.
    pub pool: [u8; 32],
    /// Running cumulative price.
    pub cumulative: u128,
    /// Price at the end of the last write.
    pub last_price_q64: u128,
    /// Time of the last accumulation.
    pub last_ts: i64,
    /// Next entry to write.
    pub index: u16,
    /// Entries written, at most `len`.
    pub filled: u16,
    /// Ring length.
    pub len: u16,
    /// Least seconds between entries.
    pub spacing: u32,
}

/// What a window read answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowRead {
    /// Time-weighted price over the window, Q64.64 quote per base.
    pub twap_q64: u128,
    /// Quote volume traded over the window.
    pub quote_volume: u128,
    /// Swaps over the window.
    pub swaps: u64,
    /// The window actually spanned (at least the one asked for).
    pub span_secs: i64,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}
fn u32_at(d: &[u8], o: usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&d[o..o + 4]);
    u32::from_le_bytes(b)
}
fn i64_at(d: &[u8], o: usize) -> i64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[o..o + 8]);
    i64::from_le_bytes(b)
}
fn u64_at(d: &[u8], o: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[o..o + 8]);
    u64::from_le_bytes(b)
}
fn u128_at(d: &[u8], o: usize) -> u128 {
    let mut b = [0u8; 16];
    b.copy_from_slice(&d[o..o + 16]);
    u128::from_le_bytes(b)
}

/// The price of a pool as Q64.64 quote per base, from its effective reserves (real plus
/// virtual). Both totals are scaled down together until they fit 64 bits, which keeps the ratio
/// to within one part in 2^63; an empty base side answers 0.
pub fn price_q64(quote_total: u128, base_total: u128) -> u128 {
    let (mut q, mut b) = (quote_total, base_total);
    while q > u64::MAX as u128 || b > u64::MAX as u128 {
        q >>= 1;
        b >>= 1;
    }
    if b == 0 {
        return 0;
    }
    (q << 64) / b
}

/// The effective-reserve price of a pool.
pub fn pool_price_q64(base_reserve: u64, quote_reserve: u64, virtual_base: u64, virtual_quote: u64) -> u128 {
    price_q64(
        quote_reserve as u128 + virtual_quote as u128,
        base_reserve as u128 + virtual_base as u128,
    )
}

/// Reads the header, checking length, discriminator and version.
pub fn header(data: &[u8], discriminator: &[u8; 8]) -> Result<Header, ReadError> {
    if data.len() < HEADER_LEN || &data[..8] != discriminator || data[O_VERSION] != VERSION {
        return Err(ReadError::Malformed);
    }
    let mut pool = [0u8; 32];
    pool.copy_from_slice(&data[O_POOL..O_POOL + 32]);
    let h = Header {
        version: data[O_VERSION],
        bump: data[O_BUMP],
        pool,
        cumulative: u128_at(data, O_CUMULATIVE),
        last_price_q64: u128_at(data, O_LAST_PRICE),
        last_ts: i64_at(data, O_LAST_TS),
        index: u16_at(data, O_INDEX),
        filled: u16_at(data, O_FILLED),
        len: u16_at(data, O_LEN),
        spacing: u32_at(data, O_SPACING),
    };
    if h.len == 0 || data.len() < account_len(h.len) || h.filled > h.len || h.index >= h.len {
        return Err(ReadError::Malformed);
    }
    Ok(h)
}

/// Entry `i` of the ring (0 ..= len - 1).
pub fn entry(data: &[u8], i: u16) -> Entry {
    let o = HEADER_LEN + ENTRY_LEN * i as usize;
    Entry {
        ts: i64_at(data, o),
        price_cumulative: u128_at(data, o + 8),
        quote_volume: u128_at(data, o + 24),
        swap_count: u64_at(data, o + 40),
    }
}

fn write_entry(data: &mut [u8], i: u16, e: &Entry) {
    let o = HEADER_LEN + ENTRY_LEN * i as usize;
    data[o..o + 8].copy_from_slice(&e.ts.to_le_bytes());
    data[o + 8..o + 24].copy_from_slice(&e.price_cumulative.to_le_bytes());
    data[o + 24..o + 40].copy_from_slice(&e.quote_volume.to_le_bytes());
    data[o + 40..o + 48].copy_from_slice(&e.swap_count.to_le_bytes());
}

fn write_header(data: &mut [u8], h: &Header) {
    data[O_VERSION] = h.version;
    data[O_BUMP] = h.bump;
    data[O_POOL..O_POOL + 32].copy_from_slice(&h.pool);
    data[O_CUMULATIVE..O_CUMULATIVE + 16].copy_from_slice(&h.cumulative.to_le_bytes());
    data[O_LAST_PRICE..O_LAST_PRICE + 16].copy_from_slice(&h.last_price_q64.to_le_bytes());
    data[O_LAST_TS..O_LAST_TS + 8].copy_from_slice(&h.last_ts.to_le_bytes());
    data[O_INDEX..O_INDEX + 2].copy_from_slice(&h.index.to_le_bytes());
    data[O_FILLED..O_FILLED + 2].copy_from_slice(&h.filled.to_le_bytes());
    data[O_LEN..O_LEN + 2].copy_from_slice(&h.len.to_le_bytes());
    data[O_SPACING..O_SPACING + 4].copy_from_slice(&h.spacing.to_le_bytes());
}

/// Fills a freshly allocated account (all zeros, `account_len(len)` bytes): the header, and one
/// entry at `now` with cumulative 0, so a read can start from the pool's creation.
#[allow(clippy::too_many_arguments)]
pub fn init(
    data: &mut [u8],
    discriminator: &[u8; 8],
    bump: u8,
    pool: [u8; 32],
    len: u16,
    spacing: u32,
    now: i64,
    price_q64: u128,
    quote_volume: u128,
    swap_count: u64,
) -> Result<(), ReadError> {
    if len == 0 || data.len() < account_len(len) {
        return Err(ReadError::Malformed);
    }
    data[..8].copy_from_slice(discriminator);
    let mut h = Header {
        version: VERSION,
        bump,
        pool,
        cumulative: 0,
        last_price_q64: price_q64,
        last_ts: now,
        index: 0,
        filled: 0,
        len,
        spacing,
    };
    write_entry(
        data,
        0,
        &Entry {
            ts: now,
            price_cumulative: 0,
            quote_volume,
            swap_count,
        },
    );
    h.index = 1 % len;
    h.filled = 1;
    write_header(data, &h);
    Ok(())
}

/// The start-of-instruction step: accrue `last_price_q64` over the seconds since `last_ts`, and
/// write an entry when the newest one is at least `spacing` seconds old. `quote_volume` and
/// `swap_count` are the pool's before this instruction. Answers whether an entry was written.
pub fn accumulate(
    data: &mut [u8],
    discriminator: &[u8; 8],
    now: i64,
    quote_volume: u128,
    swap_count: u64,
) -> Result<bool, ReadError> {
    let mut h = header(data, discriminator)?;
    if now <= h.last_ts {
        return Ok(false);
    }
    let dt = (now - h.last_ts) as u128;
    h.cumulative = h.cumulative.wrapping_add(h.last_price_q64.wrapping_mul(dt));
    h.last_ts = now;
    let newest = (h.index + h.len - 1) % h.len;
    let newest_ts = entry(data, newest).ts;
    let mut wrote = false;
    if h.filled == 0 || now - newest_ts >= h.spacing as i64 {
        write_entry(
            data,
            h.index,
            &Entry {
                ts: now,
                price_cumulative: h.cumulative,
                quote_volume,
                swap_count,
            },
        );
        h.index = (h.index + 1) % h.len;
        if h.filled < h.len {
            h.filled += 1;
        }
        wrote = true;
    }
    write_header(data, &h);
    Ok(wrote)
}

/// The end-of-instruction step: the price the instruction left.
pub fn set_price(data: &mut [u8], discriminator: &[u8; 8], price_q64: u128) -> Result<(), ReadError> {
    let mut h = header(data, discriminator)?;
    h.last_price_q64 = price_q64;
    write_header(data, &h);
    Ok(())
}

/// The time-weighted price, quote volume and swap count over at least `window` seconds ending at
/// `now`, measured from the newest entry written at or before `now - window`. `pool_quote_volume`
/// and `pool_swap_count` are the pool's current counters (read from the `Pool` account).
/// `min_window` is the reader's floor (`MIN_TWAP_SECS`).
pub fn window_read(
    data: &[u8],
    discriminator: &[u8; 8],
    now: i64,
    window: i64,
    min_window: i64,
    pool_quote_volume: u128,
    pool_swap_count: u64,
) -> Result<WindowRead, ReadError> {
    if window < min_window || window <= 0 {
        return Err(ReadError::WindowTooShort);
    }
    let h = header(data, discriminator)?;
    let target = now - window;
    let cum_now = if now > h.last_ts {
        h.cumulative
            .wrapping_add(h.last_price_q64.wrapping_mul((now - h.last_ts) as u128))
    } else {
        h.cumulative
    };
    // Newest first, back through the filled entries.
    let mut i = (h.index + h.len - 1) % h.len;
    for _ in 0..h.filled {
        let e = entry(data, i);
        if e.ts <= target {
            let span = now - e.ts;
            if span <= 0 {
                return Err(ReadError::TooOld);
            }
            let twap = cum_now.wrapping_sub(e.price_cumulative) / span as u128;
            return Ok(WindowRead {
                twap_q64: twap,
                quote_volume: pool_quote_volume.saturating_sub(e.quote_volume),
                swaps: pool_swap_count.saturating_sub(e.swap_count),
                span_secs: span,
            });
        }
        i = (i + h.len - 1) % h.len;
    }
    Err(ReadError::TooOld)
}

#[cfg(test)]
mod tests {
    use super::*;

    const D: [u8; 8] = *b"obs-test";
    const Q64: u128 = 1 << 64;

    fn fresh(len: u16, spacing: u32, now: i64, price: u128) -> [u8; 96 + 48 * 8] {
        let mut d = [0u8; 96 + 48 * 8];
        init(&mut d, &D, 254, [7u8; 32], len, spacing, now, price, 0, 0).unwrap();
        d
    }

    #[test]
    fn price_is_the_ratio_of_effective_reserves() {
        assert_eq!(pool_price_q64(100, 200, 0, 0), 2 * Q64);
        assert_eq!(pool_price_q64(50, 0, 50, 100), Q64);
        assert_eq!(price_q64(1, 0), 0);
        // Totals above 64 bits keep the ratio.
        assert_eq!(price_q64(1 << 70, 1 << 69), 2 * Q64);
        // Odd totals above 64 bits lose at most the scaled-away low bits.
        let p = price_q64(u128::from(u64::MAX) * 4 + 3, u128::from(u64::MAX) * 2 + 1);
        assert!(p.abs_diff(2 * Q64) <= 8);
    }

    #[test]
    fn a_constant_price_reads_as_itself() {
        let mut d = fresh(8, 1, 1_000, 3 * Q64);
        for t in 1_001..=1_005 {
            accumulate(&mut d, &D, t, 0, 0).unwrap();
            set_price(&mut d, &D, 3 * Q64).unwrap();
        }
        let r = window_read(&d, &D, 1_005, 4, 1, 0, 0).unwrap();
        assert_eq!(r.twap_q64, 3 * Q64);
        assert_eq!(r.span_secs, 4);
    }

    #[test]
    fn the_price_accrues_at_the_previous_close() {
        // Price 1 for 10 s, then 3 for 10 s: the 20 s TWAP is 2.
        let mut d = fresh(8, 1, 0, Q64);
        accumulate(&mut d, &D, 10, 0, 0).unwrap();
        set_price(&mut d, &D, 3 * Q64).unwrap();
        let r = window_read(&d, &D, 20, 20, 1, 0, 0).unwrap();
        assert_eq!(r.twap_q64, 2 * Q64);
    }

    #[test]
    fn one_entry_per_spacing_and_the_ring_wraps() {
        let mut d = fresh(4, 10, 0, Q64);
        // Writes at 0 (init), 10, 20, 30, 40; 15 and 25 accrue without an entry.
        let mut written = 0;
        for t in [5, 10, 15, 20, 25, 30, 40] {
            if accumulate(&mut d, &D, t, t as u128, t as u64).unwrap() {
                written += 1;
            }
        }
        assert_eq!(written, 4);
        let h = header(&d, &D).unwrap();
        assert_eq!((h.filled, h.len), (4, 4));
        // The entry at 0 was overwritten: a 40 s window has nothing old enough.
        assert_eq!(window_read(&d, &D, 40, 40, 1, 40, 40), Err(ReadError::TooOld));
        let r = window_read(&d, &D, 40, 30, 1, 40, 40).unwrap();
        assert_eq!((r.span_secs, r.quote_volume, r.swaps), (30, 30, 30));
    }

    #[test]
    fn short_windows_and_short_histories_give_no_signal() {
        let d = fresh(8, 1, 100, Q64);
        assert_eq!(window_read(&d, &D, 200, 10, 60, 0, 0), Err(ReadError::WindowTooShort));
        assert_eq!(window_read(&d, &D, 120, 60, 60, 0, 0), Err(ReadError::TooOld));
        assert_eq!(window_read(&[0u8; 10], &D, 200, 60, 60, 0, 0), Err(ReadError::Malformed));
    }

    #[test]
    fn a_one_second_spike_weighs_one_second() {
        // 600 s at price 1, then one second at 1000, then back to 1, read over 600 s.
        let mut d = fresh(8, 1, 0, Q64);
        accumulate(&mut d, &D, 600, 0, 0).unwrap();
        set_price(&mut d, &D, 1_000 * Q64).unwrap();
        accumulate(&mut d, &D, 601, 0, 0).unwrap();
        set_price(&mut d, &D, Q64).unwrap();
        let r = window_read(&d, &D, 1_200, 600, 600, 0, 0).unwrap();
        // The newest entry at or before 600 is the one written at 600: the spike second (600..601)
        // is inside the window and adds 999/span.
        let expected = (599 * Q64 + 1_000 * Q64) / 600;
        assert_eq!(r.twap_q64, expected);
        assert!(r.twap_q64 < 3 * Q64);
    }
}
