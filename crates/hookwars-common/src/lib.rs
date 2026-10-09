// Changed by Hookwars: new file (M2), shared by hookwars_armory and hookwars_items.
//! Types and pure rules shared by the armory (docs/spec/02-armory.md) and the items program
//! (docs/spec/04-templates.md): params (R7), the manifest (04 section 2.7), the equip config (04
//! section 2.3), each template's fields and forge rules (04 section 3), seeds (00 section 4.3), the
//! upgrade-authority check (00 rule 3) and the observation reader (03 section 3.1).

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

/// Parameter fields per item (`PARAM_FIELDS`, 00 section 6): the War orders template uses 11.
pub const PARAM_FIELDS: usize = 11;
/// An item's parameters.
pub type Params = [u32; PARAM_FIELDS];

/// Program ids and keys of the Hookwars deployment (00 section 3).
pub mod ids {
    use anchor_lang::prelude::Pubkey;
    /// `hookwars_armory`.
    pub const ARMORY_ID: Pubkey =
        Pubkey::from_str_const("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");
    /// `hookwars_items`.
    pub const ITEMS_ID: Pubkey =
        Pubkey::from_str_const("8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv");
    /// `hookwars_war`.
    pub const WAR_ID: Pubkey =
        Pubkey::from_str_const("5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2");
    /// The token program.
    pub const TOKEN_ID: Pubkey =
        Pubkey::from_str_const("5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618");
    /// The DEX.
    pub const SWAP_ID: Pubkey =
        Pubkey::from_str_const("AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo");
    /// The launchpad.
    pub const LAUNCH_ID: Pubkey =
        Pubkey::from_str_const("fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD");
    /// The kit.
    pub const KIT_ID: Pubkey =
        Pubkey::from_str_const("CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG");
    /// The bridge.
    pub const BRIDGE_ID: Pubkey =
        Pubkey::from_str_const("5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj");
    /// The companion program.
    pub const COMPANION_ID: Pubkey =
        Pubkey::from_str_const("HzeAN8e7HbGx8wzgd5SpduF51c7rXCTqkQKcmw44YHkK");
    /// Bridged SOL, the quote of every launch pool (the bridge's `["wrapped", NATIVE_MINT]`).
    pub const BRIDGED_SOL_MINT: Pubkey =
        Pubkey::from_str_const("7YMXcZ3AUD5pBceT4QzM2hrApoH3rEmPZUAzpKPHVvR4");
    /// `<MANAGED_HOOK_KEY>`.
    pub const MANAGED_HOOK_KEY: Pubkey =
        Pubkey::from_str_const("6ve794V3v88GFaGjRrZ1mH83Z49e6q6NZym4GZJ94mCK");
    /// `<PROTOCOL_AUTHORITY>`.
    pub const PROTOCOL_AUTHORITY: Pubkey =
        Pubkey::from_str_const("CFi9xajnSxM1WMSndVoyQHmfm6DuEdzFodfjRfhTuzxa");
    /// Who may upgrade a template program (00 rule 3; upstream `HOOK_UPGRADE_AUTHORITIES`).
    pub const HOOK_UPGRADE_AUTHORITIES: [Pubkey; 2] = [MANAGED_HOOK_KEY, PROTOCOL_AUTHORITY];
    /// The upgradeable loader.
    pub const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
        Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
    /// The BPF loader 2 (immutable programs).
    pub const BPF_LOADER_2_ID: Pubkey =
        Pubkey::from_str_const("BPFLoader2111111111111111111111111111111111");
    /// Loader v4.
    pub const LOADER_V4_ID: Pubkey =
        Pubkey::from_str_const("LoaderV411111111111111111111111111111111111");
}

