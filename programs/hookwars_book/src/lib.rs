// Changed by Hookwars: new program (hook economy, docs/spec/11-hook-economy.md section 6).
//! `hookwars_book`: a fully escrowed spot order book (R44).
//!
//! - **Material books** (`BookMarket` at `["book", base_mint]`): limit buys and sells of one craft
//!   material, priced in lamports per unit. A resting bid's lamports and a resting ask's units sit
//!   in `["book-escrow", market]` from placement to fill, cancel, eviction or expiry. `place`
//!   crosses the best opposite orders by price then time, up to `BOOK_MATCH_MAX` fills, and rests
//!   the rest; when a side is full a strictly better order evicts the worst one, which is refunded
//!   in full. The taker pays `BOOK_TAKER_BPS` and the maker `BOOK_MAKER_BPS` of each fill to the
//!   treasury.
//! - **Class bids** (`ClassBid` at `["class-bid", bidder, nonce]`): a standing bid for any item of
//!   a template at or above a level with parameters within ranges; the account holds the price.
//!   `match_class` sells one fitting item (not listed, leased or equipped) into it.
//!
//! The quote is native SOL (a deviation from bridged SOL in 11 6.2, recorded there). No margin,
//! no shorting, no order without its full escrow.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::system_program;
use hookwars_common::economy::{self as eco, counter, fee_source, skill};
use hookwars_social::profiles::{record_wallet_cpi, RecordAccs};

pub mod error;
pub mod events;
pub mod state;

use error::BookError;
use events::*;
use state::*;

declare_id!("C4k2QquxzDdgHf74tnvyyWQyGUR8xvhPo1i1gFYb639g");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "units book",
    project_url: "https://github.com/plsdontcallmyfone/units",
    contacts: "link:https://github.com/plsdontcallmyfone/units/security/advisories/new",
    policy: "https://github.com/plsdontcallmyfone/units/blob/main/SECURITY.md",
    source_code: "https://github.com/plsdontcallmyfone/units"
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn upgrade_authority(program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    require_keys_eq!(
        *program_data.key,
        hookwars_common::programdata_address(&crate::ID),
        BookError::NotUpgradeAuthority
    );
    require_keys_eq!(
        *program_data.owner,
        hookwars_common::ids::BPF_LOADER_UPGRADEABLE_ID,
        BookError::NotUpgradeAuthority
    );
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        BookError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    Ok(Some(Pubkey::new_from_array(data[13..45].try_into().unwrap())))
}

fn check_params(p: &BookParams) -> Result<()> {
    require!(
        u32::from(p.taker_bps) + u32::from(p.maker_bps) <= 10_000
            && p.slots > 0
            && usize::from(p.slots) <= SLOTS_CAP
            && p.match_max > 0,
        BookError::BadParams
    );
    Ok(())
}

fn expired(o: &Order, ts: i64) -> bool {
    o.expires_at != 0 && ts >= o.expires_at
}

/// Moves lamports out of an account this program owns (the escrow or a class bid).
fn take_lamports(from: &AccountInfo, to: &AccountInfo, amount: u64) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let mut f = from.try_borrow_mut_lamports()?;
    **f = f.checked_sub(amount).ok_or(BookError::Overflow)?;
    let mut t = to.try_borrow_mut_lamports()?;
    **t = t.checked_add(amount).ok_or(BookError::Overflow)?;
    Ok(())
}

/// Moves lamports from a signer. Zero is a no-op.
fn pay_sol<'info>(system: &AccountInfo<'info>, from: &AccountInfo<'info>, to: &AccountInfo<'info>, amount: u64) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    system_program::transfer(
        CpiContext::new(
            system.key(),
            system_program::Transfer {
                from: from.clone(),
                to: to.clone(),
            },
        ),
        amount,
    )
}

/// The token program's accounts every token call here takes.
pub struct TokenAccs<'a, 'info> {
    pub token_program: &'a AccountInfo<'info>,
    pub event_authority: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl TokenAccs<'_, '_> {
    fn check(&self) -> Result<()> {
        require_keys_eq!(*self.token_program.key, bordrless_token::ID, BookError::WrongAccount);
        require_keys_eq!(
            *self.event_authority.key,
            bordrless_token::client::event_authority(),
            BookError::WrongAccount
        );
        Ok(())
    }
}

fn create_holding<'info>(
    t: &TokenAccs<'_, 'info>,
    payer: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    holding: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *holding.key,
        bordrless_token::client::holding_address(mint.key, owner.key),
        BookError::WrongAccount
    );
    let ix = bordrless_token::client::create_holding(payer.key(), mint.key(), owner.key());
    invoke(
        &ix,
        &[
            payer.clone(),
            mint.clone(),
            owner.clone(),
            holding.clone(),
            t.system_program.clone(),
            t.event_authority.clone(),
            t.token_program.clone(),
        ],
    )?;
    Ok(())
}

