# Hookwars spec 06: the app (indexer, API, SDK, site, bots)

Status: specification, 2026-10-08, revised after the integration rulings R1 to R18 (00 section 9).
Nothing here is built. Follows `00-overview.md` (the contract): its names, seeds, slot kinds, equip
rules, money words and parameters. Event and account names are the ones parts 01 to 05 fixed (R15);
where this file once proposed other names, it now uses theirs. Every number is a named parameter
from 00 section 6, a per-template ceiling named in 04, or an upstream figure cited to its source.

Upstream references, kept unless this part says otherwise:
- `docs/architecture.md` "Off chain": the browser talks only to the site's own `/api` routes; the
  site's server proxies a backend with a server-only key; the backend holds Postgres, the indexer
  and the transaction builders; the chain is read through an RPC provider.
- `docs/hooks-v2.md` section 6 (transactions), 7.2 (one name per rule), 7.3 (what the site says),
  7.4 (surfaces), 8 (shared types, backend).
- `bordrless-sdk` (`packages/shared/src/api.ts`, `packages/sdk/src/{addresses,hooks,events,transactions}.ts`,
  `docs/integration/01..07`).

The upstream web app and backend are not in the public repositories; Hookwars writes its own in the
same shape: `apps/web` (Next.js), `apps/server` (backend, indexer, keeper), `packages/shared` (API
types, policy math pinned to Rust by fixture vectors), `packages/sdk` (builders, decoders, account
resolution).

## 1. What the app must never do

These hold on every surface, every bot post and every share card.

1. **No projections.** No yield, APR, APY, "earn per day", projected royalties, projected bounty or
   "win chance". Royalties, bounties, holder rewards, treaty inflows and war chest balances are
   realized figures read from events or accounts (upstream 7.3: "Show realized figures only"). An
   unsettled royalty is shown as what `settle_equip` would pay now, computed by the program's own
   formula from balances on chain (04 2.5); that is a present amount, not a forecast.
2. **No outcome bets.** Nothing pays on who wins a war, a siege or a season. Seasons are a table of
   on-chain counters and the on-chain winner; the prize is a share of protocol fees already
   collected (05 10.4). The words "bet", "odds" and "payout on outcome" never appear (00 4.5).
3. **No invented figures.** A figure the backend could not read is `null` and renders as a dash,
   never zero (upstream `api.ts` header). No placeholder stats, sample tokens or demo wars. Empty
   states say what is empty and what would fill it.
4. **No unverified code labelled verified.** Launch configs no longer carry a custom hook (03 4.1:
   the `custom_hook` path is removed); the only programs a mint can run are the kit in a `Locked`
   slot and registered templates. An item is "Template item, verified" only when its template's
   program's on-chain executable hash equals the `Template.code_hash` recorded at registration (02
   2.3) and the program is immutable or held by `<MANAGED_HOOK_KEY>` / `<PROTOCOL_AUTHORITY>`. A
   template whose program is not `<ITEMS_ID>`, or whose hash no longer matches, is labelled with
   upstream's wording on every surface: "Custom hook, unverified", the program id, its upgrade
   authority or "fixed", and the sentence "Written by its author, not Hookwars. It can refuse
   transfers or take part of them." (upstream hooks-v2 7.4).
5. **No custody.** The backend builds transactions, the wallet signs, the backend sends (upstream
   architecture.md). The backend holds no user key. The keeper key signs only permissionless cranks
   and pays its own fees.
6. **Spot only.** Every action is a swap, a transfer, a claim, a vote, an equip crank, a war crank, a
   roll, a forge or a quest claim.
7. **No em dashes.** Fonts follow the agencypad reference (owner, 2026-10-10): Inter for body, JetBrains Mono for headings, labels, figures and addresses. Tabular digits everywhere.

## 2. Indexer

### 2.1 Cursors and decoding

One signature cursor per program, oldest first, as upstream (architecture.md "Indexer"):
`token`, `swap`, `launch`, `kit`, `bridge`, `companion`, `armory`, `items`, `war`.

Two ways events reach the indexer:

| Programs | How they emit | How the indexer reads them |
| --- | --- | --- |
| token, swap, launch, kit, bridge, companion, armory, war, and `hookwars_items` top-level instructions (`settle_equip`, `init_equip`, `close_equip`, `init_raid_ledger`) | self-CPI (`emit_cpi!`), upstream | inner instructions to the program's own event authority, data starting with the event tag (upstream integration 03, `eventsOf`) |
| `hookwars_items` **callbacks** (R18: items are leaves, so `RaidMarked`, `ShieldTaken`, `ItemCut` are `emit!` program logs) | program logs | `Program data: <base64>` lines in `meta.logMessages`, attributed to `<ITEMS_ID>` by the invoke and success lines around them; decoded with the items IDL |

Rules:
- Every lookup table's loaded addresses go into the key list (upstream integration 03). Hookwars
  transactions load from the protocol table **and** the mint's own table (section 4.2).
- Failed transactions are skipped (`meta.err`).
- Rows are keyed by `(signature, ordinal)`; `ordinal` is the event's position in the transaction,
  inner-instruction events and log events numbered in one sequence in execution order. Re-indexing
  is idempotent.
- **Logs can be truncated** by the runtime's log limit, so a log event can be missing. Item log
  events are therefore hints: the authoritative figures for items (`EquipState.runs`,
  `collected_token`, `pool_owed`, `pool_settled`; `RaidLedger`; holding hook data) are read from
  accounts after each transaction that touches them. A transaction whose logs end in a truncation
  marker is flagged, and the affected `EquipState` and `RaidLedger` accounts are re-read.
- Events without a time take the block's.
- Program info (upgrade authority, upgradeable, executable hash) is read from ProgramData for every
  program in 00 section 3 and every template program, cached, refreshed on a timer and on every
  `TemplateRegistered` and `TemplateRetired`.

### 2.2 Holdings, hook data and the epoch byte

Upstream `token.HookDataWritten { mint, holding, owner, data }` carries a holding's full 64 bytes
whenever they are written, including by `touch` (01 section 5). The indexer keeps
`holding_hook_data` current from that event alone; it never polls holdings for hook data.

Decoding a holding (00 4.4, 01 3.4):

1. Split the 64 bytes by the mint's slot table (`data_offset`, `data_len` per slot, from
   `SlotsInitialized`).
2. A `Locked` range (the kit, bytes 0..32, R8) has no epoch byte and is decoded with upstream's kit
   layout.
3. An item range's first byte is the **epoch byte**. If it differs from the slot's current
   `data_epoch` (from the latest `SlotEquipped`), the range is **stale** and reads as empty: the
   bytes belong to an item no longer in the slot. Otherwise the bytes after it are decoded with the
   equipped template's layout (04): Raid (tag `0x01`, `season_id`, `raid_points`, `tickets`,
   `quest_period`), Shield (tag `0x02`, `origin`, `origin_at`), Half-Life (tag `0x07`, `since`).
