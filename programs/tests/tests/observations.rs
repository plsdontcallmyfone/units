// Changed by Hookwars: new file, M3a observation ring tests.
//! Hookwars M3a: the pool's observation ring (spec 03 section 3.1).

use anchor_lang::prelude::Pubkey;
use bordrless_core::observations::{self as ring, ReadError};
use bordrless_program_tests::fixture::World;
use bordrless_swap::client as swap;
use bordrless_swap::constants::{MIN_TWAP_SECS, OBS_RING_LEN, OBS_SPACING_SECS};
use bordrless_swap::instructions::{AddLiquidityArgs, CreatePoolArgs, SwapArgs};
use bordrless_swap::obs::{price_of, ring_of, OBSERVATIONS_DISCRIMINATOR as D, POOL_ACCOUNT_LEN};
use bordrless_swap::state::Pool;
use bordrless_token::client as token;
use solana_keypair::Keypair;
use solana_signer::Signer;

const SUPPLY: u64 = 1_000_000_000_000_000;
const DEPOSIT: u64 = 1_000_000_000_000;

struct Market {
    w: World,
    lp: Keypair,
    a: Pubkey,
    b: Pubkey,
    pool: Pubkey,
}

fn args(base: u64, quote: u64) -> CreatePoolArgs {
    CreatePoolArgs {
        lp_fee_bps: 30,
        hook_program: Pubkey::default(),
        hook_flags: 0,
        virtual_base: 0,
        virtual_quote: 0,
        base_amount: base,
        quote_amount: quote,
        base_hook_accounts: 0,
        quote_hook_accounts: 0,
        hook_data: vec![],
    }
}

fn keys(lp: &Keypair, a: Pubkey, b: Pubkey, treasury: Pubkey) -> swap::CreatePoolKeys {
    swap::CreatePoolKeys {
        payer: lp.pubkey(),
        authority: lp.pubkey(),
        treasury,
        base_mint: a,
        quote_mint: b,
        hook_caller: None,
    }
}

impl Market {
    fn new() -> Self {
        let mut w = World::new();
        let lp = w.env.funded(100_000_000_000);
        let a = w.mint_to_owner(&lp, 6, SUPPLY, "AAA");
        let b = w.mint_to_owner(&lp, 6, SUPPLY, "BBB");
        let pool = swap::pool_address(&a, &b, 30, None);
        let treasury = w.env.treasury.pubkey();
        let tx = w.env.send_paid_by(
            &[swap::create_pool(&keys(&lp, a, b, treasury), args(DEPOSIT, DEPOSIT), vec![])],
            &lp,
            &[],
        );
        tx.ok();
        Self { w, lp, a, b, pool }
    }

    fn swap_ix(&self, direction: u8, amount_in: u64) -> anchor_lang::solana_program::instruction::Instruction {
        swap::swap(
            &swap::SwapKeys {
                trader: self.lp.pubkey(),
                pool: self.pool,
                base_mint: self.a,
                quote_mint: self.b,
                trader_base: token::holding_address(&self.a, &self.lp.pubkey()),
                trader_quote: token::holding_address(&self.b, &self.lp.pubkey()),
                hook_program: None,
                base_mint_writable: false,
                quote_mint_writable: false,
            },
            SwapArgs {
                direction,
                amount_in,
                min_amount_out: 0,
                in_hook_accounts: 0,
                out_hook_accounts: 0,
                hook_data: vec![],
            },
            vec![],
        )
    }

    fn swap(&mut self, direction: u8, amount_in: u64) {
        let ix = self.swap_ix(direction, amount_in);
        let lp = self.lp.insecure_clone();
        self.w.env.send_paid_by(&[ix], &lp, &[]).ok();
    }

    /// The ring: the pool account's bytes after its `Pool` fields.
    fn data(&self) -> Vec<u8> {
        let account = self.w.env.account(&self.pool).expect("pool");
        ring_of(&account.data).to_vec()
    }

    fn pool_state(&self) -> Pool {
        self.w.env.read(&self.pool)
    }

    fn read(&self, window: i64) -> Result<ring::WindowRead, ReadError> {
        let p = self.pool_state();
        ring::window_read(
            &self.data(),
            &D,
            self.w.env.now,
            window,
            MIN_TWAP_SECS,
            p.quote_volume,
            p.swap_count,
        )
    }
}

#[test]
fn the_ring_opens_with_the_pool() {
    let m = Market::new();
    let data = m.data();
    assert_eq!(data.len(), ring::account_len(OBS_RING_LEN));
    let account = m.w.env.account(&m.pool).unwrap();
    assert_eq!(account.data.len(), POOL_ACCOUNT_LEN);
    assert_eq!(POOL_ACCOUNT_LEN, Pool::LEN + data.len());
    let h = ring::header(&data, &D).unwrap();
    assert_eq!(h.pool, m.pool.to_bytes());
    assert_eq!((h.len, h.spacing, h.filled, h.index), (OBS_RING_LEN, OBS_SPACING_SECS, 1, 1));
    assert_eq!(h.last_price_q64, price_of(&m.pool_state()));
    assert_eq!(h.last_ts, m.w.env.now);
    let e = ring::entry(&data, 0);
    assert_eq!((e.ts, e.price_cumulative, e.quote_volume, e.swap_count), (m.w.env.now, 0, 0, 0));
    println!(
        "budget | observation ring in the pool account | {} entries | {} bytes | pool account {} bytes (upstream {}) | rent {} lamports (upstream {})",
        OBS_RING_LEN,
        data.len(),
        POOL_ACCOUNT_LEN,
        Pool::LEN,
        m.w.env.rent(POOL_ACCOUNT_LEN),
        m.w.env.rent(Pool::LEN)
    );
}

