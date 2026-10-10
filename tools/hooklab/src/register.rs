//! The `register_template` instruction a passing report carries (02 section 3.2). The armory
//! admin signs it; the lab only builds it, so the admin can read what the report vouches for.

use anchor_lang::prelude::Pubkey;
use anchor_lang::solana_program::instruction::Instruction;
use bordrless_program_tests::armory::{armory_events, armory_ix};
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

/// The instruction, signed and paid by `admin`.
pub fn instruction(admin: Pubkey, program: Pubkey, args: RegisterTemplateArgs) -> Instruction {
    armory_ix(
        hookwars_armory::accounts::RegisterTemplate {
            admin,
            config: pda::config().0,
            template: pda::template(args.id).0,
            template_program: program,
            programdata: programdata_address(&program),
            system_program: anchor_lang::system_program::ID,
            event_authority: armory_events(),
            program: ids::ARMORY_ID,
        },
        hookwars_armory::instruction::RegisterTemplate { args },
    )
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
