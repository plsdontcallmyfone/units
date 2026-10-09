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

## 2. Done in round 2 (branch `integ`, 2026-10-09)

Changed by Hookwars: round 2 integrates M3a (DEX ring, `swap_route`) and M3b (slot launches, items
callbacks) into the app, and fixes the app audit (`~/ideas/hookwars/APP-AUDIT-1.md`).

### IDLs

Launch, swap, items and war regenerated with `scripts/solana/programs.sh idl` on the build server and
copied to `idl/` and `packages/sdk/idl/`; `idl-types.gen.ts` regenerated.

### SDK

| Module | What changed |
| --- | --- |
| `accounts.ts` | `decodePoolObservations(poolData)`: the ring is the tail of the pool account from `POOL_LEN` (411), header 96 bytes, entries 48 (`bordrless_core::observations`). The separate `Observations` account and `observationsAddress` are gone |
| `slot-launch.ts` | `prepareLaunch`, `equipLaunch`, `equipPrepared`, `createPreparedLaunch`, `refreshPoolRegistry`, `initRaidLedger`, `swapRoute`, `settleEquip` from the IDLs; pool-item account helpers; `settleDestination` mirrors the items program's `token_destination` / `pool_destination` |
| `instructions.ts` | the spec-layout builders are removed; `siege` and `counter_strike` no longer pass an observations account |
| `exact.ts`, `math.ts` | `windowReadRaw` reads the ring exactly as `bordrless_core::observations::window_read`; pinned by `app_vectors.rs` |

### API

- `/v1/launch/prepare` is staged: prepare (signed by the browser's fresh mint), one `equip_prepared`
  per launch item, the mint's lookup table (create, then extend in chunks of 20), the launch (v0
  with that table), then `refresh_pool_registry`, `init_war` and `init_raid_ledger`. It needs
  `owner`, `mint`, `name`, `symbol` and `virtualQuote` (lamports, `MIN_VIRTUAL_QUOTE` to
  `MAX_VIRTUAL_QUOTE`).
- `/v1/launch/prepare` runs in two phases: the default (`phase` "prepare") returns the prepare
  and the equips; once those land, `phase: "launch"` returns the mint's lookup table, the launch
  (deposit slices and the pool items' registries read from the chain, so the pool registry is
  whole at launch) and the war chest stage. The site makes both calls.
- `/v1/buy/prepare` (fuzz audit 1, finding 1): a buy on a launch pool cut to the exact remainder the
  curve can still fill (`remainderBuy` in `@hookwars/shared`, within max wallet). Near graduation a
  round buy is refused and `graduate` waits for the curve to fill, so clients and any crank send the
  remainder; `CurveFull` (409) says the curve is ready to graduate. There is no graduate crank in
  this app.
- Sieges that wait (fuzz audit 1, finding 4): `siege` succeeds with `SiegeWaited`. The indexer keeps
  each event in its own table, the feed maps `SiegeWaited` to `siege_waited`, the war map reads
  executed sieges only, and the bots never post `siege_waited`.
- `/v1/raid/prepare`: a two-hop `swap_route`, rival to SOL on the rival's own launch pool, then SOL
  to the target (the Raid template requires that first pool, security finding M-5).
