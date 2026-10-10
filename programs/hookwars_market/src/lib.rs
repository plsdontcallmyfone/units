// Changed by Hookwars: new program (expansion, docs/spec/10-expansion.md sections 0, 1, 4, 5); integration pass 3: buy records ITEMS_SOLD (E-6).
// Integration pass 2: end_lease reverts the leased slot through the armory (10 section 17 I-3).
// Economy (11): licences (11 sections 1.4, 2.3).
//! `hookwars_market`: items are assets. Listings and sales of item tokens (price in SOL, paid by
//! the buyer straight to the seller, the item's author and the protocol treasury), collections
//! (discovery only), item rental (the item token sits in a lease escrow for the term; the lessor's
//! rent is a share of the item's royalty paid at settlement, R32, never on top of it), and
//! commissions (a bounty in a program vault, paid when a submitted item is actually equipped in
//! the slot by the token's own equip rule). Spot only: no payout depends on a wager.
//!
//! Money: sale proceeds never sit in a vault (the buyer signs every payment); a commission bounty
//! sits in a system-owned PDA vault from opening to payment or refund; a lease fee goes from the
//! payer to the lessor at acceptance. Item tokens move out of the escrows by an ordinary transfer
//! signed by the escrow PDA (item mints have no hook, 02 2.5).

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

pub mod cpi;
pub mod error;
pub mod events;
pub mod licence;
pub mod state;

use cpi::{create_holding, holding_amount, pay_sol, read_item, transfer_plain, TokenAccs};
use error::MarketError;
use events::*;
pub use licence::*;
use state::*;

declare_id!("FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "units market",
    project_url: "https://github.com/plsdontcallmyfone/units",
    contacts: "link:https://github.com/plsdontcallmyfone/units/security/advisories/new",
    policy: "https://github.com/plsdontcallmyfone/units/blob/main/SECURITY.md",
    source_code: "https://github.com/plsdontcallmyfone/units"
}

fn now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

fn bps_of(amount: u64, bps: u16) -> Result<u64> {
    let v = u128::from(amount) * u128::from(bps) / 10_000;
    u64::try_from(v).map_err(|_| error!(MarketError::Overflow))
}

fn check_params(p: &MarketParams) -> Result<()> {
    require!(
        u32::from(p.fee_bps) + u32::from(p.author_resale_bps) <= 10_000,
        MarketError::BadParams
    );
    require!(
        usize::from(p.collection_max_templates) <= COLLECTION_TEMPLATES_CAP,
        MarketError::BadParams
    );
    require!(p.max_rent_bps <= 10_000, MarketError::BadParams);
    require!(
        p.lease_min_secs > 0 && p.lease_min_secs <= p.lease_max_secs,
        MarketError::BadParams
    );
    require!(p.commission_vote_secs > 0, MarketError::BadParams);
    Ok(())
}

fn upgrade_authority(program_data: &AccountInfo) -> Result<Option<Pubkey>> {
    require_keys_eq!(
        *program_data.key,
        hookwars_common::programdata_address(&crate::ID),
        MarketError::NotUpgradeAuthority
    );
    require_keys_eq!(
        *program_data.owner,
        hookwars_common::ids::BPF_LOADER_UPGRADEABLE_ID,
        MarketError::NotUpgradeAuthority
    );
    let data = program_data.try_borrow_data()?;
    require!(
        data.len() >= 45 && data[..4] == [3, 0, 0, 0],
        MarketError::NotUpgradeAuthority
    );
    if data[12] == 0 {
        return Ok(None);
    }
    Ok(Some(Pubkey::new_from_array(data[13..45].try_into().unwrap())))
}

/// The slot `slot` of the units slot mint `info`.
fn read_slot(info: &AccountInfo, slot: u8) -> Result<bordrless_token::state::Slot> {
    let m = Box::new(bordrless_token::client::read_mint(info)?);
    require!(slot < m.slot_count, MarketError::NoSuchSlot);
    let s = m
        .slots
        .get(usize::from(slot))
        .copied()
        .ok_or(MarketError::NoSuchSlot)?;
    require!(s.kind != bordrless_hook::slot_kind::LOCKED, MarketError::NoSuchSlot);
    Ok(s)
}

#[program]
pub mod hookwars_market {
    use super::*;

    /// Creates the config; the program's upgrade authority signs.
    pub fn init(ctx: Context<Init>, admin: Pubkey, treasury: Pubkey, params: MarketParams) -> Result<()> {
        let up = upgrade_authority(&ctx.accounts.program_data)?;
        require!(
            up == Some(ctx.accounts.authority.key()),
            MarketError::NotUpgradeAuthority
        );
        check_params(&params)?;
        let c = &mut ctx.accounts.config;
        c.version = VERSION;
        c.bump = ctx.bumps.config;
        c.admin = admin;
        c.treasury = treasury;
        c.params = params;
        c.collections = 0;
        c.reserved = [0; 32];
        Ok(())
    }

