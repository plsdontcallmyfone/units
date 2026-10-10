// Changed by Hookwars: new file (arsenal waves D and E); integration pass 2: errors in ItemsError,
// reslot_loyalty.
//! The payouts and state of arsenal waves D and E (08 sections 4.3, 4.7): Referral
//! (`set_referrer`, `settle_referral`), Loyalty Pot (`init_loyalty`, `claim_loyalty`) and First
//! Blood (`init_first_blood`). Hooks never pay (00 rule 1): every payment here leaves a vault this
//! program's PDA owns, through the token program's `transfer_from_protocol` (R16, R24).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;
use anchor_lang::solana_program::program::invoke_signed;
use hookwars_common::arsenal2::{self as a2, epoch_of};
use hookwars_common::{ids, pda, template_id};

use crate::equip::{create_or_resize, read_composite};
use crate::templates::loyalty_pot;
use crate::{EquipState, ItemsError};

/// Errors of arsenal waves D and E: `ItemsError` codes 7100 onward (integration pass 2).
pub use crate::ItemsError as ArsenalError;

/// A buyer's chosen referrer (`["referred", mint, buyer]`).
#[account]
#[derive(Debug, InitSpace)]
pub struct Referred {
    /// Layout version.
    pub version: u8,
    /// Bump.
    pub bump: u8,
    /// The token.
    pub mint: Pubkey,
    /// The buyer.
    pub buyer: Pubkey,
    /// Who is paid.
    pub referrer: Pubkey,
    /// Gross cuts recorded and not yet settled.
    pub owed: u64,
    /// Paid to the referrer, all time.
    pub paid: u64,
}

/// First Blood's state (`["first-blood", mint]`).
#[account]
#[derive(Debug, InitSpace)]
pub struct FirstBlood {
    /// Layout version.
    pub version: u8,
    /// Bump.
    pub bump: u8,
    /// The token.
    pub mint: Pubkey,
    /// UTC day (`unix / 86_400`) of the last prize.
    pub last_day: u32,
    /// Whether `last_day`'s prize was given.
    pub taken: bool,
}

/// The Loyalty Pot's state (`["loyalty", mint]`), also the owner of its quote vault.
#[account]
#[derive(Debug, InitSpace)]
pub struct LoyaltyPot {
    /// Layout version.
    pub version: u8,
    /// Bump.
    pub bump: u8,
    /// The token.
    pub mint: Pubkey,
    /// The slot whose item is the pot (or holds it as a module).
    pub slot: u8,
    /// The epoch being claimed (0 before the first roll).
    pub epoch: u32,
    /// What the epoch pays, fixed at the roll (the vault's balance then).
    pub payable: u64,
    /// Claimed of `payable`.
    pub claimed: u64,
    /// Supply outside the pool and the launch at the roll.
    pub eligible_supply: u64,
}

/// A holder's last claimed epoch (`["loyalty-claim", mint, holder]`).
#[account]
#[derive(Debug, InitSpace)]
pub struct LoyaltyClaim {
    /// Layout version.
    pub version: u8,
    /// Bump.
    pub bump: u8,
    /// Last epoch claimed.
    pub last_epoch: u32,
}

/// A referral settled.
#[event]
pub struct ReferralPaid {
    pub mint: Pubkey,
    pub buyer: Pubkey,
    pub referrer: Pubkey,
    pub gross: u64,
    pub paid: u64,
    pub bounty: u64,
}

/// A loyalty claim.
#[event]
pub struct LoyaltyClaimed {
    pub mint: Pubkey,
    pub holder: Pubkey,
    pub epoch: u32,
    pub balance: u64,
    pub amount: u64,
}

/// A loyalty epoch rolled.
#[event]
pub struct LoyaltyRolled {
    pub mint: Pubkey,
    pub epoch: u32,
    pub payable: u64,
    pub eligible_supply: u64,
}

const VERSION: u8 = 1;

/// The balance of `info` when it is the holding of `mint` owned by `owner` (0 when absent).
fn balance(info: &AccountInfo, mint: &Pubkey, owner: &Pubkey) -> Result<u64> {
    require_keys_eq!(*info.key, pda::holding(mint, owner), ItemsError::WrongAccount);
    if *info.owner != bordrless_token::ID {
        return Ok(0);
    }
    Ok(bordrless_token::client::read_holding(info)?.amount)
}

