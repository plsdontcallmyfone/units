// Changed by Hookwars: program ids, derived addresses, new programs and their fixed PDAs.
/**
 * Program addresses and the protocol's other fixed keys, the one place they are written down (the
 * same on mainnet, devnet and localnet: one set of program keypairs, generated 2026-10-07). Every
 * fixed key is a PDA with constant seeds; `packages/sdk` derives each again in its tests and fails
 * if one differs.
 */
export const PROGRAM_IDS = {
  token: '5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618',
  swap: 'AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo',
  bridge: '5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj',
  launch: 'fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD',
  taxHook: 'q9mMtM6vfJ8YMffnkUNW5XLz7xeVyyeo1HL8SA27AuX',
  /** The v2 token-rules hook (docs/hooks-v2.md §4): holder rewards, max wallet and the two locks. */
  kit: 'CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG',
  /** Half-Life (programs/half_life): an exit fee that halves every six hours a token is held, burned. */
  halfLife: '67nAgW7h9wYmM1jLzrXVNy8UDNxgVFqGTqrTEPamtokh',
  /** Companions (programs/bordrless_companion, docs/companions.md): a launch whose creator is a program, its fees bought back, shared with holders or vested by code. */
  companion: 'HzeAN8e7HbGx8wzgd5SpduF51c7rXCTqkQKcmw44YHkK',
  /** hookwars_armory (docs/spec/02-armory.md). */
  armory: '7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU',
  /** hookwars_items (docs/spec/04-templates.md): every template, one program. */
  items: '8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv',
  /** hookwars_war (docs/spec/05-war.md). */
  war: '5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2',
  /** hookwars_agents (docs/spec/09-agents.md): passports, badges, proof levels, policy wallets, bonds. */
  agents: 'GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9',
  /** hookwars_market (docs/spec/10-expansion.md): item listings, collections, rentals, commissions. */
  market: 'FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv',
  /** hookwars_social (docs/spec/10-expansion.md): achievement badges, guild halls. */
  social: 'CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx',
  /** hookwars_craft (docs/spec/11-hook-economy.md section 5): materials, recipes, repair, wear, drops. */
  craft: '39LXQBGqZtg591jkGnZi9BELQ9hp1ZngbAxu6K1cC29Y',
  /** hookwars_book (docs/spec/11-hook-economy.md section 6): a limit order book per material and class bids for items. */
  book: 'C4k2QquxzDdgHf74tnvyyWQyGUR8xvhPo1i1gFYb639g',
} as const;

/** The two authorities the launchpad lets hold a template program's upgrade key (00 rule 3). */
export const PROTOCOL_AUTHORITY = 'CFi9xajnSxM1WMSndVoyQHmfm6DuEdzFodfjRfhTuzxa';
export const MANAGED_HOOK_KEY = '6ve794V3v88GFaGjRrZ1mH83Z49e6q6NZym4GZJ94mCK';

/**
 * Half-Life, Bordrless's own token hook (programs/half_life/README.md): the `LaunchConfig` the
 * launch form's Half-Life path launches from (no kit rules, creator fee 1%, the hook with
 * `flags`), and the fee curve the program has fixed: `maxFeePpm` for tokens that just arrived,
 * halved every `halfLifeSecs`, linear within each, zero from `zeroAfterSecs`.
 */
export const HALF_LIFE = {
  program: PROGRAM_IDS.halfLife,
  /** Bordrless's mainnet Half-Life config; Hookwars has none (Half-Life is a template, 04 3.7). */
  launchConfig: null as string | null,
  /** `BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA | WRITES_HOOK_DATA`. */
  flags: 193,
  creatorFeeBps: 100,
  maxFeePpm: 200_000,
  halfLifeSecs: 21_600,
  zeroAfterSecs: 172_800,
  /** The token program's signer of the hook's callbacks (`["hook-authority", half_life]`). */
  tokenHookSigner: '9XyWNU43yubo6Rd1XEADiUMMjRk2Mtd2jGtpoQVfBD85',
  readme: 'https://github.com/BordrlessDex/bordrless-programs/tree/main/programs/half_life',
} as const;

/**
 * Half-Life's exit fee, in parts per million, for tokens `ageSecs` old: the program's `fee_ppm`
 * (20% at 0, halved every six hours, linear within each, 0 from 48 h).
 */
export function halfLifeFeePpm(ageSecs: number): number {
  if (!(ageSecs > 0)) return HALF_LIFE.maxFeePpm;
  const halvings = Math.floor(ageSecs / HALF_LIFE.halfLifeSecs);
  if (halvings >= HALF_LIFE.zeroAfterSecs / HALF_LIFE.halfLifeSecs) return 0;
  const into = Math.floor(ageSecs) % HALF_LIFE.halfLifeSecs;
  const hi = Math.floor(HALF_LIFE.maxFeePpm / 2 ** halvings);
  const lo = Math.floor(HALF_LIFE.maxFeePpm / 2 ** (halvings + 1));
  return hi - Math.floor(((hi - lo) * into) / HALF_LIFE.halfLifeSecs);
}