4. A Raid range whose `season_id` is not the current season reads `raid_points = 0` (04 3.1);
   tickets carry over.

Balances come from `token.Transferred` post-balances with a slot guard (upstream). Vote locks come
from `VoteLockSet` (01 4.3), stored per holding (`vote_locked`, `vote_lock_until`).

### 2.3 Tables

Postgres. Amounts `numeric(39,0)`, addresses `text`, times `timestamptz` plus raw unix seconds. Every
event table has `signature`, `ordinal`, `slot`, `ts`.

**Kept from upstream (shape unchanged):** launches, pools, swaps, transfers, holders, candles, kit
rewards, configs, bridge wrappers, companions, protocol fees.

**Changed:**

| Table | Change |
| --- | --- |
| `swaps` | adds `route jsonb` (`Swapped.route: RouteContext`, 03 3.2), `route_id` (the `RouteSwapped` of the same instruction, when multi-hop), `pool_item_cuts jsonb` (from `PoolItemCuts`: per slot and item, cut and burn, `discount_bps`, `pool_cuts_delta`) |
| `transfers` | adds `slot_cuts jsonb` (`Transferred.slot_cuts: Vec<SlotCut { slot, item, cut }>`, 01 6) |
| `launches` | adds `slot_table jsonb` (as created), `prepared_at` (`LaunchPrepared`), `war_chest`, `war_bps`, `mint_lookup_table` (section 4.2) |
| `companions` | adds `war_bps`, `paid_war_total` (`CompanionWarFunded`) |

**New, one row per event:**

| Table | Events | Key columns |
| --- | --- | --- |
| `route_swaps` | `RouteSwapped` | `trader`, `route_input_mint`, `route_output_mint`, `amount_in`, `amount_out`, `pools` |
| `observations_created` | `ObservationsCreated` | `pool`, `observations`, `len` |
| `pool_registry_refreshes` | `PoolRegistryRefreshed` | `mint`, `pool`, `items` |
| `templates` | `TemplateRegistered`, `TemplateRetired` | `template_id`, `program`, `code_hash`, `deploy_slot`, `kind`, `field_count`, `field_min`, `field_max`, `name`, `status`, `verified_now` (section 2.1) |
| `items` | `ItemCreated`, `LootMinted`, `Forged` | `item`, `item_mint`, `template_id`, `params` (array of `PARAM_FIELDS` u32), `manifest`, `author`, `royalty_bps`, `level`, `source` (`Authored` / `Loot` / `Forged`), `closed` (forged away) |
| `item_owners` | `Transferred` of item mints | `item`, `owner` (the holding with amount 1; 02 2.5: the armory stores no owner) |
| `slots` | `SlotsInitialized`, `SlotEquipped` | `mint`, `slot`, `kind`, `equip_rule`, `bounds`, `data_offset`, `data_len`, `equip_vault`, `item`, `program`, `flags`, `pool_flags`, `data_epoch` |
| `slot_states` | `EquipApplied`, `PerformanceCondition`, `PerformanceReverted`, plus `SlotState` account reads | `mint`, `slot`, `launch_item`, `rule`, `open_proposal`, `condition_since` |
| `equips` | `EquipInitialized`, `EquipClosed`, `EquipSettled`, `ItemCut`, plus `EquipState` reads | `mint`, `slot`, `item`, `config` (targets, role), `equipped_at`, `runs`, `collected_token`, `pool_owed`, `pool_settled`, `vault_balance` |
| `settlements` | `EquipSettled` | `mint`, `slot`, `item`, `royalty_token`, `royalty_quote`, `destination`, `amount_token`, `amount_quote`, `bounty` |
| `royalty_claims` | `RoyaltyClaimed` | `item`, `cut_mint`, `claimant`, `amount` |
| `proposals` | `ProposalCreated`, `ProposalResolved`, `EquipApplied { by: Vote }` | `proposal`, `mint`, `slot`, `nonce`, `proposer`, `item`, `vote_end`, `executable_at`, `status` (`Open`, `Passed`, `Failed`, `Executed`, `Cancelled`), `votes_for`, `votes_against`, `eligible` |
| `votes` | `VoteLocked`, `VoteUnlocked` | `proposal`, `voter`, `support`, `amount`, `until`, `closed` |
| `vote_locks` | `VoteLockSet` | `mint`, `holding`, `owner`, `amount`, `until` |
| `raids` | `RaidMarked` (log) | `mint` (the token bought, whose Raid item stamped), `rival` (the token sold), `trader`, `volume`, `points`, `loot_ticket` |
| `shield_takes` | `ShieldTaken` (log) | `mint`, `owner`, `cut` |
| `war_chests` | `WarChestCreated`, `WarFunded`, every spend | `mint`, `chest`, `treaty_inbox`, `war_state`, `balance`, `funded_total` |
| `war_state_snapshots` | after every war event, `WarState` read at that slot | `mint`, `slot`, the account (spend totals, `under_siege_until`, `siege_by_chest`, `captured`, `season`, `prev_season`) |
| `raid_ledger_snapshots` | after every swap that ran a marking item, `RaidLedger` read | `mint`, `slot`, `inbound` windows, `outbound_volume_season`, `season_id` |
| `sieges` | `SiegeExecuted`, `SiegeWaited` | `mint`, `rival_mint`, `spent`, `bought`, `bounty`, `cranker`, `captured_total`; waited rows keep `rival_price`, `rival_twap` |
| `counter_strikes` | `CounterStrikeExecuted` | `mint`, `spent`, `burned`, `bounty`, `cranker` |
| `razes` | `Razed` | `mint`, `rival_mint`, `sold`, `got`, `bounty`, `cranker`, `captured_left` |
| `captured_returns` | `CapturedReturned` | `mint`, `rival_mint`, `amount`, `treaty_item` |
| `treaty_inflows` | `TreatyInflowShared`, `TreatyTimeAccrued` | `mint`, `amount`, `bounty`, `cranker`; `treaty_item`, `secs`, `season` |
| `bounties` | `BountyClaimed` | `mint`, `owner`, `points`, `paid` |
| `loot_rolls` | `RollRequested`, `RollRevealed`, `RollCancelled` | `roll`, `mint`, `owner`, `holding`, `season`, `requested_slot`, `oracle_program`, `oracle_account`, `item`, `template_id`, `params`, `status` |
| `quests` | `QuestClaimed`, plus `QuestMark` reads | `quest_id`, `mint`, `owner`, `season`, `period` |
| `seasons` | `SeasonProposed`, `SeasonOpened`, `SeasonFinalized`, plus `Season` reads | `season`, `starts_at`, `ends_at`, `weights`, `penalize_besieged`, `eta`, `leader`, `leader_score`, `finalized`, `winner`, `score` |
| `season_candidates` | `CandidateSubmitted`, `CandidateChallenged` | `season`, `mint`, `score`, `submitted_by`, `beaten` |
| `prizes` | `PrizePaid` | `season`, `winner`, `to_winner`, `to_treasury`, `bounty`, `cranker` |
| `loot_tables` | `LootTableProposed`, plus `LootTable` reads | `season`, `entries`, `eta` |
| `war_admin` | `ConfigProposed`, `ConfigApplied`, `PendingCancelled` | `change`, `eta` |
| `holding_hook_data` | `HookDataWritten` | `holding`, `mint`, `owner`, `data`, decoded `ranges` (section 2.2) |