#[test]
fn swaps_accrue_the_price_the_previous_instruction_left() {
    let mut m = Market::new();
    let p0 = price_of(&m.pool_state());
    m.w.env.warp(200);
    m.swap(1, 10_000_000_000);
    let p1 = price_of(&m.pool_state());
    m.w.env.warp(200);
    m.swap(0, 3_000_000_000);
    let p2 = price_of(&m.pool_state());
    m.w.env.warp(200);
    assert_ne!(p0, p1);
    assert_ne!(p1, p2);
    let r = m.read(600).unwrap();
    let expected = (p0 * 200)
        .wrapping_add(p1 * 200)
        .wrapping_add(p2 * 200)
        / 600;
    assert_eq!((r.twap_q64, r.span_secs, r.swaps), (expected, 600, 2));
    assert_eq!(r.quote_volume, m.pool_state().quote_volume);
    // Two swaps in the same second write one entry and accrue nothing for that second.
    let filled = ring::header(&m.data(), &D).unwrap().filled;
    m.swap(1, 1_000_000);
    m.swap(1, 1_000_000);
    assert_eq!(ring::header(&m.data(), &D).unwrap().filled, filled + 1);
}

#[test]
fn a_one_block_spike_barely_moves_a_min_twap_read() {
    let mut m = Market::new();
    let p0 = price_of(&m.pool_state());
    m.w.env.warp(MIN_TWAP_SECS);
    // A buy of a fifth of the pool's quote side, held for one second, then sold back.
    m.swap(1, DEPOSIT / 5);
    let spike = price_of(&m.pool_state());
    m.w.env.warp(1);
    let pool = m.pool_state();
    let back = DEPOSIT - pool.base_reserve;
    m.swap(0, back);
    m.w.env.warp(MIN_TWAP_SECS);
    let r = m.read(MIN_TWAP_SECS).unwrap();
    let moved = r.twap_q64.abs_diff(p0);
    let spike_move = spike.abs_diff(p0);
    println!(
        "twap moved {moved} (q64) for a spike of {spike_move} held 1 s, window {} s",
        r.span_secs
    );
    // The spike's second weighs at most 1 / span of the read, plus what selling back left.
    let residual = price_of(&m.pool_state()).abs_diff(p0);
    assert!(moved <= spike_move / r.span_secs as u128 + residual + 1);
    assert!(moved * 50 < spike_move);
}

#[test]
fn short_histories_and_short_windows_give_no_signal() {
    let mut m = Market::new();
    assert_eq!(m.read(MIN_TWAP_SECS), Err(ReadError::TooOld));
    m.w.env.warp(MIN_TWAP_SECS);
    assert!(m.read(MIN_TWAP_SECS).is_ok());
    assert_eq!(m.read(MIN_TWAP_SECS - 1), Err(ReadError::WindowTooShort));
    assert_eq!(m.read(2 * MIN_TWAP_SECS), Err(ReadError::TooOld));
}

#[test]
fn entries_are_spaced_and_the_ring_wraps() {
    let mut m = Market::new();
    let spacing = i64::from(OBS_SPACING_SECS);
    // Half-spacing steps: an entry every second step.
    for _ in 0..(2 * OBS_RING_LEN as usize + 4) {
        m.w.env.warp(spacing / 2);
        m.swap(1, 1_000_000);
    }
    let h = ring::header(&m.data(), &D).unwrap();
    assert_eq!(h.filled, OBS_RING_LEN);
    // Coverage is the ring's length times the spacing: older windows have no signal.
    let covered = spacing * (i64::from(OBS_RING_LEN) - 1);
    assert!(covered >= MIN_TWAP_SECS);
    assert!(m.read(covered).is_ok());
    assert_eq!(m.read(covered + 2 * spacing), Err(ReadError::TooOld));
}

#[test]
fn liquidity_changes_write_the_ring() {
    let mut m = Market::new();
    m.w.env.warp(OBS_SPACING_SECS as i64);
    let ix = swap::add_liquidity(
        &swap::LiquidityKeys {
            provider: m.lp.pubkey(),
            pool: m.pool,
            base_mint: m.a,
            quote_mint: m.b,
            hook_program: None,
        },
        AddLiquidityArgs {
            base_desired: 1_000_000,
            quote_desired: 1_000_000,
            min_lp: 0,
            base_hook_accounts: 0,
            quote_hook_accounts: 0,
            hook_data: vec![],
        },
        vec![],
    );
    let lp = m.lp.insecure_clone();
    m.w.env.send_paid_by(&[ix], &lp, &[]).ok();
    let h = ring::header(&m.data(), &D).unwrap();
    assert_eq!((h.filled, h.last_ts), (2, m.w.env.now));
}

#[test]
fn the_ring_survives_every_write_of_the_pool_fields() {
    // Swaps write the `Pool` fields back (Anchor) and the ring (the DEX); neither clobbers the other.
    let mut m = Market::new();
    for i in 0..5 {
        m.w.env.warp(i64::from(OBS_SPACING_SECS));
        m.swap((i % 2) as u8, 1_000_000_000);
    }
    let p = m.pool_state();
    assert_eq!(p.swap_count, 5);
    let h = ring::header(&m.data(), &D).unwrap();
    assert_eq!((h.filled, h.pool, h.last_ts), (6, m.pool.to_bytes(), m.w.env.now));
    assert_eq!(h.last_price_q64, price_of(&p));
    assert_eq!(ring::entry(&m.data(), 5).swap_count, 4);
}
