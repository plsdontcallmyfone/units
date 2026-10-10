// Changed by Hookwars: new file (M2), shared by hookwars_armory and hookwars_items; M3b: raid ledger,
// war touches, launch reads, arsenal wave A templates, composites; expansion templates 43 to 45 (10); agents (09): AGENTS_ID, AGENTS_SIGNER, template 42 Soulbound; arsenal waves D and E. Integration pass 2: agents_record.
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
    /// `hookwars_agents` (09).
    pub const AGENTS_ID: Pubkey =
        Pubkey::from_str_const("GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9");
    /// `["agents-signer"]` under `hookwars_agents`: the mint, freeze and metadata authority of
    /// every agent badge (09 section 4.2).
    pub const AGENTS_SIGNER: Pubkey =
        Pubkey::from_str_const("79bZmFwi3tKQxa6dWnaigz29sVKCpPQ8fPufijxBiKdo");
    /// `hookwars_market` (10).
    pub const MARKET_ID: Pubkey =
        Pubkey::from_str_const("FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv");
    /// `hookwars_social` (10).
    pub const SOCIAL_ID: Pubkey =
        Pubkey::from_str_const("CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx");
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
    /// Integration pass 2 (10 section 17 I-5): items a wallet authored.
    pub const AUTHORED: &[u8] = b"authored";
    /// Integration pass 2 (10 section 17 I-5): bridged-SOL royalties a wallet claimed.
    pub const CLAIMED: &[u8] = b"claimed";
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
    /// `["authored", wallet]` (integration pass 2, I-5).
    pub fn author_counter(wallet: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::AUTHORED, wallet.as_ref()], &ARMORY_ID)
    }
    /// `["claimed", wallet]` (integration pass 2, I-5).
    pub fn claim_counter(wallet: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seeds::CLAIMED, wallet.as_ref()], &ARMORY_ID)
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
    /// Size Tiers (08 4.1).
    pub const SIZE_TIERS: u16 = 10;
    /// Side Skew (08 4.1).
    pub const SIDE_SKEW: u16 = 11;
    /// Launch Decay (08 4.1).
    pub const LAUNCH_DECAY: u16 = 12;
    /// Max Transaction (08 4.2).
    pub const MAX_TRANSACTION: u16 = 19;
    /// Dust Guard (08 4.2).
    pub const DUST_GUARD: u16 = 22;
    /// Sell Burn (08 4.6).
    pub const SELL_BURN: u16 = 32;
    // Hookwars arsenal waves B and C (08 5.1).
    /// Velocity Fee (08 4.1).
    pub const VELOCITY_FEE: u16 = 13;
    /// Impact Fee (08 4.1).
    pub const IMPACT_FEE: u16 = 14;
    /// Volatility Fee (08 4.1).
    pub const VOLATILITY_FEE: u16 = 15;
    /// Rush Hour (08 4.1).
    pub const RUSH_HOUR: u16 = 16;
    /// Cooldown (08 4.2).
    pub const COOLDOWN: u16 = 17;
    /// Daily Sell Cap (08 4.2).
    pub const DAILY_SELL_CAP: u16 = 18;
    /// Flash Guard (08 4.2).
    pub const FLASH_GUARD: u16 = 20;
    /// Dump Brake (08 4.2).
    pub const DUMP_BRAKE: u16 = 21;
    /// Streak (08 4.3).
    pub const STREAK: u16 = 26;
    /// Rank Badge (08 4.7).
    pub const RANK_BADGE: u16 = 35;
    /// Guild Tag (08 4.7).
    pub const GUILD_TAG: u16 = 39;
    /// Composite (08 2).
    pub const COMPOSITE: u16 = 41;
    /// Soulbound (09 section 4.3): the agent badge's Defense item.
    pub const SOULBOUND: u16 = 42;
    /// Coalition (10 section 8): a Relation config item naming a coalition id; no callbacks.
    pub const COALITION: u16 = 43;
    /// Boss (10 section 11.1): the boss token's Pool item marking inbound raid volume per source.
    pub const BOSS: u16 = 44;
    /// Rivalry (10 section 11.3): a Relation config item naming a rival; no callbacks.
    pub const RIVALRY: u16 = 45;
}

/// Whether template `id` may be a module of a composite (08 2.7): every known template but War
/// orders and Composite itself.
pub fn composable(id: u16) -> bool {
    // Hookwars expansion: Coalition, Boss and Rivalry are standalone config items (10 sections 8,
    // 11.1, 11.3); a composite never carries them.
    !matches!(
        id,
        template_id::WAR_ORDERS
            | template_id::COMPOSITE
            | template_id::COALITION
            | template_id::BOSS
            | template_id::RIVALRY
            | template_id::SOULBOUND
    ) && shape(id).is_some()
}

/// Most modules a composite holds (`MAX_MODULES`, 08 2.2; provisional, to measure).
pub const MAX_MODULES: usize = 4;

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
        template_id::SIZE_TIERS => (
            kind::POOL,
            &[Keep, Keep, TowardCeiling, TowardCeiling, TowardCeiling],
            true,
        ),
        template_id::SIDE_SKEW => (kind::POOL, &[TowardCeiling, TowardCeiling], true),
        template_id::LAUNCH_DECAY => (kind::POOL, &[TowardCeiling, Keep, TowardCeiling], true),
        template_id::MAX_TRANSACTION => (kind::DEFENSE, &[TowardFloor], true),
        template_id::DUST_GUARD => (kind::DEFENSE, &[Keep], true),
        template_id::SELL_BURN => (kind::POOL, &[TowardCeiling], true),
        template_id::COMPOSITE => (kind::POOL, &[Keep, Keep], false),
        // Hookwars expansion (10): config items, never forged.
        template_id::COALITION => (kind::RELATION, &[Keep, Keep], false),
        template_id::BOSS => (kind::POOL, &[Keep], false),
        template_id::RIVALRY => (kind::RELATION, &[Keep, Keep, Keep], false),
        template_id::SOULBOUND => (kind::DEFENSE, &[], false),
        // Hookwars arsenal waves B and C (08 4.1, 4.2, 4.3, 4.7).
        template_id::VELOCITY_FEE => (kind::POOL, &[Keep, TowardCeiling, TowardCeiling, TowardCeiling], true),
        template_id::IMPACT_FEE => (kind::POOL, &[TowardCeiling, TowardCeiling], true),
        template_id::VOLATILITY_FEE => (kind::POOL, &[Keep, Keep, TowardFloor, TowardCeiling], true),
        template_id::RUSH_HOUR => (kind::POOL, &[Keep, Keep, TowardCeiling, TowardCeiling], true),
        template_id::COOLDOWN => (kind::DEFENSE, &[TowardCeiling], true),
        template_id::DAILY_SELL_CAP => (kind::DEFENSE, &[TowardFloor], true),
        template_id::FLASH_GUARD => (kind::DEFENSE, &[TowardCeiling], true),
        template_id::DUMP_BRAKE => (kind::POOL, &[Keep, Keep, TowardFloor, TowardCeiling], true),
        template_id::STREAK => (kind::REWARD, &[TowardCeiling], true),
        template_id::RANK_BADGE => (kind::REWARD, &[TowardFloor, Keep, Keep], true),
        template_id::GUILD_TAG => (kind::REWARD, &[Keep], true),
        // Arsenal waves D and E (08 section 4): shapes in `arsenal2`.
        id if arsenal2::is(id) => arsenal2::shape_of(id),
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
        template_id::SIZE_TIERS if p[0] >= p[1] => Err(ParamsError::BadParams),
        template_id::LAUNCH_DECAY if p[1] > p[0] || p[2] == 0 => Err(ParamsError::BadParams),
        template_id::MAX_TRANSACTION if p[0] == 0 => Err(ParamsError::BadParams),
        template_id::COMPOSITE if p[0] == 0 || p[0] as usize > MAX_MODULES => {
            Err(ParamsError::BadParams)
        }
        // Coalition: [coalition id, max contribution bps of the member's chest].
        template_id::COALITION if p[0] == 0 || p[1] == 0 || p[1] > 10_000 => Err(ParamsError::BadParams),
        // Boss: [counting window seconds].
        template_id::BOSS if p[0] == 0 => Err(ParamsError::BadParams),
        // Rivalry: [start unix seconds, duration seconds, war budget bps of the chest].
        template_id::RIVALRY if p[1] == 0 || p[2] > 10_000 => Err(ParamsError::BadParams),
        // Hookwars arsenal waves B and C.
        template_id::VELOCITY_FEE if p[0] == 0 || p[2] == 0 => Err(ParamsError::BadParams),
        template_id::IMPACT_FEE if p[0] == 0 => Err(ParamsError::BadParams),
        template_id::VOLATILITY_FEE | template_id::DUMP_BRAKE if p[0] == 0 || p[0] >= p[1] || p[2] > 10_000 => {
            Err(ParamsError::BadParams)
        }
        template_id::RUSH_HOUR if p[0] > 23 || p[1] == 0 || p[1] > 24 => Err(ParamsError::BadParams),
        template_id::COOLDOWN | template_id::FLASH_GUARD | template_id::STREAK if p[0] == 0 => {
            Err(ParamsError::BadParams)
        }
        template_id::DAILY_SELL_CAP if p[0] == 0 || p[0] > 10_000 => Err(ParamsError::BadParams),
        template_id::RANK_BADGE if p[0] == 0 || p[1] == 0 || p[2] > u32::from(u8::MAX) => {
            Err(ParamsError::BadParams)
        }
        // Arsenal waves D and E (08 section 4).
        id if arsenal2::is(id) => arsenal2::validate(id, p),
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
        template_id::SIZE_TIERS => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            let worst = bps(p[2].max(p[3]).max(p[4]));
            m.max_cut_buy_bps = worst;
            m.max_cut_sell_bps = worst;
        }
        template_id::SIDE_SKEW => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            m.max_cut_buy_bps = bps(p[0]);
            m.max_cut_sell_bps = bps(p[1]);
        }
        template_id::LAUNCH_DECAY => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            m.max_cut_buy_bps = bps(p[0]);
            m.max_cut_sell_bps = bps(p[0]);
        }
        template_id::MAX_TRANSACTION | template_id::DUST_GUARD => {
            m.token_flags = BEFORE_TRANSFER;
            m.may_refuse = true;
        }
        template_id::SELL_BURN => {
            m.pool_flags = pool_flags::BEFORE_SWAP;
            m.may_burn = true;
            // Changed by Hookwars (security review 2 L-E): the burn rate counts as the sell-side
            // worst case, so the armory's `max_pool_item_cut_bps` caps it at every equip.
            m.max_cut_sell_bps = bps(p[0]);
        }
        // Hookwars arsenal waves B and C (08 4.1, 4.2, 4.3, 4.7). Quote-side pool cuts run on a buy's
        // before and a sell's after callback, as wave A's.
        template_id::VELOCITY_FEE => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            m.max_cut_buy_bps = bps(p[3]);
            m.max_cut_sell_bps = bps(p[3]);
        }
        template_id::IMPACT_FEE => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            m.max_cut_buy_bps = bps(p[1]);
            m.max_cut_sell_bps = bps(p[1]);
        }
        template_id::VOLATILITY_FEE => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            m.max_cut_buy_bps = bps(p[3]);
            m.max_cut_sell_bps = bps(p[3]);
        }
        template_id::RUSH_HOUR => {
            m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
            let worst = bps(p[2].max(p[3]));
            m.max_cut_buy_bps = worst;
            m.max_cut_sell_bps = worst;
        }
        template_id::DUMP_BRAKE => {
            m.pool_flags = pool_flags::AFTER_SWAP;
            m.max_cut_sell_bps = bps(p[3]);
        }
        template_id::COOLDOWN | template_id::FLASH_GUARD => {
            m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA;
            m.may_refuse = true;
            m.data_bytes = 5;
        }
        template_id::DAILY_SELL_CAP => {
            m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA;
            m.may_refuse = true;
            m.data_bytes = 11;
        }
        template_id::STREAK | template_id::RANK_BADGE => {
            m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA;
            m.data_bytes = 6;
        }
        template_id::GUILD_TAG => {
            m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA | ANSWERS_TOUCH;
            m.data_bytes = 3;
        }
        // Boss marks inbound raid volume; it never cuts, discounts or burns.
        template_id::BOSS => {
            m.pool_flags = pool_flags::AFTER_SWAP | pool_flags::MARKS;
        }
        template_id::SOULBOUND => {
            m.token_flags = BEFORE_TRANSFER;
            m.may_refuse = true;
        }
        // Arsenal waves D and E (08 section 4).
        id if arsenal2::is(id) => arsenal2::manifest(id, p, max_targets, &mut m),
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

