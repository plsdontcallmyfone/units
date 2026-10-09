// Changed by Hookwars: new file (M3b).
//! The callback engine (04 section 2): signer and account checks, the module list (a plain item is
//! one module; a composite is its `CompositeItem`, 08 section 2.3), running each module on its own
//! sub-range and extras, merging the answers, and recording cuts in `EquipState` (R1, R2).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use bordrless_hook::pool_item::{ItemPoolAnswer, ItemPoolContext};
use bordrless_hook::{Delta, Phase, PoolHookArgs, SlotReturn, TokenSlotArgs, TokenSlotOp};
use hookwars_common::composite::Module;
use hookwars_common::raid::RaidLedger;
use hookwars_common::{ids, manifest as manifest_of, pda, pool_flags, template_id, token_flags, Manifest};

use crate::equip::{create_or_resize, read_composite};
use crate::templates::{self, Env};
use crate::{
    EquipState, InitRaidLedger, ItemCut, ItemsError, PoolCallback, TokenCallback, LAUNCH_ITEMS_SIGNER,
    TOKEN_ITEMS_SIGNER, VAULT_INDEX,
};

/// The token program's signer, signing.
pub fn check_token_signer(info: &AccountInfo) -> Result<()> {
    require!(
        info.is_signer && *info.key == TOKEN_ITEMS_SIGNER,
        ItemsError::BadHookSigner
    );
    Ok(())
}

/// An equipped item, read from the head of its extras.
pub struct Loaded<'a, 'info> {
    pub item_key: Pubkey,
    pub item: hookwars_armory::state::Item,
    pub state_info: &'a AccountInfo<'info>,
    pub state: EquipState,
    pub modules: Vec<Module>,
    /// Each module's own extras.
    pub module_extras: Vec<&'a [AccountInfo<'info>]>,
}

/// Reads `Item`, `EquipState` (checked against `mint` and `slot`), the vault slot, the module
/// list, and splits the rest of the extras per module.
pub fn load<'a, 'info>(
    extras: &'a [AccountInfo<'info>],
    mint: &Pubkey,
    slot: u8,
    item: &Pubkey,
) -> Result<Loaded<'a, 'info>> {
    require!(extras.len() >= 2, ItemsError::WrongAccount);
    let item_info = &extras[0];
    require_keys_eq!(*item_info.key, *item, ItemsError::WrongItem);
    require_keys_eq!(*item_info.owner, ids::ARMORY_ID, ItemsError::WrongItem);
    let it = hookwars_armory::state::Item::try_deserialize(&mut &item_info.try_borrow_data()?[..])
        .map_err(|_| error!(ItemsError::WrongItem))?;
    let state_info = &extras[1];
    require_keys_eq!(*state_info.owner, crate::ID, ItemsError::NotEquipped);
    require_keys_eq!(*state_info.key, pda::equip_state(mint, slot).0, ItemsError::NotEquipped);
    let state = EquipState::try_deserialize(&mut &state_info.try_borrow_data()?[..])?;
    require_keys_eq!(state.item, *item, ItemsError::NotEquipped);
    let mut at = 2usize;
    if it.manifest.token_cuts() {
        at += 1;
    }
    let modules = if it.template_id == template_id::COMPOSITE {
        let info = extras.get(at).ok_or(ItemsError::BadComposite)?;
        at += 1;
        read_composite(info, item)?.modules
    } else {
        vec![Module {
            template_id: it.template_id,
            params: it.params,
            target_start: 0,
            target_count: state.config.targets.len() as u8,
            data_bytes: it.manifest.data_bytes,
            reads_module: hookwars_common::composite::NO_READ,
        }]
    };
    let mut module_extras = Vec::with_capacity(modules.len());
    for m in &modules {
        let n = templates::extra_count(m.template_id, usize::from(m.target_count));
        let end = at + n;
        require!(end <= extras.len(), ItemsError::WrongAccount);
        module_extras.push(&extras[at..end]);
        at = end;
    }
    Ok(Loaded {
        item_key: *item,
        item: it,
        state_info,
        state,
        modules,
        module_extras,
    })
}

