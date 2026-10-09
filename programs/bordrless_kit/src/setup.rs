// Changed by Hookwars: new file. Which mints the kit accepts (R9) and how war reads a rival's
// holder rewards (R10).
//! The kit's mint setup, shared by `init` and by the Hookwars war program.
//!
//! - **R9.** Upstream the kit is a mint's single hook. Under Hookwars it may instead sit in the
//!   `Locked` slot of a slot mint (`docs/spec/00-overview.md` R8, R9): the token program then calls
//!   it with the upstream arguments and applies only the bytes of its range, 0..32
//!   ([`KIT_DATA_LEN`], `docs/hooks-v2.md` 4.4: the kit never uses bytes 32..64).
//! - **R10.** `hookwars_war::siege` refuses a rival whose kit has holder rewards on, because the kit
//!   lets only wallets hold such a token (`docs/hooks-v2.md` 4.5) and a war chest is a PDA. War
//!   decides it with [`holder_rewards_on`]: the rival mint (any token-program `Mint`) and, when the
//!   kit runs on it, the rival's `KitConfig` at `KitConfig::address(mint)` (owned by the kit).

use bordrless_token::state::Mint;

use crate::constants::mint_flags;
use crate::state::KitConfig;

/// The kit's hook-data range in a holding: bytes `0..KIT_DATA_LEN`.
pub const KIT_DATA_LEN: u8 = 32;
/// The kit's registry always lists two extras (`docs/hooks-v2.md` 4.4).
pub const KIT_EXTRA_COUNT: u8 = 2;

/// The range a Locked kit slot needs for its flags: `KIT_DATA_LEN` when the kit keeps hook data
/// (holder rewards or the early-buyer lock), else none (the token program refuses a Locked range
/// for a program that never writes).
pub const fn kit_data_len(flags: u16) -> u8 {
    if flags & bordrless_hook::token_flags::WRITES_HOOK_DATA != 0 {
        KIT_DATA_LEN
    } else {
        0
    }
}

/// Whether the kit runs on `mint`: as its single hook (upstream) or in its `Locked` slot.
pub fn kit_installed(mint: &Mint) -> bool {
    if mint.uses_slots() {
        mint.locked_slot()
            .is_some_and(|(_, slot)| slot.program == crate::ID)
    } else {
        mint.hook_program == Some(crate::ID)
    }
}

/// Whether `mint` is set up as `init` requires for `modules` and a supply of `supply`: nobody can
/// mint, the max supply is the supply, and the kit runs from the first instruction and can never be
/// changed, either as the single hook with no hook authority (upstream) or as the `Locked` slot of a
/// slot mint (R9) with the kit's flags, its range at bytes `0..kit_data_len(flags)` and its two extras.
pub fn mint_setup_ok(mint: &Mint, modules: u8, supply: u64) -> bool {
    if mint.mint_authority.is_some() || mint.max_supply != supply || mint.hook_authority.is_some() {
        return false;
    }
    let flags = mint_flags(modules);
    if !mint.uses_slots() {
        return mint.hook_program == Some(crate::ID) && mint.hook_flags == flags;
    }
    if mint.hook_program.is_some() {
        return false;
    }
    match mint.locked_slot() {
        Some((_, slot)) => {
            slot.program == crate::ID
                && slot.flags == flags
                && slot.data_offset == 0
                && slot.data_len == kit_data_len(flags)
                && slot.extra_count == KIT_EXTRA_COUNT
        }
        None => false,
    }
}

/// R10: whether the mint at `mint_key` runs the kit with holder rewards on. `kit_config` is the
/// `KitConfig` at `KitConfig::address(mint_key)` (owned by the kit), `None` when there is none; a
/// config for another mint counts as none.
pub fn holder_rewards_on(
    mint_key: &anchor_lang::prelude::Pubkey,
    mint: &Mint,
    kit_config: Option<&KitConfig>,
) -> bool {
    kit_installed(mint) && kit_config.is_some_and(|c| c.mint == *mint_key && c.rewards_on())
}