/// What a window read gives (03 section 3.1 `window_read`; built by `bordrless_swap::obs::window_read`
/// on the pool account since M3a).
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

/// `Launch.created_at` (upstream layout: fixed fields only before it).
pub const LAUNCH_CREATED_AT_OFFSET: usize = 289;

/// The creation time a `Launch` account records, with the same checks as [`launch_pool`].
pub fn launch_created_at(data: &[u8], mint: &Pubkey) -> Option<i64> {
    launch_pool(data, mint)?;
    let o = LAUNCH_CREATED_AT_OFFSET;
    Some(i64::from_le_bytes(data.get(o..o + 8)?.try_into().ok()?))
}

/// Raid state shared by the items program (which writes it) and the war program (which reads it):
/// `RaidLedger` at `["raid-ledger", mint]` under items (04 section 2.9).
pub mod raid {
    use anchor_lang::prelude::*;

    /// Windows a ledger keeps (`RAID_TABLE_LEN`, 04 section 2.9; layout constant).
    pub const RAID_TABLE_LEN: usize = 8;
    /// Layout version.
    pub const VERSION: u8 = 1;
    /// `sha256("account:RaidLedger")[..8]`.
    pub const DISCRIMINATOR: [u8; 8] = [0x6e, 0x7e, 0x93, 0x3e, 0xca, 0x11, 0x7f, 0xcb];

    /// One rival's inbound raid volume (04 section 2.9).
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct RaidWindow {
        /// The rival whose holders raided us.
        pub rival_mint: Pubkey,
        /// Start of the current window.
        pub window_start: i64,
        /// Volume in the current window.
        pub volume: u64,
        /// Volume in the previous window.
        pub prev_volume: u64,
    }

    /// The raid mark (R4).
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Mark {
        /// `Clock::slot` when written; 0 none.
        pub clock_slot: u64,
        /// Who receives the raid buy.
        pub recipient: Pubkey,
        /// The rival sold.
        pub rival: Pubkey,
        /// Quote of the raid buy.
        pub quote_volume: u64,
        /// Bit i set once slot i's token half stamped it.
        pub stamped_slots: u8,
    }

    /// `RaidLedger` (04 section 2.9).
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
    pub struct RaidLedger {
        /// Layout version.
        pub version: u8,
        /// Bump.
        pub bump: u8,
        /// The token.
        pub mint: Pubkey,
        /// `WarConfig.current_season` when last rolled.
        pub season_id: u32,
        /// Raid volume our raids brought into us this season.
        pub outbound_volume_season: u64,
        /// Per rival.
        pub inbound: [RaidWindow; RAID_TABLE_LEN],
        /// The last raid buy.
        pub mark: Mark,
        /// Hookwars M3b: the Shield sell mark (who sells, in which clock slot, with which origin):
        /// the token half writes it on the seller's transfer into the pool, `pool_after_swap`
        /// reads it (04 M3b notes).
        pub sell_mark: SellMark,
        /// Reserved.
        pub reserved: [u8; 32],
    }

    /// A Shield sell mark (M3b): written by Shield's token half on a sell's input transfer.
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct SellMark {
        /// `Clock::slot`; 0 none.
        pub clock_slot: u64,
        /// The seller.
        pub seller: Pubkey,
        /// Bit i set when slot i's Shield found the seller marked.
        pub marked_slots: u8,
    }

    impl RaidLedger {
        /// Account size (discriminator included).
        pub const LEN: usize = 8 + 1 + 1 + 32 + 4 + 8 + RAID_TABLE_LEN * 56 + (8 + 32 + 32 + 8 + 1) + (8 + 32 + 1) + 32;

        /// Decodes an account's data (discriminator checked).
        pub fn decode(data: &[u8]) -> Option<Self> {
            if data.len() < 8 || data[..8] != DISCRIMINATOR {
                return None;
            }
            Self::deserialize(&mut &data[8..]).ok()
        }

        /// Encodes into `data` (discriminator first).
        pub fn encode(&self, data: &mut [u8]) -> Option<()> {
            let mut v = DISCRIMINATOR.to_vec();
            self.serialize(&mut v).ok()?;
            data.get_mut(..v.len())?.copy_from_slice(&v);
            Some(())
        }

        /// Rolling inbound raid volume from `rival` at `now` over windows of `window` seconds.
        pub fn rolling(&self, rival: &Pubkey, now: i64, window: i64) -> u64 {
            self.inbound
                .iter()
                .find(|e| e.rival_mint == *rival)
                .map(|e| rolling_volume(e, now, window))
                .unwrap_or(0)
        }

        /// This season's raid volume, when the ledger's season is `season`.
        pub fn season_volume(&self, season: u32) -> u64 {
            if self.season_id == season {
                self.outbound_volume_season
            } else {
                0
            }
        }

        /// Rolls the season (04 section 2.9).
        pub fn roll_season(&mut self, current: u32) {
            if self.season_id != current {
                self.season_id = current;
                self.outbound_volume_season = 0;
            }
        }