/// `transfer_from_protocol` of `amount` of the quote from `owner`'s holding (a PDA of this
/// program with `seeds`) to `destination`.
#[allow(clippy::too_many_arguments)]
fn pay<'info>(
    owner: &AccountInfo<'info>,
    source: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    quote_mint: &AccountInfo<'info>,
    token_program: &AccountInfo<'info>,
    events: &AccountInfo<'info>,
    amount: u64,
    seeds: &[&[u8]],
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    require!(*destination.owner == bordrless_token::ID, ItemsError::WrongAccount);
    let ix = bordrless_token::client::transfer_from_protocol(
        *owner.key,
        *source.key,
        *destination.key,
        *quote_mint.key,
        Vec::<AccountMeta>::new(),
        amount,
        crate::ID,
        seeds.iter().map(|s| s.to_vec()).collect(),
    );
    invoke_signed(
        &ix,
        &[
            owner.clone(),
            source.clone(),
            destination.clone(),
            quote_mint.clone(),
            token_program.clone(),
            events.clone(),
        ],
        &[seeds],
    )?;
    Ok(())
}

/// `set_referrer`: the buyer names a referrer once (rent paid by the buyer).
pub fn process_set_referrer(ctx: Context<SetReferrer>, referrer: Pubkey) -> Result<()> {
    require_keys_neq!(referrer, ctx.accounts.buyer.key(), ArsenalError::SelfReferral);
    let r = &mut ctx.accounts.referred;
    r.version = VERSION;
    r.bump = ctx.bumps.referred;
    r.mint = ctx.accounts.mint.key();
    r.buyer = ctx.accounts.buyer.key();
    r.referrer = referrer;
    Ok(())
}

/// `settle_referral`: pays a buyer's owed referral from the Referral vault. What reached the vault
/// for a gross cut `g` is `g` less the item's royalty and the settlement bounty (`settle_equip`),
/// so the referrer's share of `g` is the same net; this sender earns the settle bounty on it.
pub fn process_settle_referral(ctx: Context<SettleReferral>, slot: u8) -> Result<()> {
    let a = &ctx.accounts;
    let mint = a.mint.key();
    require_keys_eq!(a.equip_state.key(), pda::equip_state(&mint, slot).0, ItemsError::WrongAccount);
    require_keys_eq!(a.item.key(), a.equip_state.item, ItemsError::WrongItem);
    require_keys_eq!(*a.item.owner, ids::ARMORY_ID, ItemsError::WrongItem);
    let item = hookwars_armory::state::Item::try_deserialize(&mut &a.item.try_borrow_data()?[..])?;
    require_keys_eq!(a.armory_config.key(), pda::config().0, ItemsError::WrongAccount);
    require_keys_eq!(*a.armory_config.owner, ids::ARMORY_ID, ItemsError::WrongAccount);
    let bounty_bps = hookwars_armory::state::ArmoryConfig::try_deserialize(&mut &a.armory_config.try_borrow_data()?[..])?
        .params
        .settle_bounty_bps;
    require_keys_eq!(a.quote_mint.key(), ids::BRIDGED_SOL_MINT, ItemsError::WrongAccount);
    let quote = a.quote_mint.key();
    let (owner, bump) = a2::pda::referral_owner(&mint);
    require_keys_eq!(a.referral_owner.key(), owner, ItemsError::WrongAccount);
    let vault = balance(&a.vault, &quote, &owner)?;
    require_keys_eq!(a.referrer_quote.key(), pda::holding(&quote, &a.referred.referrer), ItemsError::WrongAccount);
    require_keys_eq!(a.cranker_quote.key(), pda::holding(&quote, &a.cranker.key()), ItemsError::WrongAccount);

    let gross = a.referred.owed;
    let keep = |x: u128, bps: u16| x * u128::from(10_000 - bps.min(10_000)) / 10_000;
    let net = keep(keep(u128::from(gross), item.royalty_bps), bounty_bps) as u64;
    let total = net.min(vault);
    require!(total > 0, ArsenalError::NothingToPay);
    // The gross this payment covers (all of it unless the vault was short).
    let covered = if total == net {
        gross
    } else {
        (u128::from(gross) * u128::from(total) / u128::from(net.max(1))) as u64
    };
    let bounty = (u128::from(total) * u128::from(bounty_bps.min(10_000)) / 10_000) as u64;
    let paid = total - bounty;
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[a2::seeds::REFERRAL, mint.as_ref(), &bump_seed];
    let (tp, ev) = (a.token_program.to_account_info(), a.token_event_authority.to_account_info());
    let (o, v, q) = (a.referral_owner.to_account_info(), a.vault.to_account_info(), a.quote_mint.to_account_info());
    pay(&o, &v, &a.referrer_quote.to_account_info(), &q, &tp, &ev, paid, seeds)?;
    pay(&o, &v, &a.cranker_quote.to_account_info(), &q, &tp, &ev, bounty, seeds)?;
    let r = &mut ctx.accounts.referred;
    r.owed -= covered;
    r.paid = r.paid.saturating_add(paid);
    emit!(ReferralPaid {
        mint,
        buyer: r.buyer,
        referrer: r.referrer,
        gross: covered,
        paid,
        bounty,
    });
    Ok(())
}