/// Seeds (00 section 4.3, 02 section 2).
pub mod seeds {
    /// `ArmoryConfig`.
    pub const CONFIG: &[u8] = b"config";
    /// The armory's signer of every call into items.
    pub const ARMORY: &[u8] = b"armory";
    /// Mint authority of item mints (revoked after the one token).
    pub const MINTER: &[u8] = b"minter";
    /// `["template", id u16 le]`.
    pub const TEMPLATE: &[u8] = b"template";
    /// `["item", item_mint]`.
    pub const ITEM: &[u8] = b"item";
    /// `["item-mint", n u64 le]`.
    pub const ITEM_MINT: &[u8] = b"item-mint";
    /// `["royalty", item]`.
    pub const ROYALTY: &[u8] = b"royalty";
    /// `["slots", mint]`.
    pub const SLOTS: &[u8] = b"slots";
    /// `["slot-state", mint, slot]`.
    pub const SLOT_STATE: &[u8] = b"slot-state";
    /// `["proposal", mint, slot, nonce u64 le]`.
    pub const PROPOSAL: &[u8] = b"proposal";
    /// `["vote", proposal, voter]`.
    pub const VOTE: &[u8] = b"vote";
    /// `["forges", wallet]`.
    pub const FORGES: &[u8] = b"forges";
    /// `["pending-params"]`.
    pub const PENDING: &[u8] = b"pending-params";
    /// The launchpad's caller of `equip_launch`: `["armory-caller", mint]` under the launchpad.
    pub const ARMORY_CALLER: &[u8] = b"armory-caller";
    /// The war program's caller of `mint_loot`: `["loot-signer"]` under the war program.
    pub const LOOT_SIGNER: &[u8] = b"loot-signer";
    /// `["equip", mint, slot]` under items.
    pub const EQUIP: &[u8] = b"equip";
    /// `["raid-ledger", mint]` under items.
    pub const RAID_LEDGER: &[u8] = b"raid-ledger";
    /// `["pool-cuts", mint]` under items.
    pub const POOL_CUTS: &[u8] = b"pool-cuts";
    /// `["launch", mint]` under the launchpad.
    pub const LAUNCH: &[u8] = b"launch";
    /// `["obs", pool]` under the DEX.
    pub const OBS: &[u8] = b"obs";
    /// `["war-config"]` under war.
    pub const WAR_CONFIG: &[u8] = b"war-config";
    /// `["war", mint]` under war.
    pub const WAR_STATE: &[u8] = b"war";
    /// `["war-signer"]` under war.
    pub const WAR_SIGNER: &[u8] = b"war-signer";
}

/// PDAs.
pub mod pda {
    use super::{ids::*, seeds};
    use anchor_lang::prelude::Pubkey;

    /// `["armory"]` under the armory.
    pub fn armory_signer() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::ARMORY], &ARMORY_ID)
    }
    /// `["minter"]` under the armory.
    pub fn minter() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::MINTER], &ARMORY_ID)
    }
    /// `["config"]` under the armory.
    pub fn config() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::CONFIG], &ARMORY_ID)
    }
    /// `["pending-params"]` under the armory.
    pub fn pending() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::PENDING], &ARMORY_ID)
    }
    /// `["template", id]`.
    pub fn template(id: u16) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::TEMPLATE, &id.to_le_bytes()], &ARMORY_ID)
    }
    /// `["item-mint", n]`.
    pub fn item_mint(n: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::ITEM_MINT, &n.to_le_bytes()], &ARMORY_ID)
    }
    /// `["item", item_mint]`.
    pub fn item(item_mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::ITEM, item_mint.as_ref()], &ARMORY_ID)
    }
    /// `["royalty", item]`.
    pub fn royalty_owner(item: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::ROYALTY, item.as_ref()], &ARMORY_ID)
    }
    /// `["slots", mint]`.
    pub fn slot_authority(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::SLOTS, mint.as_ref()], &ARMORY_ID)
    }
    /// `["slot-state", mint, slot]`.
    pub fn slot_state(mint: &Pubkey, slot: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::SLOT_STATE, mint.as_ref(), &[slot]], &ARMORY_ID)
    }
    /// `["proposal", mint, slot, nonce]`.
    pub fn proposal(mint: &Pubkey, slot: u8, nonce: u64) -> (Pubkey, u8) {
        Pubkey::find_program_address(
            &[seeds::PROPOSAL, mint.as_ref(), &[slot], &nonce.to_le_bytes()],
            &ARMORY_ID,
        )
    }
    /// `["vote", proposal, voter]`.
    pub fn vote_lock(proposal: &Pubkey, voter: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::VOTE, proposal.as_ref(), voter.as_ref()], &ARMORY_ID)
    }
    /// `["forges", wallet]`.
    pub fn forge_counter(wallet: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::FORGES, wallet.as_ref()], &ARMORY_ID)
    }
    /// `["armory-caller", mint]` under the launchpad.
    pub fn armory_caller(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::ARMORY_CALLER, mint.as_ref()], &LAUNCH_ID)
    }
    /// `["loot-signer"]` under war.
    pub fn loot_signer() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::LOOT_SIGNER], &WAR_ID)
    }
    /// `["equip", mint, slot]` under items: the `EquipState` and the owner of the equip vault.
    pub fn equip_state(mint: &Pubkey, slot: u8) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::EQUIP, mint.as_ref(), &[slot]], &ITEMS_ID)
    }
    /// `["raid-ledger", mint]` under items.
    pub fn raid_ledger(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::RAID_LEDGER, mint.as_ref()], &ITEMS_ID)
    }
    /// `["pool-cuts", mint]` under items.
    pub fn pool_cuts(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::POOL_CUTS, mint.as_ref()], &ITEMS_ID)
    }
    /// `["launch", mint]` under the launchpad.
    pub fn launch(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::LAUNCH, mint.as_ref()], &LAUNCH_ID)
    }
    /// `["obs", pool]` under the DEX.
    pub fn observations(pool: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::OBS, pool.as_ref()], &SWAP_ID)
    }
    /// `["war-config"]` under war.
    pub fn war_config() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::WAR_CONFIG], &WAR_ID)
    }
    /// `["war", mint]` under war.
    pub fn war_state(mint: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::WAR_STATE, mint.as_ref()], &WAR_ID)
    }
    /// A holding of the token standard: `["holding", mint, owner]` under the token program.
    pub fn holding(mint: &Pubkey, owner: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"holding", mint.as_ref(), owner.as_ref()], &TOKEN_ID).0
    }
}