        /// Adds `v` from `rival` at `now` (the two-window rolling rule; a full table of live
        /// windows records nothing, so dust rivals cannot evict a live raid).
        pub fn add_inbound(&mut self, rival: &Pubkey, now: i64, window: i64, v: u64) {
            let w = window.max(1);
            let idx = match self.inbound.iter().position(|e| e.rival_mint == *rival) {
                Some(i) => i,
                None => {
                    let free = self
                        .inbound
                        .iter()
                        .enumerate()
                        .filter(|(_, e)| e.rival_mint == Pubkey::default() || now >= e.window_start + 2 * w)
                        .min_by_key(|(_, e)| e.window_start)
                        .map(|(i, _)| i);
                    let Some(i) = free else { return };
                    self.inbound[i] = RaidWindow {
                        rival_mint: *rival,
                        window_start: now,
                        volume: 0,
                        prev_volume: 0,
                    };
                    i
                }
            };
            let e = &mut self.inbound[idx];
            let k = if now > e.window_start { (now - e.window_start) / w } else { 0 };
            if k == 1 {
                e.prev_volume = e.volume;
                e.volume = 0;
            } else if k >= 2 {
                e.prev_volume = 0;
                e.volume = 0;
            }
            e.window_start += k * w;
            e.volume = e.volume.saturating_add(v);
        }
    }

    /// The rolling volume of one window entry at `now` (05 section 2.5).
    pub fn rolling_volume(e: &RaidWindow, now: i64, window: i64) -> u64 {
        if window <= 0 || now < e.window_start {
            return e.volume;
        }
        let passed = (now - e.window_start) / window;
        let (current, previous, start) = match passed {
            0 => (e.volume, e.prev_volume, e.window_start),
            1 => (0, e.volume, e.window_start + window),
            _ => return 0,
        };
        let overlap = (start + window - now).clamp(0, window);
        let weighted = u128::from(previous) * overlap as u128 / window as u128;
        current.saturating_add(weighted as u64)
    }

    /// A war touch (04 section 2.10): the payload `hookwars_war` sends through `touch`.
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
    pub enum WarTouch {
        /// Spend raid points of the current season.
        SpendRaidPoints {
            /// How many.
            amount: u32,
        },
        /// Spend one loot ticket.
        SpendTicket,
        /// Add loot tickets.
        AddTicket {
            /// How many.
            amount: u16,
        },
    }

    /// The Raid range (04 section 3.1, 11 bytes): tag, season, points, tickets.
    pub const RAID_RANGE_LEN: usize = 11;
    /// The Raid range's layout tag.
    pub const RAID_TAG: u8 = 0x01;

    /// A Raid range decoded.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct RaidRange {
        /// Season of the points.
        pub season_id: u32,
        /// Raid points.
        pub raid_points: u32,
        /// Loot tickets.
        pub tickets: u16,
    }

    impl RaidRange {
        /// Reads `bytes` (zeros: never stamped). Points of another season read as 0.
        pub fn read(bytes: &[u8], current_season: u32) -> Self {
            if bytes.len() < RAID_RANGE_LEN || bytes[0] != RAID_TAG {
                return Self { season_id: current_season, ..Default::default() };
            }
            let season_id = u32::from_le_bytes(bytes[1..5].try_into().unwrap());
            let raid_points = u32::from_le_bytes(bytes[5..9].try_into().unwrap());
            let tickets = u16::from_le_bytes(bytes[9..11].try_into().unwrap());
            Self {
                season_id: current_season,
                raid_points: if season_id == current_season { raid_points } else { 0 },
                tickets,
            }
        }

        /// The bytes (all zeros when nothing is held, so the holding can close).
        pub fn write(&self) -> Vec<u8> {
            if self.raid_points == 0 && self.tickets == 0 {
                return vec![0; RAID_RANGE_LEN];
            }
            let mut v = vec![RAID_TAG];
            v.extend_from_slice(&self.season_id.to_le_bytes());
            v.extend_from_slice(&self.raid_points.to_le_bytes());
            v.extend_from_slice(&self.tickets.to_le_bytes());
            v
        }
    }
}

/// Composite items (08 section 2): the module list the armory keeps at `["composite", item]`.
pub mod composite {
    use super::*;

    /// `sha256("account:CompositeItem")[..8]`.
    pub const DISCRIMINATOR: [u8; 8] = [0x7d, 0x62, 0x96, 0xc4, 0x06, 0x4a, 0xd0, 0x29];
    /// Seed under the armory.
    pub const SEED: &[u8] = b"composite";
    /// `reads_module` when a module reads no earlier module.
    pub const NO_READ: u8 = 0xFF;

    /// One module of a composite (08 2.2).
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Module {
        /// A composable template.
        pub template_id: u16,
        /// Its fields (written out: the IDL builder cannot follow the `Params` alias).
        pub params: [u32; PARAM_FIELDS],
        /// First of its targets in the equip's targets.
        pub target_start: u8,
        /// How many targets.
        pub target_count: u8,
        /// Its sub-range length (its manifest's `data_bytes`).
        pub data_bytes: u8,
        /// An earlier module whose bytes it may read, or [`NO_READ`].
        pub reads_module: u8,
    }

    /// `CompositeItem` at `["composite", item]` under the armory (08 2.2).
    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
    pub struct CompositeItem {
        /// Layout version.
        pub version: u8,
        /// Bump.
        pub bump: u8,
        /// The item.
        pub item: Pubkey,
        /// Modules, run in this order.
        pub modules: Vec<Module>,
        /// Items fused into it (display only).
        pub provenance: Vec<Pubkey>,
    }

    impl CompositeItem {
        /// Space for `MAX_MODULES` modules and as many provenance keys.
        pub const SPACE: usize = 8 + 1 + 1 + 32 + 4 + MAX_MODULES * 50 + 4 + MAX_MODULES * 32;

        /// `["composite", item]` under the armory.
        pub fn address(item: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[SEED, item.as_ref()], &ids::ARMORY_ID)
        }