    /// The admin proposes new params and treasury; they apply after `admin_timelock_secs`.
    pub fn propose_params(ctx: Context<ProposeParams>, params: MarketParams, treasury: Pubkey) -> Result<()> {
        check_params(&params)?;
        let ready_at = now()?
            .checked_add(i64::from(ctx.accounts.config.params.admin_timelock_secs))
            .ok_or(MarketError::Overflow)?;
        let p = &mut ctx.accounts.pending;
        p.bump = ctx.bumps.pending;
        p.params = params;
        p.treasury = treasury;
        p.ready_at = ready_at;
        p.active = true;
        emit_cpi!(MarketParamsProposed { ready_at });
        Ok(())
    }

    /// Anyone applies a pending params change once it is ready.
    pub fn apply_params(ctx: Context<ApplyParams>) -> Result<()> {
        let p = &mut ctx.accounts.pending;
        let ts = now()?;
        require!(p.active && ts >= p.ready_at, MarketError::NotReady);
        ctx.accounts.config.params = p.params;
        ctx.accounts.config.treasury = p.treasury;
        p.active = false;
        emit_cpi!(MarketParamsApplied { ts });
        Ok(())
    }

    /// Lists an item: its token moves into the escrow until it sells, is delisted or expires.
    pub fn list(ctx: Context<List>, price_lamports: u64, expires_at: i64) -> Result<()> {
        let a = &ctx.accounts;
        require!(price_lamports > 0, MarketError::ZeroPrice);
        let ts = now()?;
        require!(expires_at == 0 || expires_at > ts, MarketError::ListingExpired);
        read_item(&a.item, a.item_mint.key)?;
        require!(
            holding_amount(&a.seller_holding, a.item_mint.key, a.seller.key)? == 1,
            MarketError::NotItemHolder
        );
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &a.system_program.to_account_info(),
        };
        t.check()?;
        let seller = a.seller.to_account_info();
        create_holding(&t, &seller, &a.item_mint, &a.escrow, &a.escrow_holding)?;
        transfer_plain(&t, &seller, &a.seller_holding, &a.escrow_holding, &a.item_mint, 1, &[])?;
        let l = &mut ctx.accounts.listing;
        l.version = VERSION;
        l.bump = ctx.bumps.listing;
        l.seller = ctx.accounts.seller.key();
        l.item = ctx.accounts.item.key();
        l.item_mint = ctx.accounts.item_mint.key();
        l.price_lamports = price_lamports;
        l.created_at = ts;
        l.expires_at = expires_at;
        emit_cpi!(Listed {
            item: l.item,
            item_mint: l.item_mint,
            seller: l.seller,
            price_lamports,
            expires_at,
            ts
        });
        Ok(())
    }

    /// The seller takes the item back.
    pub fn delist(ctx: Context<Unlist>) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.caller.key(),
            ctx.accounts.listing.seller,
            MarketError::NotSeller
        );
        return_from_escrow(&ctx)?;
        emit_cpi!(Delisted {
            item_mint: ctx.accounts.item_mint.key(),
            seller: ctx.accounts.seller.key(),
            ts: now()?
        });
        Ok(())
    }

    /// Anyone returns an expired listing to its seller.
    pub fn expire(ctx: Context<Unlist>) -> Result<()> {
        let e = ctx.accounts.listing.expires_at;
        require!(e != 0 && now()? > e, MarketError::NotExpired);
        return_from_escrow(&ctx)?;
        emit_cpi!(ListingExpired {
            item_mint: ctx.accounts.item_mint.key(),
            seller: ctx.accounts.seller.key(),
            ts: now()?
        });
        Ok(())
    }

    /// Buys a listed item. The buyer pays the protocol fee, the author's resale share and the
    /// seller directly; the escrow sends the item token to the buyer.
    pub fn buy(ctx: Context<Buy>, max_price: u64) -> Result<()> {
        let a = &ctx.accounts;
        let ts = now()?;
        let l = &a.listing;
        require!(l.expires_at == 0 || ts <= l.expires_at, MarketError::ListingExpired);
        require!(l.price_lamports <= max_price, MarketError::PriceMoved);
        let item = read_item(&a.item, a.item_mint.key)?;
        require_keys_eq!(a.author.key(), item.author, MarketError::WrongRecipient);
        let p = a.config.params;
        let price = l.price_lamports;
        let fee = bps_of(price, p.fee_bps)?;
        let resale = bps_of(price, p.author_resale_bps)?;
        let rest = price
            .checked_sub(fee)
            .and_then(|v| v.checked_sub(resale))
            .ok_or(MarketError::Overflow)?;
        let sys = a.system_program.to_account_info();
        let buyer = a.buyer.to_account_info();
        pay_sol(&sys, &buyer, &a.treasury, fee, &[])?;
        pay_sol(&sys, &buyer, &a.author, resale, &[])?;
        pay_sol(&sys, &buyer, &a.seller, rest, &[])?;
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        create_holding(&t, &buyer, &a.item_mint, &buyer, &a.buyer_holding)?;
        let item_mint = a.item_mint.key();
        let bump = [ctx.bumps.escrow];
        let seeds: &[&[u8]] = &[seeds::ESCROW, item_mint.as_ref(), &bump];
        transfer_plain(
            &t,
            &a.escrow,
            &a.escrow_holding,
            &a.buyer_holding,
            &a.item_mint,
            1,
            &[seeds],
        )?;
        emit_cpi!(Sold {
            item: a.item.key(),
            item_mint,
            seller: l.seller,
            buyer: a.buyer.key(),
            price,
            fee,
            resale,
            ts
        });
        // Integration pass 3 (E-6): the seller's ITEMS_SOLD when the social suffix is given.
        let (_, social) = hookwars_common::eco_cpi::split_tagged(
            ctx.remaining_accounts,
            &hookwars_common::economy::SOCIAL_ID,
            hookwars_common::eco_cpi::SOCIAL_SUFFIX,
        );
        if let Some(sfx) = social {
            let seller = ctx.accounts.listing.seller;
            hookwars_common::eco_cpi::record_wallet(sfx, &crate::ID, &seller, hookwars_common::economy::counter::ITEMS_SOLD, 1)?;
        }
        Ok(())
    }

    /// Creates a collection of active templates (discovery only, 10 section 1.5).
    pub fn create_collection<'info>(
        ctx: Context<'info, CreateCollection<'info>>,
        name: String,
        template_ids: Vec<u16>,
    ) -> Result<()> {
        require!(!name.is_empty() && name.len() <= NAME_MAX_LEN, MarketError::BadName);
        let max = usize::from(ctx.accounts.config.params.collection_max_templates);
        require!(
            !template_ids.is_empty() && template_ids.len() <= max,
            MarketError::TooManyTemplates
        );
        require!(
            ctx.remaining_accounts.len() == template_ids.len(),
            MarketError::WrongAccount
        );
        for (i, (id, info)) in template_ids.iter().zip(ctx.remaining_accounts.iter()).enumerate() {
            require!(!template_ids[..i].contains(id), MarketError::DuplicateTemplate);
            require_keys_eq!(*info.owner, hookwars_common::ids::ARMORY_ID, MarketError::UnknownTemplate);
            require_keys_eq!(*info.key, hookwars_common::pda::template(*id).0, MarketError::UnknownTemplate);
            let data = info.try_borrow_data()?;
            let t = hookwars_armory::state::Template::try_deserialize(&mut &data[..])
                .map_err(|_| error!(MarketError::UnknownTemplate))?;
            require!(
                t.id == *id && t.status == hookwars_armory::state::template_status::ACTIVE,
                MarketError::UnknownTemplate
            );
        }
        let ts = now()?;
        let config = &mut ctx.accounts.config;
        let id = config.collections;
        config.collections = id.checked_add(1).ok_or(MarketError::Overflow)?;
        let c = &mut ctx.accounts.collection;
        c.version = VERSION;
        c.bump = ctx.bumps.collection;
        c.id = id;
        c.name = name.clone();
        c.curator = ctx.accounts.curator.key();
        c.template_ids = template_ids.clone();
        c.created_at = ts;
        emit_cpi!(CollectionCreated {
            id,
            name,
            curator: ctx.accounts.curator.key(),
            template_ids,
            ts
        });
        Ok(())
    }

    /// Offers an item for rent to one token's slot. The item token moves into the lease escrow
    /// until the offer is withdrawn or the lease ends.
    pub fn offer_lease(
        ctx: Context<OfferLease>,
        token_mint: Pubkey,
        slot: u8,
        rent_bps: u16,
        fee_lamports: u64,
        term_secs: u32,
    ) -> Result<()> {
        let a = &ctx.accounts;
        let p = a.config.params;
        require!(
            term_secs >= p.lease_min_secs && term_secs <= p.lease_max_secs,
            MarketError::BadTerm
        );
        require!(rent_bps <= p.max_rent_bps, MarketError::RentTooHigh);
        require_keys_eq!(a.token_mint.key(), token_mint, MarketError::WrongAccount);
        let item = read_item(&a.item, a.item_mint.key)?;
        let s = read_slot(&a.token_mint, slot)?;
        require!(s.kind == item.manifest.kind, MarketError::DoesNotFit);
        require!(
            holding_amount(&a.lessor_holding, a.item_mint.key, a.lessor.key)? == 1,
            MarketError::NotItemHolder
        );
        let sys = a.system_program.to_account_info();
        let t = TokenAccs {
            token_program: &a.token_program,
            event_authority: &a.token_event_authority,
            system_program: &sys,
        };
        t.check()?;
        let lessor = a.lessor.to_account_info();
        create_holding(&t, &lessor, &a.item_mint, &a.lease_escrow, &a.lease_escrow_holding)?;
        transfer_plain(&t, &lessor, &a.lessor_holding, &a.lease_escrow_holding, &a.item_mint, 1, &[])?;
        let ts = now()?;
        let l = &mut ctx.accounts.lease;
        l.version = VERSION;
        l.bump = ctx.bumps.lease;
        l.lessor = ctx.accounts.lessor.key();
        l.item = ctx.accounts.item.key();
        l.item_mint = ctx.accounts.item_mint.key();
        l.token_mint = token_mint;
        l.slot = slot;
        l.rent_bps = rent_bps;
        l.fee_lamports = fee_lamports;
        l.term_secs = term_secs;
        l.starts_at = 0;
        l.ends_at = 0;
        l.state = lease_state::OFFERED;
        emit_cpi!(LeaseOffered {
            item: l.item,
            lessor: l.lessor,
            token_mint,
            slot,
            rent_bps,
            fee_lamports,
            term_secs,
            ts
        });
        Ok(())
    }

    /// Anyone accepts an offered lease, paying its fee to the lessor; the term starts now.
    pub fn accept_lease(ctx: Context<AcceptLease>) -> Result<()> {
        let fee = ctx.accounts.lease.fee_lamports;
        require!(
            ctx.accounts.lease.state == lease_state::OFFERED,
            MarketError::WrongLeaseState
        );
        pay_sol(
            &ctx.accounts.system_program.to_account_info(),
            &ctx.accounts.payer.to_account_info(),
            &ctx.accounts.lessor,
            fee,
            &[],
        )?;
        let ts = now()?;
        let l = &mut ctx.accounts.lease;
        l.state = lease_state::ACTIVE;
        l.starts_at = ts;
        l.ends_at = ts
            .checked_add(i64::from(l.term_secs))
            .ok_or(MarketError::Overflow)?;
        emit_cpi!(LeaseStarted {
            item: l.item,
            payer: ctx.accounts.payer.key(),
            starts_at: l.starts_at,
            ends_at: l.ends_at
        });
        Ok(())
    }

    /// The lessor withdraws an offer nobody accepted; the item returns.
    pub fn withdraw_offer(ctx: Context<CloseLease>) -> Result<()> {
        require_keys_eq!(
            ctx.accounts.caller.key(),
            ctx.accounts.lease.lessor,
            MarketError::NotLessor
        );
        require!(
            ctx.accounts.lease.state == lease_state::OFFERED,
            MarketError::WrongLeaseState
        );
        return_from_lease(&ctx)?;
        emit_cpi!(LeaseWithdrawn {
            item: ctx.accounts.lease.item,
            ts: now()?
        });
        Ok(())
    }

    /// Anyone ends a lease after its term; the item returns to the lessor. Reverting the slot (if
    /// the item is still equipped there) is the armory's `revert_for_lease_end` (integration
    /// request in 10).
    pub fn end_lease<'info>(ctx: Context<'info, CloseLease<'info>>) -> Result<()> {
        let l = &ctx.accounts.lease;
        require!(l.state == lease_state::ACTIVE, MarketError::WrongLeaseState);
        require!(now()? >= l.ends_at, MarketError::LeaseNotOver);
        // Integration pass 2 (10 section 17 I-3): with the armory's accounts given, the slot that
        // still holds the leased item reverts first, then the item token goes home.
        if !ctx.remaining_accounts.is_empty() {
            cpi::revert_leased_slot(ctx.remaining_accounts, l.slot, l.item)?;
        }
        return_from_lease(&ctx)?;
        emit_cpi!(LeaseEnded {
            item: l.item,
            token_mint: l.token_mint,
            slot: l.slot,
            ts: now()?
        });
        Ok(())
    }

    /// Opens a commission: a bounty for an item a token's community wants in a vote-chosen slot.
    pub fn open_commission(
        ctx: Context<OpenCommission>,
        nonce: u64,
        slot: u8,
        brief_uri: String,
        bounty_lamports: u64,
        window_secs: u32,
    ) -> Result<()> {
        let a = &ctx.accounts;
        require!(brief_uri.len() <= URI_MAX_LEN, MarketError::BadUri);
        require!(
            bounty_lamports >= a.config.params.commission_min_lamports && bounty_lamports > 0,
            MarketError::BountyTooSmall
        );
        require!(window_secs > 0, MarketError::BadTerm);
        let s = read_slot(&a.token_mint, slot)?;
        require!(
            s.equip_rule == bordrless_hook::equip_rule::VOTE
                || s.equip_rule == bordrless_hook::equip_rule::PERFORMANCE,
            MarketError::SlotNotVotable
        );
        pay_sol(
            &a.system_program.to_account_info(),
            &a.creator.to_account_info(),
            &a.vault,
            bounty_lamports,
            &[],
        )?;
        let ts = now()?;
        let closes_at = ts
            .checked_add(i64::from(window_secs))
            .ok_or(MarketError::Overflow)?;
        let key = ctx.accounts.commission.key();
        let c = &mut ctx.accounts.commission;
        c.version = VERSION;
        c.bump = ctx.bumps.commission;
        c.vault_bump = ctx.bumps.vault;
        c.creator = ctx.accounts.creator.key();
        c.token_mint = ctx.accounts.token_mint.key();
        c.slot = slot;
        c.nonce = nonce;
        c.brief_uri = brief_uri.clone();
        c.bounty_lamports = bounty_lamports;
        c.incumbent = s.item;
        c.opens_at = ts;
        c.closes_at = closes_at;
        c.state = commission_state::OPEN;
        c.winner = None;
        c.submissions = 0;
        emit_cpi!(CommissionOpened {
            commission: key,
            creator: c.creator,
            token_mint: c.token_mint,
            slot,
            bounty_lamports,
            closes_at,
            brief_uri
        });
        Ok(())
    }

    /// The holder of an item submits it while the commission is open.
    pub fn submit(ctx: Context<Submit>) -> Result<()> {
        let a = &ctx.accounts;
        let c = &a.commission;
        require!(c.state == commission_state::OPEN, MarketError::CommissionNotOpen);
        let ts = now()?;
        require!(ts < c.closes_at, MarketError::CommissionClosed);
        require_keys_eq!(a.token_mint.key(), c.token_mint, MarketError::WrongAccount);
        let item = read_item(&a.item, a.item_mint.key)?;
        let s = read_slot(&a.token_mint, c.slot)?;
        require!(s.kind == item.manifest.kind, MarketError::DoesNotFit);
        require!(
            holding_amount(&a.submitter_holding, a.item_mint.key, a.submitter.key)? == 1,
            MarketError::NotItemHolder
        );
        let commission = a.commission.key();
        let item_key = a.item.key();
        let submitter = a.submitter.key();
        let sub = &mut ctx.accounts.submission;
        sub.version = VERSION;
        sub.bump = ctx.bumps.submission;
        sub.commission = commission;
        sub.item = item_key;
        sub.submitter = submitter;
        sub.submitted_at = ts;
        let c = &mut ctx.accounts.commission;
        c.submissions = c.submissions.checked_add(1).ok_or(MarketError::Overflow)?;
        emit_cpi!(Submitted {
            commission,
            item: item_key,
            submitter,
            ts
        });
        Ok(())
    }

    /// Anyone pays the bounty to a submission's submitter once that item is equipped in the slot
    /// (and was not the incumbent when the commission opened), after the submission window.
    pub fn pay_commission(ctx: Context<PayCommission>) -> Result<()> {
        let a = &ctx.accounts;
        let c = &a.commission;
        require!(c.state == commission_state::OPEN, MarketError::CommissionNotOpen);
        let ts = now()?;
        require!(ts >= c.closes_at, MarketError::CommissionNotOpen);
        let s = read_slot(&a.token_mint, c.slot)?;
        require!(
            s.item == a.submission.item && s.item != c.incumbent,
            MarketError::NotEquipped
        );
        let amount = a.vault.lamports();
        let commission = a.commission.key();
        let bump = [c.vault_bump];
        let seeds: &[&[u8]] = &[seeds::COMMISSION_VAULT, commission.as_ref(), &bump];
        pay_sol(
            &a.system_program.to_account_info(),
            &a.vault,
            &a.submitter,
            amount,
            &[seeds],
        )?;
        let item = a.submission.item;
        let submitter = a.submission.submitter;
        let c = &mut ctx.accounts.commission;
        c.state = commission_state::PAID;
        c.winner = Some(item);
        emit_cpi!(CommissionPaid {
            commission,
            item,
            submitter,
            bounty_lamports: amount,
            ts
        });
        Ok(())
    }

    /// Anyone refunds the bounty to the creator when no submission was equipped in time.
    pub fn refund_commission(ctx: Context<RefundCommission>) -> Result<()> {
        let a = &ctx.accounts;
        let c = &a.commission;
        require!(c.state == commission_state::OPEN, MarketError::CommissionNotOpen);
        let open_until = c
            .closes_at
            .checked_add(i64::from(a.config.params.commission_vote_secs))
            .ok_or(MarketError::Overflow)?;
        let ts = now()?;
        require!(ts >= open_until, MarketError::RefundTooEarly);
        let amount = a.vault.lamports();
        let commission = a.commission.key();
        let bump = [c.vault_bump];
        let seeds: &[&[u8]] = &[seeds::COMMISSION_VAULT, commission.as_ref(), &bump];
        pay_sol(
            &a.system_program.to_account_info(),
            &a.vault,
            &a.creator,
            amount,
            &[seeds],
        )?;
        let creator = c.creator;
        ctx.accounts.commission.state = commission_state::REFUNDED;
        emit_cpi!(CommissionRefunded {
            commission,
            creator,
            bounty_lamports: amount,
            ts
        });
        Ok(())
    }

    // ---- licences (11 sections 1.4 and 2.3, R45)

    /// The market admin creates the licence parameters.
    pub fn init_licence_config(ctx: Context<InitLicenceConfig>, params: LicenceParams) -> Result<()> {
        licence::process_init_licence_config(ctx, params)
    }

    /// The market admin proposes new licence parameters (applied after the timelock).
    pub fn propose_licence_params(ctx: Context<ProposeLicenceParams>, params: LicenceParams) -> Result<()> {
        licence::process_propose_licence_params(ctx, params)
    }

    /// Anyone applies pending licence parameters once ready.
    pub fn apply_licence_params(ctx: Context<ApplyLicenceParams>) -> Result<()> {
        licence::process_apply_licence_params(ctx)
    }

    /// The item holder sets (or closes, `active = false`) its licence terms.
    #[allow(clippy::too_many_arguments)]
    pub fn set_licence_offer(
        ctx: Context<SetLicenceOffer>,
        price_lamports: u64,
        term_secs: u32,
        per: u8,
        max_live: u16,
        exclusive: bool,
        active: bool,
    ) -> Result<()> {
        licence::process_set_licence_offer(ctx, price_lamports, term_secs, per, max_live, exclusive, active)
    }

    /// Buys (or, while live, extends) a licence for a token; pays the 2.3 split.
    pub fn buy_license(ctx: Context<BuyLicense>, max_price: u64, reference: [u8; 32]) -> Result<()> {
        licence::process_buy_license(ctx, max_price, false, reference)
    }

    /// Renews an existing licence by one term at the offer's price.
    pub fn renew_license(ctx: Context<BuyLicense>, max_price: u64, reference: [u8; 32]) -> Result<()> {
        licence::process_buy_license(ctx, max_price, true, reference)
    }

    /// The holder revokes a per-period licence and refunds the unused fraction.
    pub fn revoke_license(ctx: Context<RevokeLicense>) -> Result<()> {
        licence::process_revoke_license(ctx)
    }

    /// Anyone frees the live slot of an ended licence.
    pub fn expire_license(ctx: Context<ExpireLicense>) -> Result<()> {
        licence::process_expire_license(ctx)
    }
}