/// Template ids (04 section 3).
pub mod template_id {
    /// Raid.
    pub const RAID: u16 = 1;
    /// Shield.
    pub const SHIELD: u16 = 2;
    /// Wall.
    pub const WALL: u16 = 3;
    /// Spy.
    pub const SPY: u16 = 4;
    /// Treaty.
    pub const TREATY: u16 = 5;
    /// Tribute.
    pub const TRIBUTE: u16 = 6;
    /// Half-Life.
    pub const HALF_LIFE: u16 = 7;
    /// Transfer Fee.
    pub const TRANSFER_FEE: u16 = 8;
    /// War orders.
    pub const WAR_ORDERS: u16 = 9;
}

/// Slot kinds (00 section 4.1; the same values as `bordrless_hook::slot_kind`).
pub mod kind {
    /// Fee.
    pub const FEE: u8 = 0;
    /// Reward.
    pub const REWARD: u8 = 1;
    /// Defense.
    pub const DEFENSE: u8 = 2;
    /// Relation.
    pub const RELATION: u8 = 3;
    /// Pool.
    pub const POOL: u8 = 4;
    /// Locked (never a template kind).
    pub const LOCKED: u8 = 5;
    /// War.
    pub const WAR: u8 = 6;
}

/// Token flags as the token program reads them (`bordrless_hook::slot_flags`).
pub mod token_flags {
    /// `before_transfer`.
    pub const BEFORE_TRANSFER: u16 = 1;
    /// `before_transfer` may answer a cut.
    pub const TRANSFER_RETURNS_DELTA: u16 = 1 << 6;
    /// Writes the slot's hook-data range.
    pub const WRITES_HOOK_DATA: u16 = 1 << 7;
    /// `on_touch` runs.
    pub const ANSWERS_TOUCH: u16 = 1 << 8;
    /// Mint callbacks (never on an item, R12).
    pub const MINT: u16 = (1 << 2) | (1 << 3);
}

/// Pool flags of an item (04 section 2.7).
pub mod pool_flags {
    /// `pool_before_swap`.
    pub const BEFORE_SWAP: u8 = 1;
    /// `pool_after_swap`.
    pub const AFTER_SWAP: u8 = 1 << 1;
    /// Writes the raid mark.
    pub const MARKS: u8 = 1 << 2;
}

/// What an item may do, computed once from its template and params (04 section 2.7; 16 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    /// Slot kind.
    pub kind: u8,
    /// Token flags (upstream bits plus `ANSWERS_TOUCH`); 0 without token callbacks.
    pub token_flags: u16,
    /// Pool flags; 0 without pool callbacks.
    pub pool_flags: u8,
    /// Worst-case pool cut on a buy.
    pub max_cut_buy_bps: u16,
    /// Worst-case pool cut on a sell.
    pub max_cut_sell_bps: u16,
    /// Worst-case token-side cut on a transfer.
    pub max_cut_transfer_bps: u16,
    /// Largest creator and holder fee discount.
    pub max_discount_bps: u16,
    /// Can fail a transfer.
    pub may_refuse: bool,
    /// Answers a burn.
    pub may_burn: bool,
    /// Range bytes used, without the epoch byte.
    pub data_bytes: u8,
    /// Foreign accounts it reads.
    pub reads_other_pools: u8,
}

