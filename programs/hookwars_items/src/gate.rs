// Changed by Hookwars: new file (gating, docs/spec/18-gating.md section 3.5): the external gate's
// restricted entry.
//! `gate_before`: the external gate (`hookwars_gate`) runs an item's token-side modules for a
//! Token-2022 transfer. Signed by the gate's `["items-signer"]` only. The entry never records a
//! cut (a module that answers one fails the call), never touches an `EquipState`, a vault or the
//! raid ledger, and runs the same template code as `before_transfer`. Remaining accounts: the
//! item, the composite's module list (composites), each module's extras, the item's `Wear` (items
//! that wear).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use bordrless_hook::{Phase, SlotReturn, TokenSlotArgs, TokenSlotOp};
use hookwars_common::composite::Module;
use hookwars_common::{gate::GATE_ITEMS_SIGNER, ids, manifest as manifest_of, template_id, token_flags};

use crate::equip::read_composite;
use crate::templates::{self, Env};
use crate::ItemsError;

/// What the gate passes besides the transfer: the binding's targets and role.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct GateRunArgs {
    pub token: TokenSlotArgs,
    pub targets: Vec<Pubkey>,
    pub role: u8,
}

/// The gate's callback accounts: its signer and the Token-2022 mint, then the item's extras.
#[derive(Accounts)]
pub struct GateCallback<'info> {
    /// CHECK: the gate's `["items-signer"]`, signing (checked here).
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the Token-2022 mint (read by the gate).
    pub mint: UncheckedAccount<'info>,
}

fn sub<'b>(range: &'b [u8], modules: &[Module], i: usize) -> &'b [u8] {
    let off = hookwars_common::composite::sub_offset(modules, i);
    let len = usize::from(modules[i].data_bytes);
    range.get(off..off + len).unwrap_or(&[])
}

fn put(range: &mut [u8], modules: &[Module], i: usize, bytes: &[u8]) {
    let off = hookwars_common::composite::sub_offset(modules, i);
    let len = usize::from(modules[i].data_bytes).min(bytes.len());
    if let Some(dst) = range.get_mut(off..off + len) {
        dst.copy_from_slice(&bytes[..len]);
    }
}

pub fn run<'info>(ctx: Context<'info, GateCallback<'info>>, args: GateRunArgs) -> Result<()> {
    let signer = &ctx.accounts.hook_signer;
    require!(signer.is_signer && *signer.key == GATE_ITEMS_SIGNER, ItemsError::BadHookSigner);
    let t = &args.token;
    if t.phase != Phase::Before || t.op != TokenSlotOp::Transfer {
        return Ok(());
    }
    let mint = ctx.accounts.mint.key();
    require_keys_eq!(t.mint, mint, ItemsError::WrongAccount);
    let extras = ctx.remaining_accounts;
    let item_info = extras.first().ok_or(ItemsError::WrongAccount)?;
    require_keys_eq!(*item_info.key, t.item, ItemsError::WrongItem);
    require_keys_eq!(*item_info.owner, ids::ARMORY_ID, ItemsError::WrongItem);
    let it = hookwars_armory::state::Item::try_deserialize(&mut &item_info.try_borrow_data()?[..])
        .map_err(|_| error!(ItemsError::WrongItem))?;
    // The gate refuses these at bind; checked again so this entry can never record a cut.
    require!(!it.manifest.token_cuts(), ItemsError::GateCut);
    let mut at = 1usize;
    let modules = if it.template_id == template_id::COMPOSITE {
        let info = extras.get(at).ok_or(ItemsError::BadComposite)?;
        at += 1;
        read_composite(info, &t.item)?.modules
    } else {
        vec![Module {
            template_id: it.template_id,
            params: it.params,
            target_start: 0,
            target_count: args.targets.len() as u8,
            data_bytes: it.manifest.data_bytes,
            reads_module: hookwars_common::composite::NO_READ,
        }]
    };
    let mut module_extras = Vec::with_capacity(modules.len());
    for m in &modules {
        let end = at + templates::extra_count(m.template_id, usize::from(m.target_count));
        require!(end <= extras.len(), ItemsError::WrongAccount);
        module_extras.push(&extras[at..end]);
        at = end;
    }
    let dormant = it.has_wear
        && extras
            .get(at)
            .and_then(|w| hookwars_common::eco_cpi::wear_dormant(w, &t.item))
            .unwrap_or(false);
    if dormant {
        return Ok(());
    }
    let clock = Clock::get()?;
    let total_bytes: usize = modules.iter().map(|m| usize::from(m.data_bytes)).sum();
    require!(t.source_data.len() >= total_bytes && t.destination_data.len() >= total_bytes, ItemsError::RangeTooShort);
    let mut src = t.source_data.clone();
    let mut dst = t.destination_data.clone();
    let (mut src_changed, mut dst_changed) = (false, false);
    for (i, m) in modules.iter().enumerate() {
        let man = manifest_of(m.template_id, &m.params, m.target_count).unwrap_or_default();
        if man.token_flags & token_flags::BEFORE_TRANSFER == 0 {
            continue;
        }
        let start = usize::from(m.target_start);
        let end = (start + usize::from(m.target_count)).min(args.targets.len());
        let env = Env {
            mint,
            slot: t.slot,
            item: t.item,
            module: i as u8,
            params: m.params,
            targets: &args.targets[start.min(end)..end],
            role: args.role,
            extras: module_extras[i],
            now: clock.unix_timestamp,
            clock_slot: clock.slot,
        };
        let out = templates::token_before(m.template_id, &env, t, sub(&t.source_data, &modules, i), sub(&t.destination_data, &modules, i))?;
        require!(out.cut == 0, ItemsError::GateCut);
        if let Some(b) = out.source {
            put(&mut src, &modules, i, &b);
            src_changed = true;
        }
        if let Some(b) = out.destination {
            put(&mut dst, &modules, i, &b);
            dst_changed = true;
        }
    }
    // Always answer (review 2 H-A): the runtime resets return data at every invocation.
    let answer = SlotReturn {
        deltas: vec![],
        source_data: src_changed.then_some(src),
        destination_data: dst_changed.then_some(dst),
    };
    let mut v = Vec::new();
    answer.serialize(&mut v)?;
    set_return_data(&v);
    Ok(())
}
