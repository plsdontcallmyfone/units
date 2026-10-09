// Changed by Hookwars: program ids and derived addresses; M3b slot launches (armory, items, pool-item forwarding).
//! Constants of the launchpad. Addresses this program compares against are constants rather than
//! derived on chain (`docs/hooks-v2.md` §4.12); the unit tests check each against its derivation.

use anchor_lang::prelude::Pubkey;
use bordrless_hook::pool_flags;

/// `["config"]`.
pub const CONFIG_SEED: &[u8] = b"config";
/// `["launch", mint]`.
pub const LAUNCH_SEED: &[u8] = b"launch";
/// `["kit-caller", mint]`: the only signer the kit's `init` and `graduate` accept. It signs nothing
/// else and holds nothing.
pub const KIT_CALLER_SEED: &[u8] = bordrless_kit::constants::KIT_CALLER_SEED;
/// `["kit", mint]` under the kit: a token's `KitConfig`.
pub const KIT_SEED: &[u8] = bordrless_kit::constants::KIT_SEED;
/// Bump of this program's `["hook-authority"]` (`FSHhimQGTZQHenfuRStNhNsgwuBWBLiPhGhWc2tgEKW`).
pub const HOOK_AUTHORITY_BUMP: u8 = 255;
/// Bump of `["config"]` (`CQDSEYEXuKA8MgBXpuS6D3Tyvi5iDe5hK9ifxcCyuNYh`).
pub const CONFIG_BUMP: u8 = 255;
/// This program's `["config"]`.
pub const CONFIG_ADDRESS: Pubkey =
    Pubkey::from_str_const("CQDSEYEXuKA8MgBXpuS6D3Tyvi5iDe5hK9ifxcCyuNYh");
/// This program's `["hook-authority"]`: it creates launch pools and finalizes their curves.
pub const LAUNCH_HOOK_AUTHORITY: Pubkey =
    Pubkey::from_str_const("FSHhimQGTZQHenfuRStNhNsgwuBWBLiPhGhWc2tgEKW");
/// The DEX's `["config"]`.
pub const DEX_CONFIG: Pubkey =
    Pubkey::from_str_const("HnckfEpqfiSan7VurzXtFxmHHTsKmkD2dwepwTdkrF5Z");
/// Layout version.
pub const VERSION: u8 = 1;
/// Discriminator length.
pub const DISCRIMINATOR_LEN: usize = 8;
/// The upgradeable loader.
pub const BPF_LOADER_UPGRADEABLE_ID: Pubkey =
    Pubkey::from_str_const("BPFLoaderUpgradeab1e11111111111111111111111");
/// The kit (`bordrless_kit`): the token hook a launch with rules installs. A constant; there is no
/// config field for it.
pub const KIT_ID: Pubkey = bordrless_kit::KIT_ID;
/// The kit's event authority.
pub const KIT_EVENT_AUTHORITY: Pubkey = bordrless_kit::EVENT_AUTHORITY_AND_BUMP.0;
/// The token program's signer of the kit's callbacks, `["hook-authority", KIT_ID]` under the
/// token program: what the token instructions of a launch's mint take as their hook signer (the
/// kit is the only hook a launch's mint has).
pub const TOKEN_HOOK_AUTHORITY: Pubkey = bordrless_kit::constants::TOKEN_HOOK_AUTHORITY;
/// The token program's event authority.
pub const TOKEN_EVENT_AUTHORITY: Pubkey = bordrless_token::EVENT_AUTHORITY_AND_BUMP.0;
/// The DEX's signer of every callback to this program, `["hook-authority", LAUNCH_ID]` under the
/// DEX. The DEX signs each pool hook's callbacks with a PDA of that hook's id, so a signer another
/// pool hook received and passes on in a CPI here is refused.
pub const DEX_HOOK_AUTHORITY: Pubkey =
    Pubkey::from_str_const("ACEJWkSdbGJ1T1YLJWhZ16RGWaXdRrvE7BqhB5hf3xK8");