/// A plain (hookless) transfer signed by `authority` (a signer, or a PDA of this program with
/// `seeds`).
#[allow(clippy::too_many_arguments)]
fn transfer<'info>(
    t: &TokenAccs<'_, 'info>,
    authority: &AccountInfo<'info>,
    source: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    amount: u64,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    let ix = bordrless_token::client::transfer(
        authority.key(),
        source.key(),
        destination.key(),
        mint.key(),
        None,
        vec![],
        amount,
    );
    let infos = [
        authority.clone(),
        source.clone(),
        destination.clone(),
        mint.clone(),
        t.token_program.clone(),
        t.event_authority.clone(),
    ];
    if seeds.is_empty() {
        invoke(&ix, &infos)?;
    } else {
        invoke_signed(&ix, &infos, seeds)?;
    }
    Ok(())
}

/// The accounts that settle with order owners: `(wallet, base holding)` pairs from the remaining
/// accounts, taken in the order the instruction meets the orders.
struct Pairs<'a, 'info> {
    accounts: &'a [AccountInfo<'info>],
    next: usize,
}

impl<'a, 'info> Pairs<'a, 'info> {
    fn take(&mut self, owner: &Pubkey, base_mint: &Pubkey) -> Result<(&'a AccountInfo<'info>, &'a AccountInfo<'info>)> {
        require!(self.next + 2 <= self.accounts.len(), BookError::WrongAccount);
        let (w, h) = (&self.accounts[self.next], &self.accounts[self.next + 1]);
        self.next += 2;
        require_keys_eq!(*w.key, *owner, BookError::WrongAccount);
        require_keys_eq!(
            *h.key,
            bordrless_token::client::holding_address(base_mint, owner),
            BookError::WrongAccount
        );
        Ok((w, h))
    }
}

/// Everything a book instruction needs to move base and lamports. Lamports leaving the escrow are
/// owed during the instruction and moved by `flush` after its last CPI: a direct lamport move
/// followed by a CPI that does not pass both accounts unbalances the caller's instruction.
struct Settle<'a, 'info> {
    t: TokenAccs<'a, 'info>,
    escrow: &'a AccountInfo<'info>,
    escrow_holding: &'a AccountInfo<'info>,
    base_mint: &'a AccountInfo<'info>,
    payer: &'a AccountInfo<'info>,
    market: Pubkey,
    escrow_bump: u8,
    owed: std::cell::RefCell<Vec<(AccountInfo<'info>, u64)>>,
}

impl<'info> Settle<'_, 'info> {
    /// Records `amount` lamports the escrow owes `to`.
    fn owe(&self, to: &AccountInfo<'info>, amount: u64) {
        if amount > 0 {
            self.owed.borrow_mut().push((to.clone(), amount));
        }
    }

    /// Pays everything owed; call after the instruction's last CPI.
    fn flush(&self) -> Result<()> {
        for (to, amount) in self.owed.borrow_mut().drain(..) {
            take_lamports(self.escrow, &to, amount)?;
        }
        Ok(())
    }

    /// Returns a removed order's escrow to its owner (`bounty_to` gets the bounty: the owner, or
    /// the cranker on expiry).
    fn refund(
        &self,
        side_: u8,
        o: &Order,
        wallet: &AccountInfo<'info>,
        holding: &AccountInfo<'info>,
        bounty_to: &AccountInfo<'info>,
    ) -> Result<()> {
        if side_ == side::BID {
            self.owe(wallet, o.quote_locked);
        } else if o.size > 0 {
            create_holding(&self.t, self.payer, self.base_mint, wallet, holding)?;
            let bump = [self.escrow_bump];
            let s: &[&[u8]] = &[seeds::ESCROW, self.market.as_ref(), &bump];
            transfer(&self.t, self.escrow, self.escrow_holding, holding, self.base_mint, o.size, &[s])?;
        }
        self.owe(bounty_to, o.bounty);
        Ok(())
    }
}

#[program]
pub mod hookwars_book {
    use super::*;

    // ---- config

    /// Creates the config; the program's upgrade authority signs.
    pub fn init(ctx: Context<Init>, admin: Pubkey, treasury: Pubkey, params: BookParams) -> Result<()> {
        let up = upgrade_authority(&ctx.accounts.program_data)?;
        require!(up == Some(ctx.accounts.authority.key()), BookError::NotUpgradeAuthority);
        check_params(&params)?;
        let c = &mut ctx.accounts.config;
        c.version = VERSION;
        c.bump = ctx.bumps.config;
        c.admin = admin;
        c.treasury = treasury;
        c.params = params;
        c.protocol_fees_total = 0;
        c.markets = 0;
        c.reserved = [0; 32];
        Ok(())
    }

