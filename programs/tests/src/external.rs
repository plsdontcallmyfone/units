// Changed by Hookwars: new file (protocol pass 4a), external templates in the suites.
//! External templates (Hook Lab gaps 1 to 4): `ext_template` registered through the admin queue
//! with `register_external_template`, authored and equipped through the real armory, cutting real
//! transfers into the slot's equip vault, settled by the items program. The Hook Lab pipeline runs
//! [`cut_flow`] against its starter template.

use bordrless_hook::{equip_rule, slot_kind};
use bordrless_token::instructions::SlotInit;
use bordrless_token::state::SlotBounds;
use hookwars_armory::RegisterTemplateArgs;
use hookwars_common::{ids, pda, EquipConfig, Manifest, PARAM_FIELDS};
use solana_signer::Signer;

use crate::armory::Hw;
use crate::items::{equip_state, settle_ix, transfer};
use crate::program_bytes;

/// The external template id the suites use (no built-in template has it).
pub const EXT_ID: u16 = 1000;

/// Loads `ext_template` as an immutable program.
pub fn load(hw: &mut Hw) {
    hw.w.env
        .svm
        .add_program(ext_template::ID, &program_bytes("ext_template"))
        .expect("load ext_template");
    hw.w.env.set_upgrade_authority(ext_template::ID, None);
}

/// Registration args of an external template: two fields (token cut bps, pool cut bps), open
/// authoring, one level.
pub fn args(id: u16, kind: u8, max_targets: u8) -> RegisterTemplateArgs {
    let mut field_max = [0u32; PARAM_FIELDS];
    field_max[0] = 500;
    field_max[1] = 300;
    RegisterTemplateArgs {
        id,
        code_hash: [7; 32],
        kind,
        field_count: 2,
        field_min: [0; PARAM_FIELDS],
        field_max,
        open_authoring: true,
        loot_enabled: false,
        forge_enabled: false,
        max_level: 1,
        loot_royalty_bps: 0,
        max_targets,
        name: "External".to_string(),
    }
}

/// The token-side manifest ceiling of `ext_template` (a Fee item cutting up to `max_cut_bps`).
pub fn fee_manifest(max_cut_bps: u16) -> Manifest {
    Manifest {
        kind: slot_kind::FEE,
        token_flags: bordrless_hook::slot_flags::BEFORE_TRANSFER | bordrless_hook::slot_flags::TRANSFER_RETURNS_DELTA,
        max_cut_transfer_bps: max_cut_bps,
        ..Default::default()
    }
}

/// A Fee slot that may cut up to `max_cut_bps` and write `data_len` bytes.
pub fn fee_slot(max_cut_bps: u16, data_len: u8) -> SlotInit {
    SlotInit {
        kind: slot_kind::FEE,
        equip_rule: equip_rule::VOTE,
        bounds: SlotBounds {
            max_cut_bps,
            may_refuse: true,
            may_write_data: data_len > 0,
            may_answer_touch: false,
            may_burn: false,
        },
        data_len,
        locked_program: None,
        locked_flags: 0,
        locked_extra_count: 0,
    }
}

/// Registers `ext_template` as template [`EXT_ID`] (Fee, cutting up to 500 bps).
pub fn register(hw: &mut Hw) {
    load(hw);
    let admin = hw.admin.insecure_clone();
    hw.register_external(&admin, ext_template::ID, args(EXT_ID, slot_kind::FEE, 1), fee_manifest(500)).ok();
}

/// The whole token-side flow of an external template `template_id` already registered: an item
/// with `params[0] = cut_bps` and a 10% royalty, equipped at launch on a fresh token's Fee slot
/// (cut bound `slot_max_cut_bps`, a 6-byte range), one transfer of 1,000,000 that cuts into the
/// equip vault, then `settle_equip` paying the royalty, the cranker's bounty and the rest to the
/// item's royalty holding (the template names no target). Checks every amount exactly.
pub fn cut_flow(hw: &mut Hw, template_id: u16, cut_bps: u32, slot_max_cut_bps: u16) {
    const SOL: u64 = 1_000_000_000;
    let owner = hw.w.env.funded(100 * SOL);
    let mint = hw.slot_mint(&owner, vec![fee_slot(slot_max_cut_bps, 6)]);
    let author = hw.w.env.funded(10 * SOL);
    let mut p = [0u32; PARAM_FIELDS];
    p[0] = cut_bps;
    let (tx, item, _) = hw.create_item(&author, template_id, p, 1_000);
    tx.ok();
    let it = hw.read_item(&item);
    let t: hookwars_armory::state::Template = hw.w.env.read(&pda::template(template_id).0);
    assert!(t.external);
    assert_eq!(it.manifest, t.ext_manifest);
    hw.equip_launch(&owner, &mint, Hw::entry(0, Some(item), EquipConfig::default())).ok();
    let m: bordrless_token::state::Mint = hw.w.env.read(&mint);
    assert_eq!(m.slots[0].program, t.program);
    assert_eq!(m.slots[0].item, item);
    let alice = hw.w.env.funded(SOL);
    let bob = hw.w.env.funded(SOL);
    hw.mint_to(&owner, &mint, &alice.pubkey(), 10_000_000);
    let amount = 1_000_000u64;
    transfer(hw, &alice, &mint, &bob.pubkey(), amount).ok();
    let cut = amount * u64::from(cut_bps) / 10_000;
    assert_eq!(hw.w.env.holding(&mint, &bob.pubkey()), amount - cut);
    let state = pda::equip_state(&mint, 0).0;
    assert_eq!(hw.w.env.holding(&mint, &state), cut);
    // Nothing recorded: the vault balance is what the external template collected.
    assert_eq!(equip_state(hw, &mint, 0).token_unsettled[0], 0);
    let cranker = hw.w.env.funded(SOL);
    let royalty = pda::royalty_owner(&item).0;
    let ix = settle_ix(hw, &cranker.pubkey(), &mint, 0, &[(pda::holding(&mint, &royalty), ids::ITEMS_ID)]);
    hw.w.env.send_paid_by(&[ix], &cranker, &[]).ok();
    let roy = cut * 1_000 / 10_000;
    let bounty = (cut - roy) * 50 / 10_000;
    assert_eq!(hw.w.env.holding(&mint, &cranker.pubkey()), bounty);
    assert_eq!(hw.w.env.holding(&mint, &royalty), cut - bounty);
    assert_eq!(hw.w.env.holding(&mint, &state), 0);
}
