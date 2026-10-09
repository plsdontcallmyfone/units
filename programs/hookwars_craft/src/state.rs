// Changed by Hookwars: new file (hook economy, docs/spec/11-hook-economy.md section 5).
use anchor_lang::prelude::*;
use hookwars_common::PARAM_FIELDS;

pub const VERSION: u8 = 1;
/// Most caller programs the config names (layout constant).
pub const CALLERS_CAP: usize = 8;
/// Most inputs a recipe holds (layout constant; `RECIPE_MAX_INPUTS` is at most this).
pub const INPUTS_CAP: usize = 6;
/// Longest material name and symbol.
pub const NAME_MAX_LEN: usize = 32;
pub const SYMBOL_MAX_LEN: usize = 10;

pub mod seeds {
    pub const CONFIG: &[u8] = b"craft-config";
    pub const PENDING: &[u8] = b"craft-pending";
    pub const MATERIAL: &[u8] = b"material";
    pub const MATERIAL_MINT: &[u8] = b"material-mint";
    pub const MINTER: &[u8] = b"craft-minter";
    pub const SIGNER: &[u8] = b"craft-signer";
    pub const DROP: &[u8] = b"drop";
    pub const RECIPE: &[u8] = b"recipe";
    pub const WEAR: &[u8] = b"wear";
}

/// Drop sources (11 section 5.2).
pub mod source {
    pub const SETTLE_CRANK: u8 = 0;
    pub const RAID_REVEAL: u8 = 1;
    pub const SEASON_FINISH: u8 = 2;
    pub const QUEST_CLAIM: u8 = 3;
    pub const COUNT: u8 = 4;
}

/// Recipe kinds (11 section 5.3).
pub mod recipe_kind {
    pub const CRAFT: u8 = 0;
    pub const REPAIR: u8 = 1;
}

/// The craft parameters (11 section 11; every one "to set").
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CraftParams {
    /// `RECIPE_PROTOCOL_BPS`: the protocol's share of a recipe fee; the rest goes to the season pool.
    pub recipe_protocol_bps: u16,
    /// `RECIPE_MAX_INPUTS` (at most `INPUTS_CAP`).
    pub max_inputs: u8,
    /// Length of an emission season (caps reset each season).
    pub season_secs: u32,
    /// `ADMIN_TIMELOCK_SECS` (00): delay on every params, drop rule, cap and recipe change.
    pub admin_timelock_secs: u32,
}

/// `CraftConfig` at `["craft-config"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct CraftConfig {
    pub version: u8,
    pub bump: u8,
    pub admin: Pubkey,
    /// Receives `RECIPE_PROTOCOL_BPS` of every recipe fee (11 section 2.4).
    pub treasury: Pubkey,
    /// Receives the rest of every recipe fee (the season prize pool, 05 section 14).
    pub season_pool: Pubkey,
    /// The program whose `mint_crafted` makes a crafted item (the armory; integration request).
    pub output_program: Pubkey,
    /// Programs whose `["craft-caller"]` PDA may call `drop`, `init_wear` and `wear` (R42).
    #[max_len(CALLERS_CAP)]
    pub callers: Vec<Pubkey>,
    pub params: CraftParams,
    /// Start of season 0.
    pub season_origin: i64,
    /// Every lamport of recipe fees the protocol has received (11 section 2.4).
    pub protocol_fees_total: u128,
    pub reserved: [u8; 32],
}

impl CraftConfig {
    /// The emission season at `now`.
    pub fn season_at(&self, now: i64) -> u64 {
        let secs = i64::from(self.params.season_secs.max(1));
        u64::try_from(now.saturating_sub(self.season_origin).max(0) / secs).unwrap_or(0)
    }
}

/// `PendingCraft` at `["craft-pending"]`: a config change waiting for its delay.
#[account]
#[derive(InitSpace, Debug)]
pub struct PendingCraft {
    pub bump: u8,
    pub treasury: Pubkey,
    pub season_pool: Pubkey,
    pub output_program: Pubkey,
    #[max_len(CALLERS_CAP)]
    pub callers: Vec<Pubkey>,
    pub params: CraftParams,
    pub ready_at: i64,
    pub active: bool,
}