/// `init_first_blood`: creates First Blood's state (permissionless; the payer pays rent).
pub fn process_init_first_blood(ctx: Context<InitFirstBlood>) -> Result<()> {
    let s = &mut ctx.accounts.first_blood;
    s.version = VERSION;
    s.bump = ctx.bumps.first_blood;
    s.mint = ctx.accounts.mint.key();
    Ok(())
}

/// `init_loyalty`: creates the Loyalty Pot's state for the item in `slot` (permissionless).
pub fn process_init_loyalty(ctx: Context<InitLoyalty>, slot: u8) -> Result<()> {
    let p = &mut ctx.accounts.pot;
    p.version = VERSION;
    p.bump = ctx.bumps.pot;
    p.mint = ctx.accounts.mint.key();
    p.slot = slot;
    Ok(())
}

/// The item an `EquipState` names, or the default key when the account is empty.
fn equipped_item(info: &AccountInfo) -> Result<Pubkey> {
    if *info.owner != crate::ID || info.data_is_empty() {
        return Ok(Pubkey::default());
    }
    Ok(EquipState::try_deserialize(&mut &info.try_borrow_data()?[..])?.item)
}

/// Integration pass 2 (08 arsenal 2 request 7): `reslot_loyalty(slot)` moves the pot to `slot` when
/// the slot it names no longer holds a Loyalty Pot (re-equipped elsewhere, or a wrong slot given at
/// `init_loyalty`) and `slot` does. Permissionless; the pot's epoch accounting is kept.
pub fn process_reslot_loyalty(ctx: Context<ReslotLoyalty>, slot: u8) -> Result<()> {
    let a = &ctx.accounts;
    let mint = a.mint.key();
    require!(slot != a.pot.slot, ItemsError::BadParams);
    require_keys_eq!(a.old_equip_state.key(), pda::equip_state(&mint, a.pot.slot).0, ItemsError::WrongAccount);
    require_keys_eq!(a.new_equip_state.key(), pda::equip_state(&mint, slot).0, ItemsError::WrongAccount);
    let new_item = equipped_item(&a.new_equip_state)?;
    require!(new_item != Pubkey::default(), ArsenalError::NoLoyaltyPot);
    require_keys_eq!(a.new_item.key(), new_item, ItemsError::WrongItem);
    loyalty_module(&a.new_item, &a.new_composite, &new_item)?;
    let old_item = equipped_item(&a.old_equip_state)?;
    if old_item != Pubkey::default() {
        require_keys_eq!(a.old_item.key(), old_item, ItemsError::WrongItem);
        require!(
            loyalty_module(&a.old_item, &a.old_composite, &old_item).is_err(),
            ItemsError::SlotNotEmpty
        );
    }
    ctx.accounts.pot.slot = slot;
    Ok(())
}

