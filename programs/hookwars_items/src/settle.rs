// Changed by Hookwars: new file (M3b); security review 2 M-A (stray tokens swept) and L-C (a missing
// destination holding leaves only that module unsettled).
//! `settle_equip` (04 section 2.5, 08 section 2.11): pays what a slot's item collected. For each
//! module, token side and pool side apart: the royalty (`Item.royalty_bps`) to the item's royalty
//! holding, the sender's bounty (the armory's `settle_bounty_bps`), the rest to the module's
//! destination. Every payment leaves its vault through `transfer_from_protocol` (R24) signed by the
//! vault's owner PDA; a burn is a token `burn` signed the same way.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::{invoke, invoke_signed};
use hookwars_common::composite::Module;
use hookwars_common::{ids, pda, template_id, MAX_MODULES};

use crate::equip::read_composite;
use crate::templates::{self, Destination};
use crate::{EquipSettled, ItemsError, SettleEquip};

fn split(x: u64, royalty_bps: u16, bounty_bps: u16) -> (u64, u64, u64) {
    let royalty = (u128::from(x) * u128::from(royalty_bps.min(10_000)) / 10_000) as u64;
    let bounty = (u128::from(x - royalty) * u128::from(bounty_bps.min(10_000)) / 10_000) as u64;
    (royalty, bounty, x - royalty - bounty)
}

struct Cpi<'a, 'info> {
    token_program: &'a AccountInfo<'info>,
    events: &'a AccountInfo<'info>,
    locked: &'a [AccountInfo<'info>],
}

impl<'a, 'info> Cpi<'a, 'info> {
    /// Moves `amount` from `source` (owned by `owner`, a PDA of this program with `seeds`) to
    /// `destination`.
    #[allow(clippy::too_many_arguments)]
    fn pay(
        &self,
        owner: &AccountInfo<'info>,
        source: &AccountInfo<'info>,
        destination: &AccountInfo<'info>,
        mint: &AccountInfo<'info>,
        amount: u64,
        seeds: &[&[u8]],
        with_locked: bool,
    ) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
        require!(*destination.owner == bordrless_token::ID, ItemsError::WrongAccount);
        let extras: &[AccountInfo<'info>] = if with_locked { self.locked } else { &[] };
        let metas: Vec<AccountMeta> = extras
            .iter()
            .map(|a| AccountMeta {
                pubkey: *a.key,
                is_signer: false,
                is_writable: a.is_writable,
            })
            .collect();
        let ix = bordrless_token::client::transfer_from_protocol(
            *owner.key,
            *source.key,
            *destination.key,
            *mint.key,
            metas,
            amount,
            crate::ID,
            seeds.iter().map(|s| s.to_vec()).collect(),
        );
        let mut infos = vec![
            owner.clone(),
            source.clone(),
            destination.clone(),
            mint.clone(),
            self.token_program.clone(),
            self.events.clone(),
        ];
        infos.extend(extras.iter().cloned());
        invoke_signed(&ix, &infos, &[seeds])?;
        Ok(())
    }

    fn burn(
        &self,
        owner: &AccountInfo<'info>,
        source: &AccountInfo<'info>,
        mint: &AccountInfo<'info>,
        amount: u64,
        seeds: &[&[u8]],
    ) -> Result<()> {
        if amount == 0 {
            return Ok(());
        }
        let metas: Vec<AccountMeta> = self
            .locked
            .iter()
            .map(|a| AccountMeta {
                pubkey: *a.key,
                is_signer: false,
                is_writable: a.is_writable,
            })
            .collect();
        let ix: Instruction = bordrless_token::client::burn(*owner.key, *source.key, *mint.key, None, metas, amount);
        let mut infos = vec![
            owner.clone(),
            source.clone(),
            mint.clone(),
            self.token_program.clone(),
            self.events.clone(),
        ];
        infos.extend(self.locked.iter().cloned());
        invoke_signed(&ix, &infos, &[seeds])?;
        Ok(())
    }

    fn create_holding(
        &self,
        payer: &AccountInfo<'info>,
        mint: &AccountInfo<'info>,
        owner: &AccountInfo<'info>,
        holding: &AccountInfo<'info>,
        system: &AccountInfo<'info>,
    ) -> Result<()> {
        if *holding.owner == bordrless_token::ID {
            return Ok(());
        }
        let ix = bordrless_token::client::create_holding(*payer.key, *mint.key, *owner.key);
        invoke(
            &ix,
            &[
                payer.clone(),
                mint.clone(),
                owner.clone(),
                holding.clone(),
                system.clone(),
                self.events.clone(),
                self.token_program.clone(),
            ],
        )?;
        Ok(())
    }
}

/// A destination holding that exists (L-C: one that does not is skipped, not failed).
fn exists(d: &AccountInfo) -> bool {
    *d.owner == bordrless_token::ID && d.data_len() > 0
}

