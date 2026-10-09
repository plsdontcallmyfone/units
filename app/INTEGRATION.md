# Integration checklist (app/ against the programs)

The app was built on branch `app` while the programs were being built on other branches (m2 armory
and items, m3a DEX, m4 war, kitcomp kit and companion). Everything below that says "spec layout"
was written from docs/spec and must be checked against the generated IDL or the Rust once those
branches merge. Each item names the file to change and how to verify it.

## 1. Constants

| Id | What | Where | Check |
| --- | --- | --- | --- |
| A1 | `PARAM_FIELDS` is 11 (the smallest the spec allows, 04: War orders has 11 fields) | `packages/shared/src/hookwars/params.ts` `PARAM_FIELDS_MIN`; `packages/sdk/src/hookwars/accounts.ts` `PARAM_FIELDS` (call `setParamFields`) | equal to the armory build's constant |
| A2 | `MAX_SLOTS = 4` | `packages/sdk/src/hookwars/accounts.ts` | equal to `bordrless_token/src/constants.rs` (it is, at M1) |
| A3 | Every parameter in `PARAMS` is `null` except `MAX_SLOTS` and `MAX_CUTTING_SLOTS` | `packages/shared/src/hookwars/params.ts` | fill from `ArmoryConfig`, `WarConfig`, the DEX and launch configs and the program constants; never by hand |
| A4 | `LAUNCH_ITEMS_SIGNER` derived as `["hook-authority", ITEMS_ID]` under the launchpad | `packages/shared/src/programs.ts` `HOOK_SIGNERS.launchForItems` | equal to the launchpad's signer of Pool-item callbacks (03 5.5) |
| A5 | Protocol lookup table: 22 upstream entries plus 11 Hookwars entries | `packages/shared/src/programs.ts` | equal to what `pnpm admin init` writes when the table is created; until then no table exists on devnet and prepares compile without one |

## 2. Account and event layouts written from the spec

Replace each codec with the generated IDL coder (or keep it and add a round-trip test against bytes
the program wrote in LiteSVM). Exact today: token `Mint` with the slot table, `Holding` with the vote
lock and the token events `Transferred`, `SlotsInitialized`, `SlotEquipped`, `VoteLockSet`,
`HookDataWritten` (all from the M1 Rust on main).

| Program | Accounts (spec section) | Events (spec section) |
| --- | --- | --- |
| armory | `ArmoryConfig` 2.1, `Template` 2.3, `Item` 2.4, `SlotState` 2.7, `Proposal` 2.8, `VoteLock` 2.9, `ForgeCounter` 2.10 | 02 section 13 |
| DEX | `Observations` 03 3.1 (ring length read from the data) | `RouteSwapped`, `ObservationsCreated` 03 section 9; the extended `Swapped.route` is decoded by the upstream IDL today, so `route` is missing until the IDL is regenerated |
| launch | | `PoolItemCuts`, `LaunchPrepared`, `PoolRegistryRefreshed` 03 section 9; extended `LaunchCreated` needs the regenerated IDL |
| companion | | `CompanionWarFunded` 03 section 9 |
| items | `EquipState` 04 section 4, `RaidLedger` 04 2.9 (table length read from the data) | 04 section 5; callbacks emit program logs (R18) |
| war | `WarConfig` 05 2.1 up to `last_winner` (the spec does not fix `PendingConfig`), `WarState` 05 2.4 (captured length read from the data), `Season` 10.2, `LootTable` 8.3, `RollRequest` 8.2, `QuestMark` 9 | 05 section 12 |

Known spec gaps to settle with the program authors:

- `Season` has no `version`/`bump` in the spec table; the codec has none.
- 04 2.9 names `RaidLedger.season_id` and `outbound_volume_season`; 05 2.5 calls them `season` and
  `season_volume`. The codec uses 04's names.
- 04's War orders field list (11 fields) has no `peace_returns`; 05 6.0 reads one. `WarOrdersInfo.peaceReturns` is `null` until settled.
- `ConfigProposed` / `ConfigApplied` / `PendingCancelled` carry `change` as a `u8` here; the spec
  does not fix its type.
- `ObservationsCreated.len` is a `u16` here.

## 3. Instruction builders written from the spec

`packages/sdk/src/hookwars/instructions.ts`: argument schemas follow the spec sections in the file;
account lists follow the spec where it lists them (`claim_bounty` 05 7, `roll` 05 8.2, `claim_quest`
05 9) and are best effort elsewhere (`vote`, `propose`, `settle_equip`, `claim_royalty`, `forge`,
`init_war`). Check each against the IDL's account order. `touch` follows the M1 code.

Prepares that throw a one-sentence refusal until their programs and builders exist:
`/v1/forge/prepare`, `/v1/raid/prepare` (needs `swap_route` hop groups), `/v1/launch/prepare` (needs
`prepare_launch`, `create_launch`, `init_war`, the lookup table), and the crank prepares of 06 3.3
not listed in `apps/api/src/prepares.ts`. Every prepare first checks its programs are deployed and
says which are missing.

## 4. Math to pin with fixture vectors

`packages/shared/src/hookwars/math.ts`: `forgeField` (04 2.7), `settleSplit` and `royaltyPosition`
(04 2.5), `windowRead` (03 3.1), `raidWindowAdd` and `rollingRaidVolume` (04 2.9, 05 2.5),
`bounty` (05 7), `warSpend` and `counterStrikeDue` (05 6.2, 6.3), `seasonScore` and `beatsLeader`
(05 10.2, 10.3), `mergeSlotCuts` (01 3.2). Each has unit tests from the spec formulas; add
`vectors/*.json` written by the Rust tests and a test that reads them, as upstream pins
`launch-fees.json`.

## 5. Indexer

- Event ordinals: self-CPI events are numbered in instruction order, then item log events; 06 2.1
  wants one sequence in execution order. Interleave by matching invoke depth when the programs
  exist.
- After a transaction with truncated logs, re-read the `EquipState` and `RaidLedger` accounts it
  touched (06 2.1); the flag is stored in `transactions.logs_truncated`, the re-read is not built.
- Account snapshots (`war_state_snapshots`, `raid_ledger_snapshots`) and the upstream kept tables
  (launches, pools, swaps, holders, candles) are not built; the API reads `LaunchCreated` events for
  the board.

## 6. Site

- Wallet: the launch form connects an injected Solana wallet and prepares; signing and submitting
  the prepared transactions waits for a submit route.
- Pages read the API; every figure the API cannot read renders as a dash or an empty state.