    /// The admin proposes new params and treasury; they apply after `admin_timelock_secs`.
    pub fn propose_params(ctx: Context<ProposeParams>, params: BookParams, treasury: Pubkey) -> Result<()> {
        check_params(&params)?;
        let ready_at = now()?
            .checked_add(i64::from(ctx.accounts.config.params.admin_timelock_secs))
            .ok_or(BookError::Overflow)?;
        let p = &mut ctx.accounts.pending;
        p.bump = ctx.bumps.pending;
        p.params = params;
        p.treasury = treasury;
        p.ready_at = ready_at;
        p.active = true;
        emit_cpi!(BookParamsProposed { ready_at });
        Ok(())
    }

    /// Anyone applies a pending change once it is ready.
    pub fn apply_params(ctx: Context<ApplyParams>) -> Result<()> {
        let p = &mut ctx.accounts.pending;
        require!(p.active && now()? >= p.ready_at, BookError::NotReady);
        ctx.accounts.config.params = p.params;
        ctx.accounts.config.treasury = p.treasury;
        p.active = false;
        Ok(())
    }

    // ---- material books

    /// Opens the book of a craft material (one per material). Requires `Trader >=
    /// BOOK_CREATE_LEVEL` (3.3).
    pub fn create_market(ctx: Context<CreateMarket>, tick_lamports: u64, min_size: u64) -> Result<()> {
        require!(tick_lamports > 0 && min_size > 0, BookError::BadParams);
        let a = &ctx.accounts;
        let material = {
            let info = a.material.to_account_info();
            require_keys_eq!(*info.owner, eco::CRAFT_ID, BookError::BadBase);
            let data = info.try_borrow_data()?;
            hookwars_craft::state::Material::try_deserialize(&mut &data[..]).map_err(|_| error!(BookError::BadBase))?
        };
        require_keys_eq!(
            a.material.key(),
            Pubkey::find_program_address(
                &[hookwars_craft::state::seeds::MATERIAL, &material.id.to_le_bytes()],
                &eco::CRAFT_ID
            )
            .0,
            BookError::BadBase
        );
        require_keys_eq!(material.mint, a.base_mint.key(), BookError::BadBase);
        let level = a.config.params.create_level;
        if level > 0 {
            let l = hookwars_social::profiles::wallet_level(&a.profile, &a.skills, &a.creator.key(), skill::TRADER)?;
            require!(l >= level, BookError::LevelTooLow);
        }
        let sys = a.system_program.to_account_info();
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        create_holding(
            &t,
            &a.creator.to_account_info(),
            &a.base_mint,
            &a.escrow.to_account_info(),
            &a.escrow_holding,
        )?;
        let ts = now()?;
        let market_key = ctx.accounts.market.key();
        let e = &mut ctx.accounts.escrow;
        e.bump = ctx.bumps.escrow;
        e.market = market_key;
        let m = &mut ctx.accounts.market;
        m.version = VERSION;
        m.bump = ctx.bumps.market;
        m.escrow_bump = ctx.bumps.escrow;
        m.base_mint = ctx.accounts.base_mint.key();
        m.material_id = material.id;
        m.tick_lamports = tick_lamports;
        m.min_size = min_size;
        m.seq = 0;
        m.bids = Vec::new();
        m.asks = Vec::new();
        m.created_by = ctx.accounts.creator.key();
        m.fills = 0;
        ctx.accounts.config.markets = ctx.accounts.config.markets.saturating_add(1);
        emit_cpi!(MarketCreated {
            market: market_key,
            base_mint: m.base_mint,
            tick_lamports,
            min_size,
            created_by: m.created_by,
            ts
        });
        Ok(())
    }