fn return_from_escrow(ctx: &Context<Unlist>) -> Result<()> {
    let a = &ctx.accounts;
    let sys = a.system_program.to_account_info();
    let t = TokenAccs {
        token_program: &a.token_program,
        event_authority: &a.token_event_authority,
        system_program: &sys,
    };
    t.check()?;
    let caller = a.caller.to_account_info();
    create_holding(&t, &caller, &a.item_mint, &a.seller, &a.seller_holding)?;
    let item_mint = a.item_mint.key();
    let bump = [ctx.bumps.escrow];
    let seeds: &[&[u8]] = &[seeds::ESCROW, item_mint.as_ref(), &bump];
    transfer_plain(
        &t,
        &a.escrow,
        &a.escrow_holding,
        &a.seller_holding,
        &a.item_mint,
        1,
        &[seeds],
    )
}

fn return_from_lease(ctx: &Context<CloseLease>) -> Result<()> {
    let a = &ctx.accounts;
    let sys = a.system_program.to_account_info();
    let t = TokenAccs {
        token_program: &a.token_program,
        event_authority: &a.token_event_authority,
        system_program: &sys,
    };
    t.check()?;
    let caller = a.caller.to_account_info();
    create_holding(&t, &caller, &a.item_mint, &a.lessor, &a.lessor_holding)?;
    let item_mint = a.item_mint.key();
    let bump = [ctx.bumps.lease_escrow];
    let seeds: &[&[u8]] = &[seeds::LEASE_ESCROW, item_mint.as_ref(), &bump];
    transfer_plain(
        &t,
        &a.lease_escrow,
        &a.lease_escrow_holding,
        &a.lessor_holding,
        &a.item_mint,
        1,
        &[seeds],
    )
}