impl<'a, 'info> Loaded<'a, 'info> {
    fn env<'s>(&'s self, i: usize, mint: &Pubkey, slot: u8, clock: &Clock) -> Env<'s, 'info> {
        let m = &self.modules[i];
        let start = usize::from(m.target_start);
        let end = (start + usize::from(m.target_count)).min(self.state.config.targets.len());
        Env {
            mint: *mint,
            slot,
            item: self.item_key,
            module: i as u8,
            params: m.params,
            targets: &self.state.config.targets[start.min(end)..end],
            role: self.state.config.role,
            extras: self.module_extras[i],
            now: clock.unix_timestamp,
            clock_slot: clock.slot,
        }
    }

    fn manifest(&self, i: usize) -> Manifest {
        let m = &self.modules[i];
        manifest_of(m.template_id, &m.params, m.target_count).unwrap_or_default()
    }

    fn save(&self) -> Result<()> {
        let mut data = self.state_info.try_borrow_mut_data()?;
        let mut out: &mut [u8] = &mut data[..];
        self.state.try_serialize(&mut out)
    }
}

/// The sub-range of module `i` inside a slot range.
fn sub<'b>(range: &'b [u8], modules: &[Module], i: usize) -> &'b [u8] {
    let off = hookwars_common::composite::sub_offset(modules, i);
    let len = usize::from(modules[i].data_bytes);
    range.get(off..off + len).unwrap_or(&[])
}

fn put(range: &mut [u8], modules: &[Module], i: usize, bytes: &[u8]) {
    let off = hookwars_common::composite::sub_offset(modules, i);
    let len = usize::from(modules[i].data_bytes).min(bytes.len());
    range[off..off + len].copy_from_slice(&bytes[..len]);
}

/// `before_transfer` (and nothing else answers on the token side but `on_touch`).
pub fn token_before<'info>(ctx: Context<'info, TokenCallback<'info>>, args: TokenSlotArgs) -> Result<()> {
    check_token_signer(&ctx.accounts.hook_signer)?;
    if args.phase != Phase::Before || args.op != TokenSlotOp::Transfer {
        return Ok(());
    }
    let mint = ctx.accounts.mint.key();
    let mut l = load(ctx.remaining_accounts, &mint, args.slot, &args.item)?;
    let clock = Clock::get()?;
    let total_bytes: usize = l.modules.iter().map(|m| usize::from(m.data_bytes)).sum();
    require!(args.source_data.len() >= total_bytes, ItemsError::RangeTooShort);
    let mut src = args.source_data.clone();
    let mut dst = args.destination_data.clone();
    let (mut src_changed, mut dst_changed) = (false, false);
    let mut total: u64 = 0;
    for i in 0..l.modules.len() {
        if l.manifest(i).token_flags & token_flags::BEFORE_TRANSFER == 0 {
            continue;
        }
        let env = l.env(i, &mint, args.slot, &clock);
        let out = templates::token_before(
            l.modules[i].template_id,
            &env,
            &args,
            sub(&args.source_data, &l.modules, i),
            sub(&args.destination_data, &l.modules, i),
        )?;
        if let Some(b) = out.source {
            put(&mut src, &l.modules, i, &b);
            src_changed = true;
        }
        if let Some(b) = out.destination {
            put(&mut dst, &l.modules, i, &b);
            dst_changed = true;
        }
        if out.cut > 0 {
            l.state.token_unsettled[i] = l.state.token_unsettled[i].checked_add(out.cut).ok_or(ItemsError::Overflow)?;
            total = total.checked_add(out.cut).ok_or(ItemsError::Overflow)?;
            emit!(ItemCut {
                mint,
                slot: args.slot,
                item: args.item,
                module: i as u8,
                side: 0,
                amount: out.cut,
            });
        }
    }
    require!(total <= args.amount, ItemsError::Overflow);
    l.state.runs = l.state.runs.saturating_add(1);
    if total > 0 {
        require!(l.item.manifest.token_cuts(), ItemsError::WrongAccount);
        l.state.collected_token = l.state.collected_token.saturating_add(total);
    }
    l.save()?;
    let answer = SlotReturn {
        deltas: if total > 0 {
            vec![Delta {
                amount: total,
                account: VAULT_INDEX,
            }]
        } else {
            vec![]
        },
        source_data: src_changed.then_some(src),
        destination_data: dst_changed.then_some(dst),
    };
    if answer != SlotReturn::default() {
        let mut v = Vec::new();
        answer.serialize(&mut v)?;
        set_return_data(&v);
    }
    Ok(())
}