    /// Places a limit order: crosses the best opposite orders (price, then time; expired orders
    /// are skipped), up to `BOOK_MATCH_MAX` fills, then rests what is left if it is at least
    /// `min_size`. Remaining accounts: a `(wallet, base holding)` pair for each maker filled, in
    /// book order, then one for the evicted owner when the side is full.
    #[allow(clippy::too_many_arguments)]
    pub fn place<'info>(
        mut ctx: Context<'info, Place<'info>>,
        side_: u8,
        price: u64,
        size: u64,
        post_only: bool,
        expires_at: i64,
        reference: [u8; 32],
    ) -> Result<()> {
        let ts = now()?;
        let a = &mut ctx.accounts;
        let m = &mut a.market;
        require!(side_ <= side::ASK, BookError::BadParams);
        require!(price > 0 && price % m.tick_lamports == 0, BookError::BadPrice);
        require!(size >= m.min_size, BookError::BadSize);
        require!(expires_at == 0 || expires_at > ts, BookError::BadParams);
        let p = a.config.params;
        let market = m.key();
        let base_mint_key = m.base_mint;
        let sys = a.system_program.to_account_info();
        let owner = a.owner.to_account_info();
        let escrow = a.escrow.to_account_info();
        let treasury = a.treasury.to_account_info();
        let (tp, tea) = (a.token_program.to_account_info(), a.token_event_authority.to_account_info());
        let (escrow_holding, base_mint) = (a.escrow_holding.to_account_info(), a.base_mint.to_account_info());
        let st = Settle {
            t: TokenAccs {
                token_program: &tp,
                event_authority: &tea,
                system_program: &sys,
            },
            escrow: &escrow,
            escrow_holding: &escrow_holding,
            base_mint: &base_mint,
            payer: &owner,
            market,
            escrow_bump: m.escrow_bump,
            owed: std::cell::RefCell::new(Vec::new()),
        };
        st.t.check()?;
        let mut pairs = Pairs {
            accounts: ctx.remaining_accounts,
            next: 0,
        };
        let escrow_seeds_bump = [m.escrow_bump];
        let escrow_seeds: &[&[u8]] = &[seeds::ESCROW, market.as_ref(), &escrow_seeds_bump];
        let mut remaining = size;
        let mut fills: u8 = 0;
        let mut fees_total: u64 = 0;
        // Cross.
        let mut i = 0usize;
        loop {
            if remaining == 0 || fills >= p.match_max {
                break;
            }
            let book = if side_ == side::BID { &mut m.asks } else { &mut m.bids };
            if i >= book.len() {
                break;
            }
            let o = book[i];
            if expired(&o, ts) {
                i += 1;
                continue;
            }
            let crosses = if side_ == side::BID { o.price <= price } else { o.price >= price };
            if !crosses {
                break;
            }
            require!(!post_only, BookError::WouldCross);
            let q = remaining.min(o.size);
            let quote = o.price.checked_mul(q).ok_or(BookError::Overflow)?;
            let taker_fee = eco::bps(quote, p.taker_bps);
            let maker_fee = eco::bps(quote, p.maker_bps);
            let (wallet, holding) = pairs.take(&o.owner, &base_mint_key)?;
            let mut left = o;
            left.size -= q;
            if side_ == side::BID {
                // Taker buys: pays the maker and the fees, receives base from escrow.
                pay_sol(&sys, &owner, wallet, quote - maker_fee)?;
                pay_sol(&sys, &owner, &treasury, taker_fee + maker_fee)?;
                create_holding(&st.t, &owner, st.base_mint, &owner, &a.owner_holding)?;
                transfer(&st.t, &escrow, st.escrow_holding, &a.owner_holding, st.base_mint, q, &[escrow_seeds])?;
            } else {
                // Taker sells: gives base to the maker, receives the escrowed quote less its fee.
                create_holding(&st.t, &owner, st.base_mint, wallet, holding)?;
                transfer(&st.t, &owner, &a.owner_holding, holding, st.base_mint, q, &[])?;
                st.owe(&owner, quote - taker_fee);
                st.owe(&treasury, taker_fee + maker_fee);
                left.quote_locked = left
                    .quote_locked
                    .checked_sub(quote + maker_fee)
                    .ok_or(BookError::Overflow)?;
            }
            fees_total = fees_total.saturating_add(taker_fee + maker_fee);
            fills += 1;
            remaining -= q;
            emit!(Filled {
                market,
                maker_order: o.id,
                maker: o.owner,
                taker: owner.key(),
                side: side_,
                price: o.price,
                size: q,
                taker_fee,
                maker_fee,
                reference,
                ts
            });
            if left.size == 0 {
                let book = if side_ == side::BID { &mut m.asks } else { &mut m.bids };
                book.remove(i);
                // A filled bid's leftover reserve and every filled order's bounty go home.
                let opp = if side_ == side::BID { side::ASK } else { side::BID };
                st.refund(opp, &left, wallet, holding, wallet)?;
            } else {
                let book = if side_ == side::BID { &mut m.asks } else { &mut m.bids };
                book[i] = left;
                i += 1;
            }
        }
        m.fills = m.fills.saturating_add(u64::from(fills));
        // Rest.
        let mut resting = 0;
        if remaining >= m.min_size {
            let len = if side_ == side::BID { m.bids.len() } else { m.asks.len() };
            if len >= usize::from(p.slots) {
                let worst = if side_ == side::BID { *m.bids.last().unwrap() } else { *m.asks.last().unwrap() };
                let better = if side_ == side::BID { price > worst.price } else { price < worst.price };
                require!(better, BookError::BookFull);
                if side_ == side::BID {
                    m.bids.pop();
                } else {
                    m.asks.pop();
                }
                let (wallet, holding) = pairs.take(&worst.owner, &base_mint_key)?;
                st.refund(side_, &worst, wallet, holding, wallet)?;
                emit!(Evicted {
                    market,
                    id: worst.id,
                    owner: worst.owner,
                    ts
                });
            }
            let quote = price.checked_mul(remaining).ok_or(BookError::Overflow)?;
            let mut o = Order {
                id: m.seq,
                owner: owner.key(),
                price,
                size: remaining,
                quote_locked: 0,
                bounty: p.order_bounty_lamports,
                expires_at,
            };
            if side_ == side::BID {
                o.quote_locked = quote
                    .checked_add(eco::bps_up(quote, p.maker_bps))
                    .ok_or(BookError::Overflow)?;
                pay_sol(&sys, &owner, &escrow, o.quote_locked + o.bounty)?;
            } else {
                transfer(&st.t, &owner, &a.owner_holding, st.escrow_holding, st.base_mint, remaining, &[])?;
                pay_sol(&sys, &owner, &escrow, o.bounty)?;
            }
            let at = m.insert_at(side_, price);
            if side_ == side::BID {
                m.bids.insert(at, o);
            } else {
                m.asks.insert(at, o);
            }
            m.seq += 1;
            resting = remaining;
        }
        let id = if resting > 0 { m.seq - 1 } else { u64::MAX };
        if fees_total > 0 {
            a.config.protocol_fees_total = a.config.protocol_fees_total.saturating_add(u128::from(fees_total));
            emit!(ProtocolFee {
                source: fee_source::BOOK_FILL,
                mint: Pubkey::default(),
                amount: fees_total,
                reference,
                ts
            });
        }
        if fills > 0 {
            let s = RecordAccs {
                skills: &a.skills,
                profile: &a.profile,
                caller: &a.social_caller,
                event_authority: &a.social_event_authority,
                program: &a.social_program,
            };
            record_wallet_cpi(&s, &crate::ID, counter::BOOK_FILLS, u64::from(fills))?;
        }
        emit_cpi!(Placed {
            market,
            id,
            owner: owner.key(),
            side: side_,
            price,
            size,
            resting,
            reference,
            ts
        });
        st.flush()
    }

    /// The owner cancels a resting order and gets its escrow back.
    pub fn cancel(mut ctx: Context<Cancel>, id: u64) -> Result<()> {
        let ts = now()?;
        let a = &mut ctx.accounts;
        let m = &mut a.market;
        let (side_, idx) = if let Some(i) = m.bids.iter().position(|o| o.id == id) {
            (side::BID, i)
        } else if let Some(i) = m.asks.iter().position(|o| o.id == id) {
            (side::ASK, i)
        } else {
            return err!(BookError::NoSuchOrder);
        };
        let o = if side_ == side::BID { m.bids.remove(idx) } else { m.asks.remove(idx) };
        require_keys_eq!(o.owner, a.owner.key(), BookError::NotOwner);
        let market = m.key();
        let sys = a.system_program.to_account_info();
        let owner = a.owner.to_account_info();
        let escrow = a.escrow.to_account_info();
        let (tp, tea) = (a.token_program.to_account_info(), a.token_event_authority.to_account_info());
        let (escrow_holding, base_mint) = (a.escrow_holding.to_account_info(), a.base_mint.to_account_info());
        let st = Settle {
            t: TokenAccs {
                token_program: &tp,
                event_authority: &tea,
                system_program: &sys,
            },
            escrow: &escrow,
            escrow_holding: &escrow_holding,
            base_mint: &base_mint,
            payer: &owner,
            market,
            escrow_bump: m.escrow_bump,
            owed: std::cell::RefCell::new(Vec::new()),
        };
        st.t.check()?;
        st.refund(side_, &o, &owner, &a.owner_holding, &owner)?;
        emit_cpi!(Cancelled {
            market,
            id,
            owner: o.owner,
            ts
        });
        st.flush()
    }

    /// Anyone removes up to `max` expired orders (bids first, then asks, in book order), returning
    /// their escrow to the owners and taking their bounties. Remaining accounts: a `(wallet, base
    /// holding)` pair per order removed.
    pub fn crank<'info>(mut ctx: Context<'info, Crank<'info>>, max: u8) -> Result<()> {
        let ts = now()?;
        let a = &mut ctx.accounts;
        let m = &mut a.market;
        let market = m.key();
        let base_mint_key = m.base_mint;
        let sys = a.system_program.to_account_info();
        let cranker = a.cranker.to_account_info();
        let escrow = a.escrow.to_account_info();
        let (tp, tea) = (a.token_program.to_account_info(), a.token_event_authority.to_account_info());
        let (escrow_holding, base_mint) = (a.escrow_holding.to_account_info(), a.base_mint.to_account_info());
        let st = Settle {
            t: TokenAccs {
                token_program: &tp,
                event_authority: &tea,
                system_program: &sys,
            },
            escrow: &escrow,
            escrow_holding: &escrow_holding,
            base_mint: &base_mint,
            payer: &cranker,
            market,
            escrow_bump: m.escrow_bump,
            owed: std::cell::RefCell::new(Vec::new()),
        };
        st.t.check()?;
        let mut pairs = Pairs {
            accounts: ctx.remaining_accounts,
            next: 0,
        };
        let mut done = 0u8;
        for side_ in [side::BID, side::ASK] {
            let mut i = 0;
            loop {
                let book = if side_ == side::BID { &mut m.bids } else { &mut m.asks };
                if done >= max || i >= book.len() {
                    break;
                }
                if !expired(&book[i], ts) {
                    i += 1;
                    continue;
                }
                let o = book.remove(i);
                let (wallet, holding) = pairs.take(&o.owner, &base_mint_key)?;
                st.refund(side_, &o, wallet, holding, &cranker)?;
                done += 1;
                emit!(Expired {
                    market,
                    id: o.id,
                    owner: o.owner,
                    cranker: cranker.key(),
                    bounty: o.bounty,
                    ts
                });
            }
        }
        st.flush()
    }

    // ---- class bids

    /// Opens a standing bid for any item of `class` at `price`; the bid account holds the price
    /// and the maker fee.
    pub fn place_class_bid(
        ctx: Context<PlaceClassBid>,
        nonce: u64,
        class: ClassKey,
        price: u64,
        expires_at: i64,
    ) -> Result<()> {
        let ts = now()?;
        require!(class.valid(), BookError::ClassMismatch);
        require!(price > 0, BookError::BadPrice);
        require!(expires_at == 0 || expires_at > ts, BookError::BadParams);
        let maker_fee = eco::bps(price, ctx.accounts.config.params.maker_bps);
        let bid_key = ctx.accounts.bid.key();
        pay_sol(
            &ctx.accounts.system_program.to_account_info(),
            &ctx.accounts.bidder.to_account_info(),
            &ctx.accounts.bid.to_account_info(),
            price.checked_add(maker_fee).ok_or(BookError::Overflow)?,
        )?;
        let b = &mut ctx.accounts.bid;
        b.version = VERSION;
        b.bump = ctx.bumps.bid;
        b.bidder = ctx.accounts.bidder.key();
        b.nonce = nonce;
        b.class = class;
        b.price = price;
        b.maker_fee = maker_fee;
        b.expires_at = expires_at;
        b.created_at = ts;
        emit_cpi!(ClassBidPlaced {
            bid: bid_key,
            bidder: b.bidder,
            template_id: class.template_id,
            min_level: class.min_level,
            price,
            expires_at,
            ts
        });
        Ok(())
    }

    /// The bidder closes a class bid; every lamport goes back.
    pub fn cancel_class_bid(ctx: Context<CancelClassBid>) -> Result<()> {
        emit_cpi!(ClassBidCancelled {
            bid: ctx.accounts.bid.key(),
            ts: now()?
        });
        Ok(())
    }

    /// The holder of a fitting item sells it into a class bid: the item moves to the bidder, the
    /// price less the taker fee to the seller, both fees to the treasury, the bid account closes
    /// to the bidder. Refused for an item that is listed, leased or equipped.
    pub fn match_class(ctx: Context<MatchClass>, reference: [u8; 32]) -> Result<()> {
        let ts = now()?;
        let a = &ctx.accounts;
        let b = &a.bid;
        require!(b.expires_at == 0 || ts < b.expires_at, BookError::BidExpired);
        let item_mint = a.item_mint.key();
        require_keys_eq!(*a.item.owner, hookwars_common::ids::ARMORY_ID, BookError::NotAnItem);
        require_keys_eq!(a.item.key(), hookwars_common::pda::item(&item_mint).0, BookError::NotAnItem);
        let item = {
            let data = a.item.try_borrow_data()?;
            hookwars_armory::state::Item::try_deserialize(&mut &data[..]).map_err(|_| error!(BookError::NotAnItem))?
        };
        require_keys_eq!(item.item_mint, item_mint, BookError::NotAnItem);
        require!(
            b.class.fits(item.template_id, item.level, &item.params) && item.equipped_count == 0,
            BookError::ClassMismatch
        );
        // Not listed and not leased: the market's listing and lease PDAs hold nothing.
        require_keys_eq!(a.listing.key(), hookwars_market::state::Listing::address(&item_mint).0, BookError::WrongAccount);
        require_keys_eq!(a.lease.key(), hookwars_market::state::lease_address(&a.item.key()).0, BookError::WrongAccount);
        require!(
            a.listing.data_is_empty() && a.lease.data_is_empty(),
            BookError::ClassMismatch
        );
        let h = bordrless_token::client::read_holding(&a.seller_holding)?;
        require!(
            a.seller_holding.key() == bordrless_token::client::holding_address(&item_mint, &a.seller.key())
                && h.mint == item_mint
                && h.owner == a.seller.key()
                && h.amount == 1,
            BookError::NotItemHolder
        );
        let sys = a.system_program.to_account_info();
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        let seller = a.seller.to_account_info();
        create_holding(&t, &seller, &a.item_mint, &a.bidder, &a.bidder_holding)?;
        transfer(&t, &seller, &a.seller_holding, &a.bidder_holding, &a.item_mint, 1, &[])?;
        let p = a.config.params;
        let taker_fee = eco::bps(b.price, p.taker_bps);
        let maker_fee = b.maker_fee;
        let bid_info = a.bid.to_account_info();
        let treasury = a.treasury.to_account_info();
        let (price, bidder) = (b.price, b.bidder);
        let bid_key = a.bid.key();
        let item_key = a.item.key();
        let c = &mut ctx.accounts.config;
        c.protocol_fees_total = c.protocol_fees_total.saturating_add(u128::from(taker_fee + maker_fee));
        if taker_fee + maker_fee > 0 {
            emit!(ProtocolFee {
                source: fee_source::BOOK_FILL,
                mint: Pubkey::default(),
                amount: taker_fee + maker_fee,
                reference,
                ts
            });
        }
        emit_cpi!(ClassFilled {
            bid: bid_key,
            item: item_key,
            seller: seller.key(),
            bidder,
            price,
            taker_fee,
            maker_fee,
            reference,
            ts
        });
        // Direct lamport moves last, after every CPI (see `Settle`).
        take_lamports(&bid_info, &seller, price - taker_fee)?;
        take_lamports(&bid_info, &treasury, taker_fee + maker_fee)
    }
}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + BookConfig::INIT_SPACE, seeds = [seeds::CONFIG], bump)]
    pub config: Box<Account<'info, BookConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeParams<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ BookError::NotAdmin)]
    pub config: Box<Account<'info, BookConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + PendingBook::INIT_SPACE, seeds = [seeds::PENDING], bump)]
    pub pending: Box<Account<'info, PendingBook>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApplyParams<'info> {
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, BookConfig>>,
    #[account(mut, seeds = [seeds::PENDING], bump = pending.bump)]
    pub pending: Box<Account<'info, PendingBook>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, BookConfig>>,
    /// CHECK: a craft `Material` (owner, address and layout checked in the handler).
    pub material: UncheckedAccount<'info>,
    /// CHECK: the material's mint (checked against the material).
    pub base_mint: UncheckedAccount<'info>,
    #[account(init, payer = creator, space = 8 + BookMarket::INIT_SPACE, seeds = [seeds::BOOK, base_mint.key().as_ref()], bump)]
    pub market: Box<Account<'info, BookMarket>>,
    #[account(init, payer = creator, space = 8 + BookEscrow::INIT_SPACE, seeds = [seeds::ESCROW, market.key().as_ref()], bump)]
    pub escrow: Box<Account<'info, BookEscrow>>,
    /// CHECK: the escrow's holding of the base (created here, address checked).
    #[account(mut)]
    pub escrow_holding: UncheckedAccount<'info>,
    /// CHECK: social `["skills"]` (read with every check when a level is required).
    pub skills: UncheckedAccount<'info>,
    /// CHECK: the creator's social profile, or its empty address.
    pub profile: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Place<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, BookConfig>>,
    #[account(mut, seeds = [seeds::BOOK, market.base_mint.as_ref()], bump = market.bump)]
    pub market: Box<Account<'info, BookMarket>>,
    #[account(mut, seeds = [seeds::ESCROW, market.key().as_ref()], bump = market.escrow_bump)]
    pub escrow: Box<Account<'info, BookEscrow>>,
    /// CHECK: the market's base mint.
    #[account(address = market.base_mint @ BookError::WrongAccount)]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the escrow's base holding.
    #[account(mut, address = bordrless_token::client::holding_address(&market.base_mint, &escrow.key()) @ BookError::WrongAccount)]
    pub escrow_holding: UncheckedAccount<'info>,
    /// CHECK: the owner's base holding (created when the owner buys).
    #[account(mut, address = bordrless_token::client::holding_address(&market.base_mint, &owner.key()) @ BookError::WrongAccount)]
    pub owner_holding: UncheckedAccount<'info>,
    /// CHECK: the config's treasury.
    #[account(mut, address = config.treasury @ BookError::WrongAccount)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: social `["skills"]`.
    pub skills: UncheckedAccount<'info>,
    /// CHECK: the owner's social profile, or its empty address.
    #[account(mut)]
    pub profile: UncheckedAccount<'info>,
    /// CHECK: `["social-caller"]` under this program.
    pub social_caller: UncheckedAccount<'info>,
    /// CHECK: social's event authority.
    pub social_event_authority: UncheckedAccount<'info>,
    /// CHECK: the social program (checked).
    pub social_program: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [seeds::BOOK, market.base_mint.as_ref()], bump = market.bump)]
    pub market: Box<Account<'info, BookMarket>>,
    #[account(mut, seeds = [seeds::ESCROW, market.key().as_ref()], bump = market.escrow_bump)]
    pub escrow: Box<Account<'info, BookEscrow>>,
    /// CHECK: the market's base mint.
    #[account(address = market.base_mint @ BookError::WrongAccount)]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the escrow's base holding.
    #[account(mut, address = bordrless_token::client::holding_address(&market.base_mint, &escrow.key()) @ BookError::WrongAccount)]
    pub escrow_holding: UncheckedAccount<'info>,
    /// CHECK: the owner's base holding.
    #[account(mut, address = bordrless_token::client::holding_address(&market.base_mint, &owner.key()) @ BookError::WrongAccount)]
    pub owner_holding: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Crank<'info> {
    #[account(mut)]
    pub cranker: Signer<'info>,
    #[account(mut, seeds = [seeds::BOOK, market.base_mint.as_ref()], bump = market.bump)]
    pub market: Box<Account<'info, BookMarket>>,
    #[account(mut, seeds = [seeds::ESCROW, market.key().as_ref()], bump = market.escrow_bump)]
    pub escrow: Box<Account<'info, BookEscrow>>,
    /// CHECK: the market's base mint.
    #[account(address = market.base_mint @ BookError::WrongAccount)]
    pub base_mint: UncheckedAccount<'info>,
    /// CHECK: the escrow's base holding.
    #[account(mut, address = bordrless_token::client::holding_address(&market.base_mint, &escrow.key()) @ BookError::WrongAccount)]
    pub escrow_holding: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct PlaceClassBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, BookConfig>>,
    #[account(init, payer = bidder, space = 8 + ClassBid::INIT_SPACE,
        seeds = [seeds::CLASS_BID, bidder.key().as_ref(), &nonce.to_le_bytes()], bump)]
    pub bid: Box<Account<'info, ClassBid>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CancelClassBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,
    #[account(mut, close = bidder, has_one = bidder @ BookError::NotOwner,
        seeds = [seeds::CLASS_BID, bidder.key().as_ref(), &bid.nonce.to_le_bytes()], bump = bid.bump)]
    pub bid: Box<Account<'info, ClassBid>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct MatchClass<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, BookConfig>>,
    #[account(mut, close = bidder,
        seeds = [seeds::CLASS_BID, bid.bidder.as_ref(), &bid.nonce.to_le_bytes()], bump = bid.bump)]
    pub bid: Box<Account<'info, ClassBid>>,
    /// CHECK: the bid's owner (receives the item and the closed account).
    #[account(mut, address = bid.bidder @ BookError::WrongAccount)]
    pub bidder: UncheckedAccount<'info>,
    /// CHECK: the armory item (owner, address and layout checked in the handler).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint (bound through the item).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the seller's holding of the item (read with every check).
    #[account(mut)]
    pub seller_holding: UncheckedAccount<'info>,
    /// CHECK: the bidder's holding of the item (created here, address checked).
    #[account(mut)]
    pub bidder_holding: UncheckedAccount<'info>,
    /// CHECK: the market's `["listing", item_mint]` (must hold nothing).
    pub listing: UncheckedAccount<'info>,
    /// CHECK: the market's `["lease", item]` (must hold nothing).
    pub lease: UncheckedAccount<'info>,
    /// CHECK: the config's treasury.
    #[account(mut, address = config.treasury @ BookError::WrongAccount)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the token program (checked in the handler).
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority (checked in the handler).
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