#[derive(Accounts)]
pub struct Init<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(init, payer = authority, space = 8 + MarketConfig::INIT_SPACE, seeds = [seeds::CONFIG], bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    /// CHECK: this program's ProgramData, parsed in the handler.
    pub program_data: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ProposeParams<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump, has_one = admin @ MarketError::NotAdmin)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(init_if_needed, payer = admin, space = 8 + PendingMarketParams::INIT_SPACE, seeds = [seeds::PENDING], bump)]
    pub pending: Box<Account<'info, PendingMarketParams>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct ApplyParams<'info> {
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(mut, seeds = [seeds::PENDING], bump = pending.bump)]
    pub pending: Box<Account<'info, PendingMarketParams>>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    /// CHECK: the armory `Item` of `item_mint`, read in the handler.
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint, checked against the `Item`.
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the seller's holding of the item, read in the handler.
    #[account(mut)]
    pub seller_holding: UncheckedAccount<'info>,
    /// CHECK: `["escrow", item_mint]`, a system-owned PDA that owns the escrow holding.
    #[account(seeds = [seeds::ESCROW, item_mint.key().as_ref()], bump)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK: the escrow's holding of the item (created here if missing).
    #[account(mut)]
    pub escrow_holding: UncheckedAccount<'info>,
    #[account(init, payer = seller, space = 8 + Listing::INIT_SPACE, seeds = [seeds::LISTING, item_mint.key().as_ref()], bump)]
    pub listing: Box<Account<'info, Listing>>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `delist` (the seller signs as `caller`) and `expire` (anyone signs as `caller`).