        /// Decodes an account's data (discriminator checked).
        pub fn decode(data: &[u8]) -> Option<Self> {
            if data.len() < 8 || data[..8] != DISCRIMINATOR {
                return None;
            }
            Self::deserialize(&mut &data[8..]).ok()
        }
    }

    /// Why a module list was refused (08 2.10).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum CompositeError {
        /// No module, or more than `MAX_MODULES`.
        TooManyModules,
        /// A template that may not be a module.
        NotComposable,
        /// Modules that no one slot kind can host.
        KindMismatch,
        /// Params refused by the template.
        BadParams,
        /// Two modules that conflict (08 3.2).
        ModuleConflict,
        /// Bytes past `max_bytes`.
        CompositeBytes,
        /// Target slices overlapping or past the limit.
        BadTargets,
        /// `reads_module` not an earlier module with bytes.
        BadModuleRead,
    }

    /// Whether a slot of `host` kind may run a module of `kind` (08 2.8).
    pub fn hostable(host: u8, kind_: u8) -> bool {
        match host {
            kind::FEE => kind_ == kind::FEE,
            kind::REWARD => kind_ == kind::REWARD || kind_ == kind::FEE,
            kind::DEFENSE => kind_ == kind::DEFENSE,
            kind::RELATION => kind_ == kind::RELATION,
            kind::POOL => matches!(kind_, kind::POOL | kind::FEE | kind::DEFENSE | kind::REWARD),
            _ => false,
        }
    }

    /// The host kind of a module list: the narrowest kind that hosts every module.
    pub fn host_kind(modules: &[Module]) -> Option<u8> {
        [kind::FEE, kind::REWARD, kind::DEFENSE, kind::RELATION, kind::POOL]
            .into_iter()
            .find(|h| {
                modules.iter().all(|m| {
                    shape(m.template_id).is_some_and(|s| hostable(*h, s.kind))
                })
            })
    }

    /// Validation at creation (08 2.10) and the combined manifest (08 2.4). `field` gives each
    /// template's floors and ceilings; `max_targets` and `max_bytes` are the limits.
    pub fn validate_modules(
        modules: &[Module],
        field: impl Fn(u16) -> Option<(Params, Params)>,
        max_targets: u8,
        max_bytes: u8,
    ) -> core::result::Result<Manifest, CompositeError> {
        if modules.is_empty() || modules.len() > MAX_MODULES {
            return Err(CompositeError::TooManyModules);
        }
        if modules.iter().any(|m| !composable(m.template_id)) {
            return Err(CompositeError::NotComposable);
        }
        // Integration pass 2 (08 arsenal 2 request 2): modules sharing one per-mint or per-slot
        // state may not sit together: Shield and Patience share the slot's `sell_mark` bit,
        // Mercenary and Raid both keep a Raid range, and Loyalty Pot and First Blood keep one
        // state per mint.
        let count = |id: u16| modules.iter().filter(|m| m.template_id == id).count();
        if (count(template_id::SHIELD) > 0 && count(arsenal2::PATIENCE) > 0)
            || (count(template_id::RAID) > 0 && count(arsenal2::MERCENARY) > 0)
            || count(arsenal2::LOYALTY_POT) > 1
            || count(arsenal2::FIRST_BLOOD) > 1
        {
            return Err(CompositeError::ModuleConflict);
        }
        let mut out = Manifest {
            kind: host_kind(modules).ok_or(CompositeError::KindMismatch)?,
            ..Default::default()
        };
        let mut bytes: u16 = 0;
        let mut used = [false; 256];
        let mut targets: u16 = 0;
        let mut touch_modules = 0;
        let mut markers: Option<(u8, u8)> = None;
        let sum = |a: u16, b: u16| a.saturating_add(b).min(10_000);
        for (i, m) in modules.iter().enumerate() {
            if !composable(m.template_id) {
                return Err(CompositeError::NotComposable);
            }
            let (min, max) = field(m.template_id).ok_or(CompositeError::NotComposable)?;
            check_fields(m.template_id, &min, &max, &m.params)
                .and_then(|_| validate(m.template_id, &m.params))
                .map_err(|_| CompositeError::BadParams)?;
            let mm = manifest(m.template_id, &m.params, m.target_count)
                .map_err(|_| CompositeError::BadParams)?;
            if m.data_bytes != mm.data_bytes {
                return Err(CompositeError::CompositeBytes);
            }
            if mm.token_flags & token_flags::ANSWERS_TOUCH != 0 {
                touch_modules += 1;
            }
            if mm.pool_flags & pool_flags::MARKS != 0 {
                let slice = (m.target_start, m.target_count);
                match markers {
                    Some(prev) if prev != slice => return Err(CompositeError::ModuleConflict),
                    _ => markers = Some(slice),
                }
            }
            for t in m.target_start..m.target_start.saturating_add(m.target_count) {
                if used[usize::from(t)] {
                    return Err(CompositeError::BadTargets);
                }
                used[usize::from(t)] = true;
            }
            targets += u16::from(m.target_count);
            if m.reads_module != NO_READ {
                let r = usize::from(m.reads_module);
                if r >= i || modules[r].data_bytes == 0 {
                    return Err(CompositeError::BadModuleRead);
                }
            }
            bytes += u16::from(mm.data_bytes);
            out.token_flags |= mm.token_flags;
            out.pool_flags |= mm.pool_flags;
            out.max_cut_buy_bps = sum(out.max_cut_buy_bps, mm.max_cut_buy_bps);
            out.max_cut_sell_bps = sum(out.max_cut_sell_bps, mm.max_cut_sell_bps);
            out.max_cut_transfer_bps = sum(out.max_cut_transfer_bps, mm.max_cut_transfer_bps);
            out.max_discount_bps = sum(out.max_discount_bps, mm.max_discount_bps);
            out.may_refuse |= mm.may_refuse;
            out.may_burn |= mm.may_burn;
            out.reads_other_pools = out.reads_other_pools.saturating_add(mm.reads_other_pools);
        }
        if touch_modules > 1 {
            return Err(CompositeError::ModuleConflict);
        }
        if targets > u16::from(max_targets) {
            return Err(CompositeError::BadTargets);
        }
        if bytes > u16::from(max_bytes) {
            return Err(CompositeError::CompositeBytes);
        }
        out.data_bytes = bytes as u8;
        Ok(out)
    }

    /// The byte offset of module `i`'s sub-range inside the composite's range.
    pub fn sub_offset(modules: &[Module], i: usize) -> usize {
        modules[..i].iter().map(|m| usize::from(m.data_bytes)).sum()
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
    fn raid_ledger_len_matches_its_encoding() {
        let mut v = Vec::new();
        raid::RaidLedger::default().serialize(&mut v).unwrap();
        assert_eq!(v.len() + 8, raid::RaidLedger::LEN);
    }

    #[test]
    fn inbound_rolls_two_windows() {
        let mut l = raid::RaidLedger::default();
        let r = Pubkey::new_unique();
        l.add_inbound(&r, 1_000, 100, 10);
        l.add_inbound(&r, 1_050, 100, 5);
        assert_eq!(l.inbound[0].volume, 15);
        l.add_inbound(&r, 1_120, 100, 7);
        assert_eq!((l.inbound[0].prev_volume, l.inbound[0].volume, l.inbound[0].window_start), (15, 7, 1_100));
        l.add_inbound(&r, 1_400, 100, 1);
        assert_eq!((l.inbound[0].prev_volume, l.inbound[0].volume), (0, 1));
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

/// Arsenal waves D and E (08 section 4, ids per section 6): template ids, shapes, rules,
/// manifests, and the seeds and accounts of their payouts (Referral, Loyalty Pot, First Blood).
pub mod arsenal2 {
    use super::*;

    /// Guest List (08 4.2): [min_hold, open_after_secs]; one target mint.
    pub const GUEST_LIST: u16 = 23;
    /// Loyalty Pot (08 4.3): [sell_cut_bps, epoch_secs].
    pub const LOYALTY_POT: u16 = 24;
    /// Holder Stream (08 4.3): [buy_cut_bps, sell_cut_bps].
    pub const HOLDER_STREAM: u16 = 25;
    /// Ally Pass (08 4.4): [min_hold, discount_bps]; one ally mint.
    pub const ALLY_PASS: u16 = 27;
    /// Embargo (08 4.4): [cut_bps]; 1.. target mints.
    pub const EMBARGO: u16 = 28;
    /// Mercenary (08 4.5): [points_per_unit].
    pub const MERCENARY: u16 = 29;
    /// Garrison (08 4.5): [discount_bps].
    pub const GARRISON: u16 = 30;
    /// War Levy (08 4.5): [trigger_lamports, sell_cut_bps].
    pub const WAR_LEVY: u16 = 31;
    /// Target Burn (08 4.6): [burn_bps, target_supply_bps].
    pub const TARGET_BURN: u16 = 33;
    /// Gift Ember (08 4.6): [cut_bps].
    pub const GIFT_EMBER: u16 = 34;
    /// Referral (08 4.7): [cut_bps].
    pub const REFERRAL: u16 = 36;
    /// Sell Ladder (08 4.7): [step_bps, cut_per_step_bps, max_cut_bps].
    pub const SELL_LADDER: u16 = 37;
    /// First Blood (08 4.7): [discount_bps, min_lamports].
    pub const FIRST_BLOOD: u16 = 38;
    /// Patience (08 4.7): [min_age_secs, discount_bps, source]; as built only source 1 (own age).
    pub const PATIENCE: u16 = 40;

    /// Every id of this block.
    pub const IDS: [u16; 14] = [
        GUEST_LIST, LOYALTY_POT, HOLDER_STREAM, ALLY_PASS, EMBARGO, MERCENARY, GARRISON, WAR_LEVY,
        TARGET_BURN, GIFT_EMBER, REFERRAL, SELL_LADDER, FIRST_BLOOD, PATIENCE,
    ];

    /// Hook-data bytes of Loyalty Pot (tag, `joined_epoch: u32`).
    pub const LOYALTY_BYTES: u8 = 5;
    /// Hook-data bytes of Mercenary (the Raid range).
    pub const MERCENARY_BYTES: u8 = raid::RAID_RANGE_LEN as u8;
    /// Hook-data bytes of Patience (tag, `since: u32`).
    pub const PATIENCE_BYTES: u8 = 5;
    /// Patience's only source as built: the module's own age stamp.
    pub const PATIENCE_SOURCE_OWN: u32 = 1;

    /// Whether `id` belongs to this block.
    pub fn is(id: u16) -> bool {
        IDS.contains(&id)
    }

    /// Kind, forge rules of the used fields, forgeable.
    pub fn shape_of(id: u16) -> (u8, &'static [ForgeRule], bool) {
        use ForgeRule::*;
        match id {
            GUEST_LIST => (kind::POOL, &[TowardFloor, Keep], true),
            LOYALTY_POT => (kind::POOL, &[TowardCeiling, Keep], true),
            HOLDER_STREAM => (kind::POOL, &[TowardCeiling, TowardCeiling], true),
            ALLY_PASS => (kind::POOL, &[TowardFloor, TowardCeiling], true),
            EMBARGO => (kind::POOL, &[TowardCeiling], true),
            MERCENARY => (kind::POOL, &[TowardCeiling], true),
            GARRISON => (kind::POOL, &[TowardCeiling], true),
            WAR_LEVY => (kind::POOL, &[TowardFloor, TowardCeiling], true),
            TARGET_BURN => (kind::POOL, &[TowardCeiling, Keep], true),
            GIFT_EMBER => (kind::FEE, &[TowardCeiling], true),
            REFERRAL => (kind::POOL, &[TowardCeiling], true),
            SELL_LADDER => (kind::FEE, &[Keep, TowardCeiling, TowardCeiling], true),
            FIRST_BLOOD => (kind::POOL, &[TowardCeiling, Keep], true),
            _ => (kind::POOL, &[TowardFloor, TowardCeiling, Keep], true),
        }
    }

    /// Template rules beyond floor and ceiling.
    pub fn validate(id: u16, p: &Params) -> core::result::Result<(), ParamsError> {
        let bad = match id {
            LOYALTY_POT => p[1] == 0,
            TARGET_BURN => p[1] > 10_000,
            SELL_LADDER => p[0] == 0 || p[0] > 10_000 || p[2] < p[1],
            PATIENCE => p[2] != PATIENCE_SOURCE_OWN,
            _ => false,
        };
        if bad {
            Err(ParamsError::BadParams)
        } else {
            Ok(())
        }
    }

    fn bps(v: u32) -> u16 {
        u16::try_from(v).unwrap_or(u16::MAX)
    }

    /// The manifest of an item of template `id` with `p` (08 section 4, "Callbacks").
    pub fn manifest(id: u16, p: &Params, max_targets: u8, m: &mut Manifest) {
        use token_flags::*;
        match id {
            GUEST_LIST => {
                m.pool_flags = pool_flags::BEFORE_SWAP;
                m.may_refuse = true;
                m.reads_other_pools = 1;
            }
            LOYALTY_POT => {
                m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA;
                m.pool_flags = pool_flags::AFTER_SWAP;
                m.max_cut_sell_bps = bps(p[0]);
                m.data_bytes = LOYALTY_BYTES;
            }
            HOLDER_STREAM => {
                m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
                m.max_cut_buy_bps = bps(p[0]);
                m.max_cut_sell_bps = bps(p[1]);
            }
            ALLY_PASS => {
                m.pool_flags = pool_flags::BEFORE_SWAP;
                m.max_discount_bps = bps(p[1]);
                m.reads_other_pools = 1;
            }
            EMBARGO => {
                m.pool_flags = pool_flags::BEFORE_SWAP;
                m.max_cut_buy_bps = bps(p[0]);
                m.reads_other_pools = max_targets;
            }
            MERCENARY => {
                m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA | ANSWERS_TOUCH;
                m.pool_flags = pool_flags::AFTER_SWAP | pool_flags::MARKS;
                m.data_bytes = MERCENARY_BYTES;
            }
            GARRISON => {
                m.pool_flags = pool_flags::BEFORE_SWAP;
                m.max_discount_bps = bps(p[0]);
            }
            WAR_LEVY => {
                m.pool_flags = pool_flags::AFTER_SWAP;
                m.max_cut_sell_bps = bps(p[1]);
            }
            TARGET_BURN => {
                m.pool_flags = pool_flags::BEFORE_SWAP | pool_flags::AFTER_SWAP;
                m.may_burn = true;
            }
            GIFT_EMBER => {
                m.token_flags = BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA;
                m.max_cut_transfer_bps = bps(p[0]);
            }
            REFERRAL => {
                m.pool_flags = pool_flags::BEFORE_SWAP;
                m.max_cut_buy_bps = bps(p[0]);
            }
            SELL_LADDER => {
                m.token_flags = BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA;
                m.max_cut_transfer_bps = bps(p[2]);
            }
            FIRST_BLOOD => {
                m.pool_flags = pool_flags::BEFORE_SWAP;
                m.max_discount_bps = bps(p[0]);
            }
            PATIENCE => {
                m.token_flags = BEFORE_TRANSFER | WRITES_HOOK_DATA;
                m.pool_flags = pool_flags::AFTER_SWAP;
                m.max_discount_bps = bps(p[1]);
                m.data_bytes = PATIENCE_BYTES;
            }
            _ => {}
        }
    }

    /// The targets a module may be aimed at: Guest List and Ally Pass one mint, Embargo one or
    /// more, the rest none; no role.
    pub fn targets_ok(id: u16, n: usize, role: u8) -> bool {
        role == 0
            && match id {
                GUEST_LIST | ALLY_PASS => n == 1,
                EMBARGO => n >= 1,
                _ => n == 0,
            }
    }

    /// Seeds under items.
    pub mod seeds {
        /// `["referred", mint, buyer]`: a buyer's chosen referrer (08 4.7, R22).
        pub const REFERRED: &[u8] = b"referred";
        /// `["referral", mint]`: owner of the Referral payout vault (a holding of the quote).
        pub const REFERRAL: &[u8] = b"referral";
        /// `["loyalty", mint]`: the Loyalty Pot's state and owner of its vault (a holding of the
        /// quote).
        pub const LOYALTY: &[u8] = b"loyalty";
        /// `["loyalty-claim", mint, holder]`: the last epoch a holder claimed.
        pub const LOYALTY_CLAIM: &[u8] = b"loyalty-claim";
        /// `["first-blood", mint]`: First Blood's last prize day.
        pub const FIRST_BLOOD: &[u8] = b"first-blood";
    }

    /// PDAs under items.
    pub mod pda {
        use super::seeds;
        use crate::ids::ITEMS_ID;
        use anchor_lang::prelude::Pubkey;

        /// `["referred", mint, buyer]`.
        pub fn referred(mint: &Pubkey, buyer: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[seeds::REFERRED, mint.as_ref(), buyer.as_ref()], &ITEMS_ID)
        }
        /// `["referral", mint]`.
        pub fn referral_owner(mint: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[seeds::REFERRAL, mint.as_ref()], &ITEMS_ID)
        }
        /// `["loyalty", mint]`.
        pub fn loyalty(mint: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[seeds::LOYALTY, mint.as_ref()], &ITEMS_ID)
        }
        /// `["loyalty-claim", mint, holder]`.
        pub fn loyalty_claim(mint: &Pubkey, holder: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[seeds::LOYALTY_CLAIM, mint.as_ref(), holder.as_ref()], &ITEMS_ID)
        }
        /// `["first-blood", mint]`.
        pub fn first_blood(mint: &Pubkey) -> (Pubkey, u8) {
            Pubkey::find_program_address(&[seeds::FIRST_BLOOD, mint.as_ref()], &ITEMS_ID)
        }
    }

    /// `Launch.curve_tokens + Launch.reserve_tokens` (upstream layout: curve at 177, reserve at
    /// 185): the supply the launch started with.
    pub fn launch_initial_supply(data: &[u8], mint: &Pubkey) -> Option<u64> {
        if data.len() < 193 || data[..8] != LAUNCH_DISCRIMINATOR || data[10..42] != mint.to_bytes() {
            return None;
        }
        let curve = u64::from_le_bytes(data[177..185].try_into().ok()?);
        let reserve = u64::from_le_bytes(data[185..193].try_into().ok()?);
        curve.checked_add(reserve)
    }

    /// The Loyalty epoch of `now`.
    pub fn epoch_of(now: i64, epoch_secs: u32) -> u32 {
        u32::try_from(now.max(0) / i64::from(epoch_secs.max(1))).unwrap_or(u32::MAX)
    }

    /// Sell Ladder's cut in bps for selling `amount` of `balance` (08 4.7).
    pub fn ladder_bps(p: &Params, amount: u64, balance: u64) -> u32 {
        if balance == 0 || p[0] == 0 {
            return 0;
        }
        let share = (u128::from(amount) * 10_000 / u128::from(balance)) as u64;
        let steps = share / u64::from(p[0]);
        (steps.saturating_mul(u64::from(p[1])).min(u64::from(p[2]))) as u32
    }

    /// Target Burn's burn on `base` with `supply` and the launch's `initial` supply.
    pub fn target_burn(p: &Params, base: u64, supply: u64, initial: u64) -> u64 {
        let target = (u128::from(initial) * u128::from(p[1]) / 10_000) as u64;
        if supply <= target {
            return 0;
        }
        let want = (u128::from(base) * u128::from(p[0].min(10_000)) / 10_000) as u64;
        want.min(supply - target)
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
        fn every_id_has_a_shape_and_is_composable() {
            for id in IDS {
                assert!(shape(id).is_some(), "{id}");
                assert!(composable(id), "{id}");
            }
        }

        #[test]
        fn ladder_and_target_burn() {
            let l = p(&[1_000, 50, 300]);
            assert_eq!(ladder_bps(&l, 50, 1_000), 0);
            assert_eq!(ladder_bps(&l, 250, 1_000), 100);
            assert_eq!(ladder_bps(&l, 1_000, 1_000), 300);
            let t = p(&[100, 9_000]);
            assert_eq!(target_burn(&t, 1_000_000, 1_000_000_000, 1_000_000_000), 10_000);
            assert_eq!(target_burn(&t, 1_000_000, 900_000_005, 1_000_000_000), 5);
            assert_eq!(target_burn(&t, 1_000_000, 900_000_000, 1_000_000_000), 0);
        }
    }
}