/// Serialized size of a [`Manifest`].
pub const MANIFEST_LEN: usize = 16;

impl Manifest {
    /// Whether any token-side or pool-side cut is possible.
    pub fn token_cuts(&self) -> bool {
        self.max_cut_transfer_bps > 0
    }
    /// Whether a pool-side cut (paid in the quote) is possible.
    pub fn pool_cuts(&self) -> bool {
        self.max_cut_buy_bps > 0 || self.max_cut_sell_bps > 0
    }
}

/// Targets and role chosen by the community that equips an item (04 section 2.3).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct EquipConfig {
    /// Targets; their meaning is the template's.
    pub targets: Vec<Pubkey>,
    /// 0 none; Tribute: 1 Pay, 2 Receive.
    pub role: u8,
}

/// How a field forges (04 section 2.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeRule {
    /// Unused field (must be 0).
    Unused,
    /// `max(a, b)` moved toward the ceiling by `FORGE_GAIN_BPS`.
    TowardCeiling,
    /// `min(a, b)` moved toward the floor by `FORGE_GAIN_BPS`.
    TowardFloor,
    /// Both must be equal.
    Keep,
    /// 0 means off: `TowardFloor` when both are on, else both must be equal (Transfer Fee).
    FloorWhenBothOn,
}

/// A template's static shape: kind, used fields, forge rules, which fields accept 0 as "off" below
/// their floor, and whether it can be forged at all.
#[derive(Clone, Copy, Debug)]
pub struct TemplateShape {
    /// Slot kind.
    pub kind: u8,
    /// Fields used.
    pub field_count: u8,
    /// Forge rule per field.
    pub rules: [ForgeRule; PARAM_FIELDS],
    /// Fields where 0 is allowed even when the floor is above 0.
    pub zero_off: [bool; PARAM_FIELDS],
    /// Whether two items can be forged.
    pub forgeable: bool,
}

/// The shape of template `id`, or `None` for an unknown id.
pub fn shape(id: u16) -> Option<TemplateShape> {
    use ForgeRule::*;
    let mut rules = [Unused; PARAM_FIELDS];
    let mut zero_off = [false; PARAM_FIELDS];
    let (kind, used, forgeable): (u8, &[ForgeRule], bool) = match id {
        template_id::RAID => (kind::POOL, &[TowardCeiling, TowardCeiling, TowardCeiling], true),
        template_id::SHIELD => (kind::POOL, &[TowardCeiling, TowardCeiling, Keep], true),
        template_id::WALL => (kind::DEFENSE, &[TowardFloor], true),
        template_id::SPY => (kind::POOL, &[Keep, TowardFloor, TowardFloor, TowardCeiling], true),
        template_id::TREATY => (kind::RELATION, &[Keep, Keep, Keep], false),
        template_id::TRIBUTE => (kind::RELATION, &[Keep], false),
        template_id::HALF_LIFE => (kind::FEE, &[TowardCeiling, TowardCeiling, TowardCeiling], true),
        template_id::TRANSFER_FEE => (kind::FEE, &[TowardCeiling, FloorWhenBothOn], true),
        template_id::WAR_ORDERS => (kind::WAR, &[Keep; 11], false),
        _ => return None,
    };
    rules[..used.len()].copy_from_slice(used);
    if id == template_id::TRANSFER_FEE {
        zero_off[1] = true;
    }
    Some(TemplateShape {
        kind,
        field_count: used.len() as u8,
        rules,
        zero_off,
        forgeable,
    })
}

/// Why params were refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamsError {
    /// Unknown template.
    UnknownTemplate,
    /// A field outside its floor and ceiling, or an unused field not 0.
    OutOfRange,
    /// A template rule beyond floor and ceiling failed.
    BadParams,
    /// The two items cannot be forged together.
    NotForgeable,
}

/// The per-field check (02 section 4.1 step 2): every used field inside `min..=max` (or 0 where the
/// template allows 0 as off), unused fields 0.
pub fn check_fields(id: u16, min: &Params, max: &Params, p: &Params) -> core::result::Result<(), ParamsError> {
    let s = shape(id).ok_or(ParamsError::UnknownTemplate)?;
    for i in 0..PARAM_FIELDS {
        if i >= usize::from(s.field_count) {
            if p[i] != 0 {
                return Err(ParamsError::OutOfRange);
            }
            continue;
        }
        let off = s.zero_off[i] && p[i] == 0;
        if !off && (p[i] < min[i] || p[i] > max[i]) {
            return Err(ParamsError::OutOfRange);
        }
    }
    Ok(())
}

