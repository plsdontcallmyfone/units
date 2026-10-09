// Changed by Hookwars: new file (hook economy, 11 section 5).
use anchor_lang::prelude::*;

#[event]
pub struct CraftParamsProposed {
    pub ready_at: i64,
}

#[event]
pub struct MaterialCreated {
    pub id: u16,
    pub mint: Pubkey,
    pub emission_cap_per_season: u64,
    pub ts: i64,
}

#[event]
pub struct MaterialCapProposed {
    pub id: u16,
    pub cap: u64,
    pub ready_at: i64,
}

#[event]
pub struct DropRuleProposed {
    pub source: u8,
    pub material_id: u16,
    pub per_unit_num: u64,
    pub per_unit_den: u64,
    pub ready_at: i64,
}

#[event]
pub struct Dropped {
    pub source: u8,
    pub material_id: u16,
    pub caller_program: Pubkey,
    pub recipient: Pubkey,
    pub measured: u64,
    pub amount: u64,
    pub season: u64,
    pub ts: i64,
}

#[event]
pub struct RecipeProposed {
    pub id: u16,
    pub ready_at: i64,
}

#[event]
pub struct Crafted {
    pub recipe: u16,
    pub crafter: Pubkey,
    pub template_id: u16,
    pub fee: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct Repaired {
    pub recipe: u16,
    pub item: Pubkey,
    pub holder: Pubkey,
    pub restored: u32,
    pub used: u32,
    pub fee: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}

#[event]
pub struct WearOpened {
    pub item: Pubkey,
    pub max_charges: u32,
    pub ts: i64,
}

/// Emitted once, when an item turns dormant (11 section 5.4).
#[event]
pub struct ItemWorn {
    pub item: Pubkey,
    pub ts: i64,
}

/// Every protocol collection (11 section 2.4); `mint` is the default key for native lamports.
#[event]
pub struct ProtocolFee {
    pub source: u8,
    pub mint: Pubkey,
    pub amount: u64,
    pub reference: [u8; 32],
    pub ts: i64,
}