/// The Loyalty module of the slot's item: its params and its sub-range offset.
fn loyalty_module(item_info: &AccountInfo, composite: &AccountInfo, item_key: &Pubkey) -> Result<(hookwars_common::Params, usize)> {
    require_keys_eq!(*item_info.owner, ids::ARMORY_ID, ItemsError::WrongItem);
    let item = hookwars_armory::state::Item::try_deserialize(&mut &item_info.try_borrow_data()?[..])?;
    if item.template_id == a2::LOYALTY_POT {
        return Ok((item.params, 0));
    }
    require!(item.template_id == template_id::COMPOSITE, ArsenalError::NoLoyaltyPot);
    let c = read_composite(composite, item_key)?;
    let i = c
        .modules
        .iter()
        .position(|m| m.template_id == a2::LOYALTY_POT)
        .ok_or(ArsenalError::NoLoyaltyPot)?;
    Ok((c.modules[i].params, hookwars_common::composite::sub_offset(&c.modules, i)))
}

/// `claim_loyalty`: rolls the pot when a new epoch began, then pays the holder its share of the
/// epoch's `payable` by balance, when it held through the whole previous epoch (its `joined_epoch`
/// is before the claimed epoch, or it never received since the item was equipped) and has not
/// claimed this epoch.
pub fn process_claim_loyalty(ctx: Context<ClaimLoyalty>) -> Result<()> {
    let a = &ctx.accounts;
    let mint = a.mint.key();
    let holder = a.holder.key();
    let slot = a.pot.slot;
    require_keys_eq!(a.equip_state.key(), pda::equip_state(&mint, slot).0, ItemsError::WrongAccount);
    let item_key = a.equip_state.item;
    require_keys_eq!(a.item.key(), item_key, ItemsError::WrongItem);
    let (params, offset) = loyalty_module(&a.item.to_account_info(), &a.composite.to_account_info(), &item_key)?;
    let now = Clock::get()?.unix_timestamp;
    let current = epoch_of(now, params[1]);
    require_keys_eq!(a.quote_mint.key(), ids::BRIDGED_SOL_MINT, ItemsError::WrongAccount);
    let quote = a.quote_mint.key();
    let (pot_key, pot_bump) = a2::pda::loyalty(&mint);
    let vault = balance(&a.pot_vault, &quote, &pot_key)?;

    // The pool and the launch hold no claim: out of the eligible supply.
    let m = bordrless_token::client::read_mint(&a.mint.to_account_info())?;
    let launch = pda::launch(&mint).0;
    require_keys_eq!(a.launch.key(), launch, ItemsError::WrongAccount);
    let pool = crate::templates::own_pool(&a.launch.to_account_info(), &mint).unwrap_or_default();
    let outside = balance(&a.pool_token, &mint, &pool)?.saturating_add(balance(&a.launch_token, &mint, &launch)?);

    if current > ctx.accounts.pot.epoch {
        let p = &mut ctx.accounts.pot;
        p.epoch = current;
        p.payable = vault;
        p.claimed = 0;
        p.eligible_supply = m.supply.saturating_sub(outside).max(1);
        emit!(LoyaltyRolled {
            mint,
            epoch: current,
            payable: vault,
            eligible_supply: p.eligible_supply,
        });
    }
    let a = &ctx.accounts;
    let epoch = a.pot.epoch;
    require!(epoch > 0, ArsenalError::NotEligible);

    // The holder's range: joined before the epoch (or never restamped).
    let h = bordrless_token::client::read_holding(&a.holder_token)?;
    require_keys_eq!(a.holder_token.key(), pda::holding(&mint, &holder), ItemsError::WrongAccount);
    let range = bordrless_token::slots::read_range(&h.hook_data, &m.slots[usize::from(slot)]);
    let sub = range.get(offset..offset + loyalty_pot::LEN).unwrap_or(&[]);
    if let Some(j) = loyalty_pot::joined(sub) {
        require!(j < epoch, ArsenalError::NotEligible);
    }

    // The claim receipt, created on the first claim.
    let (receipt_key, receipt_bump) = a2::pda::loyalty_claim(&mint, &holder);
    require_keys_eq!(a.receipt.key(), receipt_key, ItemsError::WrongAccount);
    let receipt_info = a.receipt.to_account_info();
    if *receipt_info.owner != crate::ID {
        let bump_seed = [receipt_bump];
        let seeds: &[&[u8]] = &[a2::seeds::LOYALTY_CLAIM, mint.as_ref(), holder.as_ref(), &bump_seed];
        create_or_resize(
            &a.holder.to_account_info(),
            &receipt_info,
            &a.system_program.to_account_info(),
            seeds,
            8 + LoyaltyClaim::INIT_SPACE,
        )?;
        let r = LoyaltyClaim {
            version: VERSION,
            bump: receipt_bump,
            last_epoch: 0,
        };
        let mut data = receipt_info.try_borrow_mut_data()?;
        let mut out: &mut [u8] = &mut data[..];
        r.try_serialize(&mut out)?;
    }
    let mut receipt = LoyaltyClaim::try_deserialize(&mut &receipt_info.try_borrow_data()?[..])?;
    require!(receipt.last_epoch < epoch, ArsenalError::NotEligible);

    let amount = (u128::from(a.pot.payable) * u128::from(h.amount) / u128::from(a.pot.eligible_supply)) as u64;
    let amount = amount.min(a.pot.payable.saturating_sub(a.pot.claimed));
    require!(amount > 0, ArsenalError::NothingToPay);
    require_keys_eq!(a.holder_quote.key(), pda::holding(&quote, &holder), ItemsError::WrongAccount);
    let bump_seed = [pot_bump];
    let seeds: &[&[u8]] = &[a2::seeds::LOYALTY, mint.as_ref(), &bump_seed];
    pay(
        &a.pot.to_account_info(),
        &a.pot_vault.to_account_info(),
        &a.holder_quote.to_account_info(),
        &a.quote_mint.to_account_info(),
        &a.token_program.to_account_info(),
        &a.token_event_authority.to_account_info(),
        amount,
        seeds,
    )?;
    receipt.last_epoch = epoch;
    {
        let mut data = receipt_info.try_borrow_mut_data()?;
        let mut out: &mut [u8] = &mut data[..];
        receipt.try_serialize(&mut out)?;
    }
    let p = &mut ctx.accounts.pot;
    p.claimed = p.claimed.saturating_add(amount);
    emit!(LoyaltyClaimed {
        mint,
        holder,
        epoch,
        balance: h.amount,
        amount,
    });
    Ok(())
}