/// Template rules beyond floor and ceiling (04 `validate_params`).
pub fn validate(id: u16, p: &Params) -> core::result::Result<(), ParamsError> {
    shape(id).ok_or(ParamsError::UnknownTemplate)?;
    match id {
        template_id::SPY if !(p[0] == 1 || p[0] == 2) => Err(ParamsError::BadParams),
        template_id::SHIELD if p[2] > 1 => Err(ParamsError::BadParams),
        template_id::TREATY if p[2] > 1 => Err(ParamsError::BadParams),
        template_id::HALF_LIFE if p[1] == 0 || p[2] == 0 => Err(ParamsError::BadParams),
        template_id::WAR_ORDERS if p[4] >= p[5] || p[8] > 1 => Err(ParamsError::BadParams),
        _ => Ok(()),
    }
}

fn bps(v: u32) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

/// The manifest of an item of template `id` with `p` (04 section 3, each template's "Manifest").
/// `max_targets` is the template's target ceiling: what the item may read is known only when it
/// is aimed, so the manifest promises the most (M2 implementation notes).
pub fn manifest(id: u16, p: &Params, max_targets: u8) -> core::result::Result<Manifest, ParamsError> {
    use token_flags::*;
    let s = shape(id).ok_or(ParamsError::UnknownTemplate)?;
    let mut m = Manifest {
        kind: s.kind,
        ..Default::default()
    };
    match id {
        template_id::RAID => {
            m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA | ANSWERS_TOUCH;
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP | pool_flags::MARKS;
            m.max_cut_buy_bps = bps(p[1]);
            m.max_discount_bps = bps(p[0]);
            m.data_bytes = 11;
            m.reads_other_pools = max_targets;
        }
        template_id::SHIELD => {
            m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA;
            m.pool_flags = pool_flags::AFTER_SWAP | pool_flags::MARKS;
            m.max_cut_sell_bps = bps(p[0]);
            m.data_bytes = 6;
            m.reads_other_pools = max_targets;
        }
        template_id::WALL => {
            m.token_flags = BEFORE_TRANSFER;
            m.may_refuse = true;
        }
        template_id::SPY => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            if p[0] == 1 {
                m.max_cut_sell_bps = bps(p[3]);
            } else {
                m.max_discount_bps = bps(p[3]);
            }
            m.reads_other_pools = 1;
        }
        template_id::TREATY => {
            m.pool_flags = pool_flags::BEFORE_SWAP;
            m.max_cut_buy_bps = bps(p[0].max(p[1]));
            m.reads_other_pools = 1;
        }
        template_id::TRIBUTE => {
            m.pool_flags = pool_flags::BEFORE_SWAP;
            m.max_cut_buy_bps = bps(p[0]);
            m.reads_other_pools = 1;
        }
        template_id::HALF_LIFE => {
            m.token_flags = BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA | WRITES_HOOK_DATA;
            m.max_cut_transfer_bps = bps(p[0].div_ceil(100));
            m.data_bytes = 5;
        }
        template_id::TRANSFER_FEE => {
            m.token_flags = BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA;
            m.max_cut_transfer_bps = bps(p[0]);
            m.may_refuse = p[1] > 0;
        }
        _ => {}
    }
    Ok(m)
}

/// `combine(a, b)` per field (04 section 2.7), clamped to `min..=max`. Deterministic and symmetric.
pub fn combine(
    id: u16,
    min: &Params,
    max: &Params,
    gain_bps: u16,
    a: &Params,
    b: &Params,
) -> core::result::Result<Params, ParamsError> {
    let s = shape(id).ok_or(ParamsError::UnknownTemplate)?;
    if !s.forgeable {
        return Err(ParamsError::NotForgeable);
    }
    let gain = u64::from(gain_bps.min(10_000));
    let up = |m: u32, c: u32| -> u32 {
        let d = u64::from(c.saturating_sub(m));
        m.saturating_add((d * gain / 10_000) as u32)
    };
    let down = |m: u32, f: u32| -> u32 {
        let d = u64::from(m.saturating_sub(f));
        m.saturating_sub((d * gain / 10_000) as u32)
    };
    let mut out = [0u32; PARAM_FIELDS];
    for i in 0..PARAM_FIELDS {
        out[i] = match s.rules[i] {
            ForgeRule::Unused => 0,
            ForgeRule::TowardCeiling => up(a[i].max(b[i]), max[i]),
            ForgeRule::TowardFloor => down(a[i].min(b[i]), min[i]),
            ForgeRule::Keep => {
                if a[i] != b[i] {
                    return Err(ParamsError::NotForgeable);
                }
                a[i]
            }
            ForgeRule::FloorWhenBothOn => {
                if a[i] > 0 && b[i] > 0 {
                    down(a[i].min(b[i]), min[i])
                } else if a[i] == b[i] {
                    a[i]
                } else {
                    return Err(ParamsError::NotForgeable);
                }
            }
        };
        if s.rules[i] != ForgeRule::Unused && !(s.zero_off[i] && out[i] == 0) {
            out[i] = out[i].clamp(min[i], max[i]);
        }
    }
    Ok(out)
}