/// The DEX's event authority.
pub const DEX_EVENT_AUTHORITY: Pubkey = bordrless_swap::EVENT_AUTHORITY_AND_BUMP.0;
/// The callbacks a launch pool subscribes to.
pub const LAUNCH_HOOK_FLAGS: u16 = pool_flags::BEFORE_INITIALIZE
    | pool_flags::BEFORE_SWAP
    | pool_flags::AFTER_SWAP
    | pool_flags::BEFORE_SWAP_RETURNS_DELTA
    | pool_flags::AFTER_SWAP_RETURNS_DELTA
    | pool_flags::BEFORE_SWAP_OVERRIDES_FEE;
/// Index, in a pool callback's account list, of the launch's quote holding (prefix of 5, then the
/// launch, then the holding): creator fees go there.
pub const QUOTE_HOLDING_INDEX: u8 = 6;
/// Index of the holder vault (the kit config's holding of the quote): holder fees go there.
pub const HOLDER_VAULT_INDEX: u8 = 7;
/// Index of the kit config, read for the holder-fee threshold.
pub const KIT_CONFIG_INDEX: u8 = 8;
/// `Launch.status`: on the curve.
pub const STATUS_CURVE: u8 = 0;
/// `Launch.status`: graduated.
pub const STATUS_GRADUATED: u8 = 1;
/// Longest `LaunchConfig.label`, in bytes.
pub const LABEL_MAX: usize = 32;

/// Who may hold a custom hook's upgrade authority (§5.8): a hook must be immutable, or upgradeable
/// only by Bordrless Studio's upgrade key (a hook Studio deployed as "managed", upgraded only
/// through Studio's review and rebuild) or by the protocol's own upgrade authority (Half-Life,
/// tax_hook). A hook anyone else can upgrade could be swapped for other code after its token
/// launched, so `create_config` and `create_launch` refuse it.
pub const HOOK_UPGRADE_AUTHORITIES: [Pubkey; 2] = [
    Pubkey::from_str_const("6ve794V3v88GFaGjRrZ1mH83Z49e6q6NZym4GZJ94mCK"),
    Pubkey::from_str_const("CFi9xajnSxM1WMSndVoyQHmfm6DuEdzFodfjRfhTuzxa"),
];

/// The BPF loader 2 and loader v4 (whose programs `check_hook_authority` reads; the upgradeable
/// loader is `BPF_LOADER_UPGRADEABLE_ID` above).
pub const BPF_LOADER_2_ID: Pubkey =
    Pubkey::from_str_const("BPFLoader2111111111111111111111111111111111");
pub const LOADER_V4_ID: Pubkey =
    Pubkey::from_str_const("LoaderV411111111111111111111111111111111111");

/// The most a listed config's author may take of a launch's creator fee: half of it.
pub const MAX_AUTHOR_SHARE_BPS: u16 = 5_000;
/// The programs a `LaunchConfig` may not name as a custom token hook: the protocol's own (the
/// kit is the launchpad's hook, installed by the inline rules), the system program and the
/// default key.
pub const PROTOCOL_PROGRAMS: [Pubkey; 7] = [
    bordrless_token::ID,
    bordrless_swap::ID,
    bordrless_kit::constants::BRIDGE_ID,
    crate::ID,
    KIT_ID,
    anchor_lang::system_program::ID,
    Pubkey::new_from_array([0; 32]),
];

