//! The randomness adapter interface (05 section 8.2, D-4 open).
//!
//! The war program names one adapter program in its config (`randomness_program`). An adapter is a
//! small program that turns the chosen oracle (Switchboard randomness, ORAO VRF, ...) into this
//! interface, so the war program never depends on one oracle's account layout:
//!
//! - `request_randomness(requester: Pubkey)`: accounts `[payer (signer, writable), requester
//!   (signer: the `RollRequest` PDA), randomness (writable), system program]`. The adapter creates
//!   or commits `randomness` for `requester` and records the slot it was requested in.
//! - the `randomness` account (owned by the adapter): the Anchor discriminator of `Randomness`, then
//!   `requester: Pubkey, request_slot: u64, fulfilled: bool, fulfilled_slot: u64, value: [u8; 32]`.
//!
//! `reveal` accepts a value only when `fulfilled`, `fulfilled_slot > RollRequest.requested_slot`,
//! and `requester` is the roll. Never a slot hash, a recent blockhash or anything a trader or a
//! cranker chooses.
//!
//! D-4 (owner, 2026-10-10): Switchboard On-Demand randomness. When `randomness_program` is one of
//! the Switchboard On-Demand programs ([`SWITCHBOARD_MAINNET`], [`SWITCHBOARD_DEVNET`]) the war
//! program reads Switchboard's own `RandomnessAccountData` with no adapter and no CPI (17-randomness):
//!
//! - `roll`: the client puts Switchboard's `randomness_commit` before `roll` in one transaction.
//!   The account must be owned by the Switchboard program, its authority must be the roll's owner,
//!   its `seed_slot` must be the previous slot (a fresh commit) and it must not be revealed for that
//!   commit (`reveal_slot < seed_slot`). The roll records `seed_slot` as `requested_slot`.
//! - `reveal`: the client puts Switchboard's `randomness_reveal` before `reveal` in one transaction.
//!   The account must still hold the roll's commit (`seed_slot == requested_slot`) and be revealed
//!   in this slot (`reveal_slot == clock.slot`, Switchboard's own freshness rule).
//!
//! The ticket is spent at `roll`, so a trader who sees a value they dislike gains nothing by not
//! revealing (the Switchboard "collateral on commit" rule).

use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

use crate::error::WarError;
use crate::foreign::disc;

/// What an adapter's randomness account holds.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Randomness {
    pub requester: Pubkey,
    pub request_slot: u64,
    pub fulfilled: bool,
    pub fulfilled_slot: u64,
    pub value: [u8; 32],
}

impl Randomness {
    pub fn read(info: &AccountInfo, program: &Pubkey) -> Result<Self> {
        require_keys_eq!(*info.owner, *program, WarError::WrongRandomness);
        let data = info.try_borrow_data()?;
        require!(
            data.len() >= 8 && data[..8] == disc::RANDOMNESS,
            WarError::WrongRandomness
        );
        Self::deserialize(&mut &data[8..]).map_err(|_| error!(WarError::WrongRandomness))
    }
}

/// Switchboard On-Demand, mainnet (`switchboard-on-demand` 0.13.0 `ON_DEMAND_MAINNET_PID`).
pub const SWITCHBOARD_MAINNET: Pubkey = Pubkey::from_str_const("SBondMDrcV3K4kxZR1HNVT7osZxAHVHgYXL5Ze1oMUv");
/// Switchboard On-Demand, devnet (`switchboard-on-demand` 0.13.0 `ON_DEMAND_DEVNET_PID`).
pub const SWITCHBOARD_DEVNET: Pubkey = Pubkey::from_str_const("Aio4gaXjXzJNVLtzwtNVmSqGKpANtXhybbkhtAC94ji2");

/// Whether `program` is a Switchboard On-Demand program (read directly, no adapter).
pub fn is_switchboard(program: &Pubkey) -> bool {
    *program == SWITCHBOARD_MAINNET || *program == SWITCHBOARD_DEVNET
}