/// Why a template program was refused (00 rule 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthorityError {
    /// Upgradeable by someone not allowed.
    Upgradeable,
    /// The ProgramData account is missing or malformed.
    ProgramDataMissing,
}

/// The ProgramData address of an upgradeable program.
pub fn programdata_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program.as_ref()], &ids::BPF_LOADER_UPGRADEABLE_ID).0
}

/// Upstream `check_hook_authority` (`bordrless_launch/src/instructions/launch_config.rs`), with
/// `HOOK_UPGRADE_AUTHORITIES`. Returns the deploy slot of an upgradeable program (its ProgramData
/// header), `None` for an immutable one.
pub fn check_program_authority(
    program: &AccountInfo,
    programdata: &AccountInfo,
) -> core::result::Result<Option<u64>, AuthorityError> {
    use ids::*;
    let allowed = |key: &[u8]| HOOK_UPGRADE_AUTHORITIES.iter().any(|a| a.as_ref() == key);
    if *program.owner == BPF_LOADER_2_ID {
        return Ok(None);
    }
    if *program.owner == LOADER_V4_ID {
        let data = program
            .try_borrow_data()
            .map_err(|_| AuthorityError::ProgramDataMissing)?;
        if data.len() < 48 {
            return Err(AuthorityError::Upgradeable);
        }
        let status = u64::from_le_bytes(data[40..48].try_into().unwrap());
        if status == 2 {
            return Ok(None);
        }
        if allowed(&data[8..40]) {
            return Ok(Some(u64::from_le_bytes(data[..8].try_into().unwrap())));
        }
        return Err(AuthorityError::Upgradeable);
    }
    if *program.owner != BPF_LOADER_UPGRADEABLE_ID {
        return Err(AuthorityError::Upgradeable);
    }
    if *programdata.key != programdata_address(program.key) {
        return Err(AuthorityError::ProgramDataMissing);
    }
    {
        let data = program
            .try_borrow_data()
            .map_err(|_| AuthorityError::ProgramDataMissing)?;
        if data.len() < 36
            || data[..4] != 2u32.to_le_bytes()
            || data[4..36] != programdata.key.to_bytes()
        {
            return Err(AuthorityError::ProgramDataMissing);
        }
    }
    if *programdata.owner != BPF_LOADER_UPGRADEABLE_ID {
        return Err(AuthorityError::ProgramDataMissing);
    }
    let data = programdata
        .try_borrow_data()
        .map_err(|_| AuthorityError::ProgramDataMissing)?;
    if data.len() < 13 || data[..4] != 3u32.to_le_bytes() {
        return Err(AuthorityError::ProgramDataMissing);
    }
    let slot = u64::from_le_bytes(data[4..12].try_into().unwrap());
    match data[12] {
        0 => Ok(Some(slot)),
        1 if data.len() >= 45 && allowed(&data[13..45]) => Ok(Some(slot)),
        _ => Err(AuthorityError::Upgradeable),
    }
}

/// The deploy slot an upgradeable program's ProgramData records now (02 section 3.3), `None` if
/// it cannot be read.
pub fn current_deploy_slot(program: &AccountInfo, programdata: &AccountInfo) -> Option<u64> {
    if *program.owner == ids::LOADER_V4_ID {
        let data = program.try_borrow_data().ok()?;
        return (data.len() >= 8).then(|| u64::from_le_bytes(data[..8].try_into().unwrap()));
    }
    if *programdata.key != programdata_address(program.key) {
        return None;
    }
    let data = programdata.try_borrow_data().ok()?;
    (data.len() >= 12 && data[..4] == 3u32.to_le_bytes())
        .then(|| u64::from_le_bytes(data[4..12].try_into().unwrap()))
}