### 2.4 Derived views

Computed from the tables above, never stored as claims:

- **Equip targets.** Targets are chosen per equip, not in the item (04 2.3): `equips.config.targets`
  and `role`. Every relation edge below comes from there.
- **War map.** Nodes: launches with a war chest, a relation slot or any war event. Edges, each
  clickable to its source rows:
  - `treaty`: one Treaty item equipped by both mints, each targeting the other (04 3.5 "Active when");
  - `tribute`: a Tribute item equipped with role `Pay` on one mint and `Receive` on the other (04 3.6);
  - `rivalry`: a Raid, Shield or Spy equip targeting the other mint;
  - `raid`: rolling inbound raid volume from the `RaidLedger` (05 2.5 formula:
    `volume + prev_volume * (window_start + RAID_WINDOW_SECS - t) / RAID_WINDOW_SECS`), from the
    raiding token to the rival;
  - `siege`: `WarState.siege_by_chest` while `under_siege_until > now`;
  - `captured`: `WarState.captured` entries, holder to rival, weight = share of the rival's supply.
- **Generals.** Per mint, holders ranked by current-season `raid_points` decoded from their Raid
  range (section 2.2), ties by raid volume from `raids`, then by earliest point. A site title only;
  it grants nothing (WARFARE.md section 5). Pool vaults, launch PDAs, war chests, treaty inboxes,
  `PoolCuts`, `EquipState` owners and royalty owners are excluded.
- **Item record.** Per item, summed over its `EquipState`s: `runs`, `collected_token`, `pool_owed`;
  realized royalties from `settlements` and `royalty_claims`; tokens equipped on from `slots`.
- **Royalty position of an item**, per cut mint, three numbers (02 section 10):
  1. *unsettled*: the equip vault balance plus `pool_owed - pool_settled`;
  2. *settles to royalty*: `floor(unsettled * royalty_bps / 10_000)` per side, exactly 04 2.5;
  3. *claimable*: the balance of `["holding", cut_mint, RoyaltyOwner]`.
- **Siege readiness** per War orders and rival: rolling inbound volume against
  `siege_threshold * SIEGE_UNIT_LAMPORTS` (04 3.9 field 0; 05 6.2), the time until
  `last_siege_at + SIEGE_INTERVAL_SECS`, and the blocking reason when there is one: the rival's kit
  has holder rewards on (`SiegeTargetHasRewards`, R10), the captured table is full, the War slot is
  empty. No estimate of when a threshold fills.
- **Counter-strike readiness:** short and long TWAPs from our `Observations` (03 3.1) against
  `counter_drop_bps`, the interval, and `OwnTokenHasRewards` when our kit pays holder rewards (05 3).
- **Season table.** Each token's `WarState.season` counters (with `raid_volume_won` from its
  `RaidLedger` when the season matches, 05 10.1) and the season's `weights`, scored with 05 10.2's
  formula in `i128`, identically to the program (fixture vectors). Leader and challenges from
  `Season` and `season_candidates`.

## 3. API

### 3.1 Conventions

Unchanged from upstream `api.ts`: `Address`, `Amount`, unix seconds, `null` for anything not read,
`PreparedTx` and the submit flow, `ApiErrorBody`. New types live beside upstream's.

### 3.2 Types