/// Integration pass 2 (09 section 21 items 3 to 5, R26): the optional agent attribution call.
///
/// A caller appends five accounts at the very end of its remaining accounts:
/// `[agents program, ["agents-caller"] under the caller, agents event authority, passport (mut),
/// actor]`. The calling program splits them off with [`agents_record::split`] before reading its
/// own remaining accounts, finishes its own effects, then calls [`agents_record::record`], which
/// invokes `hookwars_agents::record(kind, value)` signed by its `["agents-caller"]` PDA. The call
/// is built by hand so the armory, war and items need not depend on the agents crate.
pub mod agents_record {
    use super::*;
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    use anchor_lang::solana_program::program::invoke_signed;

    /// `sha256("global:record")[..8]`, the agents program's `record` discriminator (checked
    /// against the agents crate in the integ2 suite).
    pub const RECORD_DISCRIMINATOR: [u8; 8] = [222, 57, 201, 216, 199, 90, 247, 136];
    /// Seed of each caller's recorder PDA.
    pub const CALLER_SEED: &[u8] = b"agents-caller";
    /// Accounts in the suffix.
    pub const SUFFIX: usize = 5;
    /// `hookwars_agents::constants::record_kind`, copied (no crate dependency).
    pub const ITEMS_AUTHORED: u8 = 0;
    pub const ITEMS_EQUIPPED: u8 = 1;
    pub const ITEMS_FORGED: u8 = 2;
    pub const ROYALTY_CLAIM: u8 = 3;
    pub const CRANK: u8 = 4;
    pub const BOUNTY: u8 = 5;
    pub const LOOT_REVEAL: u8 = 6;