/// The `Launch` discriminator (upstream SDK, `docs/integration/01-the-standard.md`).
pub const LAUNCH_DISCRIMINATOR: [u8; 8] = [0x90, 0x33, 0x33, 0xa3, 0xce, 0x55, 0xd5, 0x26];

/// The pool a `Launch` account names (owner, discriminator and mint checked by the caller's
/// address check; offsets per upstream: mint at 10, pool at 74).
pub fn launch_pool(data: &[u8], mint: &Pubkey) -> Option<Pubkey> {
    if data.len() < 106 || data[..8] != LAUNCH_DISCRIMINATOR || data[10..42] != mint.to_bytes() {
        return None;
    }
    Some(Pubkey::new_from_array(data[74..106].try_into().unwrap()))
}

/// Byte offsets of an `Observations` account (03 section 3.1): an 8-byte discriminator, the
/// Borsh header (62 bytes), then 48-byte entries.
pub mod obs_layout {
    /// `last_price_q64: u128`.
    pub const LAST_PRICE: usize = 42;
    /// `last_ts: i64`.
    pub const LAST_TS: usize = 58;
    /// `index: u16`.
    pub const INDEX: usize = 66;
    /// `filled: u16`.
    pub const FILLED: usize = 68;
    /// First entry.
    pub const ENTRIES: usize = 70;
    /// Entry size.
    pub const ENTRY: usize = 48;
}

/// One observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {
    /// Time.
    pub ts: i64,
    /// Price accumulator (wrapping).
    pub price_cumulative: u128,
    /// The pool's `quote_volume` then.
    pub quote_volume: u128,
    /// The pool's `swap_count` then.
    pub swap_count: u64,
}

/// What a window read gives (03 section 3.1 `window_read`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowRead {
    /// Time-weighted price over the window, Q64.64.
    pub twap_q64: u128,
    /// Quote volume over the window.
    pub quote_volume: u128,
    /// Swaps over the window.
    pub swaps: u64,
    /// Seconds actually covered (from the entry used to now).
    pub seconds: i64,
}

fn entry(data: &[u8], i: usize) -> Option<Observation> {
    let o = obs_layout::ENTRIES + i * obs_layout::ENTRY;
    let e = data.get(o..o + obs_layout::ENTRY)?;
    Some(Observation {
        ts: i64::from_le_bytes(e[0..8].try_into().unwrap()),
        price_cumulative: u128::from_le_bytes(e[8..24].try_into().unwrap()),
        quote_volume: u128::from_le_bytes(e[24..40].try_into().unwrap()),
        swap_count: u64::from_le_bytes(e[40..48].try_into().unwrap()),
    })
}

/// `window_read` over the raw `Observations` bytes: the newest entry at or before `now - window`,
/// or `None` (no signal) when the ring is too short. `pool_quote_volume` and `pool_swap_count` are
/// the pool's current counters.
pub fn window_read(
    data: &[u8],
    pool_quote_volume: u128,
    pool_swap_count: u64,
    now: i64,
    window: i64,
) -> Option<WindowRead> {
    if window <= 0 || data.len() < obs_layout::ENTRIES {
        return None;
    }
    let ring = (data.len() - obs_layout::ENTRIES) / obs_layout::ENTRY;
    let filled = usize::from(u16::from_le_bytes(
        data[obs_layout::FILLED..obs_layout::FILLED + 2].try_into().ok()?,
    ))
    .min(ring);
    if filled == 0 {
        return None;
    }
    let index = usize::from(u16::from_le_bytes(
        data[obs_layout::INDEX..obs_layout::INDEX + 2].try_into().ok()?,
    )) % ring.max(1);
    let last_price = u128::from_le_bytes(
        data[obs_layout::LAST_PRICE..obs_layout::LAST_PRICE + 16]
            .try_into()
            .ok()?,
    );
    let last_ts = i64::from_le_bytes(data[obs_layout::LAST_TS..obs_layout::LAST_TS + 8].try_into().ok()?);
    let newest = entry(data, (index + ring - 1) % ring)?;
    let dt_tail = u128::try_from(now.saturating_sub(last_ts).max(0)).ok()?;
    let cum_now = newest
        .price_cumulative
        .wrapping_add(last_price.wrapping_mul(dt_tail));
    let target = now.checked_sub(window)?;
    let mut best: Option<Observation> = None;
    for i in 0..filled {
        let e = entry(data, i)?;
        if e.ts <= target && best.is_none_or(|b| e.ts > b.ts) {
            best = Some(e);
        }
    }
    let e = best?;
    let secs = now - e.ts;
    if secs <= 0 {
        return None;
    }
    Some(WindowRead {
        twap_q64: cum_now.wrapping_sub(e.price_cumulative) / secs as u128,
        quote_volume: pool_quote_volume.saturating_sub(e.quote_volume),
        swaps: pool_swap_count.saturating_sub(e.swap_count),
        seconds: secs,
    })
}