/**
 * The DEX upgrade of 2026-10-08 (docs/hooks-v2.md §3.1, "The LP fee of a launch pool is
 * Bordrless's"): from this slot on mainnet a launch pool's LP fee (the sniper fee included) is
 * taken in SOL and paid to Bordrless with the protocol fee; a launch pool created before it keeps
 * its LP fee in the pool, compounding, as its traders were told. The slot is the DEX program's
 * last upgrade, read from its ProgramData account; the time is that slot's block time
 * (`getBlockTime`, unix seconds), which the backend compares a pool's `created_at` against (the
 * pools table keeps a timestamp, not a slot). Devnet and localnet were deployed after the rule:
 * every launch pool there pays Bordrless.
 */
export const LP_FEE_TO_PROTOCOL_FROM_SLOT = 454_439_142;
export const LP_FEE_TO_PROTOCOL_FROM_TIME = 1_791_434_238;

/**
 * Whether a pool's LP fee goes to Bordrless (`LaunchSummary.lpFeeToProtocol`): every launch pool's
 * (the share model), whenever it was created. The upgraded DEX keys the fee's destination on the
 * pool's fee model alone, so a launch pool opened before the upgrade compounded its LP fee until
 * that slot and pays Bordrless since. An ordinary pool's LP fee is its liquidity's. `createdAt`
 * and `cluster` are kept for the record of when the rule began (the constants above).
 */
export function lpFeeToProtocol(shareModel: boolean, _createdAt: number, _cluster: 'mainnet' | 'devnet' | 'localnet'): boolean {
  return shareModel;
}

/** The wSOL mint, which stands for native SOL on the bridge. */
export const NATIVE_MINT = 'So11111111111111111111111111111111111111112';
export const TOKEN_PROGRAM_ID = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA';
export const TOKEN_2022_PROGRAM_ID = 'TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb';
export const SYSTEM_PROGRAM_ID = '11111111111111111111111111111111';
export const ASSOCIATED_TOKEN_PROGRAM_ID = 'ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL';
export const COMPUTE_BUDGET_PROGRAM_ID = 'ComputeBudget111111111111111111111111111111';

/**
 * The signers of hook callbacks (docs/hooks-v2.md, Review fixes, note 1): the token program signs
 * every callback to a token hook with `["hook-authority", hook_program]` under the token program,
 * and the DEX every callback to a pool hook with `["hook-authority", hook_program]` under the DEX.
 * A hook accepts only its own signer, so a signer one hook receives and passes on is refused
 * everywhere else. The v1 global signers (`D7gU…`, `HoUi…`) are gone.
 */
export const HOOK_SIGNERS = {
  /** The token program's signer of the kit's callbacks (bump 253). */
  tokenForKit: 'B1rkktspgQt6ghUrQBRBU5JhLxSKbayB2zt2UkcFdZQT',
  /** The token program's signer of `tax_hook`'s callbacks (bump 255). */
  tokenForTaxHook: '7TXEjARSzFuojgJEywFa9tGd2zD4yYPVshkLBKNPXstS',
  /** The DEX's signer of the launch's pool callbacks (bump 255). */
  dexForLaunch: 'ACEJWkSdbGJ1T1YLJWhZ16RGWaXdRrvE7BqhB5hf3xK8',
  /** The token program's signer of `hookwars_items` callbacks (bump 255). */
  tokenForItems: 'GtuzTUqRkfbWLarc8hhn7WWTkkuPXavZBQGb8MH43kwN',
  /** The launchpad's signer of Pool-item callbacks, `["hook-authority", ITEMS]` under the launchpad (bump 255; 03 5.5 LAUNCH_ITEMS_SIGNER). */
  launchForItems: '7WNwXG1tgUs2Srx4wPxBiRfhtMvMkZUZCDuyvrGAYLnX',
} as const;

