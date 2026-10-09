//! What every war step shares: calling other programs by built instruction (the companion's way,
//! upstream `invoke.rs`), signer seeds, balances, paying SOL, reading the War orders, the kit and the
//! Raid range.

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::system_program;
use bordrless_hook::{slot_flags, slot_kind};
use bordrless_kit::constants::modules;
use bordrless_token::client::{self as token_client, Hook};
use bordrless_token::state::{Mint, Slot};

use crate::constants::*;
use crate::error::WarError;
use crate::foreign::{Item, Template};
use crate::state::WarOrders;

/// Invokes `ix` with the account infos of its keys, found among `available`, signing with `seeds`.
/// A caller who leaves an account out gets `MissingAccount`; one who passes another gets nothing
/// from it (the callee checks every account it is given).
pub fn invoke_built<'info>(
    ix: &Instruction,
    available: &[AccountInfo<'info>],
    seeds: &[&[&[u8]]],
) -> Result<()> {
    let mut infos = Vec::with_capacity(ix.accounts.len() + 1);
    for meta in &ix.accounts {
        infos.push(find(available, &meta.pubkey)?.clone());
    }
    infos.push(find(available, &ix.program_id)?.clone());
    invoke_signed(ix, &infos, seeds)?;
    Ok(())
}

pub fn find<'a, 'info>(
    available: &'a [AccountInfo<'info>],
    key: &Pubkey,
) -> Result<&'a AccountInfo<'info>> {
    available
        .iter()
        .find(|a| a.key == key)
        .ok_or(error!(WarError::MissingAccount))
}

/// Named accounts followed by the remaining ones.
pub fn available<'info>(
    named: Vec<AccountInfo<'info>>,
    remaining: &[AccountInfo<'info>],
) -> Vec<AccountInfo<'info>> {
    let mut all = named;
    all.extend_from_slice(remaining);
    all
}

/// The first `len` remaining accounts as metas (a token slice the client resolved: program,
/// signer, extras per slot; the token program checks every one of them).
pub fn slice_metas(remaining: &[AccountInfo], len: usize) -> Result<Vec<AccountMeta>> {
    require!(remaining.len() >= len, WarError::MissingAccount);
    Ok(remaining[..len]
        .iter()
        .map(|a| AccountMeta {
            pubkey: *a.key,
            is_signer: false,
            is_writable: a.is_writable,
        })
        .collect())
}

/// Signer seeds `[prefix, key, bump]`.
pub struct KeyedSeeds {
    prefix: &'static [u8],
    key: Pubkey,
    bump: [u8; 1],
}

impl KeyedSeeds {
    pub fn new(prefix: &'static [u8], key: Pubkey, bump: u8) -> Self {
        Self {
            prefix,
            key,
            bump: [bump],
        }
    }

    pub fn seeds(&self) -> [&[u8]; 3] {
        [self.prefix, self.key.as_ref(), &self.bump]
    }
}

/// Signer seeds `[prefix, bump]`.
pub struct SoloSeeds {
    prefix: &'static [u8],
    bump: [u8; 1],
}

impl SoloSeeds {
    pub fn new(prefix: &'static [u8], bump: u8) -> Self {
        Self {
            prefix,
            bump: [bump],
        }
    }

    pub fn seeds(&self) -> [&[u8]; 2] {
        [self.prefix, &self.bump]
    }
}

/// A holding's balance; 0 for one that does not exist yet.
pub fn balance(available: &[AccountInfo], mint: &Pubkey, owner: &Pubkey) -> Result<u64> {
    let info = find(available, &token_client::holding_address(mint, owner))?;
    if info.owner != &TOKEN_ID {
        return Ok(0);
    }
    Ok(token_client::read_holding(info)?.amount)
}