/// The fields of Switchboard's `RandomnessAccountData` the war program reads. The account is
/// `discriminator (8) | authority (32) | queue (32) | seed_slothash (32) | seed_slot u64 |
/// oracle (32) | reveal_slot u64 | value (32) | 224 reserved`, 408 bytes (`#[repr(C)]`, little
/// endian), as `switchboard-on-demand` 0.13.0 `accounts/randomness.rs` defines it. Live accounts
/// are 480 bytes (read on devnet 2026-10-10, 168,519 accounts, all 480): the fields read here sit at
/// the same offsets, the rest is Switchboard's.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SbRandomness {
    pub authority: Pubkey,
    pub seed_slot: u64,
    pub reveal_slot: u64,
    pub value: [u8; 32],
}

impl SbRandomness {
    /// `RandomnessAccountData::DISCRIMINATOR`.
    pub const DISCRIMINATOR: [u8; 8] = [10, 66, 229, 135, 220, 239, 217, 114];
    /// The struct's length with its discriminator: the shortest account accepted.
    pub const LEN: usize = 408;
    /// A live account's length (the test fixture's).
    pub const ACCOUNT_LEN: usize = 480;
    const AUTHORITY: usize = 8;
    const SEED_SLOT: usize = 8 + 96;
    const REVEAL_SLOT: usize = 8 + 104 + 32;
    const VALUE: usize = 8 + 104 + 40;

    /// Reads the account, owned by `program` (one of the Switchboard programs).
    pub fn read(info: &AccountInfo, program: &Pubkey) -> Result<Self> {
        require_keys_eq!(*info.owner, *program, WarError::WrongRandomness);
        let data = info.try_borrow_data()?;
        Self::parse(&data)
    }

    pub fn parse(data: &[u8]) -> Result<Self> {
        require!(
            data.len() >= Self::LEN && data[..8] == Self::DISCRIMINATOR,
            WarError::WrongRandomness
        );
        let u64_at = |at: usize| u64::from_le_bytes(data[at..at + 8].try_into().unwrap());
        Ok(Self {
            authority: Pubkey::new_from_array(data[Self::AUTHORITY..Self::AUTHORITY + 32].try_into().unwrap()),
            seed_slot: u64_at(Self::SEED_SLOT),
            reveal_slot: u64_at(Self::REVEAL_SLOT),
            value: data[Self::VALUE..Self::VALUE + 32].try_into().unwrap(),
        })
    }

    /// The test fixture's bytes (the same layout).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut d = vec![0u8; Self::ACCOUNT_LEN];
        d[..8].copy_from_slice(&Self::DISCRIMINATOR);
        d[Self::AUTHORITY..Self::AUTHORITY + 32].copy_from_slice(self.authority.as_ref());
        d[Self::SEED_SLOT..Self::SEED_SLOT + 8].copy_from_slice(&self.seed_slot.to_le_bytes());
        d[Self::REVEAL_SLOT..Self::REVEAL_SLOT + 8].copy_from_slice(&self.reveal_slot.to_le_bytes());
        d[Self::VALUE..Self::VALUE + 32].copy_from_slice(&self.value);
        d
    }
}

/// `sha256("global:request_randomness")[..8]`.
pub const REQUEST_RANDOMNESS: [u8; 8] = [0xd5, 0x05, 0xad, 0xa6, 0x25, 0xec, 0x1f, 0x12];

/// The adapter's `request_randomness` instruction.
pub fn request_randomness(
    program: Pubkey,
    payer: Pubkey,
    requester: Pubkey,
    randomness: Pubkey,
) -> Instruction {
    let mut data = REQUEST_RANDOMNESS.to_vec();
    data.extend_from_slice(requester.as_ref());
    Instruction {
        program_id: program,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(requester, true),
            AccountMeta::new(randomness, false),
            AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        ],
        data,
    }
}