/** Fixed PDAs of the programs (programs-summary §5), with the seeds each is derived from. */
export const FIXED_ADDRESSES = {
  /** `["__event_authority"]` under the token program. */
  tokenEventAuthority: 'DDC3wgjnqERxZxZfnpPp85bMxtENvmuUULwbr9DeS61R',
  /** `["__event_authority"]` under the DEX. */
  swapEventAuthority: 'H9iWcdJusEqJbe48kFTxoi4Txukv6b3k4WMSDBEk5v6q',
  /** `["config"]` under the DEX. */
  swapConfig: 'HnckfEpqfiSan7VurzXtFxmHHTsKmkD2dwepwTdkrF5Z',
  /** `["__event_authority"]` under the bridge. */
  bridgeEventAuthority: '3Pr3u2ZBAcu6iyQPPmmv6re2WtHWBd9XrnhsrpxYFxt3',
  /** `["config"]` under the bridge. */
  bridgeConfig: 'ESF7JpQnFjY4fnpyLU9untb2Lk2iERdRUYkKDT39JeXD',
  /** `["wrapper", NATIVE_MINT]` under the bridge. */
  solWrapper: 'cV8fnD5stDwDKfurkDSgoZyH3ET5wEt13D7nvsqVanH',
  /** `["sol-vault"]` under the bridge: the lamports behind bridged SOL. */
  solVault: '892C8jwbdsmYHjGFxEbFF2Erzk8xUuxQr4YtPJ6E639G',
  /** `["wrapped", NATIVE_MINT]` under the bridge: bridged SOL, the quote of every launch (no hook). */
  bridgedSolMint: '7YMXcZ3AUD5pBceT4QzM2hrApoH3rEmPZUAzpKPHVvR4',
  /** `["__event_authority"]` under the launchpad. */
  launchEventAuthority: 'HyVKmTcrhRJa93tV4RGsBAGzntUnogfZPHRhy9eCVLp5',
  /** `["hook-authority"]` under the launchpad: it creates launch pools and finalizes their curves. */
  launchHookAuthority: 'FSHhimQGTZQHenfuRStNhNsgwuBWBLiPhGhWc2tgEKW',
  /** `["config"]` under the launchpad. */
  launchConfig: 'CQDSEYEXuKA8MgBXpuS6D3Tyvi5iDe5hK9ifxcCyuNYh',
  /** `["__event_authority"]` under the kit. */
  kitEventAuthority: 'Eny7mnBmodv7DaU8Qe5ppkfxW5NZSQfsKASKrbcCbRkL',
  /** `["hook-authority"]` under the kit: it signs the token program's `write_hook_data` in `claim`. */
  kitHookAuthority: 'Fu5LjHRW9e9tqn3Mhem3frUPDJxTzGf5ZPiBTnYKtsYX',
  /** `["__event_authority"]` under the companion program. */
  companionEventAuthority: '3qEasVHTNuKNzVTuuWfN4wzUyDHpjptSRktbKZsdCzPt',
  /** `["__event_authority"]` under the armory. */
  armoryEventAuthority: 'CGkhzUyHFrXdDuKj6D54p2UCDCR5XGgFQ2JYryLyQJvz',
  /** `["armory"]` under the armory: signs every CPI into hookwars_items (02 2.2). */
  armorySigner: '2NSNJ4W51G5ZZSc4wVqkjyR8yr5yzuEjquKv7iprzSHP',
  /** `["__event_authority"]` under hookwars_items. */
  itemsEventAuthority: 'HB36BhdS9w3QwNehSXTpY8zhjE3eBNddTWA2maz1oNRD',
  /** `["__event_authority"]` under hookwars_war. */
  warEventAuthority: 'EBF8RneDiFpweZLxk4Vy1yprQSomUY9QAJqX5ciYwdES',
  /** `["war-config"]` under war (05 2.1). */
  warConfig: '8ZHH2dqdJirSbrPGR7fNDU7D8wPjLaSJ3XWRU6AJrk71',
  /** `["war-signer"]` under war: the only caller Raid accepts on touch (04 2.10). */
  warSigner: 'HCBAfMMN6JMUfnJ64H4Kxs6r3gaTpJCeFr5dyrDMENoe',
  /** `["loot-signer"]` under war: the only caller mint_loot accepts (00 4.3). */
  lootSigner: 'DK5PUA6578wDF96DAEqxqiUDypvqPvdaoYASisgyPuZv',
  /** `["prize-vault"]` under war: the DEX fee collector (R14). */
  prizeVault: '9jg1hh6jm21yWkp9NmdvbPNFeKd5wNGgKRYZDRiBbcY',
} as const;

/**
 * The protocol lookup table (docs/hooks-v2.md §6; programs-summary §2.7): the fixed addresses (22 upstream, 11 appended by Hookwars)
 * no top-level instruction invokes, in the order the programs' tests load them and `pnpm admin
 * init` must write them (a v0 message names table entries by index). Programs a transaction
 * invokes at top level (the kit for claims and shares) are kept in the static keys by the v0
 * compiler. Per-mint accounts cannot be in it.
 */