/// Creates `owner`'s holding of `mint` when it does not exist, `payer` (a signer) paying.
pub fn ensure_holding(
    available: &[AccountInfo],
    payer: Pubkey,
    mint: Pubkey,
    owner: Pubkey,
) -> Result<()> {
    let info = find(available, &token_client::holding_address(&mint, &owner))?;
    if info.owner == &TOKEN_ID {
        return Ok(());
    }
    invoke_built(
        &token_client::create_holding(payer, mint, owner),
        available,
        &[],
    )
}

/// Pays `to` `lamports` of `from`'s bridged SOL as SOL: unwrapped to `from` (a system-owned PDA of
/// this program, signing with `seeds`), then sent on, so `from` ends with what it had.
pub fn pay_sol<'info>(
    available: &[AccountInfo<'info>],
    seeds: &[&[u8]],
    from: Pubkey,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    invoke_built(
        &bordrless_bridge::client::unwrap_sol(from, lamports),
        available,
        &[seeds],
    )?;
    transfer_lamports(available, seeds, from, to, lamports)
}

/// A system transfer from `from` (a system-owned PDA signing with `seeds`) to `to`.
pub fn transfer_lamports<'info>(
    available: &[AccountInfo<'info>],
    seeds: &[&[u8]],
    from: Pubkey,
    to: &AccountInfo<'info>,
    lamports: u64,
) -> Result<()> {
    if lamports == 0 {
        return Ok(());
    }
    let from = find(available, &from)?.clone();
    let system = find(available, &system_program::ID)?.clone();
    system_program::transfer(
        CpiContext::new_with_signer(
            *system.key,
            system_program::Transfer {
                from,
                to: to.clone(),
            },
            &[seeds],
        ),
        lamports,
    )
}

/// The token-hook of `mint` as token instructions take it: `None` on a slot mint (its slices go in
/// the extras) or a hookless one; the upstream single hook otherwise.
pub fn token_hook(mint: &Mint) -> Option<Hook> {
    if mint.uses_slots() {
        None
    } else {
        mint.hook_program.map(Hook::of)
    }
}

/// Whether `mint` runs the kit (the upstream single hook, or a `Locked` slot).
pub fn has_kit(mint: &Mint) -> bool {
    if mint.uses_slots() {
        mint.locked_slot().is_some_and(|(_, s)| s.program == KIT_ID)
    } else {
        mint.hook_program == Some(KIT_ID)
    }
}

/// Whether the kit of `mint` (if it has one) has holder rewards on. A kit token must pass its
/// `KitConfig` (`["kit", mint]` under the kit).
pub fn kit_rewards_on(mint_key: &Pubkey, mint: &Mint, kit_config: Option<&AccountInfo>) -> Result<bool> {
    if !has_kit(mint) {
        return Ok(false);
    }
    let info = kit_config.ok_or(error!(WarError::MissingAccount))?;
    require_keys_eq!(
        *info.key,
        bordrless_kit::client::kit_config_address(mint_key),
        WarError::WrongAccount
    );
    let kit = bordrless_kit::client::read_kit_config(info)?;
    Ok(kit.has(modules::HOLDER_REWARDS))
}

/// The `War` slot of `mint` (05 section 5: `MissingWarSlot` without one).
pub fn war_slot(mint: &Mint) -> Result<Slot> {
    mint.active_slots()
        .iter()
        .find(|s| s.kind == slot_kind::WAR)
        .copied()
        .ok_or(error!(WarError::MissingWarSlot))
}

/// The War orders of `mint` (05 section 6.0): the item the `War` slot names, of a template of kind
/// `War` whose program is the items program. An empty slot is `NoWarOrders`.
pub fn war_orders(mint: &Mint, item: &AccountInfo, template: &AccountInfo) -> Result<WarOrders> {
    let slot = war_slot(mint)?;
    require!(slot.item != Pubkey::default(), WarError::NoWarOrders);
    require_keys_eq!(*item.key, slot.item, WarError::WrongWarOrders);
    let it = Item::read(item)?;
    require_keys_eq!(
        *template.key,
        Template::address(it.template_id),
        WarError::WrongWarOrders
    );
    let t = Template::read(template)?;
    require!(
        t.kind == slot_kind::WAR && t.program == ITEMS_ID && t.id == it.template_id,
        WarError::WrongWarOrders
    );
    Ok(WarOrders { params: it.params })
}

