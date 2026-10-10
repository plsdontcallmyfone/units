// Changed by Hookwars: new file (expansion, 10). Integration pass 2: revert_leased_slot (I-3).
//! Calls into the token program and the system program. Item mints are plain units mints (no
//! hook, supply 1: 02 section 2.5), so moving an item token out of a market escrow is an ordinary
//! `transfer` signed by the escrow PDA. A protocol transfer (R16) would be needed only for slot
//! mints, which no item is; see the integration requests in 10 for adding this program to
//! `PROTOCOL_SOURCE_PROGRAMS` if that ever changes.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::Instruction;
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use anchor_lang::system_program;

use crate::error::MarketError;

/// The token program's accounts every call here takes.
pub struct TokenAccs<'a, 'info> {
    pub token_program: &'a AccountInfo<'info>,
    pub event_authority: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> TokenAccs<'a, 'info> {
    /// Checks the accounts are the token program's.
    pub fn check(&self) -> Result<()> {
        require_keys_eq!(
            *self.token_program.key,
            bordrless_token::ID,
            MarketError::WrongAccount
        );
        require_keys_eq!(
            *self.event_authority.key,
            bordrless_token::client::event_authority(),
            MarketError::WrongAccount
        );
        Ok(())
    }
}

fn call<'info>(ix: &Instruction, infos: &[AccountInfo<'info>], seeds: &[&[&[u8]]]) -> Result<()> {
    if seeds.is_empty() {
        invoke(ix, infos)?;
    } else {
        invoke_signed(ix, infos, seeds)?;
    }
    Ok(())
}

/// `create_holding` (idempotent): `owner`'s holding of `mint`.
pub fn create_holding<'info>(
    t: &TokenAccs<'_, 'info>,
    payer: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    owner: &AccountInfo<'info>,
    holding: &AccountInfo<'info>,
) -> Result<()> {
    require_keys_eq!(
        *holding.key,
        bordrless_token::client::holding_address(mint.key, owner.key),
        MarketError::WrongAccount
    );
    let ix = bordrless_token::client::create_holding(payer.key(), mint.key(), owner.key());
    call(
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
        &[],
    )
}

/// `transfer` of a plain (hookless) mint, signed by `authority` (a signer of this instruction, or
/// a PDA of this program when `seeds` is given).
pub fn transfer_plain<'info>(
    t: &TokenAccs<'_, 'info>,
    authority: &AccountInfo<'info>,
    source: &AccountInfo<'info>,
    destination: &AccountInfo<'info>,
    mint: &AccountInfo<'info>,
    amount: u64,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    let ix = bordrless_token::client::transfer(
        authority.key(),
        source.key(),
        destination.key(),
        mint.key(),
        None,
        vec![],
        amount,
    );
    call(
        &ix,
        &[
            authority.clone(),
            source.clone(),
            destination.clone(),
            mint.clone(),
            t.token_program.clone(),
            t.event_authority.clone(),
        ],
        seeds,
    )
}

/// Moves `lamports` from `from` (a signer or a system-owned PDA of this program with `seeds`) to
/// `to`. Zero is a no-op.
pub fn pay_sol<'info>(
    system: &AccountInfo<'info>,
    from: &AccountInfo<'info>,
    to: &AccountInfo<'info>,
    lamports: u64,
    seeds: &[&[&[u8]]],
) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    let cpi = CpiContext::new_with_signer(
        system.key(),
        system_program::Transfer {
            from: from.clone(),
            to: to.clone(),
        },
        seeds,
    );
    system_program::transfer(cpi, lamports)
}

/// Reads an armory `Item` (owner, address and discriminator checked) and checks it is the item of
/// `item_mint`.
pub fn read_item(info: &AccountInfo, item_mint: &Pubkey) -> Result<hookwars_armory::state::Item> {
    require_keys_eq!(*info.owner, hookwars_common::ids::ARMORY_ID, MarketError::NotAnItem);
    require_keys_eq!(
        *info.key,
        hookwars_common::pda::item(item_mint).0,
        MarketError::NotAnItem
    );
    let data = info.try_borrow_data()?;
    let item = hookwars_armory::state::Item::try_deserialize(&mut &data[..])
        .map_err(|_| error!(MarketError::NotAnItem))?;
    require_keys_eq!(item.item_mint, *item_mint, MarketError::NotAnItem);
    Ok(item)
}

/// `owner`'s holding of `mint`, read with every check, and its amount.
pub fn holding_amount(info: &AccountInfo, mint: &Pubkey, owner: &Pubkey) -> Result<u64> {
    require_keys_eq!(
        *info.key,
        bordrless_token::client::holding_address(mint, owner),
        MarketError::WrongAccount
    );
    let h = bordrless_token::client::read_holding(info)?;
    require_keys_eq!(h.mint, *mint, MarketError::WrongAccount);
    require_keys_eq!(h.owner, *owner, MarketError::WrongAccount);
    Ok(h.amount)
}

/// Integration pass 2 (10 section 17 I-3): asks the armory to revert the leased slot. `rem` is the
/// armory's `revert_for_lease_end` account list, `["market-caller"]` first, then its refresh
/// accounts; the market signs as `["market-caller"]`.
pub fn revert_leased_slot<'info>(rem: &[AccountInfo<'info>], slot: u8, item: Pubkey) -> Result<()> {
    use anchor_lang::solana_program::instruction::AccountMeta;
    use anchor_lang::InstructionData;
    let (caller, bump) = hookwars_common::market::caller();
    let first = rem.first().ok_or(error!(crate::error::MarketError::WrongAccount))?;
    require_keys_eq!(first.key(), caller, crate::error::MarketError::WrongAccount);
    let accounts = rem
        .iter()
        .enumerate()
        .map(|(i, a)| AccountMeta {
            pubkey: a.key(),
            // The market signs as the caller; the payer of the armory's equip signed the transaction.
            is_signer: i == 0 || a.is_signer,
            is_writable: a.is_writable,
        })
        .collect();
    let ix = Instruction {
        program_id: hookwars_armory::ID,
        accounts,
        data: hookwars_armory::instruction::RevertForLeaseEnd { slot, item }.data(),
    };
    call(&ix, rem, &[&[b"market-caller", &[bump]]])
}