export const PROTOCOL_LOOKUP_TABLE_ADDRESSES: readonly string[] = [
  FIXED_ADDRESSES.tokenEventAuthority,
  HOOK_SIGNERS.tokenForKit,
  FIXED_ADDRESSES.swapEventAuthority,
  HOOK_SIGNERS.dexForLaunch,
  FIXED_ADDRESSES.swapConfig,
  FIXED_ADDRESSES.bridgeEventAuthority,
  FIXED_ADDRESSES.bridgeConfig,
  FIXED_ADDRESSES.solWrapper,
  FIXED_ADDRESSES.solVault,
  FIXED_ADDRESSES.launchEventAuthority,
  FIXED_ADDRESSES.launchHookAuthority,
  FIXED_ADDRESSES.launchConfig,
  FIXED_ADDRESSES.kitEventAuthority,
  FIXED_ADDRESSES.kitHookAuthority,
  PROGRAM_IDS.kit,
  FIXED_ADDRESSES.bridgedSolMint,
  SYSTEM_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  // Appended 2026-10-08 for launches through a companion (docs/companions.md): the companion's event
  // authority and the programs it invokes, so its launch transaction fits (1,142 bytes instead of 1,266).
  FIXED_ADDRESSES.companionEventAuthority,
  PROGRAM_IDS.launch,
  PROGRAM_IDS.swap,
  PROGRAM_IDS.token,
  // Appended by Hookwars (docs/spec/06-app.md 4.1): fixed addresses of the new programs. Entries are
  // only ever appended; the SDK checks the first entries, never the length.
  FIXED_ADDRESSES.armoryEventAuthority,
  FIXED_ADDRESSES.armorySigner,
  FIXED_ADDRESSES.itemsEventAuthority,
  HOOK_SIGNERS.tokenForItems,
  HOOK_SIGNERS.launchForItems,
  FIXED_ADDRESSES.warEventAuthority,
  FIXED_ADDRESSES.warConfig,
  FIXED_ADDRESSES.warSigner,
  FIXED_ADDRESSES.lootSigner,
  FIXED_ADDRESSES.prizeVault,
  PROGRAM_IDS.items,
];

/** Token hook flags (`Mint.hookFlags`). */
export const TOKEN_HOOK_FLAGS = {
  BEFORE_TRANSFER: 1 << 0,
  AFTER_TRANSFER: 1 << 1,
  BEFORE_MINT: 1 << 2,
  AFTER_MINT: 1 << 3,
  BEFORE_BURN: 1 << 4,
  AFTER_BURN: 1 << 5,
  TRANSFER_RETURNS_DELTA: 1 << 6,
  /** v2: the hook may write the 64 bytes of hook data each holding keeps. */
  WRITES_HOOK_DATA: 1 << 7,
} as const;

/** Every token hook flag (`token_flags::ALL`). */
export const TOKEN_HOOK_FLAGS_ALL = 255;

/** Pool hook flags (`Pool.hookFlags`). */
export const POOL_HOOK_FLAGS = {
  BEFORE_INITIALIZE: 1 << 0,
  AFTER_INITIALIZE: 1 << 1,
  BEFORE_ADD_LIQUIDITY: 1 << 2,
  AFTER_ADD_LIQUIDITY: 1 << 3,
  BEFORE_REMOVE_LIQUIDITY: 1 << 4,
  AFTER_REMOVE_LIQUIDITY: 1 << 5,
  BEFORE_SWAP: 1 << 6,
  AFTER_SWAP: 1 << 7,
  /** v2: also allows the answer's `burn`. */
  BEFORE_SWAP_RETURNS_DELTA: 1 << 8,
  /** v2: also allows the answer's `burn`. */
  AFTER_SWAP_RETURNS_DELTA: 1 << 9,
  BEFORE_SWAP_OVERRIDES_FEE: 1 << 10,
} as const;

/** Every pool hook flag (`pool_flags::ALL`). */
export const POOL_HOOK_FLAGS_ALL = 2_047;

/** A launch pool's flags (`LAUNCH_HOOK_FLAGS`): before initialize, before and after swap, both deltas, the fee override: 1985. */
export const LAUNCH_POOL_HOOK_FLAGS =
  POOL_HOOK_FLAGS.BEFORE_INITIALIZE |
  POOL_HOOK_FLAGS.BEFORE_SWAP |
  POOL_HOOK_FLAGS.AFTER_SWAP |
  POOL_HOOK_FLAGS.BEFORE_SWAP_RETURNS_DELTA |
  POOL_HOOK_FLAGS.AFTER_SWAP_RETURNS_DELTA |
  POOL_HOOK_FLAGS.BEFORE_SWAP_OVERRIDES_FEE;

/** Bytes of hook data every holding keeps (`HOOK_DATA_LEN`). */
export const HOOK_DATA_LEN = 64;

/** Names of the flags, for the docs and the token page. */
export function describeHookFlags(flags: number, table: Record<string, number>): string[] {
  return Object.entries(table)
    .filter(([, bit]) => (flags & bit) !== 0)
    .map(([name]) => name.toLowerCase().replace(/_/g, ' '));
}
