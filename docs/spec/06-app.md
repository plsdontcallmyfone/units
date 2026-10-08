# Hookwars spec 06: the app (indexer, API, SDK, site, bots)

Status: specification, 2026-10-08. Nothing here is built. Follows `00-overview.md` (the contract):
its names, seeds, slot kinds, equip rules, money words and parameters. Every number this part uses
is a named parameter from 00 section 6 or an upstream figure cited to its source.

Upstream references, kept unless this part says otherwise:
- `docs/architecture.md` "Off chain": the browser talks only to the site's own `/api` routes; the
  site's server proxies a backend with a server-only key; the backend holds Postgres, the indexer
  and the transaction builders; the chain is read through an RPC provider.
- `docs/hooks-v2.md` section 6 (transactions), 7.2 (one name per rule), 7.3 (what the site says),
  7.4 (surfaces), 8 (shared types, backend).
- `bordrless-sdk` (`packages/shared/src/api.ts`, `packages/sdk/src/{addresses,hooks,events,transactions}.ts`,
  `docs/integration/01..07`).

The upstream web app and backend are not in the public repositories; Hookwars writes its own,
in the same shape: `apps/web` (Next.js), `apps/server` (backend, indexer, keeper),
`packages/shared` (API types, policy math pinned to Rust by fixture vectors), `packages/sdk`
(instruction builders, decoders, account resolution).

## 1. What the app must never do

These hold on every surface, every bot post and every share card.

1. **No projections.** No yield, APR, APY, "earn per day", projected royalties, projected bounty or
   "win chance". Royalties, bounties, holder rewards and war chest balances are realized figures
   read from events or accounts (upstream 7.3: "Show realized figures only").
2. **No outcome bets.** The app offers nothing that pays on who wins a war, a siege or a season.
   Seasons are shown as a table of on-chain counters and the on-chain winner, nothing more. The
   words "bet", "odds", "payout on outcome" never appear (00 section 4.5).
3. **No invented figures.** A figure the backend could not read is `null` and renders as a dash,
   never zero (upstream `api.ts` header rule). No placeholder stats, no sample tokens, no demo
   wars. Empty states say what is empty and what would fill it.
4. **No unverified code labelled verified.** An item is "Template item" only when its template is
   registered in the armory and the template program's on-chain code hash equals the recorded
   `code_hash` (02). Anything else in a slot (a `Locked` slot holding a program that is neither the
   kit nor a registered template) is labelled exactly as upstream labels a custom hook: "Custom hook,
   unverified", with the program id, its upgrade authority or "fixed", and the sentence "Written by
   the creator, not Hookwars. It can refuse transfers or take part of them." (upstream 7.4).
5. **No custody.** The backend builds transactions, the wallet signs, the backend sends (upstream
   architecture.md). The backend holds no user key. The keeper key signs only permissionless cranks
   and pays its own fees.
6. **Spot only.** Every action the app builds is a swap, a transfer, a claim, a vote lock, an equip
   crank, a war crank, a roll, a forge or a quest claim.
7. **No em dashes, no monospace.** Sans fonts only (Geist), labels, numbers and addresses included.

## 2. Indexer

### 2.1 Cursors and decoding

One signature cursor per program, oldest first, as upstream (architecture.md "Indexer"):

| Program | Cursor key |
| --- | --- |
| `bordrless_token` | `token` |
| `bordrless_swap` | `swap` |
| `bordrless_launch` | `launch` |
| `bordrless_kit` | `kit` |
| `bordrless_bridge` | `bridge` |
| `bordrless_companion` | `companion` |
| `hookwars_armory` | `armory` |
| `hookwars_items` | `items` |
| `hookwars_war` | `war` |

- Events are Anchor event CPIs, decoded from `meta.innerInstructions` with the loaded addresses of
  every lookup table included in the key list (upstream integration 03). Hookwars transactions load
  from the protocol table **and** the mint's own table (section 4.2), so both tables' loaded
  addresses go into the key list.
- Failed transactions are skipped (`meta.err`), as upstream.
- Rows are keyed by `(signature, ordinal)` where `ordinal` is the event's position in the
  transaction. Re-indexing is idempotent.
- A transaction touching several programs is decoded once per cursor that sees it; the unique key
  makes the duplicate a no-op.
- Each decoded row stores `slot` and the block time; events that carry no time (upstream's kit
  events do not) take the block's.
- Program info (upgrade authority, upgradeable, executable hash) is read from ProgramData for every
  program in 00 section 3 and every template program, cached, refreshed on a timer and on any
  `TemplateRegistered` event.

### 2.2 Holdings and hook data

Upstream `token.HookDataWritten { mint, holding, owner, data }` carries a holding's full 64 bytes
whenever a hook writes them (`packages/sdk/src/events.ts`). The indexer keeps `holding_hook_data`
current from that event alone; it never polls holdings for hook data. Each row is split into slot
ranges using the mint's slot table (`data_offset`, `data_len` from 01), and each range is decoded
with its template's layout (04): raid points, loot tickets, age, quest marks.

Balances come from `token.Transferred` post-balances with a slot guard (upstream).

### 2.3 Tables

Postgres. Amounts are `numeric(39,0)` (u128 headroom), addresses `text`, times `timestamptz` plus
the raw unix seconds. Every event table has `signature`, `ordinal`, `slot`, `ts`.

