// Changed by Hookwars: new file, a test-only stand-in for hookwars_items (war suites).
//! `war_items_stub`: test-only, never deployed. Declared at the items program id so the war
//! suites can run `touch` through a Raid slot before the real items program is merged. It does
//! exactly what 04 section 3.1 says the Raid item does on `on_touch` (payloads 04 section 2.10),
//! accepting them only from the war program's `["war-signer"]`; and it declares the `RaidLedger`
//! layout of 04 section 2.9, so the suites write ledgers with Anchor's own discriminator.

#![allow(unexpected_cfgs)]

use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::set_return_data;
use bordrless_hook::{SlotReturn, TokenSlotArgs, TokenSlotOp};

declare_id!("8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv");

/// The token program's signer of every callback to this program: `["hook-authority", items]`
/// under the token program.
pub const TOKEN_HOOK_SIGNER: Pubkey =
    Pubkey::from_str_const("GtuzTUqRkfbWLarc8hhn7WWTkkuPXavZBQGb8MH43kwN");
/// `["war-signer"]` under the war program.
pub const WAR_SIGNER: Pubkey = Pubkey::from_str_const("HCBAfMMN6JMUfnJ64H4Kxs6r3gaTpJCeFr5dyrDMENoe");
/// The Raid range (04 section 3.1): tag, season, points, tickets.
pub const RAID_RANGE_LEN: usize = 11;
pub const RAID_TAG: u8 = 0x01;
pub const RAID_TABLE_LEN: usize = 8;

/// 04 section 2.10.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarTouch {
    SpendRaidPoints { amount: u32 },
    SpendTicket,
    AddTicket { amount: u16 },
}

#[program]
pub mod war_items_stub {
    use super::*;

    /// The Raid item's `on_touch`.
    pub fn on_touch(ctx: Context<SlotCallback>, args: TokenSlotArgs) -> Result<()> {
        let _ = &ctx.accounts.hook_signer;
        require!(args.op == TokenSlotOp::Touch, StubError::BadCall);
        require_keys_eq!(args.authority, WAR_SIGNER, StubError::NotWarSigner);
        require!(args.source_data.len() == RAID_RANGE_LEN, StubError::BadCall);
        let payload = WarTouch::deserialize(&mut args.payload.as_slice())
            .map_err(|_| error!(StubError::BadCall))?;
        let mut r = args.source_data.clone();
        if r[0] != RAID_TAG {
            r = vec![0u8; RAID_RANGE_LEN];
            r[0] = RAID_TAG;
        }
        let mut points = u32::from_le_bytes(r[5..9].try_into().unwrap());
        let mut tickets = u16::from_le_bytes(r[9..11].try_into().unwrap());
        match payload {
            WarTouch::SpendRaidPoints { amount } => {
                points = points.checked_sub(amount).ok_or(StubError::NotEnoughPoints)?;
            }
            WarTouch::SpendTicket => {
                tickets = tickets.checked_sub(1).ok_or(StubError::NoTicket)?;
            }
            WarTouch::AddTicket { amount } => {
                tickets = tickets.saturating_add(amount);
            }
        }
        r[5..9].copy_from_slice(&points.to_le_bytes());
        r[9..11].copy_from_slice(&tickets.to_le_bytes());
        let answer = SlotReturn {
            deltas: vec![],
            source_data: Some(r),
            destination_data: None,
        };
        let mut out = Vec::new();
        answer.serialize(&mut out)?;
        set_return_data(&out);
        Ok(())
    }
}

/// A slot callback: the token prefix (no extras).
#[derive(Accounts)]
pub struct SlotCallback<'info> {
    /// CHECK: the token program's signer for this program (address- and signer-checked).
    #[account(signer, address = TOKEN_HOOK_SIGNER @ StubError::BadHookSigner)]
    pub hook_signer: UncheckedAccount<'info>,
    /// CHECK: the mint.
    pub mint: UncheckedAccount<'info>,
    /// CHECK: source.
    pub source: UncheckedAccount<'info>,
    /// CHECK: destination.
    pub destination: UncheckedAccount<'info>,
    /// CHECK: authority.
    pub authority: UncheckedAccount<'info>,
}

/// 04 section 2.9.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RaidWindow {
    pub rival_mint: Pubkey,
    pub window_start: i64,
    pub volume: u64,
    pub prev_volume: u64,
}

/// 04 section 2.9.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mark {
    pub clock_slot: u64,
    pub recipient: Pubkey,
    pub rival: Pubkey,
    pub quote_volume: u64,
    pub stamped_slots: u8,
}

/// `RaidLedger` at `["raid-ledger", mint]` (04 section 2.9).
#[account]
#[derive(Debug)]
pub struct RaidLedger {
    pub version: u8,
    pub bump: u8,
    pub mint: Pubkey,
    pub season_id: u32,
    pub outbound_volume_season: u64,
    pub inbound: [RaidWindow; RAID_TABLE_LEN],
    pub mark: Mark,
    pub reserved: [u8; 32],
}

#[error_code]
pub enum StubError {
    #[msg("bad call")]
    BadCall,
    #[msg("the hook signer is not the token program's")]
    BadHookSigner,
    #[msg("only the war signer may touch")]
    NotWarSigner,
    #[msg("not enough raid points")]
    NotEnoughPoints,
    #[msg("no ticket")]
    NoTicket,
}