/// Hard ceilings `init_config` and `set_config` enforce on the token-rule bounds (§5.2); no config
/// can raise them.
pub mod ceilings {
    /// Holder fee per side.
    pub const HOLDER_FEE_BPS: u16 = 500;
    /// Burn per side.
    pub const BURN_BPS: u16 = 500;
    /// Creator fee + holder fee + burn on one side.
    pub const RULES_FEE_BPS: u16 = 1_000;
    /// The smallest a config may set as its smallest max wallet: a cap must be something.
    pub const MIN_MAX_WALLET_BPS: u16 = 1;
    /// Max wallet stays below the whole supply (the kit's own bound).
    pub const MAX_WALLET_BPS: u16 = bordrless_kit::constants::MAX_MAX_WALLET_BPS;
    /// Creator wallet lock: 365 days (the kit's own bound).
    pub const CREATOR_LOCK_SECS: u32 = 365 * 86_400;
    /// Early-buyer window.
    pub const EARLY_WINDOW_SECS: u32 = 3_600;
    /// Early-buyer unlock, counted from the launch: 30 days (the kit's own bound).
    pub const EARLY_LOCK_SECS: u32 = 30 * 86_400;
    /// The smallest launch supply, so the kit's `min_eligible` (supply / 1,000) is at least 1.
    pub const MIN_SUPPLY: u64 = bordrless_kit::constants::MIN_SUPPLY;
    /// The largest launch supply the kit installs on, which bounds its reward math.
    pub const MAX_SUPPLY: u64 = bordrless_kit::constants::MAX_SUPPLY;
}

/// Hookwars M3b (spec 03 sections 4 and 5): slot launches.
pub mod hookwars {
    use anchor_lang::prelude::Pubkey;

    /// The armory: its `equip_launch` is the only instruction `equip_prepared` forwards.
    pub const ARMORY_ID: Pubkey = bordrless_token::constants::ARMORY_ID;
    /// The items program: owns `["pool-cuts", mint]`.
    pub const ITEMS_ID: Pubkey = bordrless_token::constants::ITEMS_ID;
    /// The war program: owns `["war-chest", mint]`.
    pub const WAR_ID: Pubkey = bordrless_token::constants::WAR_ID;
    /// `["prepared", mint]`: a prepared slot launch.
    pub const PREPARED_SEED: &[u8] = b"prepared";
    /// `["armory-caller", mint]`: signs the armory's `equip_launch`, and nothing else.
    pub const ARMORY_CALLER_SEED: &[u8] = b"armory-caller";
    /// `["pool-cuts", mint]` under the items program: owner of the holding that takes every
    /// pool item's cut (R2).
    pub const POOL_CUTS_SEED: &[u8] = b"pool-cuts";
    /// `["war-chest", mint]` under the war program.
    pub const WAR_CHEST_SEED: &[u8] = b"war-chest";
    /// `sha256("global:equip_launch")[..8]`, the armory's instruction `equip_prepared` forwards
    /// (checked against Anchor's derivation in the unit tests).
    pub const EQUIP_LAUNCH_DISCRIMINATOR: [u8; 8] = [48, 11, 21, 138, 90, 194, 210, 75];
    /// Index, in a slot launch's pool callback account list, of the `PoolCuts` quote holding
    /// (prefix of 5, upstream's four extras at 5 to 8, then this one): every pool item's cut on a
    /// side goes there as one delta.
    pub const POOL_CUTS_INDEX: u8 = 9;
    /// Pool flags of an item, as the armory writes them into a slot (`hookwars_common::pool_flags`):
    /// `pool_before_swap`.
    pub const ITEM_POOL_BEFORE: u16 = 1;
    /// `pool_after_swap`.
    pub const ITEM_POOL_AFTER: u16 = 1 << 1;
    /// `sha256("global:pool_before_swap")[..8]`, the pool items' before-swap callback.
    pub const POOL_BEFORE_SWAP_DISCRIMINATOR: [u8; 8] = [52, 167, 134, 185, 26, 179, 21, 24];
    /// `sha256("global:pool_after_swap")[..8]`, the pool items' after-swap callback.
    pub const POOL_AFTER_SWAP_DISCRIMINATOR: [u8; 8] = [216, 69, 224, 21, 151, 250, 159, 125];
    /// `Launch.slot_launch`: a launch made with `prepare_launch` and `create_prepared_launch`,
    /// whose mint runs a slot table.
    pub const SLOT_LAUNCH: u8 = 1;
}