```ts
export type SlotKind = 'fee' | 'reward' | 'defense' | 'relation' | 'pool' | 'locked' | 'war';
export type EquipRule = 'locked' | 'vote' | 'performance';
export type ItemSource = 'authored' | 'loot' | 'forged';
export type ProposalStatus = 'open' | 'passed' | 'failed' | 'executed' | 'cancelled';

/** 01 1.1, fixed for life. */
export interface SlotBounds { maxCutBps: number; mayRefuse: boolean; mayWriteData: boolean; mayAnswerTouch: boolean }

/** 04 2.7 manifest as stored in the Item (02 2.4). */
export interface ItemManifest { kind: SlotKind; tokenFlags: number; poolFlags: number; maxCutBuyBps: number; maxCutSellBps: number; maxCutTransferBps: number; dataBytes: number; mayRefuse: boolean; marks: boolean }

export interface TemplateInfo {
  templateId: number;
  name: string;                         // Raid, Shield, Wall, Spy, Treaty, Tribute, Half-Life, Transfer Fee, War orders
  sentence: string;                     // 04's one-line description
  program: Address;
  kind: SlotKind;
  fields: { index: number; name: string; min: number; max: number; forge: 'towardCeiling' | 'towardFloor' | 'keep' | 'none' }[];
  codeHash: string;
  verified: boolean | null;             // section 1 rule 4; null when not checked yet
  upgradeAuthority: Address | null;
  upgradeable: boolean | null;
  status: 'active' | 'retired';
  openAuthoring: boolean; lootEnabled: boolean; forgeEnabled: boolean; maxLevel: number;
  items: number;
}

export interface RoyaltyPosition {
  cutMint: Address; symbol: string;
  unsettled: Amount | null;             // equip vaults plus pool_owed - pool_settled, over every equip
  settlesToRoyalty: Amount | null;      // 04 2.5 formula on `unsettled`
  claimable: Amount | null;             // the royalty holding's balance
  claimedTotal: Amount | null;
}

export interface ItemSummary {
  item: Address; itemMint: Address;
  templateId: number; templateName: string; kind: SlotKind;
  params: number[];                     // PARAM_FIELDS fields, as on chain
  paramsText: string;                   // the template sentence filled with the params, built by shared code
  manifest: ItemManifest;
  level: number; source: ItemSource;
  author: Address; owner: Address | null;
  royaltyBps: number;
  runs: Amount | null;
  royalties: RoyaltyPosition[];
  equippedOn: { mint: Address; symbol: string; slot: number; targets: Address[]; role: 'none' | 'pay' | 'receive' }[];
  closed: boolean;                      // forged away
}

export interface SlotInfo {
  slot: number; kind: SlotKind; equipRule: EquipRule; bounds: SlotBounds;
  noticeSecs: number | null;
  dataRange: { offset: number; len: number };
  dataEpoch: number;
  item: ItemSummary | null;
  targets: Address[]; role: 'none' | 'pay' | 'receive';
  locked: { program: Address; label: 'kit' } | null;     // the kit in slot 0 (03 4.1)
  launchItem: Address | null;
  performance: { rule: Record<string, number>; conditionSince: number | null } | null;
  openProposal: ProposalInfo | null;
}

export interface ProposalInfo {
  proposal: Address; mint: Address; slot: number; nonce: number;
  proposer: Address; item: ItemSummary | null;          // null empties the slot
  voteEnd: number; executableAt: number;
  status: ProposalStatus;
  votesFor: Amount; votesAgainst: Amount;
  /** Live, as `finalize` computes it (02 6.4): supply - pool base vault - launch holding. */
  eligibleNow: Amount | null;
  quorumBps: number;                    // VOTE_QUORUM_BPS
  needsSettleFirst: boolean;            // the outgoing item has unsettled vaults (02 section 10)
  myVote: { amount: Amount; support: boolean; until: number } | null;
}

export interface WarOrdersInfo {
  item: Address; level: number;
  siegeThreshold: Amount; siegeSpendBps: number; siegeTwapSecs: number;
  counterDropBps: number; counterShortSecs: number; counterLongSecs: number; counterIntervalSecs: number; counterSpendBps: number;
  razeEnabled: boolean; peaceReturns: boolean;
  bountyRate: Amount; crankBountyBps: number;
}

export interface CapturedInfo { rivalMint: Address; symbol: string; amount: Amount; cost: Amount; capturedAt: number; shareOfSupply: number | null; razeAllowanceLeft: Amount | null }

export interface WarInfo {
  mint: Address;
  chest: Address | null; chestBalance: Amount | null; fundedTotal: Amount | null;
  spent: { siege: Amount; counter: Amount; bounties: Amount; cranks: Amount } | null;
  razedProceeds: Amount | null;
  treatyInbox: { address: Address; balance: Amount | null; sharedTotal: Amount | null } | null;
  orders: WarOrdersInfo | null;          // null: the War slot is empty, every war action is off (05 6.0)
  underSiege: { byChest: Address; byMint: Address | null; until: number } | null;
  captured: CapturedInfo[];
  inbound: { rival: Address; symbol: string; rolling: Amount }[];
  siege: SiegeReadiness[];
  counter: { ready: boolean; shortTwap: string | null; longTwap: string | null; nextAt: number | null; blocked: string | null };
  season: Record<string, string> | null;
}

export interface SiegeReadiness {
  rival: Address; rivalSymbol: string;
  rolling: Amount; threshold: Amount;
  nextAt: number | null;
  /** One sentence naming the rule, or null when a siege is due: R10 reads
   *  "RIVAL pays holder rewards, so no war chest can hold it (SiegeTargetHasRewards)". */
  blocked: string | null;
}

export interface TreatyInfo {
  treatyItem: Address;
  kind: 'treaty' | 'tribute';
  partyA: { mint: Address; symbol: string; equipped: boolean; role: 'none' | 'pay' | 'receive' };
  partyB: { mint: Address; symbol: string; equipped: boolean; role: 'none' | 'pay' | 'receive' };
  active: boolean;
  params: number[]; paramsText: string;
  returnsCaptured: boolean;
  secsThisSeason: Amount | null;
}

export type BattleKind = 'raid' | 'siege' | 'siege_waited' | 'counter_strike' | 'raze' | 'return' | 'treaty_on' | 'treaty_off' | 'treaty_shared' | 'equip' | 'proposal' | 'settle' | 'bounty' | 'roll' | 'loot' | 'forge' | 'quest' | 'season' | 'prize';
export interface BattleEvent { kind: BattleKind; ts: number; signature: string; ordinal: number; mint: Address; otherMint: Address | null; actor: Address | null; amount: Amount | null; detail: Record<string, string> }

export interface MyHoldingWar {
  mint: Address;
  raidPoints: number | null; tickets: number | null;     // current season, epoch-checked
  bountyClaimable: Amount | null;                        // min(points * bounty_rate, BOUNTY_MAX_PER_CLAIM, chest balance), 05 7
  voteLocked: { amount: Amount; until: number } | null;
  shieldOrigin: { rival: Address; until: number } | null;
  halfLifeSince: number | null;
  quests: { questId: 1 | 2; name: 'Raid' | 'Forge'; claimable: boolean; lastPeriod: number | null; reason: string | null }[];
}

export interface General { owner: Address; raidPoints: number; raidVolume: Amount; rank: number }

export interface LootRollInfo { roll: Address; mint: Address; status: 'requested' | 'revealed' | 'cancelled'; requestedAt: number; expiresAt: number; item: ItemSummary | null }

export interface LootTableInfo { season: number; eta: number; entries: { templateId: number; templateName: string; weight: number; ranges: { min: number; max: number }[] }[] }

export interface SeasonInfo {
  number: number; startsAt: number; endsAt: number; challengeEndsAt: number;
  weights: Record<string, Amount>; penalizeBesieged: boolean;
  table: { mint: Address; symbol: string; score: string; counters: Record<string, string> }[];
  leader: { mint: Address; score: string } | null;
  finalized: boolean; winner: Address | null;
  prizePaid: Amount | null;                              // realized, Season.prize_paid
}

export interface PrizeVaultInfo { vault: Address; lamports: Amount | null; quoteHolding: Amount | null; lastWinner: Address | null; shareBps: number }
```

Upstream types gain:

```ts
// LaunchSummary gains: slots: SlotInfo[]; warChest: Address | null; warBps: number;
//   relations: { treaties: number; rivals: Address[] }; underSiege: boolean; raidVolume24h: Amount | null;
// LaunchDetail gains: war: WarInfo | null; treaties: TreatyInfo[]; proposals: ProposalInfo[];
// LaunchConfigData: `customHook` is removed (03 4.1); gains slots: SlotSpecInput[];
// SwapQuoteRequest gains: from?: Address;           // a rival mint: swap_route rival -> bridged SOL -> target
// SwapQuote gains:
//   route: Address[];                               // mints in hop order
//   raid: { rival: Address; discountBps: number; tollBps: number; points: number; lootTicket: boolean } | null;
//   slotCuts: { slot: number; templateName: string; amount: Amount; to: 'war_chest' | 'treaty_inbox' | 'burn' | 'collector' }[];
//   shield: { cutBps: number; until: number } | null;   // a sell a Shield charges, with the time it stops
//   voteLockBlocks: { locked: Amount; until: number } | null;
//   blocked gains reasons: 'vote_lock' | 'wall';
// LaunchPrepareRequest gains: slots: SlotSpecInput[]; viaCompanion: { warBps: number } | null;
// LaunchPrepareResponse: transactions in order prepare_launch, create_launch, init_war (when a War slot), lookup table
// PortfolioEntry gains: war: MyHoldingWar | null;
// Portfolio gains: items: ItemSummary[]; rolls: LootRollInfo[];
// Overview gains: programs.armory, programs.items, programs.war (and programInfo for each); season: { number; endsAt } | null; prizeVault: PrizeVaultInfo | null;

export interface SlotSpecInput {                     // 03 4.1 SlotSpec
  kind: SlotKind; equipRule: EquipRule; bounds: SlotBounds; noticeSecs: number; dataLen: number;
  launchItem: Address | null; targets: Address[]; role: 'none' | 'pay' | 'receive';
  rule: Record<string, number> | null;              // Performance (02 6.7)
}
```