- `/v1/settle/prepare`: `settle_equip` with each module's destinations.
- Every prepare compiles v0 with the protocol table (`PROTOCOL_LOOKUP_TABLE`, checked against the
  protocol's addresses) plus its stage tables and refuses anything over 1,232 bytes (A-6).
- `/v1/submit` sends a wallet-signed transaction and waits for confirmation, so the site never
  holds the RPC URL (A-2).

### Site

The launch form makes a fresh mint key in the page, asks for the virtual SOL reserve, prepares
the stages, signs the mint's stages, has the wallet sign all of them in one prompt
(`signAllTransactions`), and sends them in order through `/api/v1/submit` (`lib/sign.ts`).

### Operator settings (not protocol parameters)

`DATABASE_URL` (required, no default), `RPC_URL`, `PROTOCOL_LOOKUP_TABLE`, `RATE_PREPARE_CAPACITY`,
`RATE_PREPARE_PER_SEC`, `RATE_READ_CAPACITY`, `RATE_READ_PER_SEC`, `MAP_WINDOW_SECS`.

## 3. Done in app v2 (branch `appv2`, 2026-10-09)

### IDLs and SDK

- IDLs regenerated on server B for all 14 non-test programs (new: agents, market, social; items
  updated) into `idl/` and `packages/sdk/idl/`. Two needed server-only patches of a copy of the
  programs to build (never committed; see the requests in section 4).
- `packages/shared`: `arsenal.ts` (templates 10 to 45 with fields, forge rules, range bytes and 08's
  sentences; floors and ceilings named, never valued), `FAMILY_OF`, range decoders for tags 0x11,
  0x12, 0x14, 0x18, 0x1a, 0x23, 0x27, 0x28; program ids for agents, market, social.
- `packages/sdk`: codecs for every agents, market, social account and the items payout accounts;
  events of every program from the IDLs, program-log events for items and launch (R18); PDAs;
  `expansion.ts` builders for every agents, market, social and payout instruction the site uses,
  including `equip_badge` (forwards the armory's `equip_launch` as the agents caller) and
  `issue_badge`.

### Indexer and API

- Cursors for agents, market, social; `mint_tables` recorded from a `LaunchCreated` transaction's
  lookup table and loaded by every prepare that names the mint (`mint` or `tokenMint`); views
  `item_sales`, `item_listings`, `badge_awards`, `passports`. Event tables of events with no own
  field now get valid DDL.
- Reads (`src/reads-expansion.ts`): `/v1/agents?sort=`, `/v1/agents/:passport`,
  `/v1/market/listings`, `/v1/market/items/:itemMint` (sales from `Sold` events only, lineage from
  `Forged` events), `/v1/market/leases`, `/v1/market/collections`, `/v1/commissions`,
  `/v1/commissions/:address`, `/v1/guilds`, `/v1/guilds/:id`, `/v1/badges`. A missing account
  reads as `null`.
- Prepares (`src/expansion-prepares.ts`), all `POST /v1/<route>/prepare`: market list, delist,
  buy, collections, lease offer/accept/withdraw/end; commissions open/submit/pay/refund; badges
  claim; guilds create/deposit/propose/approve/execute; agents register, profile, badge
  equip/issue, policy init/limits/freeze/withdraw, bonds post/resolve; referral set/settle,
  loyalty init/claim, first-blood init.

### Site

- `components/action.tsx`: one form for every non-launch prepare: wallet connect, prepare with
  the wallet as owner, sign all in one prompt, send each through `/api/v1/submit`. Used on the
  token page (buy, raid, propose, vote, count, close vote, settle, bounty or open chest, quest,
  roll, referrer, loyalty, commission), the armory item page (royalty claim, forge) and every new
  page.
- New pages: `/agents` (league by one track-record counter, no prize; register),
  `/agents/[passport]` (proof levels with what each proves, record, links, policy wallet, bonds,
  badge steps, operator actions), `/marketplace` (listings, collections), `/marketplace/items/
  [itemMint]` (price line and table from sales only, lineage tree, listing, rental),
  `/marketplace/rentals`, `/commissions`, `/commissions/[address]`, `/guilds`, `/guilds/[id]`,
  `/badges`, `/armory/templates`, `/armory/templates/[id]` (sentence with field names, fields with
  registered floors and ceilings, manifest, holder data, payout forms for 24, 36, 38). Nav gains
  Templates, Commissions and a Community menu.
- Docs: `scripts/build-docs.mjs` compiles `docs/guide` (SUMMARY.md as the sidebar, heading
  anchors, mermaid shown as labelled source, links to /docs routes) at build time; 18 pages from
  main's guide build. Falls back to `apps/web/docs-sample` when the guide is absent.
- `MOCK_DATA=1` shows a "Demo data" chip and a banner on every page.
- Screenshots: `app/screenshots/{desktop,mobile}_*.png` for every new page, checked for status,
  horizontal overflow, page errors and em dashes (all clean).

## 4. Requests to the program owners (from app v2)

| Program | Request |
| --- | --- |
| hookwars_items | Merge `ArsenalError` (payouts.rs, offset 7100) and `SoulboundError` (templates/soulbound.rs, offset 7000) into the one `#[error_code]` enum: Anchor's IDL build refuses more than one ("Multiple error definitions are not allowed"). The committed IDL carries all 30 codes. |
| hookwars_agents | `PolicyLimits.tracked: Vec<(Pubkey, u64, u64)>`: the IDL builder cannot express tuples. A struct `TrackedLimit { mint, per_action, per_day }` has the same Borsh bytes; the committed IDL uses it. |
| hookwars_armory | `equip_launch` accepts only the launchpad's `["armory-caller", mint]`; the agents badge needs the agents program's caller too (09 section 21 request 1). Until then `agents/badge/equip` fails simulation with `NotLaunchCaller`. |

## 5. Remaining

| Area | What to do |
| --- | --- |
| SOL price | The app has no SOL price source, so the launch form asks for the virtual reserve in SOL instead of a dollar market cap |
| Arsenal shapes pinned | templates 10 to 45 are in `arsenal.ts`; pin their shapes in `exact.ts` against `app_vectors.rs` (programs/tests, not app-owned); composite range sub-decoding |
| Indexer | execution-order ordinals, re-read after truncated logs, account snapshots, upstream kept tables (06 2.1, 2.3) |
| Simulation against the programs | New prepares are tested for exact IDL account lists on a mocked chain; nobody has yet simulated them against deployed agents, market, social programs (not on devnet) |
| Agent flows needing a second signer | `register_passport` with an agent key other than the operator, `link_social` (ed25519 statement), `submit_attestation`, `post_bond` together with the two proposals |
| Agents league per season | 09 asks for per-season sums of indexed events; the site ranks by the on-chain lifetime counters until the indexer derives season sums |
| Operator page, broker page | `/operators/:key`, `/agents/:passport/broker` (09 section 10) not built |
| Protocol lookup table on devnet | create it and set `PROTOCOL_LOOKUP_TABLE`; prepares compile without it until then |
