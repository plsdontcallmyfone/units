# Integration checklist (app/ against the programs)

The app was first built on branch `app` from the spec while the programs were built on other
branches. Round 1 of integration (branch `appint`, 2026-10-09) switched everything whose program is
on main to the programs' generated IDLs. This file says what is done and what remains, each item
naming the file to change and how to verify it.

## 1. Done in round 1

### IDLs

`anchor idl build` of every deployed program (scripts/solana/programs.sh idl) on the build server,
copied to `idl/` (repo root) and `app/packages/sdk/idl/`. To refresh after a program change: run
the same command, copy the JSON files again, then `node packages/sdk/scripts/gen-idl-types.mjs`
(writes `packages/sdk/src/hookwars/idl-types.gen.ts`).

The armory and items IDLs needed one source change to build: the `Params` type alias
(`[u32; PARAM_FIELDS]` from `hookwars-common`) is not an IDL type, so instruction arguments, account
fields and events of `hookwars_armory` and `hookwars_items` now spell `[u32; PARAM_FIELDS]`. The type
is the same, so the programs' behaviour and bytes are unchanged.

### SDK (`packages/sdk/src/hookwars/`)

| Module | What it is now |
| --- | --- |
| `idl.ts` | A coder for Anchor 1.x IDLs: accounts, events, instruction accounts in IDL order with PDAs derived from the IDL seeds (constants, other accounts, arguments), the event authority and the program filled in, optional accounts left out passed as the program id |
| `from-idl.ts` | The IDLs of token, armory, items, war, kit and companion; their account codecs, event schemas and argument schemas as `codec.ts` schemas; `idlIx` |
| `idl-types.gen.ts` | Generated TypeScript types, one namespace per program |
| `accounts.ts` | Token `Mint`, `Holding`; armory `ArmoryConfig`, `PendingParams`, `Template`, `Item`, `SlotState`, `Proposal`, `VoteLock`, `ForgeCounter`; items `EquipState`; war `WarConfig`, `WarState`, `Season`, `LootTable`, `RollRequest`, `QuestMark`: all from the IDLs. `PARAM_FIELDS` and `MAX_SLOTS` are read from the IDL arrays |
| `events.ts` | Every event of token, armory, items, war and companion from the IDLs |
| `instructions.ts` | Token `touch`; armory `create_item`, `vote`, `propose`, `finalize`, `execute`, `cancel`, `close_vote`, `close_proposal`, `claim_royalty`, `forge`; war `init_war`, `record_funding`, `siege`, `counter_strike`, `raze`, `return_captured`, `share_treaty_inflow`, `accrue_treaty_time`, `claim_bounty`, `roll`, `reveal`, `cancel_roll`, `claim_quest`, `close_quest_mark`, `open_season`, `submit_candidate`, `finalize_season`, `split_protocol_fees`: all through `idlIx`. Remaining accounts follow the programs' Rust clients (`hookwars_war::client`) |
| `war-context.ts` | Reads the War orders and the Raid slot from a mint's slot table and the armory `Item`s, and resolves the Raid slot's touch extras as the token program resolves them for `touch` (`[signer, mint, holding, holding, caller]`) |

Accounts whose seeds use a field of another account's data (for example `war_state` seeded by
`war_state.mint`) cannot be derived from the IDL alone; the builders pass them explicitly.

### Pinned to the Rust

- `programs/tests/tests/app_vectors.rs` renders `programs/tests/vectors/hookwars-math.json` (copied
  to `packages/shared/vectors/`) from the Rust the programs run, and fails and rewrites the file when
  the Rust changes:
  - `hookwars_common::{shape, combine, manifest, window_read, PerformanceRule::holds}`;
  - `hookwars_war` `Season::score`, `loot::draw`, `bps_of`;
  - 22 instructions built by `hookwars_war::client` and `bordrless_token::client::touch`.
- `packages/shared/src/hookwars/exact.ts` is the exact TypeScript port; `exact.test.ts` runs every
  case. `packages/sdk/src/hookwars/instructions.vectors.test.ts` requires each IDL builder to equal
  the Rust client's instruction (program, every key with its signer and writable flags, data).

### Spec mismatches settled by the merged code

| Was | Settled |
| --- | --- |
| `RaidLedger.season_id` / `outbound_volume_season` (04) against `season` / `season_volume` (05) | 04's names; the war program's reader (`foreign.rs`) uses them |
| War orders `peace_returns` | Not a War orders field: the Treaty item's `returns_captured` (field 2) gates peace returns. `WarOrdersInfo.peaceReturns` removed; `exact.WAR_ORDERS` and `exact.TREATY_RETURNS_CAPTURED` give the field indices |
| `Season` without `version` and `bump` | The IDL has both |
| Type of `change` in `ConfigProposed`, `ConfigApplied`, `PendingCancelled` | From the IDL |
| `PARAM_FIELDS` | 11, read from the IDL |
| Transfer Fee's `max_wallet_bps` forge rule | `floorWhenBothOn` (0 means off), as `hookwars_common::shape` says |

### API prepares

Built now (each first checks the programs are deployed): votes, proposals, finalize, cancel, close
vote, royalties, forge (reads `items_minted`, refuses mixed templates and equipped items), bounties,
quests, rolls (reads the randomness program from `WarConfig`), war init, record funding, treaty
time, season submit, finalize and open, prize split. `apps/api/src/prepares.test.ts` builds them
against a mocked chain holding accounts encoded with the IDL codecs.

### Other

- `app/pnpm-lock.yaml` is committed (it was missing from main).
- `Cargo.lock` regenerated: main's lockfile lacked the m4 merge's packages (`hookwars-war`, the war
  stubs), so `cargo test --locked` refused it.

## 2. Remaining

| Area | Waits for | What to do |
| --- | --- | --- |
| DEX `Observations`, `swap_route`, `RouteContext`, `RouteSwapped`, `ObservationsCreated` | M3a merge | Regenerate the swap IDL; drop the spec `Observations` codec and the `swap` entries of `SPEC_ONLY` in `events.ts`; build `swap_route` with `idlIx`; check `OBS_LAYOUT` in `exact.ts` against the real account (the M2 raw reader `hookwars_common::obs_layout` is what the armory and war read today) |
| Launchpad: `prepare_launch`, `create_launch` with slots, pool-item forwarding, `PoolItemCuts`, `LaunchPrepared`, `PoolRegistryRefreshed` | M3b | Regenerate the launch IDL; replace the `launch` spec entries; un-refuse `/v1/launch/prepare` |
| Items callbacks, `settle_equip`, `init_raid_ledger`, `RaidLedger`, `RaidMarked`, `ShieldTaken`, `ItemCut`, `EquipSettled` | M3b | Regenerate the items IDL; replace `decodeRaidLedger` and the `items` spec events; build `settle_equip` with `idlIx` |
| Composites and the arsenal templates (08) | Arsenal waves | Add each template's site sentence and fields to `packages/shared/src/hookwars/templates.ts` and its shape to `exact.ts`, pinned by `app_vectors.rs` |
| Raid prepare (`/v1/raid/prepare`) | M3a and M3b | A two-hop `swap_route` from the rival through bridged SOL |
| Indexer: execution-order ordinals, re-read after truncated logs, account snapshots, upstream kept tables | none | As before (06 2.1, 2.3) |
| Wallet signing and a submit route on the site | none | The launch form and every prepare return unsigned v0 transactions today |
| Per-mint address lookup tables | devnet | `pnpm admin init` equivalent for units; prepares compile without tables until then |