### 3.3 Routes

Reads (GET):

| Route | Returns |
| --- | --- |
| `/v1/templates` | `TemplateInfo[]` |
| `/v1/items?template=&kind=&owner=&equipped=&source=&sort=royalties\|runs\|equipped\|new` | `{ items: ItemSummary[]; next: string \| null }` |
| `/v1/items/:item` | `ItemSummary & { history: BattleEvent[] }` |
| `/v1/launches/:mint/slots` | `SlotInfo[]` |
| `/v1/launches/:mint/proposals?status=` | `ProposalInfo[]` |
| `/v1/launches/:mint/war` | `WarInfo` |
| `/v1/launches/:mint/generals` | `General[]` |
| `/v1/launches/:mint/treaties` | `TreatyInfo[]` |
| `/v1/map` | `{ nodes: { mint: Address; symbol: string; image: string \| null; chest: Amount \| null; underSiege: boolean }[]; edges: { kind: 'treaty' \| 'tribute' \| 'rivalry' \| 'raid' \| 'siege' \| 'captured'; from: Address; to: Address; weight: Amount \| null; since: number \| null }[] }` |
| `/v1/feed?mint=&kind=&before=` | `{ events: BattleEvent[]; next: string \| null }` |
| `/v1/seasons/current`, `/v1/seasons/:n` | `SeasonInfo` |
| `/v1/seasons/:n/loot` | `LootTableInfo` |
| `/v1/prize-vault` | `PrizeVaultInfo` |
| `/v1/quests` | the two quests (05 9) with their conditions in one sentence each |
| `/v1/wallet/:owner/war` | `{ holdings: MyHoldingWar[]; items: ItemSummary[]; rolls: LootRollInfo[] }` |
| `/v1/cards/:signature/:ordinal.png` | a share card (section 7.2) |

Prepares (POST, each `{ transactions: PreparedTx[] }`, built in the order the programs require):

| Route | Builds |
| --- | --- |
| `/v1/raid/prepare` | `swap_route` rival -> bridged SOL -> target (03 3.3), every hop's slices, the SOL unwrap when chosen |
| `/v1/proposals/prepare` | `propose(slot, item)` with the equip's targets and role (02 6.2, 04 2.3) |
| `/v1/votes/prepare` | `vote(support, amount)` (02 6.3; tokens lock in place, R11) |
| `/v1/proposals/:p/finalize/prepare` | `finalize` after `vote_end` |
| `/v1/proposals/:p/execute/prepare` | `settle_equip` for the outgoing item's vaults, then `execute` (02 section 10) |
| `/v1/performance/prepare` | `check_performance(slot)`, preceded by `settle_equip` when a revert would close an unsettled equip |
| `/v1/settle/prepare` | `settle_equip(mint, slot)` for the listed equips (permissionless, bounty) |
| `/v1/royalties/prepare` | `settle_equip` for each of the item's equips with an unsettled balance, then `claim_royalty` per cut mint with a balance, then `unwrapSolAbove` for bridged SOL |
| `/v1/items/create/prepare` | `create_item(template_id, params, royalty_bps)` |
| `/v1/forge/prepare` | refuses while either item is equipped (`ItemEquipped`); otherwise settle, claim, then `forge` in the last transaction (02 section 10) |
| `/v1/rolls/prepare` | `roll(mint, nonce)`; later `reveal`; after `ROLL_EXPIRY_SECS`, `cancel_roll` |
| `/v1/quests/prepare` | `claim_quest(quest_id, mint, period)` |
| `/v1/bounties/prepare` | `claim_bounty(mint)` and the SOL unwrap |
| `/v1/war/:mint/crank/prepare` | whichever is due and allowed: `record_funding`, `siege`, `counter_strike`, `raze` (only with `raze_enabled`), `return_captured` (only with `peace_returns` and an active treaty), `share_treaty_inflow`, `accrue_treaty_time`; the program re-checks |
| `/v1/war/init/prepare` | `init_war(mint)` for a token with a War slot and no chest |
| `/v1/seasons/crank/prepare` | `open_season`, `submit_candidate`, `finalize_season`, `split_protocol_fees` |
| `/v1/launch/prepare` | section 6.5's transaction list |

Every prepare simulates first and refuses with the program's error translated into one sentence
that names the rule and, when there is one, the time it lifts (upstream: errors sit next to the
button).

## 4. Transactions

### 4.1 Rules (kept)

- v0 messages with the protocol lookup table, checked by `checkProtocolLookupTable` (upstream
  `transactions.ts`). Hookwars appends the new fixed addresses (armory and war configs, event
  authorities, `TOKEN_ITEMS_SIGNER`, `LAUNCH_ITEMS_SIGNER`, `ArmorySigner`, `["war-signer"]`,
  `["loot-signer"]`, `["prize-vault"]`); entries are only appended, and the SDK checks the first
  entries, never the length.
- Programs invoked at top level stay in static keys.
- Simulate (signatures not verified, blockhash replaced) and request the units used plus 15%
  (upstream hooks-v2 6). Fallback budgets when simulation fails come from 07's measured figures.
- Built by the backend, signed by the wallet, sent by the backend, followed until landed or expired.
- A fresh mint keypair per launch attempt; a broadcast mint key is never re-signed (hooks-v2 1.5).
  Since a launch is now several transactions signed by the same mint key (section 6.5), "attempt"
  means the whole sequence: the key is discarded only when the sequence is abandoned before
  `prepare_launch` lands; once it has landed, the same key finishes the launch.

### 4.2 The mint's lookup table

A transfer or swap of a slotted token carries one slice per called slot (01 2.2): the slot's
program, its signer, and the item registry's extras (`["bordrless-hook-accounts", mint, item]`,
01 section 7). A launch-pool swap also carries the launchpad's fixed extras 5 to 12 and each Pool
item's extras (03 4.3, 5.5). Upstream's launch with one custom hook was already about 1,200 of
1,232 bytes (half_life README).

- **Created at launch,** as the last transaction of the launch sequence (section 6.5): an address
  lookup table owned by the backend's table key, extended with every per-mint address a trade needs:
  launch, pool, vaults, `Observations`, kit config and vault, `PoolCuts` and its holding,
  `RaidLedger`, `WarState`, war chest and treaty inbox holdings, each slot's `EquipState`, equip vault,
  registry and extras.