/// `settle_equip`.
pub fn process<'info>(ctx: Context<'info, SettleEquip<'info>>, slot: u8) -> Result<()> {
    let a = &ctx.accounts;
    let mint_key = a.mint.key();
    let state_key = a.equip_state.key();
    let (expected_state, state_bump) = pda::equip_state(&mint_key, slot);
    require_keys_eq!(state_key, expected_state, ItemsError::WrongAccount);
    let item_key = a.equip_state.item;
    require!(item_key != Pubkey::default(), ItemsError::NotEquipped);
    require_keys_eq!(a.item.key(), item_key, ItemsError::WrongItem);
    require_keys_eq!(*a.item.owner, ids::ARMORY_ID, ItemsError::WrongItem);
    let item = hookwars_armory::state::Item::try_deserialize(&mut &a.item.try_borrow_data()?[..])
        .map_err(|_| error!(ItemsError::WrongItem))?;
    let modules: Vec<Module> = if item.template_id == template_id::COMPOSITE {
        read_composite(&a.composite.to_account_info(), &item_key)?.modules
    } else {
        vec![Module {
            template_id: item.template_id,
            params: item.params,
            target_start: 0,
            target_count: a.equip_state.config.targets.len() as u8,
            ..Default::default()
        }]
    };
    require_keys_eq!(*a.armory_config.owner, ids::ARMORY_ID, ItemsError::WrongAccount);
    require_keys_eq!(a.armory_config.key(), pda::config().0, ItemsError::WrongAccount);
    let bounty_bps = hookwars_armory::state::ArmoryConfig::try_deserialize(&mut &a.armory_config.try_borrow_data()?[..])?
        .params
        .settle_bounty_bps;
    let royalty_owner = pda::royalty_owner(&item_key).0;
    require_keys_eq!(a.royalty_owner.key(), royalty_owner, ItemsError::WrongAccount);
    let (pool_cuts, pool_cuts_bump) = pda::pool_cuts(&mint_key);
    require_keys_eq!(a.pool_cuts.key(), pool_cuts, ItemsError::WrongAccount);
    require_keys_eq!(a.quote_mint.key(), ids::BRIDGED_SOL_MINT, ItemsError::WrongAccount);
    let quote = a.quote_mint.key();
    for (info, m, o) in [
        (&a.royalty_token, &mint_key, &royalty_owner),
        (&a.royalty_quote, &quote, &royalty_owner),
        (&a.cranker_token, &mint_key, &a.cranker.key()),
        (&a.cranker_quote, &quote, &a.cranker.key()),
        (&a.pool_cuts_holding, &quote, &pool_cuts),
    ] {
        require_keys_eq!(info.key(), pda::holding(m, o), ItemsError::WrongAccount);
    }
    let rest = ctx.remaining_accounts;
    require!(rest.len() >= 2 * modules.len(), ItemsError::WrongAccount);
    let (dests, locked) = rest.split_at(2 * modules.len());

    let token_program = a.token_program.to_account_info();
    let events = a.token_event_authority.to_account_info();
    let cpi = Cpi {
        token_program: &token_program,
        events: &events,
        locked,
    };
    let system = a.system_program.to_account_info();
    let cranker = a.cranker.to_account_info();
    let mint_info = a.mint.to_account_info();
    let quote_info = a.quote_mint.to_account_info();
    let state_info = a.equip_state.to_account_info();
    let pool_cuts_info = a.pool_cuts.to_account_info();
    let slot_seed = [slot];
    let state_bump_seed = [state_bump];
    let state_seeds: &[&[u8]] = &[hookwars_common::seeds::EQUIP, mint_key.as_ref(), &slot_seed, &state_bump_seed];
    let cuts_bump_seed = [pool_cuts_bump];
    let cuts_seeds: &[&[u8]] = &[hookwars_common::seeds::POOL_CUTS, mint_key.as_ref(), &cuts_bump_seed];
    let royalty_bps = item.royalty_bps;

    let token_owed: [u64; MAX_MODULES] = a.equip_state.token_unsettled;
    let pool_owed: [u64; MAX_MODULES] = a.equip_state.pool_unsettled;
    // What stays owed: a module whose destination holding does not exist yet is skipped and keeps
    // its counter (L-C); anyone can create the holding (`create_holding`) and settle again.
    let mut token_left: [u64; MAX_MODULES] = [0; MAX_MODULES];
    let mut pool_left: [u64; MAX_MODULES] = [0; MAX_MODULES];
    let (mut r_t, mut b_t, mut r_q, mut b_q, mut paid_t, mut paid_q, mut burned) = (0u64, 0u64, 0u64, 0u64, 0u64, 0u64, 0u64);
    let any_token = token_owed.iter().any(|x| *x > 0);
    let any_quote = pool_owed.iter().any(|x| *x > 0);
    if any_token {
        cpi.create_holding(&cranker, &mint_info, &cranker, &a.cranker_token.to_account_info(), &system)?;
    }
    if any_quote {
        cpi.create_holding(&cranker, &quote_info, &cranker, &a.cranker_quote.to_account_info(), &system)?;
    }
    let vault = match a.equip_vault.as_ref() {
        Some(v) => v.to_account_info(),
        None => a.equip_state.to_account_info(),
    };
    if any_token {
        require!(a.equip_vault.is_some(), ItemsError::WrongAccount);
    }
    // M-A: tokens anyone sent to the vault beyond the recorded cuts would block `close_equip` for
    // ever; they are burned here (no module recorded them, so no one is owed them).
    let mut stray = 0u64;
    if a.equip_vault.is_some() {
        require_keys_eq!(vault.key(), pda::holding(&mint_key, &state_key), ItemsError::WrongAccount);
        if *vault.owner == bordrless_token::ID && vault.data_len() > 0 {
            let balance = bordrless_token::client::read_holding(&vault)?.amount;
            let recorded: u64 = token_owed.iter().try_fold(0u64, |s, x| s.checked_add(*x)).ok_or(ItemsError::Overflow)?;
            stray = balance.saturating_sub(recorded);
        }
    }
    for (i, m) in modules.iter().enumerate() {
        let start = usize::from(m.target_start);
        let end = (start + usize::from(m.target_count)).min(a.equip_state.config.targets.len());
        let targets = &a.equip_state.config.targets[start.min(end)..end];
        let x = token_owed.get(i).copied().unwrap_or(0);
        if x > 0 {
            let (royalty, bounty, left) = split(x, royalty_bps, bounty_bps);
            match templates::token_destination(m.template_id, targets) {
                Destination::Burn => {
                    cpi.burn(&state_info, &vault, &mint_info, left, state_seeds)?;
                    burned += left;
                    r_t += royalty;
                    b_t += bounty;
                }
                Destination::Owner(o) => {
                    let d = &dests[2 * i];
                    require_keys_eq!(d.key(), pda::holding(&mint_key, &o), ItemsError::WrongAccount);
                    if exists(d) {
                        cpi.pay(&state_info, &vault, d, &mint_info, left, state_seeds, true)?;
                        paid_t += left;
                        r_t += royalty;
                        b_t += bounty;
                    } else {
                        token_left[i] = x;
                    }
                }
                Destination::None => return err!(ItemsError::WrongAccount),
            }
        }
        let y = pool_owed.get(i).copied().unwrap_or(0);
        if y > 0 {
            let (royalty, bounty, left) = split(y, royalty_bps, bounty_bps);
            match templates::pool_destination(m.template_id, &mint_key, targets) {
                Destination::Owner(o) => {
                    let d = &dests[2 * i + 1];
                    require_keys_eq!(d.key(), pda::holding(&quote, &o), ItemsError::WrongAccount);
                    if exists(d) {
                        cpi.pay(&pool_cuts_info, &a.pool_cuts_holding.to_account_info(), d, &quote_info, left, cuts_seeds, false)?;
                        paid_q += left;
                        r_q += royalty;
                        b_q += bounty;
                    } else {
                        pool_left[i] = y;
                    }
                }
                _ => return err!(ItemsError::WrongAccount),
            }
        }
    }
    if stray > 0 {
        cpi.burn(&state_info, &vault, &mint_info, stray, state_seeds)?;
        burned += stray;
    }
    cpi.pay(&state_info, &vault, &a.royalty_token.to_account_info(), &mint_info, r_t, state_seeds, true)?;
    cpi.pay(&state_info, &vault, &a.cranker_token.to_account_info(), &mint_info, b_t, state_seeds, true)?;
    let cuts_holding = a.pool_cuts_holding.to_account_info();
    cpi.pay(&pool_cuts_info, &cuts_holding, &a.royalty_quote.to_account_info(), &quote_info, r_q, cuts_seeds, false)?;
    cpi.pay(&pool_cuts_info, &cuts_holding, &a.cranker_quote.to_account_info(), &quote_info, b_q, cuts_seeds, false)?;

    let total_quote: u64 = pool_owed.iter().sum::<u64>() - pool_left.iter().sum::<u64>();
    let s = &mut ctx.accounts.equip_state;
    s.token_unsettled = token_left;
    s.pool_unsettled = pool_left;
    s.pool_settled = s.pool_settled.checked_add(total_quote).ok_or(ItemsError::Overflow)?;
    emit!(EquipSettled {
        mint: mint_key,
        slot,
        item: item_key,
        royalty_token: r_t,
        royalty_quote: r_q,
        amount_token: paid_t,
        amount_quote: paid_q,
        burned,
        bounty_token: b_t,
        bounty_quote: b_q,
    });
    Ok(())
}