/// The `Performance` rule (02 section 6.7, 16 bytes).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PerformanceRule {
    /// `QuoteVolume` 0, `Twap` 1, `SwapCount` 2.
    pub metric: u8,
    /// The window measured.
    pub window_secs: u32,
    /// The longer window it is compared with.
    pub base_window_secs: u32,
    /// `Below` 0, `Above` 1.
    pub op: u8,
    /// `rate(window) op rate(base_window) * ratio_bps / 10_000`.
    pub ratio_bps: u16,
    /// How long it must hold.
    pub hold_secs: u32,
}

impl PerformanceRule {
    /// Whether the condition holds, given the two window reads (rates per second, cross-multiplied
    /// so no division loses precision). `None` (no signal) is false.
    pub fn holds(&self, short: Option<WindowRead>, base: Option<WindowRead>) -> bool {
        let (Some(s), Some(b)) = (short, base) else {
            return false;
        };
        let (lhs, rhs) = match self.metric {
            0 => (
                s.quote_volume.saturating_mul(b.seconds as u128).saturating_mul(10_000),
                b.quote_volume
                    .saturating_mul(s.seconds as u128)
                    .saturating_mul(u128::from(self.ratio_bps)),
            ),
            1 => (
                s.twap_q64.saturating_mul(10_000),
                b.twap_q64.saturating_mul(u128::from(self.ratio_bps)),
            ),
            2 => (
                u128::from(s.swaps)
                    .saturating_mul(b.seconds as u128)
                    .saturating_mul(10_000),
                u128::from(b.swaps)
                    .saturating_mul(s.seconds as u128)
                    .saturating_mul(u128::from(self.ratio_bps)),
            ),
            _ => return false,
        };
        if self.op == 0 {
            lhs < rhs
        } else {
            lhs > rhs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(v: &[u32]) -> Params {
        let mut a = [0; PARAM_FIELDS];
        a[..v.len()].copy_from_slice(v);
        a
    }

    #[test]
    fn manifest_is_sixteen_bytes() {
        let m = Manifest::default();
        let mut v = Vec::new();
        m.serialize(&mut v).unwrap();
        assert_eq!(v.len(), MANIFEST_LEN);
    }

    #[test]
    fn forge_rules_are_symmetric_and_capped() {
        let min = p(&[0, 0, 0]);
        let max = p(&[1000, 500, 100]);
        let a = p(&[100, 400, 10]);
        let b = p(&[300, 200, 90]);
        let ab = combine(template_id::RAID, &min, &max, 5_000, &a, &b).unwrap();
        let ba = combine(template_id::RAID, &min, &max, 5_000, &b, &a).unwrap();
        assert_eq!(ab, ba);
        assert_eq!(ab[..3], [650, 450, 95]);
        let full = combine(template_id::RAID, &min, &max, 10_000, &a, &b).unwrap();
        assert_eq!(full[..3], [1000, 500, 100]);
    }

    #[test]
    fn keep_and_unforgeable() {
        let min = p(&[1, 60, 1, 0]);
        let max = p(&[2, 600, 100, 50]);
        assert_eq!(
            combine(template_id::SPY, &min, &max, 1, &p(&[1, 60, 1, 0]), &p(&[2, 60, 1, 0])),
            Err(ParamsError::NotForgeable)
        );
        assert_eq!(
            combine(template_id::TREATY, &min, &max, 1, &p(&[1]), &p(&[1])),
            Err(ParamsError::NotForgeable)
        );
    }

    #[test]
    fn zero_is_off_for_the_transfer_fee_wallet_cap() {
        let min = p(&[0, 100]);
        let max = p(&[300, 10_000]);
        assert!(check_fields(template_id::TRANSFER_FEE, &min, &max, &p(&[10, 0])).is_ok());
        assert!(check_fields(template_id::TRANSFER_FEE, &min, &max, &p(&[10, 50])).is_err());
        assert!(check_fields(template_id::TRANSFER_FEE, &min, &max, &p(&[10, 100, 1])).is_err());
    }
}