**Kept from upstream (shape unchanged):** launches, pools, swaps (`Swapped`), transfers, holders,
candles, kit rewards, configs, bridge wrappers, companions, protocol fees.

**Changed:**

| Table | Change |
| --- | --- |
| `swaps` | adds `route jsonb` (the hops of a multi-hop swap, from 03's route context in `Swapped`), `route_from_mint` (the first hop's input mint when it is not bridged SOL), `raid_against` (the rival mint when a Raid item marked the swap, from 05's mark event) |
| `transfers` | adds `slot_cuts jsonb`: per slot index, the cuts it answered and their recipients (from 01's `Transferred` extension) |
| `launches` | adds `slot_table jsonb` (kind, bounds, equip rule, notice, data range per slot, as created), `mint_lookup_table` (section 4.2), `war_chest`, `war_bps` |

**New:**

| Table | Rows from | Key columns |
| --- | --- | --- |
| `templates` | `TemplateRegistered` | `template_id`, `program`, `code_hash`, `kind`, `ceiling jsonb`, `data_len`, `registered_by`, `verified_now bool` (code hash rechecked, section 2.1) |
| `items` | `ItemCreated`, `LootMinted`, `Forged`, item-mint transfers | `item`, `item_mint`, `template_id`, `params jsonb`, `kind`, `author`, `owner` (from the item mint's holder), `royalty_bps`, `level`, `origin` (`registered` / `loot` / `forge`), `burned bool` |
| `item_stats` | `Transferred` / `Swapped` deltas into royalty holdings, `RoyaltyClaimed` | `item`, `runs`, `collected` per mint, `royalty_accrued`, `royalty_claimed`, `equipped_count` |
| `slots` | `SlotsInitialized`, `SlotEquipped` | `mint`, `slot`, `kind`, `bounds jsonb`, `equip_rule`, `notice_secs`, `data_offset`, `data_len`, `item`, `program`, `equipped_at`, `pending_item`, `effective_at` |
| `proposals` | `ProposalCreated`, `ProposalResolved` | `proposal`, `mint`, `slot`, `nonce`, `item`, `proposer`, `opens_at`, `closes_at`, `effective_at`, `status` (`open` / `passed` / `failed` / `applied` / `cancelled`), `votes_for`, `votes_against`, `locked_total` |
| `vote_locks` | `VoteLocked`, `VoteUnlocked` | `proposal`, `voter`, `amount`, `side`, `unlock_at`, `released bool` |
| `performance_checks` | `PerformanceReverted`, failed-condition cranks are not events | `mint`, `slot`, `from_item`, `to_item`, `reason jsonb` |
| `royalty_claims` | `RoyaltyClaimed` | `item`, `owner`, `mint`, `amount` |
| `treaties` | derived from `slots` | `treaty_item`, `party_a`, `party_b`, `active bool` (both sides equip it), `since`, `terms jsonb` |
| `war_chests` | `WarChestCreated`, `WarFunded`, every spend event | `mint`, `chest`, `sol_balance` (read from the account after each event), `funded_total`, `spent_total` |
| `war_state_snapshots` | after every war event, the `WarState` account read at that slot | `mint`, `slot`, `state jsonb` (raid volume per rival in the window, sieges won, captured holdings, season counters) |
| `raids` | 05's raid mark event (per raid swap) | `mint` (the token bought, whose Raid item marked the swap), `rival` (the token sold), `trader`, `volume_lamports`, `points`, `loot_ticket bool` |
| `sieges` | `SiegeExecuted` | `attacker_mint`, `target_mint`, `spent_lamports`, `tokens_captured`, `cranker`, `bounty` |
| `counter_strikes` | `CounterStrikeExecuted` | `mint`, `spent_lamports`, `tokens_burned`, `trigger jsonb`, `cranker`, `bounty` |
| `razes` | `Razed` | `holder_mint`, `target_mint`, `tokens_sold`, `lamports_out` |
| `captured` | derived: sieges minus razes minus returns | `holder_mint`, `target_mint`, `amount`, `share_of_supply` |
| `captured_returns` | `CapturedReturned` | `from_mint`, `to_mint`, `amount`, `treaty_item` |
| `bounties` | `BountyClaimed` | `mint`, `owner`, `points_spent`, `lamports` |
| `loot_rolls` | `RollRequested`, `RollRevealed` | `roll`, `holding`, `owner`, `mint`, `season`, `requested_slot`, `revealed_slot`, `item` (null until revealed), `template_id`, `params` |
| `forges` | `Forged` | `input_a`, `input_b`, `output`, `owner`, `template_id`, `level` |
| `quests` | `QuestClaimed` | `quest_id`, `owner`, `mint`, `season`, `reward` (a loot ticket) |
| `seasons` | `SeasonOpened`, `SeasonFinalized`, `PrizePaid` | `number`, `starts_at`, `ends_at`, `challenge_ends_at`, `score_formula jsonb`, `winner_mint`, `prize_lamports` |
| `season_candidates` | `CandidateSubmitted`, `CandidateChallenged` | `season`, `mint`, `score`, `submitted_by`, `beaten_by` |
| `holding_hook_data` | `HookDataWritten` | `holding`, `mint`, `owner`, `data bytea`, decoded `ranges jsonb` |

Note on `raids`: the raiding token is the one being **bought** (its Raid item lowers the fee for
holders of the rival who sell into it). A row names that token (`mint`), the rival whose holders were
raided (`rival`), the trader, the SOL volume and the points stamped. Column names follow 05's mark
event once 05 fixes them (Interfaces).

### 2.4 Derived views

Computed from the tables above, never stored as claims:

- **War map.** Nodes: launches with at least one war-relevant event or relation slot. Edges, each
  with its source rows:
  - `treaty` (both parties equip the treaty item: `treaties.active`);
  - `tribute` (a one-sided treaty item: payer to receiver);
  - `rivalry` (a Spy or Raid item names the other mint in its params);
  - `raid` (raid rows in the current window: weight = SOL volume, direction from the raiding token to the rival);
  - `siege` and `captured` (captured holdings: holder to target, weight = share of target supply).
- **Generals.** Per mint, holders ranked by raid points decoded from `holding_hook_data` (the Raid
  template's range, 04), ties broken by raid volume, then by earliest point. A general is a site
  label only; it grants nothing on chain (WARFARE.md section 5). The list excludes pool vaults,
  launch PDAs, war chests and royalty owners.
- **Item leaderboards.** By royalties claimed plus accrued (realized), by runs, by tokens equipped.
  Per template and overall.
- **Siege progress.** For each Siege item equipped: the raid volume from its rival in the current
  window (read from the latest `WarState`) against the item's threshold parameter. A bar from two
  on-chain numbers; no estimate of when it fills.
- **Season table.** Each token's counters from its latest `WarState` and the published score
  formula of the season (on chain, 05), computed exactly as the program does; the leading candidate
  and any open challenge from `season_candidates`. Pinned to the Rust formula by fixture vectors in
  `packages/shared`.

## 3. API

### 3.1 Conventions

Unchanged from upstream `api.ts`: `Address` and `Amount` (decimal string of base units), unix
seconds, `null` for anything not read, `PreparedTx` and the submit flow, `ApiErrorBody`. All new
types live in `packages/shared/src/api.ts` beside upstream's.

### 3.2 Types

```ts
export type SlotKind = 'fee' | 'reward' | 'defense' | 'relation' | 'pool' | 'locked';
export type EquipRule = 'locked' | 'vote' | 'performance';
export type ItemOrigin = 'registered' | 'loot' | 'forge';

/** What a slot allows, fixed at launch (00 section 5, rule 2). */
export interface SlotBounds {
  maxCutBuyBps: number;
  maxCutSellBps: number;
  maxCutTransferBps: number;
  mayRefuse: boolean;
  mayBurn: boolean;
  maySetFee: boolean;
}

/** What an item claims it may do; always within its slot's bounds once equipped. */
export interface ItemManifest extends SlotBounds {
  hookDataBytes: number;
  /** Other accounts it reads (pools of other tokens for Spy and Raid, treaty counterparties). */
  reads: Address[];
}

export interface TemplateInfo {
  templateId: number;
  name: string;                       // 'Raid', 'Shield', 'Wall', 'Spy', 'Treaty', 'Tribute', ...
  program: Address;
  kind: SlotKind;
  ceiling: Record<string, string>;    // parameter name -> ceiling, as on chain
  codeHash: string;
  /** The on-chain code hash still equals `codeHash`; null when not checked yet. */
  verified: boolean | null;
  upgradeAuthority: Address | null;
  upgradeable: boolean | null;
  items: number;
}

export interface ItemSummary {
  item: Address;
  itemMint: Address;
  templateId: number;
  templateName: string;
  kind: SlotKind;
  params: Record<string, string>;
  manifest: ItemManifest;
  level: number;
  origin: ItemOrigin;
  author: Address;
  owner: Address | null;              // null while it could not be read
  royaltyBps: number;
  runs: Amount | null;
  royaltyAccrued: Amount | null;      // realized, lamports-equivalent per mint listed in `royaltyByMint`
  royaltyClaimed: Amount | null;
  royaltyByMint: { mint: Address; symbol: string; accrued: Amount; claimable: Amount }[];
  equippedOn: { mint: Address; symbol: string; slot: number }[];
}

export interface SlotInfo {
  slot: number;
  kind: SlotKind;
  bounds: SlotBounds;
  equipRule: EquipRule;
  noticeSecs: number;
  dataRange: { offset: number; len: number };
  item: ItemSummary | null;
  /** The program in the slot when it is not a registered template item (a locked kit or custom hook). */
  program: { address: Address; label: 'kit' | 'half-life' | 'custom'; upgradeAuthority: Address | null; upgradeable: boolean | null } | null;
  pending: { item: ItemSummary; effectiveAt: number; proposal: Address | null } | null;
  /** Performance rule as stored on chain, rendered as a sentence by the site. */
  performanceRule: Record<string, string> | null;
}

export interface ProposalInfo {
  proposal: Address;
  mint: Address;
  slot: number;
  item: ItemSummary;
  proposer: Address;
  opensAt: number;
  closesAt: number;
  effectiveAt: number | null;
  status: 'open' | 'passed' | 'failed' | 'applied' | 'cancelled';
  votesFor: Amount;
  votesAgainst: Amount;
  quorum: Amount;                     // from VOTE_QUORUM_BPS and the supply at creation, as the program computes it
  myLock: { amount: Amount; side: 'for' | 'against'; unlockAt: number } | null;
}

export interface TreatyInfo {
  treatyItem: Address;
  partyA: { mint: Address; symbol: string; equipped: boolean };
  partyB: { mint: Address; symbol: string; equipped: boolean };
  active: boolean;
  terms: Record<string, string>;
  since: number | null;
  paidAToB: Amount | null;            // realized
  paidBToA: Amount | null;
}

export interface WarChestInfo {
  mint: Address;
  chest: Address;
  warBps: number;
  solBalance: Amount | null;
  fundedTotal: Amount | null;
  spentTotal: Amount | null;
  captured: { mint: Address; symbol: string; amount: Amount; shareOfSupply: number | null }[];
  bountyRate: Amount | null;          // lamports per point, as posted on chain
}

export interface WarStateInfo {
  mint: Address;
  windowStartsAt: number | null;
  raidVolumeByRival: { rival: Address; symbol: string; volume: Amount }[];
  siegesWon: number;
  seasonCounters: Record<string, string>;
  readAtSlot: number;
}

export interface SiegeProgress {
  mint: Address;
  rival: Address;
  rivalSymbol: string;
  volume: Amount;
  threshold: Amount;                  // the Siege item's parameter
  nextAllowedAt: number | null;       // SIEGE_INTERVAL_SECS after the last siege
}

export type BattleKind = 'raid' | 'siege' | 'counter_strike' | 'raze' | 'treaty_on' | 'treaty_off' | 'equip' | 'proposal' | 'bounty' | 'loot' | 'forge' | 'season';
export interface BattleEvent {
  kind: BattleKind;
  ts: number;
  signature: string;
  mint: Address;
  otherMint: Address | null;
  actor: Address | null;
  amount: Amount | null;
  detail: Record<string, string>;
}

export interface MyHoldingWar {
  mint: Address;
  raidPoints: Amount | null;
  raidVolume: Amount | null;
  lootTickets: number | null;
  bountyClaimable: Amount | null;     // points times the posted rate, capped by BOUNTY_MAX_PER_CLAIM, as the program computes
  quests: { questId: number; done: boolean; claimable: boolean }[];
}

export interface General {
  owner: Address;
  raidPoints: Amount;
  raidVolume: Amount;
  rank: number;
}

export interface LootRollInfo {
  roll: Address;
  status: 'requested' | 'revealed' | 'expired';
  requestedAt: number;
  item: ItemSummary | null;
}

export interface SeasonInfo {
  number: number;
  startsAt: number;
  endsAt: number;
  challengeEndsAt: number;
  scoreFormula: Record<string, string>;
  table: { mint: Address; symbol: string; score: Amount; counters: Record<string, string> }[];
  candidate: { mint: Address; score: Amount; submittedBy: Address } | null;
  winner: { mint: Address; prize: Amount | null } | null;
}

export interface RaidLink {
  /** The token being bought. */
  target: Address;
  /** The rival sold to buy it. */
  from: Address;
  /** Optional: who called it (a general's wallet) for attribution on the site; never on chain. */
  caller: Address | null;
}
```

Upstream types gain:

```ts
// LaunchSummary gains: slots: SlotInfo[]; warChest: Address | null; warBps: number | null;
//   relations: { treaties: number; rivals: Address[] }; raidVolume24h: Amount | null;
// LaunchDetail gains: war: WarStateInfo | null; chest: WarChestInfo | null; treaties: TreatyInfo[];
//   proposals: ProposalInfo[]; siege: SiegeProgress[];
// SwapQuoteRequest gains: from?: Address;   // a rival mint: route rival -> bridged SOL -> target
// SwapQuote gains:
//   route: Address[];                         // mints in hop order
//   raid: { rival: Address; feeBps: number; points: Amount; lootTicket: boolean } | null;
//   slotCuts: { slot: number; templateName: string; amount: Amount; to: 'royalty' | 'chest' | 'holders' | 'furnace' | 'other' }[];
//   shieldFeeBps: number | null;              // a sell that a Shield would charge more, with the reason
// LaunchPrepareRequest gains:
//   slots: { kind: SlotKind; bounds: SlotBounds; equipRule: EquipRule; noticeSecs: number; dataLen: number;
//            item: Address | null; performanceRule: Record<string, string> | null }[];
//   warBps: number;                           // 0..WAR_BPS_MAX
// PortfolioEntry gains: war: MyHoldingWar | null;
// Portfolio gains: items: ItemSummary[]; royaltiesClaimable: { item: Address; mint: Address; amount: Amount }[];
//   rolls: LootRollInfo[];
// Overview gains: programs.armory, programs.items, programs.war; programInfo for each;
//   season: { number: number; endsAt: number } | null;
```

### 3.3 Routes

Reads (GET):

| Route | Returns |
| --- | --- |
| `/v1/templates` | `TemplateInfo[]` |
| `/v1/items?template=&kind=&owner=&equipped=&sort=royalties\|runs\|equipped\|new` | `{ items: ItemSummary[]; next: string \| null }` |
| `/v1/items/:item` | `ItemSummary & { history: BattleEvent[] }` |
| `/v1/launches/:mint/slots` | `SlotInfo[]` |
| `/v1/launches/:mint/proposals?status=` | `ProposalInfo[]` |
| `/v1/launches/:mint/war` | `{ state: WarStateInfo \| null; chest: WarChestInfo \| null; siege: SiegeProgress[] }` |
| `/v1/launches/:mint/generals` | `General[]` |
| `/v1/launches/:mint/treaties` | `TreatyInfo[]` |
| `/v1/map?window=current` | `{ nodes: { mint; symbol; image; chest: Amount \| null }[]; edges: { kind; from; to; weight: Amount \| null; since: number \| null }[] }` |
| `/v1/feed?mint=&kind=&before=` | `{ events: BattleEvent[]; next: string \| null }` |
| `/v1/seasons/current`, `/v1/seasons/:n` | `SeasonInfo` |
| `/v1/quests` | quest definitions as on chain (05), each with its on-chain check described in one sentence |
| `/v1/wallet/:owner/war` | `{ holdings: MyHoldingWar[]; items: ItemSummary[]; royalties: ...; rolls: LootRollInfo[] }` |
| `/v1/cards/:signature/:ordinal.png` | a share card (section 7) |

Prepares (POST, each returns `{ transactions: PreparedTx[] }`):

| Route | Builds |
| --- | --- |
| `/v1/raid/prepare` | the multi-hop swap rival -> bridged SOL -> target (03), with every hop's hook slices, the SOL unwrap when the user chose SOL |
| `/v1/proposals/prepare` | `propose` for a slot and item (02) |
| `/v1/votes/prepare` | `lock_vote` / `unlock_vote` |
| `/v1/equip/prepare` | `apply_equip` after notice, or `check_performance`; permissionless, any wallet may send |
| `/v1/royalties/prepare` | `claim_royalty` for up to N item and mint pairs per transaction (N from measurement, as upstream packs about 4 claims) |
| `/v1/items/register/prepare` | `create_item` from a registered template and parameters |
| `/v1/forge/prepare` | `forge(item_a, item_b)` |
| `/v1/rolls/prepare` | `roll` (spends a ticket) and, once the oracle has answered, `reveal` |
| `/v1/quests/prepare` | `touch` plus `claim_quest` |
| `/v1/bounties/prepare` | `touch` plus `claim_bounty` |
| `/v1/war/crank/prepare` | `siege`, `counter_strike`, `raze`, `return_captured`, `submit_candidate`, `challenge`, `finalize_season`: whichever is due; the program re-checks |
| `/v1/launch/prepare` | upstream's launch, plus the slot table, `war_bps`, the war chest and the mint's lookup table (section 4.2) |

Every prepare simulates first and refuses with the program's error, translated into one sentence
that names the rule and the time it lifts when there is one (upstream: "Errors sit next to the
button").

## 4. Transactions

### 4.1 Rules (kept)

- v0 messages only, with the protocol lookup table, checked by `checkProtocolLookupTable` (upstream
  `transactions.ts`). Hookwars extends the protocol table with the new programs' fixed addresses
  (config PDAs, event authorities, hook signers for `hookwars_items`); new entries are only ever
  appended, and the SDK checks the first entries, never the length (upstream rule).
- Programs invoked at top level stay in static keys (a v0 message cannot load an invoked program
  from a table).
- The backend simulates each prepared transaction (signatures not verified, blockhash replaced) and
  requests the units used plus 15% (upstream hooks-v2 section 6). Fallback budgets when simulation
  fails come from 07's measured figures, never from guesses here.
- Built by the backend, signed by the wallet, sent by the backend, followed until it lands or
  expires (upstream architecture.md).
- A fresh mint keypair for every launch attempt; a broadcast mint key is never re-signed (upstream
  hooks-v2 1.5).

### 4.2 The mint's lookup table

A transfer or swap of a slotted token carries, per equipped item, the item's program, the token
program's hook signer for it, and the item's extra accounts (01, 02). Upstream's launch with one
custom hook is already about 1,200 of 1,232 bytes (half_life README, "Launching with it").

- **Created at launch.** `launch/prepare` adds, after the launch lands, a transaction that creates
  an address lookup table owned by the backend's table authority key and extends it with every
  per-mint address a trade needs: the launch, pool, vaults, kit config and vault, war state, war
  chest, and each equipped item's account, registry and extras.
- **Extended on equip.** `equip/prepare` appends the incoming item's addresses before the
  `apply_equip` transaction. Addresses are appended, never removed; the table is never closed while
  the token trades.
- **Warm-up.** A table extended in slot `s` is usable from slot `s + 1`; the backend waits for the
  extension to be confirmed before building trades that need it.
- **Recorded.** `launches.mint_lookup_table` holds its address; the API returns it so third-party
  terminals can use it. A client without it can still build a transaction if it fits.
- **Who pays.** The table's rent is paid by the launch transaction's payer as part of the launch,
  shown on the launch review ("Plus the rent of the token's new accounts", upstream launch page).

### 4.3 Slot-aware hook account resolution (SDK)

Upstream resolves one hook slice per mint: `[hook program, token hook signer, registry extras...]`
(`resolveHookAccounts`, `fetchTokenHook`). Hookwars resolves **one slice per equipped slot**, in slot
order:

```ts
export interface SlotSlice {
  slot: number;
  program: PublicKey;
  signer: PublicKey;                 // tokenHookSigner(program)
  extras: AccountMeta[];             // from the item's registry for this mint, resolved as upstream
}
export interface TokenHookSlices { mint: PublicKey; slices: SlotSlice[]; counts: number[] }

/** Reads the mint's slot table and every equipped item's registry; resolves prefix-dependent seeds. */
export function fetchTokenHookSlices(connection: Connection, args: {
  mint: PublicKey; source: PublicKey; destination: PublicKey;
  authority: PublicKey; sourceOwner: PublicKey; destinationOwner: PublicKey;
}): Promise<TokenHookSlices>;

/** Flattens slices into remaining accounts in the order 01 fixes, with the per-slot counts the instruction takes. */
export function sliceAccounts(s: TokenHookSlices): { metas: AccountMeta[]; counts: number[] };
```

- A swap carries the input mint's slices, the output mint's slices and the pool hook's slice
  (upstream carries three lists); for a launch pool the pool slice includes the **pool-kind items**
  the launchpad forwards to (03), resolved the same way. A multi-hop swap carries this per hop.
- Duplicate keys collapse in the message, so several items of the one `hookwars_items` program cost
  their program key and signer once.
- `counts` is passed as instruction arguments, exactly as upstream passes "how many accounts each
  takes" (architecture.md "The extra-accounts registry").

### 4.4 SDK additions

One module per new program, mirroring upstream's layout (`addresses.ts`, `instructions.ts`,
`accounts.ts`, `events.ts`, `coders.ts` with the IDLs):

- **addresses:** every seed of 00 section 4.3: `itemAddress`, `templateAddress`, `royaltyOwner`,
  `royaltyHolding`, `slotAuthority`, `proposalAddress`, `voteLockAddress`, `warChestAddress`,
  `warStateAddress`, `seasonAddress`, `rollAddress`, `observationsAddress`; the new program ids.
- **decoders:** `decodeTemplate`, `decodeItem`, `decodeProposal`, `decodeVoteLock`,
  `decodeWarChest`, `decodeWarState`, `decodeSeason`, `decodeRoll`, `decodeObservations`, and the
  `Mint` decoder extended with the slot table (01); per-template range decoders
  (`raidRange(hookData, range)`, `ageRange`, `questRange`) as upstream's `halfLifeSince`.
- **builders:** `armory.*`, `items.*` (template-specific `prepare` where a template needs per-mint
  state), `war.*`, `token.touch`, `swap.swapRoute` (multi-hop), and the launch builder extended with
  slots and `war_bps`.
- **events:** `typedEvent` extended with every event in section 9's list.
- **policy math (`packages/shared`):** the slot merge (01), royalty split (02), quote with slot cuts
  and route fee (03, 04), siege and counter-strike caps (05), bounty claimable, season score; each
  pinned to the Rust reference by fixture vectors as upstream pins `launch-fees.json`.

## 5. Backend

Same shape as upstream: one server with the API, the indexer, the transaction builders, the
submitter and a keeper.

- **Keeper (permissionless cranks only).** Upstream's keeper sends `collect_protocol_fees_sol` and
  graduations. Hookwars adds, each only when the program would accept it (checked by simulation):
  `apply_equip` after notice, `check_performance`, `siege`, `counter_strike`, `reveal` for rolls
  whose oracle answer is in, `submit_candidate` and `finalize_season` after a season. The keeper
  never sends `raze` or `return_captured` on its own initiative: those are a community's decision
  (05) and the site exposes them as buttons. Bounties the keeper earns go to its own key; the
  amounts are recorded like anyone's.
- **Quote mirror.** Quotes include each slot's cut, the royalty inside it, a raid discount by route,
  a Shield's higher sell fee with its reason, and the war chest's share of the creator fee, matching
  the program exactly (fixture vectors). A Half-Life-style fee that depends on the sender's hook data
  is read from `holding_hook_data`, and a sell quote is simulated for the exact figure, as upstream
  integration 05 recommends for Half-Life.
- **Program info.** Upgrade authority and executable hash of every program and template, cached.
- **Ports.** Local development ports are claimed in this repository's `CLAUDE.md` before binding,
  after `lsof -ti :<port>` (machine rule); none is fixed by this part.

## 6. Site

Next.js, sans fonts only (Geist), no monospace anywhere (addresses included). Every figure comes
from the API; a null renders as a dash. Every empty state names what is missing.

### 6.1 Navigation

Projects, Launch, War room, Armory, Seasons, Marketplace, Portfolio, Bridge, Explorer, Docs.
Upstream's Studio stays and gains "Make an item".

### 6.2 War room (`/war`)

- **Map.** Nodes are tokens; edges are treaties, tribute, rivalries, live raids and captured
  holdings (section 2.4), each edge clickable to its source events. Window selector: current raid
  window, season. Empty: "No wars yet. A war starts when a token equips a Raid or Spy item naming
  another token." with a link to the Armory.
- **Battle feed.** `BattleEvent`s newest first, live: raids (attacker, defender, SOL volume),
  sieges (spent, captured), counter-strikes (spent, burned), razes, treaties on and off, equips and
  proposals, bounties, loot, forges, season results. Every row links its transaction.
- **Siege bars.** Per active Siege item: raid volume from its rival in the window against the item's
  threshold, and when the next siege is allowed. No forecast.
- **War chests.** Largest chests by SOL balance, each with funded and spent totals and captured
  holdings.
- **Season strip.** Current season, time left, leading candidate, open challenge.

### 6.3 Armory (`/armory`)

- **Templates.** Each registered template: what it does in one sentence, its kind, its ceiling per
  parameter, its program with "verified" (code hash matches) or the mismatch stated, its upgrade
  authority or "fixed".
- **Items.** Grid with filters (template, kind, level, origin, equipped or not, for sale) and sorts
  (royalties realized, runs, tokens equipped, newest). An item card: template, parameters in words
  ("Raid: buyers who sold BONKX pay 0%, 0.5% of each raid trade to the chest"), level, author,
  owner, royalty rate, royalties realized, tokens equipped on.
- **Item page.** Everything above plus history (equips, runs per day as counted from events,
  royalty claims, transfers), the marketplace listing, and "Propose for a slot" (picks a token where
  the item fits a slot's bounds).
- **Forge.** Pick two items of one template you own; the page shows both parameter sets and the
  result exactly as `forge` computes it (shared math), capped by the ceiling; one signature.
- **Loot.** Your tickets, "Roll" (spends one), your pending rolls with their status, and your
  revealed items. The season's drop table is shown as on chain (templates, parameter ranges, drop
  weights), labelled "drop table"; never "odds".
- **My items and royalties.** Items owned, royalties claimable per item and mint, "Claim all".

### 6.4 Token page (`/t/:mint`)

Upstream's page (chart, trade panel, holders, trades, rules panel, fee table, verify line), plus:

- **Slots panel.** One row per slot: kind, the item in it (template sentence, parameters, level,
  owner, royalty rate) or the program with its label, the slot's bounds in words, its equip rule and
  notice, and any pending change with the time it takes effect. Locked slots say "Locked at launch".
- **Proposals.** Open proposals with the item, votes for and against, quorum, time left, and "Lock
  to vote" (amount, side; shows when the lock ends). Closed proposals with their result.
- **Treaties.** Active and one-sided treaties, terms in words, realized amounts paid each way.
- **War.** The war chest (balance, funded, spent), captured holdings by token with share of supply,
  raid volume by rival in the window, siege bars, the generals list, and crank buttons for whatever
  is due (`raze` and `return_captured` only where the token's own rule allows the caller).
- **My holding.** Raid points, raid volume, loot tickets, bounty claimable, quests done and
  claimable, each with its button.
- **Join the raid.** When the token has a Raid item and the wallet holds the rival: "Sell RIVAL for
  TICKER: raid fee X%" with the quote breakdown (route, raid fee, points, whether a loot ticket is
  earned). One signature.
- **Trade breakdown** (upstream rule: it adds up) gains the slot cuts, the royalty inside each, the
  chest's share, the raid discount or Shield surcharge, each with who receives it.

### 6.5 Launch form (`/launch`)

Upstream's three steps (token, rules, review) become four:

1. Token (unchanged).
2. Rules (unchanged: presets, Custom, Build your own; the kit fills a `Locked` slot).
3. **Slots.** Up to `MAX_SLOTS`: per slot its kind, bounds, equip rule, notice (between
   `MIN_NOTICE_SECS` and `MAX_NOTICE_SECS`), data length, the item at launch (from the wallet's items
   or the armory, filtered by fit) or empty, and for `Performance` the rule's condition. `war_bps`
   (0 to `WAR_BPS_MAX`). The form shows the total cut per side across slots against the token-wide
   maximum and blocks above it. Each choice has one line of consequence.
4. Review: every permanent choice listed ("fixed at launch: the creator can't change these",
   upstream), what can change later (the items in non-locked slots, by their rules), the launch fee,
   rent including the lookup table, and the first buy.

### 6.6 Generals (`/t/:mint/generals`, `/generals`)

Per token, the ranked list (section 2.4) with points and raid volume. Global page: top generals
across tokens by raid volume. A general's page lists the raids they joined (from events). The page
states: "Generals are the top raiders by on-chain points. It is a title; it gives no control."

### 6.7 Quests (`/quests`)

Only quests whose condition the program checks (05): each with its condition in one sentence, the
reward (a loot ticket), and the wallet's state per token. Site-only badges, if any, are a separate
list labelled "Badges: no reward".

### 6.8 Seasons (`/seasons`)

Current season: table of tokens with their on-chain counters and the score computed by the
published formula; candidate and challenges; time to end and to the end of the challenge window;
the prize as "a share of Hookwars's protocol fees for the next season, paid to the winner's war
chest" with the realized amount after it is paid. Past seasons with winners and prizes paid.

### 6.9 Kept pages

Projects board (cards gain one slots line: "Raid vs BONKX · Shield · Holders 1%", a war chest
figure, and filters "at war", "has treaty", "raid targets"), Marketplace (items join configs as
listings), Portfolio (items, royalties, rolls, tickets, war holdings), Bridge, Explorer (new
account kinds: item, template, proposal, war chest, war state, season), Docs (the standard, slots,
items, relations, war, with the parameters printed from the chain).

## 7. Bots and share cards

### 7.1 Alerts

Telegram and X bots read the indexer's `BattleEvent` stream; they never read the chain directly
and never hold a user key.

| Trigger | Post |
| --- | --- |
| A raid row above a size the bot's operator sets (an operator setting, not a protocol parameter) | "TICKER is raiding RIVAL: X SOL in the last window. Join: <link>" |
| A siege bar crossing a share of its threshold the operator sets | "Siege on RIVAL at N% of threshold" |
| `SiegeExecuted` | "TICKER besieged RIVAL: spent X SOL, captured Y RIVAL" |
| `CounterStrikeExecuted` | "TICKER struck back: X SOL bought and burned" |
| Treaty activated or ended | "TICKER and OTHER signed: <terms>" |
| A general's call | posted only when the general signs the call message with their wallet on the site |
| Season finalized | "Season N: TICKER wins" |

The link is `/{target}/raid?from={rival}&caller={wallet?}` on the site: the page loads the quote
from `/v1/raid/prepare` and asks for one signature. Every post links the transaction it reports.
Rate limits and quiet hours are operator settings.

### 7.2 Share cards

`/v1/cards/:signature/:ordinal.png` renders a card from one indexed event and nothing else: "raided
RIVAL with X SOL", "won a siege", "forged a level N Shield", "claimed X SOL in bounties", "season N
winner". The card shows the transaction's short signature and the slot; a card for a signature the
indexer does not hold is a 404. No PnL, no projected figures.

## 8. Checks before this part is called done

- Re-index a devnet range from scratch twice: identical tables.
- The quote mirror equals simulation on every fixture: a buy and a sell with two slots cutting, a
  raid route, a Shield sell, a war split of the creator fee.
- Every page loads with an empty database and shows its empty states with no thrown error.
- A token with a non-template program in a locked slot shows "Custom hook, unverified" on every
  surface (board card, token page header and slots panel, trade panel, portfolio, explorer).
- A template whose on-chain hash no longer matches shows the mismatch and loses "verified".
- No page renders a monospace font (computed style check) and no file contains U+2014.
- Bots: a replayed event range produces the same posts once (idempotent by `(signature, ordinal)`).

## 9. Interfaces (what this part needs from 01 to 05)

Names this part assumes; the owning part fixes them, and this file follows.

**01 token slots**
- `Mint` decoder: slot table entries with `kind`, `bounds`, `equip_rule`, `notice_secs`,
  `data_offset`, `data_len`, `item`, `program`, `pending_item`, `effective_at`.
- Events: `SlotsInitialized { mint, slots }`, `SlotEquipped { mint, slot, item, program, by }`.
- `Transferred` extended with per-slot cuts (`slot_cuts: Vec<{ slot, deltas }>`) so royalty and
  chest receipts are attributable to a slot.
- `HookDataWritten` kept (full 64 bytes) and emitted on `touch` when data changes.
- The order of slices in remaining accounts and the `counts` argument of transfers and swaps.

**02 armory**
- Accounts: `Template`, `Item`, `Proposal`, `VoteLock`.
- Events: `TemplateRegistered`, `ItemCreated`, `RoyaltyClaimed`, `ProposalCreated`,
  `VoteLocked`, `VoteUnlocked`, `ProposalResolved`, `EquipApplied` (or 01's `SlotEquipped` alone),
  `PerformanceReverted`, `LootMinted`, `Forged`.
- How quorum is computed (which supply, when snapshotted) so `ProposalInfo.quorum` matches.
- How many royalty claims fit one transaction (measured, 07).

**03 DEX and launch**
- `Swapped` extended with `route` (hop mints) or a separate `RouteSwapped` event per multi-hop
  instruction; the pool-kind items' cuts inside `deltas_in` / `deltas_out` with their slot.
- `Observations` account layout and the TWAP read helper's math (mirrored in `packages/shared`).
- `LaunchCreated` extended with the slot table, `war_bps` and the war chest.
- Companion event for the war split (`WarFunded` or a field on the existing claim event).

**04 templates**
- Per template: id, name, kind, parameters with ceilings, manifest fields, hook-data layout (tag
  byte, fields, offsets) for `raidRange`, `ageRange`, `questRange` decoders, and the one-sentence
  description the site prints.
- Treaty and Tribute: how the two parties are named in params, so `treaties` can be derived.

**05 war**
- Accounts: `WarChest`, `WarState` (window start, raid volume per rival, sieges won, captured
  holdings, season counters), `Season`, roll request.
- Events: the raid mark (`RaidMarked { mint, rival, trader, volume, points, loot_ticket }`),
  `WarChestCreated`, `WarFunded`, `SiegeExecuted`, `CounterStrikeExecuted`, `Razed`,
  `CapturedReturned`, `BountyClaimed`, `RollRequested`, `RollRevealed`, `QuestClaimed`,
  `SeasonOpened`, `CandidateSubmitted`, `CandidateChallenged`, `SeasonFinalized`, `PrizePaid`.
- Who may send `raze` and `return_captured` (the site shows the buttons only to those callers).
- The season score formula's on-chain representation, so the site computes it identically.
- Quest definitions (ids, conditions) and how a roll request names its oracle account.