/// Accounts of `set_referrer`.
#[derive(Accounts)]
pub struct SetReferrer<'info> {
    /// The buyer (pays rent).
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// CHECK: the token (a mint of the token program).
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// Created here, once.
    #[account(
        init,
        payer = buyer,
        space = 8 + Referred::INIT_SPACE,
        seeds = [a2::seeds::REFERRED, mint.key().as_ref(), buyer.key().as_ref()],
        bump
    )]
    pub referred: Account<'info, Referred>,
    pub system_program: Program<'info, System>,
}

/// Accounts of `settle_referral`.
#[derive(Accounts)]
pub struct SettleReferral<'info> {
    /// Sends the instruction, receives the bounty.
    #[account(mut)]
    pub cranker: Signer<'info>,
    /// CHECK: the token.
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// The buyer's record.
    #[account(mut, seeds = [a2::seeds::REFERRED, mint.key().as_ref(), referred.buyer.as_ref()], bump = referred.bump)]
    pub referred: Account<'info, Referred>,
    /// The Referral slot's `EquipState`.
    pub equip_state: Account<'info, EquipState>,
    /// CHECK: the item (armory `Item`, its royalty).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the armory config (the settle bounty).
    pub armory_config: UncheckedAccount<'info>,
    /// CHECK: `["referral", mint]` (system-owned signer).
    pub referral_owner: UncheckedAccount<'info>,
    /// CHECK: its holding of the quote.
    #[account(mut)]
    pub vault: UncheckedAccount<'info>,
    /// CHECK: the referrer's holding of the quote.
    #[account(mut)]
    pub referrer_quote: UncheckedAccount<'info>,
    /// CHECK: the cranker's holding of the quote.
    #[account(mut)]
    pub cranker_quote: UncheckedAccount<'info>,
    /// CHECK: the quote mint (bridged SOL).
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
}