/// `Material` at `["material", id]` (11 section 5.2). Its mint is `["material-mint", id]`, a plain
/// units mint with no hook whose mint authority is `["craft-minter"]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Material {
    pub version: u8,
    pub bump: u8,
    pub id: u16,
    pub mint: Pubkey,
    #[max_len(NAME_MAX_LEN)]
    pub name: String,
    pub emission_cap_per_season: u64,
    pub emitted_this_season: u64,
    pub season: u64,
    pub emitted_total: u64,
    pub burned_total: u64,
    /// A cap change waiting for its delay (0 = none).
    pub pending_cap: u64,
    pub pending_cap_at: i64,
}

/// `DropRule` at `["drop", source]` (11 section 5.2). The amount dropped is
/// `measured * per_unit_num / per_unit_den`, capped by the material's season cap.
#[account]
#[derive(InitSpace, Debug)]
pub struct DropRule {
    pub version: u8,
    pub bump: u8,
    pub source: u8,
    pub material_id: u16,
    pub per_unit_num: u64,
    pub per_unit_den: u64,
    /// The rule drops nothing before this time.
    pub effective_at: i64,
    pub pending: Option<DropTerms>,
    pub pending_at: i64,
}

/// The settable part of a drop rule.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DropTerms {
    pub material_id: u16,
    pub per_unit_num: u64,
    pub per_unit_den: u64,
}

/// One recipe input.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecipeInput {
    pub material_id: u16,
    pub amount: u64,
}

/// The settable part of a recipe (11 section 5.3).
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Debug, Default, PartialEq, Eq)]
pub struct RecipeTerms {
    pub kind: u8,
    #[max_len(INPUTS_CAP)]
    pub inputs: Vec<RecipeInput>,
    /// `RECIPE_FEE_LAMPORTS` of this recipe.
    pub fee_lamports: u64,
    /// `Craft`: the template made. `Repair`: the template repaired.
    pub template_id: u16,
    /// `Craft`: parameter ranges the output program draws from (equal bounds fix a field).
    pub param_min: [u32; PARAM_FIELDS],
    pub param_max: [u32; PARAM_FIELDS],
    /// `Repair`: charges restored.
    pub charges_restored: u32,
    /// The `Crafter` level required (0 = none).
    pub min_level: u8,
    pub active: bool,
}

/// `Recipe` at `["recipe", id]`.
#[account]
#[derive(InitSpace, Debug)]
pub struct Recipe {
    pub version: u8,
    pub bump: u8,
    pub id: u16,
    pub terms: RecipeTerms,
    /// Usable from this time (a new recipe waits for the timelock too).
    pub effective_at: i64,
    pub pending: Option<RecipeTerms>,
    pub pending_at: i64,
    pub uses: u64,
}

/// `Wear` at `["wear", item]` (11 section 5.4): the item's charges, kept here rather than in the
/// armory `Item` (the items engine reads it, integration request).
#[account]
#[derive(InitSpace, Debug)]
pub struct Wear {
    pub version: u8,
    pub bump: u8,
    pub item: Pubkey,
    /// 0 = never wears.
    pub max_charges: u32,
    pub used: u32,
    pub dormant: bool,
    pub repairs: u32,
    pub updated_at: i64,
}

impl Wear {
    /// Charges left (`u32::MAX` when the item never wears).
    pub fn left(&self) -> u32 {
        if self.max_charges == 0 {
            return u32::MAX;
        }
        self.max_charges.saturating_sub(self.used)
    }
}

pub fn config_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::CONFIG], &crate::ID)
}
pub fn pending_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::PENDING], &crate::ID)
}
pub fn material_address(id: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::MATERIAL, &id.to_le_bytes()], &crate::ID)
}
pub fn material_mint_address(id: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::MATERIAL_MINT, &id.to_le_bytes()], &crate::ID)
}
pub fn minter_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::MINTER], &crate::ID)
}
pub fn signer_address() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::SIGNER], &crate::ID)
}
pub fn drop_address(source: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::DROP, &[source]], &crate::ID)
}
pub fn recipe_address(id: u16) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::RECIPE, &id.to_le_bytes()], &crate::ID)
}
pub fn wear_address(item: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seeds::WEAR, item.as_ref()], &crate::ID)
}

/// Reads an item's wear for the items engine (integration request): `(max_charges, used,
/// dormant)`, or `None` when `info` is not this item's `Wear` (an item without one never wears).
pub fn read_wear(info: &AccountInfo, item: &Pubkey) -> Result<Option<Wear>> {
    if *info.owner != crate::ID || *info.key != wear_address(item).0 || info.data_is_empty() {
        return Ok(None);
    }
    let data = info.try_borrow_data()?;
    Ok(Some(Wear::try_deserialize(&mut &data[..])?))
}
