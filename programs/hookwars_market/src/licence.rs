// Changed by Hookwars: new file (hook economy, docs/spec/11-hook-economy.md sections 1.4 and 2.3); integration pass 3: Template.author_bps (E-7).
//! Licences: a token pays an item's holder for the right to equip the item for a term (R45).
//!
//! - `LicenceOffer` at `["licence-offer", item]`: the item holder's terms (price, term, `per`,
//!   `max_live`, `exclusive`). These terms belong in the armory's `AccessPolicy.licence_terms`
//!   (11 section 1.2); until the armory lane adds it, they live here and the armory's equip gate
//!   reads `License` (integration request).
//! - `License` at `["license", item, token_mint]`: one per item and token. `buy_license` writes or
//!   extends it, `renew_license` extends it, `revoke_license` (per-period terms only) refunds the
//!   unused fraction from the holder's own wallet, `expire_license` frees the live slot.
//! - Payment (11 section 2.3, R37): `LICENCE_PROTOCOL_BPS` to the market treasury, then the
//!   template author's share (`author_bps`, paid to `Template.registered_by`) of the remainder,
//!   the rest to the item holder at payment time (R45). The payer signs every transfer; nothing
//!   sits in a vault.

use anchor_lang::prelude::*;
use hookwars_common::economy::{self as eco, counter, fee_source};
use hookwars_social::profiles::{record_wallet_cpi, RecordAccs};

use crate::cpi::{holding_amount, pay_sol, read_item};
use crate::error::MarketError;

/// This module's errors are `MarketError` variants (one error enum per program, so the IDL builds).
pub type LicenceError = MarketError;
use crate::state::{seeds as market_seeds, MarketConfig};

pub mod lseeds {
    pub const CONFIG: &[u8] = b"licence-config";
    pub const OFFER: &[u8] = b"licence-offer";
    pub const LICENSE: &[u8] = b"license";
}

/// `per` values.
pub mod per {
    pub const PER_TOKEN: u8 = 0;
    pub const PER_PERIOD: u8 = 1;
}

/// Licence parameters (11 section 11; every one "to set").
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LicenceParams {
    /// `LICENCE_PROTOCOL_BPS`.
    pub protocol_bps: u16,
    /// The template author's share of the remainder, until `Template.author_bps` exists (R34).
    pub author_bps: u16,
    /// `LICENCE_MIN_SECS`, `LICENCE_MAX_SECS`.
    pub min_secs: u32,
    pub max_secs: u32,
}

/// `LicenceConfig` at `["licence-config"]` (admin = the market admin).
#[account]
#[derive(InitSpace, Debug)]
pub struct LicenceConfig {
    pub bump: u8,
    pub params: LicenceParams,
    pub pending: Option<LicenceParams>,
    pub pending_at: i64,
    pub protocol_fees_total: u128,
    pub licences_sold: u64,
}

/// `LicenceOffer` at `["licence-offer", item]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct LicenceOffer {
    pub bump: u8,
    pub item: Pubkey,
    pub item_mint: Pubkey,
    pub set_by: Pubkey,
    pub price_lamports: u64,
    pub term_secs: u32,
    pub per: u8,
    /// How many tokens may hold a live licence at once (1 with `exclusive`).
    pub max_live: u16,
    pub exclusive: bool,
    pub live: u16,
    pub active: bool,
    pub updated_at: i64,
}

/// `License` at `["license", item, token_mint]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct License {
    pub bump: u8,
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub payer: Pubkey,
    /// Paid since `starts_at` (the span a revocation refunds against).
    pub price_paid: u64,
    pub starts_at: i64,
    pub ends_at: i64,
    /// 0 = not revoked.
    pub revoked_at: i64,
    pub per: u8,
    /// Whether it holds one of the offer's live slots.
    pub counted: bool,
}

impl License {
    /// Live at `ts`: paid through, not revoked. The armory's equip gate reads this.
    pub fn live_at(&self, ts: i64) -> bool {
        self.revoked_at == 0 && ts < self.ends_at
    }
}


#[event]
pub struct LicenceOfferSet {
    pub item: Pubkey,
    pub holder: Pubkey,
    pub price_lamports: u64,
    pub term_secs: u32,
    pub per: u8,
    pub max_live: u16,
    pub exclusive: bool,
    pub active: bool,
    pub ts: i64,
}