/// Accounts of `init_first_blood`.
#[derive(Accounts)]
pub struct InitFirstBlood<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the token.
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// Created here.
    #[account(
        init,
        payer = payer,
        space = 8 + FirstBlood::INIT_SPACE,
        seeds = [a2::seeds::FIRST_BLOOD, mint.key().as_ref()],
        bump
    )]
    pub first_blood: Account<'info, FirstBlood>,
    pub system_program: Program<'info, System>,
}

/// Accounts of `init_loyalty`.
#[derive(Accounts)]
pub struct InitLoyalty<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the token.
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// Created here.
    #[account(
        init,
        payer = payer,
        space = 8 + LoyaltyPot::INIT_SPACE,
        seeds = [a2::seeds::LOYALTY, mint.key().as_ref()],
        bump
    )]
    pub pot: Account<'info, LoyaltyPot>,
    pub system_program: Program<'info, System>,
}

/// Accounts of `reslot_loyalty` (integration pass 2).
#[derive(Accounts)]
pub struct ReslotLoyalty<'info> {
    /// CHECK: the token.
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    #[account(mut, seeds = [a2::seeds::LOYALTY, mint.key().as_ref()], bump = pot.bump)]
    pub pot: Account<'info, LoyaltyPot>,
    /// CHECK: `["equip", mint, pot.slot]` (checked in the handler).
    pub old_equip_state: UncheckedAccount<'info>,
    /// CHECK: the item it holds, if any (checked in the handler).
    pub old_item: UncheckedAccount<'info>,
    /// CHECK: that item's composite list, or any account (checked in the handler).
    pub old_composite: UncheckedAccount<'info>,
    /// CHECK: `["equip", mint, slot]` (checked in the handler).
    pub new_equip_state: UncheckedAccount<'info>,
    /// CHECK: the item it holds (checked in the handler).
    pub new_item: UncheckedAccount<'info>,
    /// CHECK: that item's composite list, or any account (checked in the handler).
    pub new_composite: UncheckedAccount<'info>,
}

/// Accounts of `claim_loyalty`.
#[derive(Accounts)]
pub struct ClaimLoyalty<'info> {
    /// The holder (pays the receipt's rent on the first claim).
    #[account(mut)]
    pub holder: Signer<'info>,
    /// CHECK: the token.
    #[account(owner = bordrless_token::ID)]
    pub mint: UncheckedAccount<'info>,
    /// The pot.
    #[account(mut, seeds = [a2::seeds::LOYALTY, mint.key().as_ref()], bump = pot.bump)]
    pub pot: Account<'info, LoyaltyPot>,
    /// CHECK: the pot's holding of the quote.
    #[account(mut)]
    pub pot_vault: UncheckedAccount<'info>,
    /// The pot slot's `EquipState`.
    pub equip_state: Account<'info, EquipState>,
    /// CHECK: the item (armory `Item`).
    pub item: UncheckedAccount<'info>,
    /// CHECK: the composite's module list, or this program's id.
    pub composite: UncheckedAccount<'info>,
    /// CHECK: the holder's holding of the token (its range and balance).
    pub holder_token: UncheckedAccount<'info>,
    /// CHECK: the holder's holding of the quote.
    #[account(mut)]
    pub holder_quote: UncheckedAccount<'info>,
    /// CHECK: `["loyalty-claim", mint, holder]`, created on the first claim.
    #[account(mut)]
    pub receipt: UncheckedAccount<'info>,
    /// CHECK: our `Launch`.
    pub launch: UncheckedAccount<'info>,
    /// CHECK: the launch pool's holding of the token (may not exist).
    pub pool_token: UncheckedAccount<'info>,
    /// CHECK: the launch's holding of the token (may not exist).
    pub launch_token: UncheckedAccount<'info>,
    /// CHECK: the quote mint (bridged SOL).
    pub quote_mint: UncheckedAccount<'info>,
    /// CHECK: the token program.
    #[account(address = bordrless_token::ID)]
    pub token_program: UncheckedAccount<'info>,
    /// CHECK: the token program's event authority.
    pub token_event_authority: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
