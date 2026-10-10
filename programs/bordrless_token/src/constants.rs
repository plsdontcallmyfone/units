// Changed by Hookwars: R20 protocol-transfer marker; slot constants and the Hookwars program ids the token program checks.
//! Constants of the token standard.

/// Seed of a holding: `["holding", mint, owner]`.
pub const HOLDING_SEED: &[u8] = b"holding";
/// Layout version written into new accounts.
pub const VERSION: u8 = 1;
/// Longest name.
pub const MAX_NAME: usize = 32;
/// Longest symbol.
pub const MAX_SYMBOL: usize = 10;
/// Longest metadata URI.
pub const MAX_URI: usize = 200;
/// Most decimals a mint may have.
pub const MAX_DECIMALS: u8 = 12;
/// Discriminator length.
pub const DISCRIMINATOR_LEN: usize = 8;

use anchor_lang::prelude::Pubkey;

/// Slots per mint (parameter `MAX_SLOTS`, docs/spec/00-overview.md section 6). Provisional build
/// value for the M1 measurements; set from them (docs/spec/01-token-slots.md, M1 notes).
pub const MAX_SLOTS: usize = 4;
/// Item slots that may answer a cut on one transfer (parameter `MAX_CUTTING_SLOTS`). With a
/// Locked slot that may cut, one fewer (R1: total deltas at most `MAX_DELTAS`).
pub const MAX_CUTTING_SLOTS: usize = 3;
/// `hookwars_armory`: its `["slots", mint]` PDA alone may change a mint's slots and vote locks.
pub const ARMORY_ID: Pubkey = Pubkey::from_str_const("7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU");
/// `hookwars_items`: owner program of equip vaults (`["equip", mint, slot]`).
pub const ITEMS_ID: Pubkey = Pubkey::from_str_const("8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv");
/// `hookwars_war`.
pub const WAR_ID: Pubkey = Pubkey::from_str_const("5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2");
/// The DEX (an item program may never be it).
pub const SWAP_ID: Pubkey = Pubkey::from_str_const("AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo");
/// The launchpad (an item program may never be it; Pool slots keep its signer bump).
pub const LAUNCH_ID: Pubkey = Pubkey::from_str_const("fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD");
/// The bridge (an item program may never be it).
pub const BRIDGE_ID: Pubkey = Pubkey::from_str_const("5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj");
/// `hookwars_market` (integration pass 2, 10 section 17 I-1).
pub const MARKET_ID: Pubkey = Pubkey::from_str_const("FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv");
/// `hookwars_social` (integration pass 2, 10 section 17 I-1): guild treasuries pay out.
pub const SOCIAL_ID: Pubkey = Pubkey::from_str_const("CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx");
/// Programs whose PDAs may send protocol payouts that skip item slots (R16).
pub const PROTOCOL_SOURCE_PROGRAMS: [Pubkey; 5] = [ITEMS_ID, ARMORY_ID, WAR_ID, MARKET_ID, SOCIAL_ID];
/// Seed of [`PROTOCOL_TRANSFER_MARKER`].
pub const PROTOCOL_TRANSFER_SEED: &[u8] = b"protocol-transfer";
/// Hookwars R20: in a `transfer_from_protocol` the Locked slot is told this as `authority`. It is
/// `["protocol-transfer"]` under this program, a PDA this program never signs for, so no ordinary
/// transfer can carry it: a Locked hook (the kit) reads it as "the source is a verified protocol
/// vault (R16)". Checked against its derivation in the tests.
pub const PROTOCOL_TRANSFER_MARKER: Pubkey =
    Pubkey::from_str_const("HHwFz2okVQVyTE3HLsKSM2a7u4Y2DW729MbmkSeNoWEG");
