// Changed by Hookwars: program ids and derived addresses; observation ring and route constants.
//! Constants of the DEX.

/// `["config"]`.
pub const CONFIG_SEED: &[u8] = b"config";
/// `["pool", base_mint, quote_mint, lp_fee_bps (u16 LE), hook_program or default]`.
pub const POOL_SEED: &[u8] = b"pool";
/// `["lp", pool]`.
pub const LP_SEED: &[u8] = b"lp";
/// Hookwars parameter `OBS_RING_LEN` (to set, spec 00 section 6): entries in the observation ring
/// every pool account carries after its `Pool` fields (spec 03 section 3.1, M3a notes).
/// This build value is a TEST value measured in `programs/tests/tests/budgets.rs`.
pub const OBS_RING_LEN: u16 = 32;
/// Hookwars: least seconds between two ring entries (to set with `OBS_RING_LEN`: together they
/// fix the history a ring covers, `OBS_RING_LEN * OBS_SPACING_SECS` seconds at least). TEST value.
pub const OBS_SPACING_SECS: u32 = 30;
/// Hookwars parameter `MIN_TWAP_SECS` (to set, owner decision): the shortest window any reader of
/// another pool's observations may use. Readers pass it to `bordrless_core::observations::window_read`.
/// TEST value.
pub const MIN_TWAP_SECS: i64 = 300;
/// Hookwars parameter `MAX_ROUTE_HOPS` (to set by measurement): hops in one `swap_route`. Build
/// value measured in `budgets.rs`.
pub const MAX_ROUTE_HOPS: usize = 3;
/// Bump of `["config"]` (`HnckfEpqfiSan7VurzXtFxmHHTsKmkD2dwepwTdkrF5Z`).
pub const CONFIG_BUMP: u8 = 255;
/// Layout version.
pub const VERSION: u8 = 1;
/// Largest LP fee a pool or a hook may set.
pub const MAX_LP_FEE_BPS: u16 = 9_000;
/// Largest flat protocol fee the config may set (ordinary pools).
pub const MAX_PROTOCOL_FEE_BPS: u16 = 1_000;
/// Largest share of the hooks' cuts the config may set for launch pools: the whole cut.
pub const MAX_PROTOCOL_SHARE_BPS: u16 = 10_000;
/// `Pool.fee_model`: a flat rate of the quote (`protocol_fee_bps`): the pools anyone opens.
pub const FEE_MODEL_FLAT: u8 = 0;
/// `Pool.fee_model`: a share of what the pool's hooks cut (`protocol_share_bps`), in the quote:
/// the pools the launchpad opens (a curve created by `LAUNCHPAD_ID` as its own hook). A launch
/// whose rules collect nothing pays nothing. A curve any other hook program creates is an
/// ordinary pool for the fee: the flat rate, so nobody can open a fee-free pool by deploying a
/// program that signs with its own `["hook-authority"]`.
pub const FEE_MODEL_SHARE: u8 = 1;
/// The launchpad (`bordrless_launch`): the one hook program whose curves get the share model.
/// The test suite checks it equals `bordrless_launch::ID` (the DEX cannot depend on the launch
/// program, which depends on the DEX).
pub const LAUNCHPAD_ID: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::Pubkey::from_str_const("fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD");
/// Discriminator length.
pub const DISCRIMINATOR_LEN: usize = 8;
/// Name of every LP mint.
pub const LP_NAME: &str = "Bordrless LP";
/// Symbol of every LP mint.
pub const LP_SYMBOL: &str = "BLP";
/// The upgradeable loader.
pub const BPF_LOADER_UPGRADEABLE_ID: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");

/// The bridge (`bordrless_bridge`): `collect_protocol_fees_sol` calls its `unwrap_sol`. Constants
/// rather than a crate dependency; the unit tests check each against the bridge's own.
pub const BRIDGE_ID: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::Pubkey::from_str_const("5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj");
/// Bridged SOL: the bridge's `["wrapped", NATIVE_MINT]`, the quote of every launch pool.
pub const BRIDGED_SOL_MINT: anchor_lang::prelude::Pubkey =
    anchor_lang::prelude::Pubkey::from_str_const("7YMXcZ3AUD5pBceT4QzM2hrApoH3rEmPZUAzpKPHVvR4");
/// The bridge's `unwrap_sol` instruction discriminator.
pub const UNWRAP_SOL_DISCRIMINATOR: [u8; 8] = [99, 40, 14, 105, 45, 107, 172, 201];

#[cfg(test)]
mod bridge_tests {
    use anchor_lang::Discriminator;

    #[test]
    fn the_bridge_constants_are_the_bridges() {
        let native = anchor_lang::prelude::Pubkey::from_str_const(
            "So11111111111111111111111111111111111111112",
        );
        assert_eq!(super::BRIDGE_ID, bordrless_bridge::ID);
        assert_eq!(
            super::BRIDGED_SOL_MINT,
            bordrless_bridge::client::wrapped_mint_address(&native)
        );
        assert_eq!(
            super::UNWRAP_SOL_DISCRIMINATOR,
            bordrless_bridge::instruction::UnwrapSol::DISCRIMINATOR
        );
    }
}