    /// The caller program's recorder PDA.
    pub fn caller(program: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[CALLER_SEED], program)
    }

    /// The agents program's event authority.
    pub fn event_authority() -> Pubkey {
        Pubkey::find_program_address(&[b"__event_authority"], &ids::AGENTS_ID).0
    }

    /// The five suffix metas a client appends (passport writable).
    pub fn suffix_metas(program: &Pubkey, passport: &Pubkey, actor: &Pubkey) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(ids::AGENTS_ID, false),
            AccountMeta::new_readonly(caller(program).0, false),
            AccountMeta::new_readonly(event_authority(), false),
            AccountMeta::new(*passport, false),
            AccountMeta::new_readonly(*actor, false),
        ]
    }

    /// Splits the suffix off when the remaining accounts end with one for `program`.
    pub fn split<'a, 'info>(
        rem: &'a [AccountInfo<'info>],
        program: &Pubkey,
    ) -> (&'a [AccountInfo<'info>], Option<&'a [AccountInfo<'info>]>) {
        let n = rem.len();
        if n >= SUFFIX
            && rem[n - SUFFIX].key() == ids::AGENTS_ID
            && rem[n - SUFFIX + 1].key() == caller(program).0
        {
            (&rem[..n - SUFFIX], Some(&rem[n - SUFFIX..]))
        } else {
            (rem, None)
        }
    }

    /// Invokes `record(kind, value)` when a suffix was given and its actor is `expected_actor`.
    /// Without a suffix this is a no-op, so every caller stays usable without the agents program.
    pub fn record<'info>(
        suffix: Option<&[AccountInfo<'info>]>,
        program: &Pubkey,
        expected_actor: &Pubkey,
        kind: u8,
        value: u64,
    ) -> Result<()> {
        let Some(s) = suffix else { return Ok(()) };
        let (c, bump) = caller(program);
        if s.len() != SUFFIX
            || s[1].key() != c
            || s[2].key() != event_authority()
            || s[4].key() != *expected_actor
        {
            return Err(ProgramError::InvalidArgument.into());
        }
        let mut data = RECORD_DISCRIMINATOR.to_vec();
        data.push(kind);
        data.extend_from_slice(&value.to_le_bytes());
        let ix = Instruction {
            program_id: ids::AGENTS_ID,
            accounts: vec![
                AccountMeta::new_readonly(c, true),
                AccountMeta::new(s[3].key(), false),
                AccountMeta::new_readonly(s[4].key(), false),
                AccountMeta::new_readonly(s[2].key(), false),
                AccountMeta::new_readonly(ids::AGENTS_ID, false),
            ],
            data,
        };
        invoke_signed(
            &ix,
            &[s[1].clone(), s[3].clone(), s[4].clone(), s[2].clone(), s[0].clone()],
            &[&[CALLER_SEED, &[bump]]],
        )?;
        Ok(())
    }
}

/// Integration pass 2 (10 section 17 I-3, I-4): the market's addresses and the `Lease` layout,
/// read by the armory and items without a crate dependency (the market depends on the armory).
pub mod market {
    use super::*;

    /// `["escrow", item_mint]`: owner of a listed item's holding.
    pub fn escrow(item_mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"escrow", item_mint.as_ref()], &ids::MARKET_ID).0
    }
    /// `["lease-escrow", item_mint]`: owner of a leased item's holding.
    pub fn lease_escrow(item_mint: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"lease-escrow", item_mint.as_ref()], &ids::MARKET_ID).0
    }
    /// `["lease", item]`.
    pub fn lease(item: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"lease", item.as_ref()], &ids::MARKET_ID).0
    }
    /// `["market-caller"]`: the market's signer of `revert_for_lease_end`.
    pub fn caller() -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"market-caller"], &ids::MARKET_ID)
    }
    /// `Lease.state` Active.
    pub const LEASE_ACTIVE: u8 = 1;

    /// Length of the optional lease suffix of `settle_equip`'s remaining accounts:
    /// `[<MARKET_ID>, ["lease", item], lessor's token holding, lessor's quote holding]`.
    pub const RENT_SUFFIX: usize = 4;

    /// Splits the lease suffix off `rem` when it ends with one for `item` (10 section 17 I-7).
    pub fn split_rent<'a, 'info>(
        rem: &'a [AccountInfo<'info>],
        item: &Pubkey,
    ) -> (&'a [AccountInfo<'info>], Option<&'a [AccountInfo<'info>]>) {
        let n = rem.len();
        if n >= RENT_SUFFIX
            && rem[n - RENT_SUFFIX].key() == ids::MARKET_ID
            && rem[n - RENT_SUFFIX + 1].key() == lease(item)
        {
            (&rem[..n - RENT_SUFFIX], Some(&rem[n - RENT_SUFFIX..]))
        } else {
            (rem, None)
        }
    }

    /// What the armory and items read of a `Lease` (discriminator, version, bump, lessor, item,
    /// item_mint, token_mint, slot, rent_bps, fee_lamports, term_secs, starts_at, ends_at, state).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct LeaseView {
        pub lessor: Pubkey,
        pub token_mint: Pubkey,
        pub slot: u8,
        pub rent_bps: u16,
        pub ends_at: i64,
        pub state: u8,
    }

    /// Reads a `Lease` owned by the market; `None` for an empty or foreign account.
    pub fn read_lease(info: &AccountInfo) -> Option<LeaseView> {
        if info.owner != &ids::MARKET_ID {
            return None;
        }
        let d = info.try_borrow_data().ok()?;
        if d.len() < 170 {
            return None;
        }
        let key = |o: usize| Pubkey::try_from(&d[o..o + 32]).ok();
        Some(LeaseView {
            lessor: key(10)?,
            token_mint: key(106)?,
            slot: d[138],
            rent_bps: u16::from_le_bytes([d[139], d[140]]),
            ends_at: i64::from_le_bytes(d[161..169].try_into().ok()?),
            state: d[169],
        })
    }
}

/// Hookwars hook economy (docs/spec/11-hook-economy.md): program ids, caller seeds, the counters
/// levels are computed from, the level function and the fee splits every economy program uses.
/// Appended for the economy branch; nothing above changes.
pub mod economy {
    use anchor_lang::prelude::*;

    /// `hookwars_craft` (11 section 5).
    pub const CRAFT_ID: Pubkey = Pubkey::from_str_const("39LXQBGqZtg591jkGnZi9BELQ9hp1ZngbAxu6K1cC29Y");
    /// `hookwars_book` (11 section 6).
    pub const BOOK_ID: Pubkey = Pubkey::from_str_const("C4k2QquxzDdgHf74tnvyyWQyGUR8xvhPo1i1gFYb639g");
    /// `hookwars_market` (10).
    pub const MARKET_ID: Pubkey = Pubkey::from_str_const("FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv");
    /// `hookwars_social` (10).
    pub const SOCIAL_ID: Pubkey = Pubkey::from_str_const("CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx");
    /// The SPL Memo program, version 2 (11 section 4.1).
    pub const MEMO_PROGRAM_ID: Pubkey =
        Pubkey::from_str_const("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

    /// Seed of the PDA a protocol program signs with when it calls craft `drop` or `wear` (R42).
    pub const CRAFT_CALLER_SEED: &[u8] = b"craft-caller";
    /// Seed of the PDA a protocol program signs with when it calls social `record_wallet` (R43).
    pub const SOCIAL_CALLER_SEED: &[u8] = b"social-caller";

    /// `PDA([seed], program)`: the caller PDA of `program`.
    pub fn caller_pda(seed: &[u8], program: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[seed], program)
    }

    /// Indexes of the wallet counters levels are computed from (11 section 3.3).
    pub mod counter {
        pub const ITEMS_AUTHORED: u8 = 0;
        pub const TEMPLATES_REGISTERED: u8 = 1;
        pub const LICENCES_SOLD: u8 = 2;
        pub const LICENCE_REVENUE_LAMPORTS: u8 = 3;
        pub const ITEMS_SOLD: u8 = 4;
        pub const ITEMS_CRAFTED: u8 = 5;
        pub const REPAIRS: u8 = 6;
        pub const BOOK_FILLS: u8 = 7;
        pub const TREATIES_HELD: u8 = 8;
        pub const RAIDS: u8 = 9;
        /// Number of counters a `Profile` keeps (layout constant).
        pub const COUNT: usize = 16;
    }

    /// Skill ids (11 section 3.3).
    pub mod skill {
        pub const BUILDER: u8 = 0;
        pub const CRAFTER: u8 = 1;
        pub const TRADER: u8 = 2;
        pub const DIPLOMAT: u8 = 3;
    }