- **Extended on equip.** `execute` and `check_performance` prepares append the incoming item's
  addresses first, in their own transaction.
- **Warm-up.** A table extended in slot `s` is usable from slot `s + 1`; the backend waits for the
  extension to confirm.
- **Recorded** in `launches.mint_lookup_table` and returned by the API for third-party terminals.
- **Rent** is shown on the launch review with the other new accounts.

### 4.3 Slot-aware account resolution (SDK)

```ts
export interface SlotSlice { slot: number; program: PublicKey; signer: PublicKey; extras: AccountMeta[] }
export interface TokenHookSlices { mint: PublicKey; slices: SlotSlice[]; counts: number[] }

/** Reads the mint's slot table and each called slot's registry; resolves prefix-dependent seeds (01 2.2). */
export function fetchTokenHookSlices(connection: Connection, args: {
  mint: PublicKey; source: PublicKey; destination: PublicKey;
  authority: PublicKey; sourceOwner: PublicKey; destinationOwner: PublicKey;
  /** R16: a payout from a protocol vault passes ProtocolSource and only the Locked slot runs. */
  protocolSource?: { program: PublicKey; seeds: Buffer[] };
}): Promise<TokenHookSlices>;
export function sliceAccounts(s: TokenHookSlices): { metas: AccountMeta[]; counts: number[] };

/** Pool items for a launch-pool hop (03 5.5): ITEMS_ID, LAUNCH_ITEMS_SIGNER, registry extras per Pool slot. */
export function fetchPoolItemSlices(connection: Connection, mint: PublicKey): Promise<SlotSlice[]>;
```

- Which slots are called on a transfer follows 01 2.1, including Pool slots with a token half
  (I-01.1 in 04) and **R16**: transfers out of equip vaults, `PoolCuts`, royalty holdings, war
  chests and treaty inboxes carry `ProtocolSource` and run only the `Locked` slot. Royalty claims,
  settlements, bounties and razes are built that way.
- A `swap_route` carries, per hop, its group in the order 03 3.3 fixes.
- Duplicate keys collapse in the message: several items of `<ITEMS_ID>` cost their program and
  signer once.

### 4.4 SDK additions

One module per new program (`addresses`, `instructions`, `accounts`, `events`, IDL coders):

- **addresses:** 00 4.3's seeds (`itemAddress`, `templateAddress`, `royaltyOwner`, `slotAuthority`,
  `slotStateAddress`, `proposalAddress`, `voteAddress`, `forgeCounterAddress`, `equipStateAddress`,
  `equipVault`, `poolCutsAddress`, `raidLedgerAddress`, `itemRegistryAddress`, `warChestAddress`,
  `treatyInboxAddress`, `warStateAddress`, `warConfigAddress`, `seasonAddress`, `lootTableAddress`,
  `rollAddress`, `questMarkAddress`, `prizeVaultAddress`, `observationsAddress`,
  `preparedLaunchAddress`, `armoryCallerAddress`, `launchMintAuthority`).
- **decoders:** `Mint` with the slot table and `Holding` with `vote_locked` / `vote_lock_until`
  (01); `Template`, `Item`, `SlotState`, `Proposal`, `VoteLock`, `ForgeCounter` (02); `Observations`
  (03); `EquipState`, `RaidLedger` (04); `WarConfig`, `WarState`, `Season`, `LootTable`,
  `RollRequest`, `QuestMark` (05). Range decoders with the epoch check: `raidRange`, `shieldRange`,
  `halfLifeRange`, `kitRange`.
- **builders:** every instruction the prepares in 3.3 use, plus `token.touch` for completeness.
- **events:** `typedEvent` for every self-CPI event, and `itemLogEvents(logs)` for the item log
  events (R18).
- **shared math (`packages/shared`):** slot merge (01 3.2), `settle_equip` split (04 2.5), quotes
  with pool-item cuts, discounts and Shield (03 5.3, 04 3), observation TWAP (03 3.1), rolling raid
  volume (05 2.5), siege and counter-strike spend (05 6.2, 6.3), bounty (05 7), season score (05
  10.2), forge result (04 2.7, clamped as 02 9.1 step 2). Each pinned to the Rust by fixture vectors,
  as upstream pins `launch-fees.json`.

## 5. Backend

Same shape as upstream: API, indexer, builders, submitter, keeper.

- **Keeper.** Upstream's keeper sends `collect_protocol_fees_sol` and graduations. Hookwars adds,
  each only when simulation shows the program accepts it: `settle_equip` (bounty), `finalize` and
  `execute` of passed proposals (with settles first), `check_performance`, `record_funding`,
  `siege`, `counter_strike`, `share_treaty_inflow`, `accrue_treaty_time`, `reveal` once the oracle
  has answered, `open_season`, `submit_candidate` for the leading token after `ends_at`,
  `finalize_season`, `split_protocol_fees`. The keeper does **not** send `raze` or
  `return_captured`: both are allowed by the holders' War orders (05 6.4, 6.5), and the site offers
  them as buttons so a person chooses the moment. Bounties the keeper earns go to its own key and
  are recorded like anyone's.
- **Quote mirror.** Quotes include the kit's rules (upstream), each pool item's cut and discount
  (03 5.3), the toll, a Shield sell cut from the seller's range, token-side slot cuts (Half-Life
  from the sender's `since`), vote locks and an active Wall. Where a figure depends on hook data,
  the sell is also simulated for the exact amount, as upstream integration 05 recommends for
  Half-Life.
- **Program info,** cached, as section 2.1.
- **Ports.** Local development ports are claimed in this repository's `CLAUDE.md` after
  `lsof -ti :<port>` (machine rule); none is fixed here.

## 6. Site

Next.js, Inter and JetBrains Mono as section 1 rule 7 says. Every figure from the API; null renders as a dash; every empty
state names what is missing.

### 6.1 Navigation

Projects, Launch, War room, Armory, Seasons, Marketplace, Portfolio, Bridge, Explorer, Docs.
Upstream's Studio stays and gains "Make an item" (create from a template with open authoring).

### 6.2 War room (`/war`)

- **Map** (section 2.4 edges). Empty: "No wars yet. A war starts when a token equips a Raid, Shield
  or Spy aimed at another token." with a link to the Armory.
- **Battle feed:** `BattleEvent`s newest first, live, each linking its transaction.
- **Sieges:** per War orders and rival, readiness bars (rolling inbound volume against the
  threshold) with the next allowed time and, when blocked, the reason sentence (R10:
  "RIVAL pays holder rewards, so no war chest can hold it"). A "Siege now" button when due.
- **Under siege:** tokens whose `under_siege_until` is in the future, the besieging token, the time
  it ends, and whether a Wall is active.
- **War chests:** largest by balance, with funded, spent and captured.
- **Season strip:** current season, time left, leader, open challenge, prize vault balance.

### 6.3 Armory (`/armory`)