/// Whether `item` is in one of `mint`'s slots.
pub fn equips(mint: &Mint, item: &Pubkey) -> bool {
    mint.active_slots().iter().any(|s| s.item == *item)
}

// ------------------------------------------------------------------------------- the Raid range

/// The Raid range's bytes as an item sees them (04 section 3.1): tag, season, points, tickets.
pub const RAID_RANGE_LEN: usize = 11;
/// The Raid range's layout tag.
pub const RAID_TAG: u8 = 0x01;

/// A holding's Raid range, decoded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaidRange {
    pub season_id: u32,
    pub raid_points: u32,
    pub tickets: u16,
}

impl RaidRange {
    /// Decodes the range (zeros, or another tag: never stamped).
    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.len() < RAID_RANGE_LEN || bytes[0] != RAID_TAG {
            return Self::default();
        }
        let u32_at = |i: usize| u32::from_le_bytes(bytes[i..i + 4].try_into().unwrap_or([0; 4]));
        Self {
            season_id: u32_at(1),
            raid_points: u32_at(5),
            tickets: u16::from_le_bytes(bytes[9..11].try_into().unwrap_or([0; 2])),
        }
    }

    /// Points of season `current` (another season's count 0).
    pub fn points_in(&self, current: u32) -> u32 {
        if self.season_id == current {
            self.raid_points
        } else {
            0
        }
    }
}

/// The touch payloads the Raid item accepts from the war signer (04 section 2.10).
/// REPLACE with the items program's `WarTouch` at integration.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarTouch {
    SpendRaidPoints { amount: u32 },
    SpendTicket,
    AddTicket { amount: u16 },
}

/// The slot at `index` if it is a Raid slot this program can touch: the items program's, answering
/// touch, with a Raid-sized range.
pub fn raid_slot(mint: &Mint, index: u8) -> Result<Slot> {
    require!(index < mint.slot_count, WarError::WrongRaidSlot);
    let slot = mint.slots[usize::from(index)];
    require!(
        slot.program == ITEMS_ID
            && slot.flags & slot_flags::ANSWERS_TOUCH != 0
            && usize::from(slot.data_len) == RAID_RANGE_LEN + 1,
        WarError::WrongRaidSlot
    );
    Ok(slot)
}

/// The Raid range of `holding_info` (a holding of the mint) under `slot`.
pub fn read_raid(holding_info: &AccountInfo, slot: &Slot) -> Result<RaidRange> {
    let holding = token_client::read_holding(holding_info)?;
    let bytes = bordrless_token::slots::read_range(&holding.hook_data, slot);
    Ok(RaidRange::decode(&bytes))
}

/// `touch` of `holding` through slot `index` with `payload`, signed by the war signer. `extras`
/// are the slot's own accounts (`extra_count` of them) the client passed.
#[allow(clippy::too_many_arguments)]
pub fn touch_raid<'info>(
    available: &[AccountInfo<'info>],
    war_signer: Pubkey,
    war_signer_bump: u8,
    mint: Pubkey,
    holding: Pubkey,
    index: u8,
    payload: WarTouch,
    extras: Vec<AccountMeta>,
) -> Result<()> {
    let mut data = Vec::new();
    payload.serialize(&mut data)?;
    let ix = token_client::touch(war_signer, mint, holding, ITEMS_ID, index, data, extras);
    let bump = [war_signer_bump];
    let seeds: [&[u8]; 2] = [WAR_SIGNER_SEED, &bump];
    invoke_built(&ix, available, &[&seeds])
}