#[event_cpi]
#[derive(Accounts)]
pub struct Unlist<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    /// CHECK: the listing's seller; receives the listing's rent and the item.
    #[account(mut, address = listing.seller @ MarketError::WrongRecipient)]
    pub seller: UncheckedAccount<'info>,
    #[account(mut, close = seller, seeds = [seeds::LISTING, item_mint.key().as_ref()], bump = listing.bump)]
    pub listing: Box<Account<'info, Listing>>,
    /// CHECK: the listed item's mint (bound by the listing's seeds).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: `["escrow", item_mint]`.
    #[account(seeds = [seeds::ESCROW, item_mint.key().as_ref()], bump)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK: the escrow's holding (the token program checks it on transfer).
    #[account(mut)]
    pub escrow_holding: UncheckedAccount<'info>,
    /// CHECK: the seller's holding (created if missing; address checked).
    #[account(mut)]
    pub seller_holding: UncheckedAccount<'info>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(mut, close = seller, seeds = [seeds::LISTING, item_mint.key().as_ref()], bump = listing.bump)]
    pub listing: Box<Account<'info, Listing>>,
    /// CHECK: the listing's seller.
    #[account(mut, address = listing.seller @ MarketError::WrongRecipient)]
    pub seller: UncheckedAccount<'info>,
    /// CHECK: the item's author, checked against the `Item` in the handler.
    #[account(mut)]
    pub author: UncheckedAccount<'info>,
    /// CHECK: the market treasury.
    #[account(mut, address = config.treasury @ MarketError::WrongRecipient)]
    pub treasury: UncheckedAccount<'info>,
    /// CHECK: the armory `Item`, read in the handler.
    #[account(address = listing.item @ MarketError::NotAnItem)]
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint (bound by the listing's seeds).
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: `["escrow", item_mint]`.
    #[account(seeds = [seeds::ESCROW, item_mint.key().as_ref()], bump)]
    pub escrow: UncheckedAccount<'info>,
    /// CHECK: the escrow's holding.
    #[account(mut)]
    pub escrow_holding: UncheckedAccount<'info>,
    /// CHECK: the buyer's holding of the item (created if missing; address checked).
    #[account(mut)]
    pub buyer_holding: UncheckedAccount<'info>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct CreateCollection<'info> {
    #[account(mut)]
    pub curator: Signer<'info>,
    #[account(mut, seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(
        init,
        payer = curator,
        space = 8 + Collection::INIT_SPACE,
        seeds = [seeds::COLLECTION, &config.collections.to_le_bytes()],
        bump
    )]
    pub collection: Box<Account<'info, Collection>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct OfferLease<'info> {
    #[account(mut)]
    pub lessor: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    /// CHECK: the armory `Item`, read in the handler.
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint, checked against the `Item`.
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the lessor's holding of the item, read in the handler.
    #[account(mut)]
    pub lessor_holding: UncheckedAccount<'info>,
    /// CHECK: the token that may equip the item, read in the handler.
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: `["lease-escrow", item_mint]`.
    #[account(seeds = [seeds::LEASE_ESCROW, item_mint.key().as_ref()], bump)]
    pub lease_escrow: UncheckedAccount<'info>,
    /// CHECK: the lease escrow's holding (created here).
    #[account(mut)]
    pub lease_escrow_holding: UncheckedAccount<'info>,
    #[account(init, payer = lessor, space = 8 + Lease::INIT_SPACE, seeds = [seeds::LEASE, item.key().as_ref()], bump)]
    pub lease: Box<Account<'info, Lease>>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct AcceptLease<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, seeds = [seeds::LEASE, lease.item.as_ref()], bump = lease.bump)]
    pub lease: Box<Account<'info, Lease>>,
    /// CHECK: the lessor, receives the fee.
    #[account(mut, address = lease.lessor @ MarketError::WrongRecipient)]
    pub lessor: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// `withdraw_offer` (the lessor signs as `caller`) and `end_lease` (anyone signs).
#[event_cpi]
#[derive(Accounts)]
pub struct CloseLease<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(mut, close = lessor, seeds = [seeds::LEASE, lease.item.as_ref()], bump = lease.bump)]
    pub lease: Box<Account<'info, Lease>>,
    /// CHECK: the lessor; receives the lease's rent and the item.
    #[account(mut, address = lease.lessor @ MarketError::WrongRecipient)]
    pub lessor: UncheckedAccount<'info>,
    /// CHECK: the leased item's mint.
    #[account(address = lease.item_mint @ MarketError::WrongAccount)]
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: `["lease-escrow", item_mint]`.
    #[account(seeds = [seeds::LEASE_ESCROW, item_mint.key().as_ref()], bump)]
    pub lease_escrow: UncheckedAccount<'info>,
    /// CHECK: the lease escrow's holding.
    #[account(mut)]
    pub lease_escrow_holding: UncheckedAccount<'info>,
    /// CHECK: the lessor's holding (created if missing; address checked).
    #[account(mut)]
    pub lessor_holding: UncheckedAccount<'info>,
    /// CHECK: the token program, checked in the handler.
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority, checked in the handler.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct OpenCommission<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    /// CHECK: the token whose slot is commissioned, read in the handler.
    pub token_mint: UncheckedAccount<'info>,
    #[account(
        init,
        payer = creator,
        space = 8 + Commission::INIT_SPACE,
        seeds = [seeds::COMMISSION, token_mint.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub commission: Box<Account<'info, Commission>>,
    /// CHECK: `["commission-vault", commission]`, a system-owned PDA holding the bounty.
    #[account(mut, seeds = [seeds::COMMISSION_VAULT, commission.key().as_ref()], bump)]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct Submit<'info> {
    #[account(mut)]
    pub submitter: Signer<'info>,
    #[account(mut, seeds = [seeds::COMMISSION, commission.token_mint.as_ref(), &commission.nonce.to_le_bytes()], bump = commission.bump)]
    pub commission: Box<Account<'info, Commission>>,
    /// CHECK: the commission's token, read in the handler.
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: the armory `Item`, read in the handler.
    pub item: UncheckedAccount<'info>,
    /// CHECK: the item's mint, checked against the `Item`.
    pub item_mint: UncheckedAccount<'info>,
    /// CHECK: the submitter's holding of the item, read in the handler.
    pub submitter_holding: UncheckedAccount<'info>,
    #[account(
        init,
        payer = submitter,
        space = 8 + Submission::INIT_SPACE,
        seeds = [seeds::SUBMISSION, commission.key().as_ref(), item.key().as_ref()],
        bump
    )]
    pub submission: Box<Account<'info, Submission>>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct PayCommission<'info> {
    #[account(mut, seeds = [seeds::COMMISSION, commission.token_mint.as_ref(), &commission.nonce.to_le_bytes()], bump = commission.bump)]
    pub commission: Box<Account<'info, Commission>>,
    #[account(
        seeds = [seeds::SUBMISSION, commission.key().as_ref(), submission.item.as_ref()],
        bump = submission.bump,
        has_one = commission @ MarketError::WrongAccount
    )]
    pub submission: Box<Account<'info, Submission>>,
    /// CHECK: the commission's token, read in the handler.
    #[account(address = commission.token_mint @ MarketError::WrongAccount)]
    pub token_mint: UncheckedAccount<'info>,
    /// CHECK: the submitter, receives the bounty.
    #[account(mut, address = submission.submitter @ MarketError::WrongRecipient)]
    pub submitter: UncheckedAccount<'info>,
    /// CHECK: the bounty vault.
    #[account(mut, seeds = [seeds::COMMISSION_VAULT, commission.key().as_ref()], bump = commission.vault_bump)]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[event_cpi]
#[derive(Accounts)]
pub struct RefundCommission<'info> {
    #[account(seeds = [seeds::CONFIG], bump = config.bump)]
    pub config: Box<Account<'info, MarketConfig>>,
    #[account(mut, seeds = [seeds::COMMISSION, commission.token_mint.as_ref(), &commission.nonce.to_le_bytes()], bump = commission.bump)]
    pub commission: Box<Account<'info, Commission>>,
    /// CHECK: the creator, receives the refund.
    #[account(mut, address = commission.creator @ MarketError::WrongRecipient)]
    pub creator: UncheckedAccount<'info>,
    /// CHECK: the bounty vault.
    #[account(mut, seeds = [seeds::COMMISSION_VAULT, commission.key().as_ref()], bump = commission.vault_bump)]
    pub vault: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
