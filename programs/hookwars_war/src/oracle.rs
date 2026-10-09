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