    /// Thresholds per skill (layout constant).
    pub const MAX_LEVELS: usize = 8;

    /// One skill: a counter and the thresholds of its levels (ascending; 0 ends the list).
    #[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct SkillDef {
        pub id: u8,
        pub counter: u8,
        pub thresholds: [u64; MAX_LEVELS],
    }

    impl SkillDef {
        /// Thresholds strictly ascending until the first 0, then only zeros.
        pub fn valid(&self) -> bool {
            if usize::from(self.counter) >= counter::COUNT {
                return false;
            }
            let mut prev = 0u64;
            let mut ended = false;
            for t in self.thresholds {
                if ended {
                    if t != 0 {
                        return false;
                    }
                } else if t == 0 {
                    ended = true;
                } else if t <= prev {
                    return false;
                } else {
                    prev = t;
                }
            }
            true
        }
    }

    /// The level `value` reaches against `thresholds`: how many thresholds, from the first, it
    /// meets. A pure function: levels are computed, never stored (R43).
    pub fn level(thresholds: &[u64; MAX_LEVELS], value: u64) -> u8 {
        let mut l = 0u8;
        for t in thresholds {
            if *t == 0 || value < *t {
                break;
            }
            l += 1;
        }
        l
    }

    /// Sources of `ProtocolFee` events (11 section 2.4).
    pub mod fee_source {
        pub const DEX_SHARE: u8 = 0;
        pub const LAUNCH_LP: u8 = 1;
        pub const ITEM_RUN: u8 = 2;
        pub const SALE: u8 = 3;
        pub const LICENCE: u8 = 4;
        pub const LEASE: u8 = 5;
        pub const BOOK_FILL: u8 = 6;
        pub const RECIPE: u8 = 7;
    }

    /// `floor(amount * bps / 10_000)`.
    pub fn bps(amount: u64, bps: u16) -> u64 {
        (u128::from(amount) * u128::from(bps) / 10_000) as u64
    }

    /// `ceil(amount * bps / 10_000)`.
    pub fn bps_up(amount: u64, bps: u16) -> u64 {
        (u128::from(amount) * u128::from(bps)).div_ceil(10_000) as u64
    }

    /// Licence split (11 section 2.3): protocol first, then the template author's share of the
    /// remainder, the rest to the item holder. Sums exactly to `price`.
    pub fn licence_split(price: u64, protocol_bps: u16, author_bps: u16) -> (u64, u64, u64) {
        let protocol = bps(price, protocol_bps);
        let rest = price - protocol;
        let author = bps(rest, author_bps);
        (protocol, author, rest - author)
    }

    /// Token-side run waterfall (11 section 2.2) for a settled balance `b`: returns
    /// `(protocol, author, rent, holder, bounty, rest)`, summing exactly to `b`.
    pub fn run_waterfall(
        b: u64,
        item_protocol_bps: u16,
        royalty_bps: u16,
        author_bps: u16,
        rent_bps: u16,
        bounty_bps: u16,
    ) -> (u64, u64, u64, u64, u64, u64) {
        let protocol = bps(b, item_protocol_bps);
        let b1 = b - protocol;
        let royalty = bps(b1, royalty_bps);
        let author = bps(royalty, author_bps);
        let rent = bps(royalty - author, rent_bps);
        let holder = royalty - author - rent;
        let bounty = bps(b1 - royalty, bounty_bps);
        let rest = b1 - royalty - bounty;
        (protocol, author, rent, holder, bounty, rest)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn levels_count_met_thresholds() {
            let t = [10, 100, 1_000, 0, 0, 0, 0, 0];
            assert_eq!(level(&t, 0), 0);
            assert_eq!(level(&t, 10), 1);
            assert_eq!(level(&t, 999), 2);
            assert_eq!(level(&t, u64::MAX), 3);
            assert!(SkillDef { id: 0, counter: 0, thresholds: t }.valid());
            assert!(!SkillDef { id: 0, counter: 0, thresholds: [5, 5, 0, 0, 0, 0, 0, 0] }.valid());
            assert!(!SkillDef { id: 0, counter: 0, thresholds: [5, 0, 7, 0, 0, 0, 0, 0] }.valid());
            assert!(!SkillDef { id: 0, counter: 99, thresholds: t }.valid());
        }

        #[test]
        fn splits_sum_exactly() {
            for price in [0u64, 1, 7, 999, 1_000_000_007, u64::MAX / 3] {
                let (p, a, h) = licence_split(price, 250, 1_000);
                assert_eq!(p + a + h, price);
                let w = run_waterfall(price, 300, 2_500, 1_000, 5_000, 100);
                assert_eq!(w.0 + w.1 + w.2 + w.3 + w.4 + w.5, price);
            }
            assert_eq!(bps_up(1, 1), 1);
            assert_eq!(bps(1, 1), 0);
        }
    }
}

/// Integration pass 3 (docs/spec/13-integration-3.md): hand-built calls from the armory, items, war
/// and market into craft (`init_wear`, `wear`, `drop`) and social (`record_wallet`), plus the
/// optional account suffixes that carry their accounts. Built by hand so no protocol program needs
/// a crate dependency on craft or social (both depend on the armory, social on war: a cycle).
pub mod eco_cpi {
    use super::economy as eco;
    use super::*;
    use anchor_lang::solana_program::hash::hash;
    use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
    use anchor_lang::solana_program::program::invoke_signed;

    /// `sha256("global:<name>")[..8]`.
    pub fn disc(name: &str) -> [u8; 8] {
        let h = hash(format!("global:{name}").as_bytes()).to_bytes();
        let mut d = [0u8; 8];
        d.copy_from_slice(&h[..8]);
        d
    }