- **Templates:** the nine of 04 section 3, each with its sentence, kind, fields with floor, ceiling
  and forge rule, program, verification state, upgrade authority or "fixed", and status.
- **Items:** grid with filters (template, kind, level, source, equipped, for sale) and sorts. A card
  shows the template sentence filled with the params ("Raid: buyers who sold a target pay D% less
  creator and holder fee", D read from `discount_bps`; built by shared code from the actual fields), level, author, owner, royalty rate, royalties realized, tokens equipped on with their
  targets.
- **Item page:** the above, history, the marketplace listing, and its **royalty position** per cut
  mint (unsettled, settles to royalty, claimable) with "Settle and claim" (section 3.3).
  "Propose for a slot" lists tokens where the item fits a slot (02 6.1) and asks for the targets.
  Selling or sending an item shows its unsettled and unclaimed royalty first, with "Settle and
  claim" offered before the listing (02 section 10: the buyer would receive them).
- **Forge:** two items of one forgeable template the wallet holds, both unequipped. The page shows
  both parameter sets and the exact result (04 2.7, clamped). It settles and claims both items'
  royalties in the transactions before `forge` and will not build the forge while either has an
  unsettled or unclaimed balance it can read (02 section 10). War orders are not forgeable (04 3.9).
- **Loot:** tickets per token, "Roll", pending rolls with their expiry (`ROLL_EXPIRY_SECS`) and a
  note that a cancelled roll does not return its ticket (05 8.2), revealed items, and the season's
  drop table from `LootTable` (templates, weights, parameter ranges), labelled "drop table".
- **My items:** owned items, royalty positions, "Settle and claim all".

### 6.4 Token page (`/t/:mint`)

Upstream's page (chart, trade panel, holders, trades, rules panel, fee table, verify line), plus:

- **Slots panel.** One row per slot: kind, the item (template sentence, level, owner, royalty) and
  its targets and role, or the kit in slot 0 ("Locked at launch"), the bounds in words, the equip
  rule and notice, the launch item for a `Performance` slot with its rule as a sentence and
  "condition true since" from `PerformanceCondition`, and any open proposal.
- **War orders** (the `War` slot, 04 3.9): each field in words ("Siege when raids from a rival pass
  N SOL in the window; spend at most X% of the chest per siege"), raze and peace returns on or off,
  bounty rate. Empty: "No war orders: every war action is off. Funding still accrues." Changing the
  orders is a proposal like any slot.
- **Proposals:** item, votes, live eligible supply and the quorum it implies, time left, "Vote"
  (amount, side; "These tokens stay in your wallet and cannot be sold or sent until <vote end>",
  R11). After the vote: "Finalize", then after notice "Execute" (which settles the outgoing item
  first).
- **War:** chest balance, funded, spent by kind, captured holdings by token with share of supply,
  inbound raid volume by rival, siege readiness with reasons, counter-strike readiness (or
  "This token pays holder rewards, so its chest cannot buy it back: counter-strikes are off",
  `OwnTokenHasRewards`), "Under siege by TOKEN until <time>" from `siege_by_chest`, the generals, and
  crank buttons for what is due. Raze and Return show only when the War orders allow them; Return
  also needs an active treaty with that rival (05 6.5).
- **Treaties and treaty inbox:** active and pending treaties and tributes, terms in words, inbox
  balance, total shared to holders, and "Share with holders" (`share_treaty_inflow`; "Shared SOL
  reaches holders over the next hour", upstream 7.3). Without kit holder rewards: "This token cannot
  receive treaty payments: it has no holder rewards" (05 6.6).
- **My holding:** raid points and tickets this season, bounty claimable with "Claim", quest status
  with "Claim", vote lock and its end, Shield origin and when it expires, Half-Life age.
- **Join the raid:** when the token has a Raid aimed at a rival the wallet holds: "Sell RIVAL for
  TICKER: creator and holder fees cut by X%, toll Y%" with the route quote (points, whether a loot
  ticket is earned). One signature.
- **Trade breakdown** (upstream: it adds up) gains each slot cut with where it goes, the discount,
  the toll, a Shield cut, and blockers: a vote lock ("N tokens are locked by your vote until T") and
  a Wall ("TOKEN is under siege: no wallet may hold more than X% until T").

### 6.5 Launch form (`/launch`)

Upstream's three steps become four, and launching becomes a sequence (03 4.3):

1. Token (unchanged).
2. Rules (upstream presets, Custom, Build your own from a `LaunchConfig`; configs now carry slots,
   not a custom hook). Any kit module puts the kit in slot 0, `Locked`, with 32 bytes; the form
   shows the 32 bytes left for items.
3. **Slots:** up to `MAX_SLOTS`. Per slot: kind, equip rule, bounds, notice (between
   `MIN_NOTICE_SECS` and `MAX_NOTICE_SECS`), the launch item (from the wallet's items or the armory,
   filtered by fit) with its targets and role, and for `Performance` the rule. The form sums each
   template's bytes plus one epoch byte against the bytes left, counts cutting slots against
   `MAX_CUTTING_SLOTS`, sums Pool caps against `MAX_POOL_ITEM_CUT_BPS`, and blocks with 03 4.2's
   reason. A `War` slot gets War orders. **War chest funding** comes from launching through a
   companion with `war_bps` (0 to `WAR_BPS_MAX`, 03 6.1); a plain launch funds its chest only from
   items, partners and donations, and the form says so.
4. **Review:** every permanent choice ("fixed at launch: the creator can't change these", upstream),
   what can change later and how, the launch fee, rent of every new account including the lookup
   table, and the first buy.

**The sequence after signing**, shown as numbered steps with their state:

| Step | Transaction | Signers |
| --- | --- | --- |
| 1 | `prepare_launch` (mint with slots, `equip_launch` of every launch item) | creator, mint key |
| 2 | `create_launch` (supply, pool, registry with indices 9 to 12) | creator, mint key |
| 3 | `init_war` when there is a `War` slot | anyone; the site sends it with the creator as payer |
| 4 | the mint's lookup table, create and extend | the backend's table key; rent from the creator in step 2 |
| 5 | the creator's first buy, if any (upstream: never merged into the launch) | creator |

If step 1 lands and the creator leaves, the launch page offers "Finish launch", which resumes at
step 2 with the same mint key; nobody else can launch a prepared mint (03 4.3, `NotPrepared`,
`WrongCreator`). A companion launch runs steps 1 and 2 through the companion (03 6.3).

### 6.6 Generals (`/t/:mint/generals`, `/generals`)

Per token, the ranked list (section 2.4). Global: top generals across tokens by raid volume. A
general's page lists their raids. The page states: "Generals are the top raiders by on-chain points.
It is a title; it gives no control."

### 6.7 Quests (`/quests`)

The two quests the program checks (05 9): **Raid** (hold `QUEST_RAID_POINTS` raid points this
season; claiming spends them) and **Forge** (forge an item since your last Forge claim). One claim
per `QUEST_PERIOD_SECS`, recorded in `QuestMark`. Each pays one loot ticket. The page states that a
holding quest does not exist because its proof would move with the tokens (05 9). Site-only badges,
if any, are listed apart as "Badges: no reward".

### 6.8 Seasons (`/seasons`)

Current season: table of tokens with their counters and the score from the published weights
(and whether being besieged subtracts); leader and challenges; time to `ends_at` and to the end of
the challenge window; the prize as "a share (SEASON_PRIZE_SHARE_BPS) of Hookwars's protocol fees
collected during the next season, paid to the winner's war chest" with the realized `prize_paid`.
The prize vault balance and "Split protocol fees" (`split_protocol_fees`) are shown. Proposed
seasons and loot tables with their `eta` (timelock, 05 11). Past seasons with winners and prizes
paid.

### 6.9 Kept pages

Projects board (cards gain a slots line, e.g. "Raid vs RIVAL · Shield · Holders 1%", a chest
figure, an "under siege" mark, and filters "at war", "under siege", "has treaty"); Marketplace
(items join configs as listings); Portfolio (items with royalty positions, rolls, tickets, war
holdings, vote locks); Bridge; Explorer (new account kinds: item, template, proposal, slot state,
equip, raid ledger, war state, war chest, treaty inbox, season, loot table, roll, quest mark);
Docs (the standard, slots, items, relations, war, with parameters printed from the chain).

## 7. Bots and share cards

### 7.1 Alerts

Telegram and X bots read the indexer's `BattleEvent` stream; they never read the chain directly and
never hold a user key.

| Trigger | Post |
| --- | --- |
| `RaidMarked` rows above a size the operator sets (an operator setting, not a protocol parameter) | "TICKER is raiding RIVAL holders: X SOL in the window. Join: <link>" |
| siege readiness crossing a share of its threshold the operator sets | "Siege on RIVAL at N% of the threshold" |
| `SiegeExecuted` | "TICKER besieged RIVAL: spent X SOL, captured Y RIVAL" |
| `CounterStrikeExecuted` | "TICKER struck back: X SOL bought and burned" |
| `Razed`, `CapturedReturned` | "TICKER sold N captured RIVAL" / "TICKER returned N RIVAL under their treaty" |
| treaty becomes active or ends (from `equips`) | "TICKER and OTHER signed: <terms>" |
| `SeasonFinalized`, `PrizePaid` | "Season N: TICKER wins" / "Prize paid: X SOL to TICKER's war chest" |
| a general's call | only when the general signs the call message with their wallet on the site |

The link opens `/t/{target}/raid?from={rival}&caller={wallet?}`; the page loads the quote from
`/v1/raid/prepare` and asks for one signature. Every post links the transaction it reports. Rate
limits and quiet hours are operator settings.

### 7.2 Share cards

`/v1/cards/:signature/:ordinal.png` renders a card from one indexed event and nothing else: "raided
RIVAL with X SOL", "won a siege", "forged a level N Shield", "claimed X SOL in bounties", "season N
winner". It shows the short signature and the slot; a signature the indexer does not hold is a 404.
No PnL, no projected figures.

## 8. Checks before this part is called done

- Re-index a devnet range from scratch twice: identical tables, including item log events.
- A transaction with truncated logs: item figures still match the accounts (section 2.1).
- The quote mirror equals simulation on every fixture: a buy with two pool items and a discount, a
  raid route, a Shield sell, a Half-Life sell, a vote-locked sell refused, a Wall refusal.
- Royalty position: after a settle, `unsettled` falls to 0 and `claimable` rises by exactly the
  `EquipSettled` royalty; after a claim, `claimable` falls by the `RoyaltyClaimed` amount.
- A re-equipped slot: every holding's old range reads empty (epoch byte), with no holding touched.
- Forge is refused by the site while either item has an unsettled or unclaimed balance.
- A launch abandoned after step 1 resumes with "Finish launch" and the same mint key.
- Every page loads with an empty database and shows its empty states.
- A template whose program is not `<ITEMS_ID>`, or whose hash no longer matches, shows "Custom hook,
  unverified" on every surface.
- No file contains U+2014; headings, labels and figures render JetBrains Mono (screenshot pass).
- Bots: a replayed event range posts once (idempotent by `(signature, ordinal)`).

## 9. Open questions for other parts

Answered by the revised parts and removed: event names (R15), raid marks and windows (`RaidLedger`,
R3, R4), royalty flow (R1, R2, `settle_equip`, `claim_royalty`), route context (R5,
`PoolHookArgs.route`, `RouteSwapped`), quorum (computed live at `finalize`, 02 6.4), launch flow
(R12, R17), who may raze and return (05 6.4, 6.5), the score formula (05 10.2), quest definitions
and the roll's oracle account (05 8.2, 9), the prize (R14).

Still open:

| Id | Question | Owner |
| --- | --- | --- |
| Q6-1 | 02 6.4 says 01 "also emits `SlotItemSet`"; 01 section 6 names the event `SlotEquipped`. This file uses `SlotEquipped`. | resolved: 02 now says `SlotEquipped` |
| Q6-2 | 05 section 12 says 04 names the raid event `RaidStamped`; 04 section 5 names it `RaidMarked`. This file uses `RaidMarked`. | resolved: 05 now says `RaidMarked` |
| Q6-3 | Quest touch payloads: 04 2.10 has `MarkQuest { kind, period }` (marker in the Raid range); 05 9 uses `AddTicket` and a `QuestMark` PDA (marker off the holding) and drops `Hold`. This file follows 05. | resolved: 04 follows 05 (QuestMark PDA, AddTicket) |
| Q6-4 | Equip vault address: 01 3.3 and 04 2.5 use the holding owned by `["equip", mint, slot]`; 02 5.1 writes `["equip-vault", equip_state]`. This file uses 01 and 04. | resolved: 02 now uses the 01 and 04 address |
| Q6-5 | Treaty and Tribute are kind `Relation` (pool side) in 04's table, while 00 4.1 says `Relation` runs on token transfers. The site shows them under Relation either way; the slot table must say which callbacks they get. | resolved: 00 4.1 now gives Relation a pool half where 04 says so |
| Q6-6 | How many royalty claims and settlements fit one transaction; `MAX_ROUTE_HOPS`; the mint lookup table's size at `MAX_SLOTS`. | 07 (measure) |
| Q6-7 | The one-line sentence per template the site prints: 04 has descriptions per template but no fixed sentence with placeholders per field. Proposed: 04 adds one. | resolved: 04 gives one sentence per template |
| Q6-8 | `Item.manifest` field names (02 2.4 references 04 2.7's 16-byte struct; `ItemManifest` above is this file's reading of it). | resolved: 04 2.7 lists the manifest fields |
