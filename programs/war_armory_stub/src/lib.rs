// Changed by Hookwars: new file, a test-only stand-in for hookwars_armory (war suites).
//! `war_armory_stub`: test-only, never deployed. Declared at the armory program id so the war
//! suites can run `reveal` before the real armory is merged. `mint_loot` accepts only the war
//! program's `["loot-signer"]` (02 section 4.3) and records what it was asked in
//! `["loot-log", owner]`; it mints nothing. The `Item`, `Template` and `ForgeCounter` layouts of
//! 02 are declared here so the suites write them with Anchor's own discriminators.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;

declare_id!("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");

/// `["loot-signer"]` under the war program.
pub const LOOT_SIGNER: Pubkey = Pubkey::from_str_const("DK5PUA6578wDF96DAEqxqiUDypvqPvdaoYASisgyPuZv");
/// `PARAM_FIELDS` (02, 04: at least 11).
pub const PARAM_FIELDS: usize = 11;

#[program]
pub mod war_armory_stub {
    use super::*;

    /// `mint_loot(owner, template_id, params)`: records the request.
    pub fn mint_loot(
        ctx: Context<MintLoot>,
        owner: Pubkey,
        template_id: u16,
        params: [u32; PARAM_FIELDS],
    ) -> Result<()> {
        require_keys_eq!(owner, ctx.accounts.owner.key(), StubError::BadCall);
        let log = &mut ctx.accounts.loot_log;
        log.owner = owner;
        log.count += 1;
        log.last_template_id = template_id;
        log.last_params = params;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct MintLoot<'info> {
    /// CHECK: the war program's loot signer (address- and signer-checked).
    #[account(signer, address = LOOT_SIGNER @ StubError::NotLootSigner)]
    pub loot_signer: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: the owner.
    pub owner: UncheckedAccount<'info>,
    #[account(mut, seeds = [b"loot-log", owner.key().as_ref()], bump)]
    pub loot_log: Account<'info, LootLog>,
}

/// What `mint_loot` was asked, per owner (created by the suites with `put`).
#[account]
#[derive(Debug, Default)]
pub struct LootLog {
    pub owner: Pubkey,
    pub count: u64,
    pub last_template_id: u16,
    pub last_params: [u32; PARAM_FIELDS],
}

/// 04 section 2.7: 16 bytes, field order fixed.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Manifest {
    pub kind: u8,
    pub token_flags: u16,
    pub pool_flags: u8,
    pub max_cut_buy_bps: u16,
    pub max_cut_sell_bps: u16,
    pub max_cut_transfer_bps: u16,
    pub max_discount_bps: u16,
    pub may_refuse: bool,
    pub may_burn: bool,
    pub data_bytes: u8,
    pub reads_other_pools: u8,
}

/// `Item` (02 section 2.4).
#[account]
#[derive(Debug)]
pub struct Item {
    pub version: u8,
    pub bump: u8,
    pub item_mint: Pubkey,
    pub template_id: u16,
    pub params: [u32; PARAM_FIELDS],
    pub manifest: Manifest,
    pub author: Pubkey,
    pub royalty_bps: u16,
    pub level: u8,
    pub source: u8,
    pub equipped_count: u32,
    pub royalty_owner_bump: u8,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

/// `Template` (02 section 2.3).
#[account]
#[derive(Debug)]
pub struct Template {
    pub version: u8,
    pub bump: u8,
    pub id: u16,
    pub program: Pubkey,
    pub code_hash: [u8; 32],
    pub deploy_slot: Option<u64>,
    pub kind: u8,
    pub field_count: u8,
    pub field_min: [u32; PARAM_FIELDS],
    pub field_max: [u32; PARAM_FIELDS],
    pub open_authoring: bool,
    pub loot_enabled: bool,
    pub forge_enabled: bool,
    pub max_level: u8,
    pub loot_royalty_bps: u16,
    pub status: u8,
    pub name: String,
    pub registered_by: Pubkey,
    pub created_at: i64,
    pub reserved: [u8; 32],
}

/// `ForgeCounter` (02 section 2.10).
#[account]
#[derive(Debug)]
pub struct ForgeCounter {
    pub wallet: Pubkey,
    pub count: u64,
    pub bump: u8,
}

#[error_code]
pub enum StubError {
    #[msg("bad call")]
    BadCall,
    #[msg("only the war program's loot signer may mint loot")]
    NotLootSigner,
}