    /// An Anchor program's event authority.
    pub fn event_authority(program: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"__event_authority"], program).0
    }

    /// Splits a suffix of `len` accounts whose first account is `tag` off the end of `rem`.
    pub fn split_tagged<'a, 'info>(
        rem: &'a [AccountInfo<'info>],
        tag: &Pubkey,
        len: usize,
    ) -> (&'a [AccountInfo<'info>], Option<&'a [AccountInfo<'info>]>) {
        let n = rem.len();
        if n >= len && rem[n - len].key == tag {
            (&rem[..n - len], Some(&rem[n - len..]))
        } else {
            (rem, None)
        }
    }

    /// Craft's `["craft-config"]`.
    pub fn craft_config() -> Pubkey {
        Pubkey::find_program_address(&[b"craft-config"], &eco::CRAFT_ID).0
    }
    /// Craft's `["wear", item]`.
    pub fn wear_address(item: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"wear", item.as_ref()], &eco::CRAFT_ID).0
    }
    /// Social's `["skills"]`.
    pub fn skills_address() -> Pubkey {
        Pubkey::find_program_address(&[b"skills"], &eco::SOCIAL_ID).0
    }
    /// Social's `["profile", wallet]`.
    pub fn profile_address(wallet: &Pubkey) -> Pubkey {
        Pubkey::find_program_address(&[b"profile", wallet.as_ref()], &eco::SOCIAL_ID).0
    }

    /// Reads craft's `Wear` at `["wear", item]` without the craft crate: `Some(dormant)` when the
    /// account is the item's wear record owned by craft, `None` otherwise. Layout after the
    /// 8-byte discriminator: version, bump, item (32), max_charges (u32), used (u32), dormant.
    pub fn wear_dormant(info: &AccountInfo, item: &Pubkey) -> Option<bool> {
        if *info.owner != eco::CRAFT_ID || *info.key != wear_address(item) {
            return None;
        }
        let d = info.try_borrow_data().ok()?;
        if d.len() < 8 + 2 + 32 + 4 + 4 + 1 || d[10..42] != item.as_ref()[..] {
            return None;
        }
        Some(d[50] != 0)
    }

    fn cpi<'info>(program: Pubkey, name: &str, args: Vec<u8>, metas: Vec<AccountMeta>, infos: &[AccountInfo<'info>], seeds: &[&[u8]]) -> Result<()> {
        let mut data = disc(name).to_vec();
        data.extend(args);
        invoke_signed(&Instruction { program_id: program, accounts: metas, data }, infos, &[seeds])?;
        Ok(())
    }

    /// Accounts of the craft suffix `[craft program, craft config, craft event authority, the
    /// caller's ["craft-caller"] PDA, ...]` an instruction appends; `CRAFT_HEAD` of them.
    pub const CRAFT_HEAD: usize = 4;
    /// The init-wear suffix: head + `wear (mut)` + `system program`.
    pub const INIT_WEAR_SUFFIX: usize = CRAFT_HEAD + 2;
    /// The settle suffix: head + `wear` + drop accounts `drop_rule, material, material_mint,
    /// minter, recipient, recipient_holding`.
    pub const SETTLE_CRAFT_SUFFIX: usize = CRAFT_HEAD + 7;
    /// The reveal suffix (war): head + drop accounts (6).
    pub const DROP_SUFFIX: usize = CRAFT_HEAD + 6;

    fn check_head(s: &[AccountInfo], caller_program: &Pubkey) -> Result<u8> {
        let (caller, bump) = eco::caller_pda(eco::CRAFT_CALLER_SEED, caller_program);
        require!(
            *s[0].key == eco::CRAFT_ID && *s[1].key == craft_config() && *s[2].key == event_authority(&eco::CRAFT_ID) && *s[3].key == caller,
            ErrorCode::ConstraintAddress
        );
        Ok(bump)
    }

    /// `craft::init_wear(caller_program, item, max_charges)` from an init-wear suffix.
    pub fn init_wear<'info>(s: &[AccountInfo<'info>], caller_program: &Pubkey, payer: &AccountInfo<'info>, item: &Pubkey, max_charges: u32) -> Result<()> {
        require!(s.len() == INIT_WEAR_SUFFIX, ErrorCode::AccountNotEnoughKeys);
        let bump = check_head(s, caller_program)?;
        require!(*s[4].key == wear_address(item), ErrorCode::ConstraintAddress);
        let metas = vec![
            AccountMeta::new_readonly(*s[3].key, true),
            AccountMeta::new(*payer.key, true),
            AccountMeta::new_readonly(*s[1].key, false),
            AccountMeta::new(*s[4].key, false),
            AccountMeta::new_readonly(*s[5].key, false),
            AccountMeta::new_readonly(*s[2].key, false),
            AccountMeta::new_readonly(eco::CRAFT_ID, false),
        ];
        let mut args = caller_program.as_ref().to_vec();
        args.extend_from_slice(item.as_ref());
        args.extend(max_charges.to_le_bytes());
        let infos = [s[3].clone(), payer.clone(), s[1].clone(), s[4].clone(), s[5].clone(), s[2].clone(), s[0].clone()];
        cpi(eco::CRAFT_ID, "init_wear", args, metas, &infos, &[eco::CRAFT_CALLER_SEED, &[bump]])
    }

    /// `craft::wear(caller_program, runs)` when the settle suffix's wear slot holds the item's
    /// `Wear`; a no-op otherwise.
    pub fn wear<'info>(s: &[AccountInfo<'info>], caller_program: &Pubkey, item: &Pubkey, runs: u32) -> Result<()> {
        require!(s.len() >= CRAFT_HEAD + 1, ErrorCode::AccountNotEnoughKeys);
        if runs == 0 || wear_dormant(&s[4], item).is_none() {
            return Ok(());
        }
        let bump = check_head(s, caller_program)?;
        let metas = vec![
            AccountMeta::new_readonly(*s[3].key, true),
            AccountMeta::new_readonly(*s[1].key, false),
            AccountMeta::new(*s[4].key, false),
            AccountMeta::new_readonly(*s[2].key, false),
            AccountMeta::new_readonly(eco::CRAFT_ID, false),
        ];
        let mut args = caller_program.as_ref().to_vec();
        args.extend(runs.to_le_bytes());
        let infos = [s[3].clone(), s[1].clone(), s[4].clone(), s[2].clone(), s[0].clone()];
        cpi(eco::CRAFT_ID, "wear", args, metas, &infos, &[eco::CRAFT_CALLER_SEED, &[bump]])
    }

    /// The common accounts a `drop` needs beside the suffix.
    pub struct DropCommon<'a, 'info> {
        pub payer: &'a AccountInfo<'info>,
        pub token_program: &'a AccountInfo<'info>,
        pub token_event_authority: &'a AccountInfo<'info>,
        pub system_program: &'a AccountInfo<'info>,
    }

    /// `craft::drop(caller_program, source, measured)` from the six drop accounts `d` (`drop_rule,
    /// material, material_mint, minter, recipient, recipient_holding`); a no-op when the rule slot
    /// is not a craft account (no rule set up) or `measured` is 0. `drop` never fails for a spent
    /// cap (11 section 13.1 item 4).
    pub fn drop<'info>(head: &[AccountInfo<'info>], d: &[AccountInfo<'info>], c: &DropCommon<'_, 'info>, caller_program: &Pubkey, source: u8, measured: u64) -> Result<()> {
        require!(head.len() >= CRAFT_HEAD && d.len() == 6, ErrorCode::AccountNotEnoughKeys);
        if measured == 0 || *d[0].owner != eco::CRAFT_ID || d[0].data_is_empty() {
            return Ok(());
        }
        let bump = check_head(head, caller_program)?;
        let metas = vec![
            AccountMeta::new_readonly(*head[3].key, true),
            AccountMeta::new(*c.payer.key, true),
            AccountMeta::new_readonly(*head[1].key, false),
            AccountMeta::new(*d[0].key, false),
            AccountMeta::new(*d[1].key, false),
            AccountMeta::new(*d[2].key, false),
            AccountMeta::new_readonly(*d[3].key, false),
            AccountMeta::new_readonly(*d[4].key, false),
            AccountMeta::new(*d[5].key, false),
            AccountMeta::new_readonly(*c.token_program.key, false),
            AccountMeta::new_readonly(*c.token_event_authority.key, false),
            AccountMeta::new_readonly(*c.system_program.key, false),
            AccountMeta::new_readonly(*head[2].key, false),
            AccountMeta::new_readonly(eco::CRAFT_ID, false),
        ];
        let mut args = caller_program.as_ref().to_vec();
        args.push(source);
        args.extend(measured.to_le_bytes());
        let infos = [
            head[3].clone(), c.payer.clone(), head[1].clone(), d[0].clone(), d[1].clone(), d[2].clone(), d[3].clone(), d[4].clone(), d[5].clone(),
            c.token_program.clone(), c.token_event_authority.clone(), c.system_program.clone(), head[2].clone(), head[0].clone(),
        ];
        cpi(eco::CRAFT_ID, "drop", args, metas, &infos, &[eco::CRAFT_CALLER_SEED, &[bump]])
    }

    /// The social suffix `[social program, skills, profile (mut), social event authority, the
    /// caller's ["social-caller"] PDA]`.
    pub const SOCIAL_SUFFIX: usize = 5;

    /// `social::record_wallet(caller_program, counter, value)` for `wallet` from a social suffix;
    /// a no-op when the profile slot is not the wallet's profile (the wallet has none) or social
    /// does not list the caller (social itself returns early then).
    pub fn record_wallet<'info>(s: &[AccountInfo<'info>], caller_program: &Pubkey, wallet: &Pubkey, counter: u8, value: u64) -> Result<()> {
        require!(s.len() == SOCIAL_SUFFIX, ErrorCode::AccountNotEnoughKeys);
        let (caller, bump) = eco::caller_pda(eco::SOCIAL_CALLER_SEED, caller_program);
        require!(
            *s[0].key == eco::SOCIAL_ID && *s[1].key == skills_address() && *s[3].key == event_authority(&eco::SOCIAL_ID) && *s[4].key == caller,
            ErrorCode::ConstraintAddress
        );
        if *s[2].key != profile_address(wallet) || *s[2].owner != eco::SOCIAL_ID || value == 0 {
            return Ok(());
        }
        let metas = vec![
            AccountMeta::new_readonly(caller, true),
            AccountMeta::new_readonly(*s[1].key, false),
            AccountMeta::new(*s[2].key, false),
            AccountMeta::new_readonly(*s[3].key, false),
            AccountMeta::new_readonly(eco::SOCIAL_ID, false),
        ];
        let mut args = caller_program.as_ref().to_vec();
        args.push(counter);
        args.extend(value.to_le_bytes());
        let infos = [s[4].clone(), s[1].clone(), s[2].clone(), s[3].clone(), s[0].clone()];
        cpi(eco::SOCIAL_ID, "record_wallet", args, metas, &infos, &[eco::SOCIAL_CALLER_SEED, &[bump]])
    }

    /// Client side: the init-wear suffix metas for `caller_program` and `item`.
    pub fn init_wear_metas(caller_program: &Pubkey, item: &Pubkey) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(eco::CRAFT_ID, false),
            AccountMeta::new_readonly(craft_config(), false),
            AccountMeta::new_readonly(event_authority(&eco::CRAFT_ID), false),
            AccountMeta::new_readonly(eco::caller_pda(eco::CRAFT_CALLER_SEED, caller_program).0, false),
            AccountMeta::new(wear_address(item), false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ]
    }

    /// Client side: the social suffix metas for `caller_program` and `wallet`.
    pub fn social_metas(caller_program: &Pubkey, wallet: &Pubkey) -> Vec<AccountMeta> {
        vec![
            AccountMeta::new_readonly(eco::SOCIAL_ID, false),
            AccountMeta::new_readonly(skills_address(), false),
            AccountMeta::new(profile_address(wallet), false),
            AccountMeta::new_readonly(event_authority(&eco::SOCIAL_ID), false),
            AccountMeta::new_readonly(eco::caller_pda(eco::SOCIAL_CALLER_SEED, caller_program).0, false),
        ]
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn discriminators_match_the_spec() {
            assert_eq!(disc("mint_crafted"), [121, 196, 34, 30, 90, 249, 235, 177]);
            assert_eq!(disc("record"), crate::agents_record::RECORD_DISCRIMINATOR);
        }
    }
}