/// `on_touch`: routed to the one module that answers touches (08 section 2.7).
pub fn touch<'info>(ctx: Context<'info, TokenCallback<'info>>, args: TokenSlotArgs) -> Result<()> {
    check_token_signer(&ctx.accounts.hook_signer)?;
    require!(args.op == TokenSlotOp::Touch, ItemsError::BadParams);
    let mint = ctx.accounts.mint.key();
    let l = load(ctx.remaining_accounts, &mint, args.slot, &args.item)?;
    let clock = Clock::get()?;
    let mut src = args.source_data.clone();
    for i in 0..l.modules.len() {
        if l.manifest(i).token_flags & token_flags::ANSWERS_TOUCH == 0 {
            continue;
        }
        let env = l.env(i, &mint, args.slot, &clock);
        if let Some(b) = templates::touch(l.modules[i].template_id, &env, &args, sub(&args.source_data, &l.modules, i))? {
            put(&mut src, &l.modules, i, &b);
            let answer = SlotReturn {
                deltas: vec![],
                source_data: Some(src),
                destination_data: None,
            };
            let mut v = Vec::new();
            answer.serialize(&mut v)?;
            set_return_data(&v);
            return Ok(());
        }
    }
    Ok(())
}

/// Pool callbacks (03 section 5, 04 section 2.8).
pub fn pool<'info>(
    ctx: Context<'info, PoolCallback<'info>>,
    args: PoolHookArgs,
    item_ctx: ItemPoolContext,
    before: bool,
) -> Result<()> {
    let signer = &ctx.accounts.hook_signer;
    require!(
        signer.is_signer && *signer.key == LAUNCH_ITEMS_SIGNER,
        ItemsError::BadHookSigner
    );
    let mint = ctx.accounts.base_mint.key();
    require_keys_eq!(args.base_mint, mint, ItemsError::WrongAccount);
    let mut l = load(ctx.remaining_accounts, &mint, item_ctx.slot, &item_ctx.item)?;
    let clock = Clock::get()?;
    let bit = if before { pool_flags::BEFORE_SWAP } else { pool_flags::AFTER_SWAP };
    let mut answer = ItemPoolAnswer::default();
    let mut discount: u32 = 0;
    for i in 0..l.modules.len() {
        if l.manifest(i).pool_flags & bit == 0 {
            continue;
        }
        let env = l.env(i, &mint, item_ctx.slot, &clock);
        let out = templates::pool(l.modules[i].template_id, &env, &args, &item_ctx, before)?;
        discount += u32::from(out.discount_bps);
        answer.burn = answer.burn.checked_add(out.burn).ok_or(ItemsError::Overflow)?;
        if out.cut > 0 {
            answer.cut = answer.cut.checked_add(out.cut).ok_or(ItemsError::Overflow)?;
            l.state.pool_unsettled[i] = l.state.pool_unsettled[i].checked_add(out.cut).ok_or(ItemsError::Overflow)?;
            emit!(ItemCut {
                mint,
                slot: item_ctx.slot,
                item: item_ctx.item,
                module: i as u8,
                side: if args.direction == templates::BUY { 1 } else { 2 },
                amount: out.cut,
            });
        }
    }
    answer.discount_bps = discount.min(10_000) as u16;
    require!(answer.cut <= item_ctx.side_amount, ItemsError::Overflow);
    l.state.pool_owed = l.state.pool_owed.checked_add(answer.cut).ok_or(ItemsError::Overflow)?;
    l.state.runs = l.state.runs.saturating_add(1);
    l.save()?;
    if answer != ItemPoolAnswer::default() {
        let mut v = Vec::new();
        answer.serialize(&mut v)?;
        set_return_data(&v);
    }
    Ok(())
}

/// `init_raid_ledger`.
pub fn process_init_raid_ledger(ctx: Context<InitRaidLedger>) -> Result<()> {
    let mint = ctx.accounts.mint.key();
    let (key, bump) = pda::raid_ledger(&mint);
    let info = ctx.accounts.raid_ledger.to_account_info();
    require_keys_eq!(info.key(), key, ItemsError::WrongAccount);
    if *info.owner == crate::ID {
        return Ok(());
    }
    let bump_seed = [bump];
    let seeds: &[&[u8]] = &[hookwars_common::seeds::RAID_LEDGER, mint.as_ref(), &bump_seed];
    create_or_resize(
        &ctx.accounts.payer.to_account_info(),
        &info,
        &ctx.accounts.system_program.to_account_info(),
        seeds,
        RaidLedger::LEN,
    )?;
    let l = RaidLedger {
        version: hookwars_common::raid::VERSION,
        bump,
        mint,
        ..Default::default()
    };
    let mut data = info.try_borrow_mut_data()?;
    l.encode(&mut data).ok_or(ItemsError::WrongAccount)?;
    Ok(())
}
