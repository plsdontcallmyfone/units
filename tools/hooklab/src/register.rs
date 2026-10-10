//! The `register_template` instruction a passing report carries (02 section 3.2). The armory
//! admin signs it; the lab only builds it, so the admin can read what the report vouches for.
//! Changed by Hookwars (protocol pass 4a): an external template registers with
//! `register_external_template` (its declared manifest), behind the armory admin queue (review 1
//! L-1): the report carries the `queue_admin` instruction and the one that applies after the
//! timelock.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_program_tests::armory::{armory_events, armory_ix, fill_queued, queue_ix};
use hookwars_armory::RegisterTemplateArgs;
use hookwars_common::{ids, pda, programdata_address};
use serde_json::{json, Value};

use crate::manifest::Manifest;
use crate::hex;

/// The args of `register_template` for `manifest` under `id`, with `code_hash`.
pub fn args(manifest: &Manifest, id: u16, code_hash: [u8; 32]) -> Result<RegisterTemplateArgs, String> {
    let kind = manifest.kind_byte().ok_or("unknown kind")?;
    let (field_min, field_max) = manifest.field_bounds();
    let r = &manifest.registry;
    Ok(RegisterTemplateArgs {
        id,
        code_hash,
        kind,
        field_count: manifest.params.field_count,
        field_min,
        field_max,
        open_authoring: r.open_authoring,
        loot_enabled: r.loot_enabled,
        forge_enabled: r.forge_enabled,
        max_level: r.max_level,
        loot_royalty_bps: r.loot_royalty_bps,
        max_targets: r.max_targets,
        name: manifest.name.clone(),
    })
}

/// The manifest ceiling the armory stores for an external template (`hookwars_common::Manifest`):
/// the kind, the slot flags its callbacks set, its transfer cut, refusal, burn and range bytes.
/// Pool callbacks are not in this lab yet, so the pool fields are 0.
pub fn armory_manifest(manifest: &Manifest) -> Result<hookwars_common::Manifest, String> {
    Ok(hookwars_common::Manifest {
        kind: manifest.kind_byte().ok_or("unknown kind")?,
        token_flags: manifest.flags(),
        pool_flags: 0,
        max_cut_buy_bps: 0,
        max_cut_sell_bps: 0,
        max_cut_transfer_bps: manifest.max_cut_transfer_bps,
        max_discount_bps: 0,
        may_refuse: manifest.may_refuse,
        may_burn: manifest.may_burn,
        data_bytes: manifest.data_bytes,
        reads_other_pools: 0,
    })
}

/// `register_external_template`, signed and paid by `admin`, with its queue entry filled.
pub fn instruction(admin: Pubkey, program: Pubkey, args: RegisterTemplateArgs, manifest: hookwars_common::Manifest) -> Instruction {
    let mut ix = armory_ix(
        hookwars_armory::accounts::RegisterTemplate {
            admin,
            config: pda::config().0,
            template: pda::template(args.id).0,
            template_program: program,
            programdata: programdata_address(&program),
            system_program: anchor_lang::system_program::ID,
            queued: Pubkey::default(),
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::RegisterExternalTemplate { args, manifest },
    );
    fill_queued(&mut ix, &[program]);
    ix
}

/// The `queue_admin` instruction the admin sends first (the registration applies after
/// `admin_timelock_secs`).
pub fn queue_instruction(admin: Pubkey, program: Pubkey, register: &Instruction) -> Instruction {
    queue_ix(&admin, &register.data, &[program]).0
}

fn ix_json(ix: &Instruction) -> Value {
    json!({
        "program_id": ix.program_id.to_string(),
        "accounts": ix.accounts.iter().map(|a| json!({
            "pubkey": a.pubkey.to_string(),
            "is_signer": a.is_signer,
            "is_writable": a.is_writable,
        })).collect::<Vec<_>>(),
        "data_hex": hex(&ix.data),
    })
}

/// The instruction as JSON (accounts in order, data in hex), plus the args it encodes, the
/// manifest ceiling and the `queue_admin` instruction to send first.
pub fn to_json_queued(ix: &Instruction, args: &RegisterTemplateArgs, manifest: &hookwars_common::Manifest, queue: &Instruction) -> Value {
    let mut v = to_json(ix, args);
    v["queue_admin"] = ix_json(queue);
    v["manifest"] = json!({
        "kind": manifest.kind,
        "token_flags": manifest.token_flags,
        "max_cut_transfer_bps": manifest.max_cut_transfer_bps,
        "may_refuse": manifest.may_refuse,
        "may_burn": manifest.may_burn,
        "data_bytes": manifest.data_bytes,
    });
    v
}

/// The instruction as JSON (accounts in order, data in hex), plus the args it encodes.
pub fn to_json(ix: &Instruction, args: &RegisterTemplateArgs) -> Value {
    json!({
        "program_id": ix.program_id.to_string(),
        "accounts": ix.accounts.iter().map(|a| json!({
            "pubkey": a.pubkey.to_string(),
            "is_signer": a.is_signer,
            "is_writable": a.is_writable,
        })).collect::<Vec<_>>(),
        "data_hex": hex(&ix.data),
        "args": {
            "id": args.id,
            "code_hash": hex(&args.code_hash),
            "kind": args.kind,
            "field_count": args.field_count,
            "field_min": args.field_min.to_vec(),
            "field_max": args.field_max.to_vec(),
            "open_authoring": args.open_authoring,
            "loot_enabled": args.loot_enabled,
            "forge_enabled": args.forge_enabled,
            "max_level": args.max_level,
            "loot_royalty_bps": args.loot_royalty_bps,
            "max_targets": args.max_targets,
            "name": args.name,
        },
    })
}