#[event]
pub struct LicenceBought {
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub payer: Pubkey,
    pub holder: Pubkey,
    pub price: u64,
    pub protocol: u64,
    pub author: u64,
    pub to_holder: u64,
    pub ends_at: i64,
    pub renewal: bool,
    pub reference: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct LicenceRevoked {
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub refund: u64,
    pub ts: i64,
}

#[event]
pub struct LicenceExpired {
    pub item: Pubkey,
    pub token_mint: Pubkey,
    pub ts: i64,
}

#[event]
pub struct LicenceParamsProposed {
    pub ready_at: i64,
}

/// Every protocol collection (11 section 2.4); `mint` is the default key for native lamports.
#[event]
pub struct ProtocolFee {
    pub source: u8,
    pub mint: Pubkey,
    pub amount: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn check_licence_params(p: &LicenceParams) -> Result<()> {
    require!(
        p.protocol_bps <= 10_000 && p.author_bps <= 10_000 && p.min_secs > 0 && p.min_secs <= p.max_secs,
        LicenceError::BadLicenceParams
    );
    Ok(())
}

pub fn process_init_licence_config(ctx: Context<InitLicenceConfig>, params: LicenceParams) -> Result<()> {
    check_licence_params(&params)?;
    let c = &mut ctx.accounts.licence_config;
    c.bump = ctx.bumps.licence_config;
    c.params = params;
    c.pending = None;
    c.pending_at = 0;
    c.protocol_fees_total = 0;
    c.licences_sold = 0;
    Ok(())
}

pub fn process_propose_licence_params(ctx: Context<ProposeLicenceParams>, params: LicenceParams) -> Result<()> {
    check_licence_params(&params)?;
    let ready_at = now()?
        .checked_add(i64::from(ctx.accounts.config.params.admin_timelock_secs))
        .ok_or(MarketError::Overflow)?;
    let c = &mut ctx.accounts.licence_config;
    c.pending = Some(params);
    c.pending_at = ready_at;
    emit_cpi!(LicenceParamsProposed { ready_at });
    Ok(())
}

pub fn process_apply_licence_params(ctx: Context<ApplyLicenceParams>) -> Result<()> {
    let c = &mut ctx.accounts.licence_config;
    let p = c.pending.ok_or(MarketError::NotReady)?;
    require!(now()? >= c.pending_at, MarketError::NotReady);
    c.params = p;
    c.pending = None;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn process_set_licence_offer(
    ctx: Context<SetLicenceOffer>,
    price_lamports: u64,
    term_secs: u32,
    per_: u8,
    max_live: u16,
    exclusive: bool,
    active: bool,
) -> Result<()> {
    let a = &ctx.accounts;
    read_item(&a.item, a.item_mint.key)?;
    require!(
        holding_amount(&a.holder_holding, a.item_mint.key, a.holder.key)? == 1,
        MarketError::NotItemHolder
    );
    let p = a.licence_config.params;
    require!(
        price_lamports > 0
            && term_secs >= p.min_secs
            && term_secs <= p.max_secs
            && per_ <= per::PER_PERIOD
            && max_live > 0
            && (!exclusive || max_live == 1),
        LicenceError::BadLicenceTerms
    );
    let ts = now()?;
    let o = &mut ctx.accounts.offer;
    if o.item == Pubkey::default() {
        o.bump = ctx.bumps.offer;
        o.item = ctx.accounts.item.key();
        o.item_mint = ctx.accounts.item_mint.key();
        o.live = 0;
    }
    // Lowering `max_live` never cuts live licences; it only stops new ones.
    o.set_by = ctx.accounts.holder.key();
    o.price_lamports = price_lamports;
    o.term_secs = term_secs;
    o.per = per_;
    o.max_live = max_live;
    o.exclusive = exclusive;
    o.active = active;
    o.updated_at = ts;
    emit_cpi!(LicenceOfferSet {
        item: o.item,
        holder: o.set_by,
        price_lamports,
        term_secs,
        per: per_,
        max_live,
        exclusive,
        active,
        ts
    });
    Ok(())
}

/// `buy_license` and `renew_license`: pays the split and writes or extends the licence.
pub fn process_buy_license(mut ctx: Context<BuyLicense>, max_price: u64, renew_only: bool, reference: [u8; 32]) -> Result<()> {
    let ts = now()?;
    let a = &ctx.accounts;
    let o = &a.offer;
    require!(o.active, LicenceError::NotLicensable);
    require!(o.price_lamports <= max_price, MarketError::PriceMoved);
    let item = read_item(&a.item, a.item_mint.key)?;
    // The holder at payment time (R45): the wallet holding the item now.
    require!(
        holding_amount(&a.holder_holding, a.item_mint.key, a.holder.key)? == 1,
        MarketError::NotItemHolder
    );
    // The template's author (R34).
    require_keys_eq!(*a.template.owner, hookwars_common::ids::ARMORY_ID, MarketError::WrongAccount);
    require_keys_eq!(a.template.key(), hookwars_common::pda::template(item.template_id).0, MarketError::WrongAccount);
    let (registered_by, template_author_bps) = {
        let data = a.template.try_borrow_data()?;
        let t = hookwars_armory::state::Template::try_deserialize(&mut &data[..])
            .map_err(|_| error!(MarketError::WrongAccount))?;
        (t.registered_by, t.author_bps)
    };
    require_keys_eq!(a.author.key(), registered_by, MarketError::WrongRecipient);
    require_keys_eq!(*a.token_mint.owner, bordrless_token::ID, MarketError::WrongAccount);
    let lp = a.licence_config.params;
    let price = o.price_lamports;
    // Integration pass 3 (E-7): the template's own share once the admin has set one; the licence
    // config's share stays the fallback for templates registered before.
    let author_bps = if template_author_bps > 0 { template_author_bps } else { lp.author_bps };
    let (protocol, author, to_holder) = eco::licence_split(price, lp.protocol_bps, author_bps);
    let sys = a.system_program.to_account_info();
    let payer = a.payer.to_account_info();
    pay_sol(&sys, &payer, &a.treasury, protocol, &[])?;
    pay_sol(&sys, &payer, &a.author, author, &[])?;
    pay_sol(&sys, &payer, &a.holder, to_holder, &[])?;
    let term = i64::from(o.term_secs);
    let s = RecordAccs {
        skills: &a.skills,
        profile: &a.holder_profile,
        caller: &a.social_caller,
        event_authority: &a.social_event_authority,
        program: &a.social_program,
    };
    record_wallet_cpi(&s, &crate::ID, a.holder.key, counter::LICENCES_SOLD, 1)?;
    record_wallet_cpi(&s, &crate::ID, a.holder.key, counter::LICENCE_REVENUE_LAMPORTS, to_holder)?;
    let holder = a.holder.key();
    let item_key = a.item.key();
    let token_mint = a.token_mint.key();
    let payer_key = a.payer.key();
    let accs = &mut ctx.accounts;
    let l = &mut accs.license;
    let fresh = l.item == Pubkey::default();
    let renewal = !fresh && l.live_at(ts);
    require!(!renew_only || !fresh, LicenceError::NotLive);
    if renewal {
        l.ends_at = l.ends_at.checked_add(term).ok_or(MarketError::Overflow)?;
        l.price_paid = l.price_paid.saturating_add(price);
    } else {
        // A new term (fresh, expired or revoked): it takes a live slot.
        let offer = &mut accs.offer;
        if l.counted {
            offer.live = offer.live.saturating_sub(1);
        }
        require!(offer.live < offer.max_live, LicenceError::LicenceSoldOut);
        offer.live += 1;
        l.bump = ctx.bumps.license;
        l.item = item_key;
        l.token_mint = token_mint;
        l.starts_at = ts;
        l.ends_at = ts.checked_add(term).ok_or(MarketError::Overflow)?;
        l.revoked_at = 0;
        l.price_paid = price;
        l.counted = true;
    }
    l.payer = payer_key;
    l.per = accs.offer.per;
    let ends_at = l.ends_at;
    let c = &mut accs.licence_config;
    c.protocol_fees_total = c.protocol_fees_total.saturating_add(u128::from(protocol));
    c.licences_sold = c.licences_sold.saturating_add(1);
    if protocol > 0 {
        emit!(ProtocolFee {
            source: fee_source::LICENCE,
            mint: Pubkey::default(),
            amount: protocol,
            reference,
            ts
        });
    }
    emit_cpi!(LicenceBought {
        item: item_key,
        token_mint,
        payer: payer_key,
        holder,
        price,
        protocol,
        author,
        to_holder,
        ends_at,
        renewal,
        reference,
        ts
    });
    Ok(())
}

/// The holder revokes a per-period licence, refunding the unused fraction from its own wallet.
/// Removal from the slot then follows R39 (the armory's `enforce_access` after the notice).
pub fn process_revoke_license(mut ctx: Context<RevokeLicense>) -> Result<()> {
    let ts = now()?;
    let a = &ctx.accounts;
    require!(
        holding_amount(&a.holder_holding, a.item_mint.key, a.holder.key)? == 1,
        MarketError::NotItemHolder
    );
    let l = &a.license;
    require!(l.live_at(ts), LicenceError::NotLive);
    require!(l.per == per::PER_PERIOD, LicenceError::NotRevocable);
    let span = l.ends_at.saturating_sub(l.starts_at).max(1);
    let left = l.ends_at.saturating_sub(ts).max(0);
    let refund = u64::try_from(u128::from(l.price_paid) * u128::try_from(left).unwrap_or(0) / u128::try_from(span).unwrap_or(1))
        .map_err(|_| error!(MarketError::Overflow))?;
    pay_sol(
        &a.system_program.to_account_info(),
        &a.holder.to_account_info(),
        &a.payer,
        refund,
        &[],
    )?;
    let (item, token_mint) = (l.item, l.token_mint);
    let accs = &mut ctx.accounts;
    let l = &mut accs.license;
    l.revoked_at = ts;
    l.ends_at = ts;
    if l.counted {
        l.counted = false;
        accs.offer.live = accs.offer.live.saturating_sub(1);
    }
    emit_cpi!(LicenceRevoked {
        item,
        token_mint,
        refund,
        ts
    });
    Ok(())
}

/// Anyone frees the live slot of an ended licence.
pub fn process_expire_license(ctx: Context<ExpireLicense>) -> Result<()> {
    let ts = now()?;
    let l = &mut ctx.accounts.license;
    require!(!l.live_at(ts) && l.counted, LicenceError::NotEnded);
    l.counted = false;
    ctx.accounts.offer.live = ctx.accounts.offer.live.saturating_sub(1);
    emit_cpi!(LicenceExpired {
        item: l.item,
        token_mint: l.token_mint,
        ts
    });
    Ok(())
}

#[derive(Accounts)]
pub struct InitLicenceConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [market_seeds::CONFIG], bump = config.bump, has_one = admin @ MarketError::NotAdmin)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(init, payer = admin, space = 8 + LicenceConfig::INIT_SPACE, seeds = [lseeds::CONFIG], bump)]
    pub licence_config: Box<Account<'info, LicenceConfig>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeLicenceParams<'info> {
    pub admin: Signer<'info>,
    #[account(seeds = [market_seeds::CONFIG], bump = config.bump, has_one = admin @ MarketError::NotAdmin)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(mut, seeds = [lseeds::CONFIG], bump = licence_config.bump)]
    pub licence_config: Box<Account<'info, LicenceConfig>>,
}

#[derive(Accounts)]
pub struct ApplyLicenceParams<'info> {
    #[account(mut, seeds = [lseeds::CONFIG], bump = licence_config.bump)]
    pub licence_config: Box<Account<'info, LicenceConfig>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct SetLicenceOffer<'info> {
    #[account(mut)]
    pub holder: Signer<'info>,
    #[account(seeds = [lseeds::CONFIG], bump = licence_config.bump)]
    pub licence_config: Box<Account<'info, LicenceConfig>>,
    /// CHECK: the armory item (checked with `read_item`).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint (bound through the item).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the holder's holding of the item (read with every check).
    pub holder_holding: UncheckedAccount<'info>,
    #[account(init_if_needed, payer = holder, space = 8 + LicenceOffer::INIT_SPACE,
        seeds = [lseeds::OFFER, item.key().as_ref()], bump)]
    pub offer: Box<Account<'info, LicenceOffer>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct BuyLicense<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(seeds = [market_seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(mut, seeds = [lseeds::CONFIG], bump = licence_config.bump)]
    pub licence_config: Box<Account<'info, LicenceConfig>>,
    #[account(mut, seeds = [lseeds::OFFER, item.key().as_ref()], bump = offer.bump)]
    pub offer: Box<Account<'info, LicenceOffer>>,
    #[account(init_if_needed, payer = payer, space = 8 + License::INIT_SPACE,
        seeds = [lseeds::LICENSE, item.key().as_ref(), token_mint.key().as_ref()], bump)]
    pub license: Box<Account<'info, License>>,
    /// CHECK: the armory item (checked with `read_item`).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint (bound through the item).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the item's template (owner, address and layout checked).
    pub template: UncheckedAccount<'info>,
    /// CHECK: the licensed token's mint (owned by the token program).
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: the wallet holding the item now (checked through its holding).
    #[account(mut)]
    pub holder: UncheckedAccount<'info>,
    /// CHECK: the holder's holding of the item (read with every check).
    pub holder_holding: UncheckedAccount<'info>,
    /// CHECK: the template's `registered_by`.
    #[account(mut)]
    pub author: UncheckedAccount<'info>,
    /// CHECK: the market treasury.
    #[account(mut, address = config.treasury @ MarketError::WrongRecipient)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: social `["skills"]`.
    pub skills: UncheckedAccount<'info>,
    /// CHECK: the holder's social profile, or its empty address.
    #[account(mut)]
    pub holder_profile: UncheckedAccount<'info>,
    /// CHECK: `["social-caller"]` under this program.
    pub social_caller: UncheckedAccount<'info>,
    /// CHECK: social's event authority.
    pub social_event_authority: UncheckedAccount<'info>,
    /// CHECK: the social program (checked).
    pub social_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct RevokeLicense<'info> {
    #[account(mut)]
    pub holder: Signer<'info>,
    /// CHECK: the item's mint (bound through the offer).
    #[account(address = offer.item_mint @ MarketError::WrongAccount)]
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the holder's holding of the item (read with every check).
    pub holder_holding: UncheckedAccount<'info>,
    #[account(mut, seeds = [lseeds::OFFER, offer.item.as_ref()], bump = offer.bump)]
    pub offer: Box<Account<'info, LicenceOffer>>,
    #[account(mut, seeds = [lseeds::LICENSE, offer.item.as_ref(), license.token_mint.as_ref()], bump = license.bump)]
    pub license: Box<Account<'info, License>>,
    /// CHECK: the licence's payer (receives the refund).
    #[account(mut, address = license.payer @ MarketError::WrongRecipient)]
    pub payer: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ExpireLicense<'info> {
    #[account(mut, seeds = [lseeds::OFFER, offer.item.as_ref()], bump = offer.bump)]
    pub offer: Box<Account<'info, LicenceOffer>>,
    #[account(mut, seeds = [lseeds::LICENSE, offer.item.as_ref(), license.token_mint.as_ref()], bump = license.bump)]
    pub license: Box<Account<'info, License>>,
}

pub fn licence_config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[lseeds::CONFIG], &crate::ID)
}
pub fn licence_offer_address(item: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[lseeds::OFFER, item.as_ref()], &crate::ID)
}
pub fn license_address(item: &Pubkey, token_mint: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[lseeds::LICENSE, item.as_ref(), token_mint.as_ref()], &crate::ID)
}

/// For the armory's equip gate (integration request): whether `info` is a live `License` of
/// `item` for `token_mint` at `ts` (owner, address and layout checked).
pub fn license_live(info: &AccountInfo, item: &Pubkey, token_mint: &Pubkey, ts: i64) -> Result<bool> {
    if *info.owner != crate::ID || *info.key != license_address(item, token_mint).0 || info.data_is_empty() {
        return Ok(false);
    }
    let data = info.try_borrow_data()?;
    let l = License::try_deserialize(&mut &data[..])?;
    Ok(l.item == *item && l.token_mint == *token_mint && l.live_at(ts))
}
