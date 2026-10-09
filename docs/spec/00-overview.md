# Hookwars spec 00: overview and contract

Status: specification, 2026-10-08. Nothing here is built or deployed. This file is the contract
for every other part of `docs/spec/`: names, programs, seeds, shared types, parameters and the
rules of writing. Where a part disagrees with this file, the part is wrong.

Upstream: Bordrless at `43688f3` (2026-10-08, "Companions: a launch's creator as a program, with no
keeper"). Its documents stay the reference for everything Hookwars keeps unchanged:
`docs/architecture.md`, `docs/hooks-v2.md` (wins over architecture.md), `docs/companions.md`,
`programs/half_life/README.md`. Hookwars parts cite them by section instead of repeating them.

## 1. What Hookwars is

Bordrless's rails, kept: a token standard whose hooks answer (cuts, burns, fee, 64 bytes per
holding), a DEX with pool hooks and virtual reserves, a curve launchpad, the kit's launch rules,
Half-Life, companions, listed configs, the bridge.

Added:

1. **Slots.** A token runs up to `MAX_SLOTS` hooks at once, each in a slot with bounds fixed at
   launch. What fills a slot can change by the slot's equip rule.
2. **Items.** A hook in a slot is an **item**: an owned, tradable record of a template plus
   parameters, whose owner earns a royalty from what it collects, every time it runs.
3. **Pool slots.** Items can act on swaps of a launch pool (direction, price, route), not only on
   token transfers.
4. **Relations.** Items read other tokens' markets (time-weighted), see a trade's route, and form
   treaties that both tokens equip.
5. **War.** A war chest per token funds sieges, counter-strikes and bounties; raid points and loot
   tickets live in holdings; loot mints items; crafting merges them; seasons crown a winner on chain.

Spot only: every effect is a transfer, a cut, a burn, a fee change or a spot swap.

## 2. Parts and owners

| File | Covers |
| --- | --- |
| `00-overview.md` | this contract |
| `01-token-slots.md` | token program changes: slot table in the mint, calling and merging items, hook-data ranges, `touch`, slot authority |
| `02-armory.md` | `hookwars_armory`: items, templates registry, royalty vaults and claims, equip rules (vote, performance, notice), loot minting, crafting |
| `03-dex-launch.md` | DEX changes (observation ring, multi-hop route context), launchpad changes (pool slots, slot table at launch, war companion split), companion change |
| `04-templates.md` | `hookwars_items`: the audited template hooks (Raid, Shield, Wall, Spy, Treaty, Tribute, plus Half-Life and the transfer fee as templates), their parameters, ceilings and hook-data layouts |
| `05-war.md` | `hookwars_war`: war chest, war state, siege, counter-strike, raze, bounties, loot tickets and randomness, quests, seasons and the king-of-the-hill challenge |
| `06-app.md` | indexer, API types, SDK additions, site pages (war room, map, armory, token page), bots, share cards |
| `07-budgets-tests.md` | transaction budgets, call-depth table, test plan per milestone, security checklist |
| `08-arsenal.md` | composite items and the arsenal of templates (ids 10 to 41), compatibility model, presets, build waves |
| `09-agents.md` | agent identity: passports, soulbound badge (template 42), proof levels, attribution, policy wallets, diplomat bonds |
| `11-hook-economy.md` | access modes (open, gated, licensed, leased, exclusive), fee waterfall, agents as builders, levels, memo messaging and directives, materials, recipes, wear, order book |
| `10-expansion.md` | hooks as assets (market, collections, lineage, rental), social, economy and events packs; templates 43 to 45 |

## 3. Programs

| Program | Status | Role |
| --- | --- | --- |
| `bordrless_token` | changed | mints, holdings, transfers; **slots** replace the single hook; `touch` |
| `bordrless_swap` | changed | DEX; **observation ring** per pool; **route context** for multi-hop swaps |
| `bordrless_launch` | changed | launchpad; launches create a slot table; its pool hook **forwards to pool-kind items** |
| `bordrless_companion` | changed | gains `war_bps` in its split, paid to the token's war chest |
| `bordrless_kit` | changed | launch rules in a `Locked` slot (bytes 0..32); `init` accepts a slot mint (R9); treats war chests as excluded owners where it can (R10) |
| `bordrless_bridge` | kept | unchanged |
| `half_life`, `tax_hook` | kept | kept as programs; their behaviour is also offered as templates in `hookwars_items` |
| `hook_tester` | kept | test-only |
| `hookwars_armory` | new | items, royalties, equip rules, loot minting, crafting |
| `hookwars_items` | new | one program implementing every template; behaviour chosen by the item's template id and parameters |
| `hookwars_war` | new | war chests, war state, siege, counter-strike, raze, bounties, loot tickets, quests, seasons |
| `hookwars_agents` | new (09) | agent passports, soulbound badges, proof levels, track record, policy wallets, diplomat bonds |
| `hookwars_market` | new (10) | item listings and sales, collections, rentals, commissions |
| `hookwars_social` | new (10) | achievement badges, guild halls, agent leagues |
| `hookwars_craft` | new (11) | materials, drops, recipes, repairs, item charges |
| `hookwars_book` | new (11) | escrowed order book for materials and standing bids for items |

Crate names stay `bordrless_*` for changed programs until a rename is decided (D-3), so upstream
diffs stay readable. Every changed file carries a header line: `Changed by Hookwars: <what>`.

Program ids: new keypairs for every deployed program, never Bordrless's. Until generated, parts
write `<TOKEN_ID>`, `<SWAP_ID>`, `<LAUNCH_ID>`, `<KIT_ID>`, `<BRIDGE_ID>`, `<COMPANION_ID>`,
`<ARMORY_ID>`, `<ITEMS_ID>`, `<WAR_ID>`, and `<PROTOCOL_AUTHORITY>`, `<MANAGED_HOOK_KEY>` for the
two keys Bordrless hard-codes in `HOOK_UPGRADE_AUTHORITIES`.

## 4. Shared names

### 4.1 Slot kinds

| Kind | Value | Runs on | Answers it may give |
| --- | --- | --- | --- |
| `Fee` | 0 | token transfers | cuts (token side) |
| `Reward` | 1 | token transfers | cuts, hook data |
| `Defense` | 2 | token transfers | refuse, hook data |
| `Relation` | 3 | token transfers, and launch pool swaps through the launchpad when its template has a pool half (04 lists which) | cuts, hook data; reads other pools |
| `Pool` | 4 | launch pool swaps, through the launchpad | fee override, cuts, burn (within the launchpad's own rules and the slot's bounds), war-state marks |
| `Locked` | 5 | as the kit does today | whatever the equipped program's flags allow; never re-equipped |
| `War` | 6 | nothing (no callbacks) | none; its item's parameters configure `hookwars_war` (siege threshold, counter-strike trigger) |

### 4.2 Equip rules

| Rule | Value | Meaning |
| --- | --- | --- |
| `Locked` | 0 | the launch item stays for ever |
| `Vote` | 1 | holders vote by locking tokens; equips after `notice_secs` |
| `Performance` | 2 | accepts votes like `Vote`, and also reverts to the launch item when its on-chain condition holds for its hold time; anyone cranks |

### 4.3 Seeds (all new; upstream seeds unchanged)

| Account | Program | Seeds |
| --- | --- | --- |
| `Item` | armory | `["item", item_mint]` |
| `Template` | armory | `["template", template_id: u16 le]` |
| `RoyaltyOwner` (system-owned, signs) | armory | `["royalty", item]` |
| royalty holding | token | `["holding", mint, RoyaltyOwner]` |
| `SlotAuthority` (signs slot changes) | armory | `["slots", mint]` |
| `Proposal` | armory | `["proposal", mint, slot: u8, nonce: u64 le]` |
| `VoteLock` | armory | `["vote", proposal, voter]` |
| `WarChest` (system-owned, holds SOL and holdings) | war | `["war-chest", mint]` |
| `WarState` | war | `["war", mint]` |
| `Season` | war | `["season", number: u32 le]` |
| `LootTicketQueue` / roll request | war | `["roll", holding, nonce: u64 le]` |
| `Observations` | swap | `["obs", pool]` |
| `RaidLedger` (raid marks, inbound raid windows, pool-cut accrual) | items | `["raid-ledger", mint]` |
| `EquipState`, equip vault owner | items | `["equip", mint, slot: u8]` |
| `PoolCuts` vault owner | items | `["pool-cuts", mint]` |
| item registry | items | `["bordrless-hook-accounts", mint, item]` |
| war signers | war | `["war-signer"]`, `["loot-signer"]`, `["war-config"]`, `["prize-vault"]`, `["loot", season: u32 le]` |
| armory loot caller check | war | `["loot-signer"]` is the only signer `mint_loot` accepts |
| armory's own signer (calls into items) | armory | `["armory"]` |
| launchpad signer for `equip_launch` | launch | `["armory-caller", mint]` |
| `TreatyInbox` owner | war | `["treaty-inbox", mint]` |
| `CompositeItem` | armory | `["composite", item]` |
| preset recipe | armory | `["preset", id: u16 le]` |
| referral record | items | `["referred", mint, holder]` |
| `QuestMark` | war | `["quest", holding, quest_id: u8, period: u32 le]` (05 is authoritative) |
| hook signer | token, swap, launch | upstream `["hook-authority", program]`, unchanged |

### 4.4 Hook data

Upstream: 64 bytes per holding, written only by the mint's hook. Hookwars splits them into
**ranges**, one per slot, fixed when the slot is created (`data_offset`, `data_len`); an item may
read and write only its own range; the token program enforces it. The **first byte of every item
range is the token program's epoch byte** (01): it changes on every equip, so a previous item's
bytes read as empty. An item sees `data_len - 1` bytes and its own layout tag is the first byte it
sees; templates (04) declare their byte count without the epoch byte. A `Locked` slot (the kit)
uses the upstream arguments with the full 64 bytes, but the token program applies only the bytes
inside its range (R8). The holding can be closed only when all 64 bytes are zero (upstream rule).

### 4.5 Money words

Use upstream's names on every surface: creator fee, holder rewards, burn, max wallet, creator
wallet lock, early-buyer lock, sniper fee. New names: **royalty**, **war chest**, **raid**,
**siege**, **counter-strike**, **raze**, **bounty**, **loot**, **forge**, **season**. Never "yield",
"APR", "APY", "reflections", "tax", "bet", "odds" or "payout on outcome".

## 5. Rules every part follows

1. **Hooks never pay.** Anything that pays someone pays from a vault a program owns (war chest,
   royalty vault, kit reward vault), by an instruction with its own checks.
2. **Bounds are fixed at launch.** A slot's kind, bounds, equip rule, notice and data range never
   change after the launch. Only the item in it changes.
3. **An item's code never changes under it.** An item's template program must be immutable or
   upgradeable only by `<MANAGED_HOOK_KEY>` or `<PROTOCOL_AUTHORITY>` (upstream
   `HOOK_UPGRADE_AUTHORITIES`), checked at template registration; the code hash is recorded.
4. **Loot and crafting mint parameters, never code.** Every item is a registered template plus a
   parameter set inside the template's ceiling.
5. **Permissionless cranks pay a bounty** of at most `MAX_CRANK_BOUNTY_BPS` of what they move,
   from the vault they act on, and check on chain that they are due.
6. **No live calls between tokens.** A token is never called because another token traded.
   Relations read state already on chain (observations, war state, treaties).
7. **Reads of other markets are time-weighted**, never spot, over at least `MIN_TWAP_SECS`.
8. **Measure before quoting.** Compute, bytes, trace entries and call depth are measured in LiteSVM
   tests and written in 07 with the test that measured them.
9. **Spot only. No em dashes. No invented numbers.**

## 6. Parameters

Every number Hookwars introduces. All are **to set**: by measurement (M) or by the owner (O).
Parts refer to them by name only.

| Name | Meaning | Set by |
| --- | --- | --- |
| `MAX_SLOTS` | slots per mint | M (bytes, compute) |
| `MAX_CUTTING_SLOTS` | slots that may answer cuts on one transfer (upstream `MAX_DELTAS` is 3, and a royalty takes one delta per cutting item) | M |
| `MAX_ROYALTY_BPS` | ceiling on an item's royalty share of its own cuts | O |
| `MIN_NOTICE_SECS`, `MAX_NOTICE_SECS` | bounds on a slot's notice | O |
| `VOTE_PERIOD_SECS`, `VOTE_QUORUM_BPS` | holder vote | O |
| `OBS_RING_LEN`, `MIN_TWAP_SECS` | observation ring size and the shortest window a read may use | M, O |
| `MAX_ROUTE_HOPS` | hops in a multi-hop swap | M |
| `MAX_CRANK_BOUNTY_BPS` | crank bounty ceiling (upstream companion: at most 1%) | O |
| `SIEGE_MAX_SPEND_BPS`, `SIEGE_INTERVAL_SECS` | siege spend per call (of the chest) and spacing | O |
| `COUNTER_STRIKE_MAX_SPEND_BPS` | counter-strike spend per call | O |
| `RAZE_MAX_BPS_PER_INTERVAL`, `RAZE_INTERVAL_SECS` | how fast captured holdings may be sold | O |
| `BOUNTY_MAX_PER_CLAIM` | bounty cap per claim | O |
| `LOOT_MIN_RAID_LAMPORTS` | smallest raid buy that earns a loot ticket | O |
| `SEASON_SECS`, `CHALLENGE_SECS` | season length, challenge window | O |
| `SEASON_PRIZE_SHARE_BPS` | share of the protocol's fee share paid to the season's winner | O |
| `WAR_BPS_MAX` | ceiling on a companion's `war_bps` | O |
| `MAX_POOL_ITEM_CUT_BPS` | ceiling on all Pool-item cuts on one side of a swap, inside upstream's 3% per side | O |
| `PARAM_FIELDS` | number of `u32` parameter fields per item (layout constant) | M |
| `FORGE_GAIN_BPS` | how far a forge moves each field toward its ceiling | O |
| `POINT_UNIT_LAMPORTS` | quote volume per raid point | O |
| `MAX_CAPTURED`, `RAID_TABLE_LEN`, `RAID_WINDOW_SECS` | war state table sizes and raid window | M, O |
| `SIEGE_MAX_PREMIUM_BPS`, `SIEGE_SLIPPAGE_BPS`, `COUNTER_STRIKE_MIN_INTERVAL_SECS` | siege and counter-strike guards | O |
| `ROLL_EXPIRY_SECS`, `LOOT_TABLE_LEN` | loot roll expiry, loot table size | O, M |
| `QUEST_*` | quest periods and thresholds (05) | O |
| per-template ceilings | named in 04, one per parameter field | O |
| `MAX_ITEM_TARGETS`, `SIEGE_UNIT_LAMPORTS` | items per-equip targets; siege threshold unit (04) | O |
| `ADMIN_TIMELOCK_SECS` | delay on every admin setter of armory and war (D-9) | O |
| `MAX_MODULES`, `ITEM_DATA_MAX` | modules per composite; composite param storage | M |
| agents and expansion parameters | every named parameter in 09 and 10 (section 13 of 10 lists 27; 09 lists 17) | O |
| arsenal parameters | every floor, ceiling and named limit in 08 (e.g. `EMBARGO_MAX_TARGETS`, `LOYALTY_MIN_EPOCH_SECS`, `STREAK_MAX`, `MERC_MAX_POINTS_PER_UNIT`, `COOLDOWN_MAX_SECS`, `DECAY_MIN_SECS`, `DECAY_MAX_SECS`, `CAP_MIN_BPS`) | O |

`PARAM_FIELDS` is at least 11 (the War orders template, 04).

## 7. Decisions open at spec level

| Id | Question | Recommendation |
| --- | --- | --- |
| D-1 | Slot table inside `Mint` or in its own account | Inside `Mint` (no extra account per transfer); new mints only, so the layout is free |
| D-2 | One program for every template, or one per template | One (`hookwars_items`): one audit, one deploy, one hook signer |
| D-3 | Rename `bordrless_*` crates | Not before M2; keep upstream diffs readable |
| D-4 | Randomness oracle for loot | Choose in M5 after checking current Solana support (candidates: Switchboard randomness, ORAO VRF) |
| D-5 | Who runs the equip-rule logic | `hookwars_armory`, signing as `SlotAuthority`; the token program only accepts slot changes signed by it |
| D-6 | War chest: a new program or companion state | New program `hookwars_war` holds the chest; the companion only routes `war_bps` to it |

The owner-level decisions (names, royalty ceiling, season prize and the rest) are in
`~/ideas/hookwars/DECISIONS.md`.

## 8. Milestones

| Id | Journey |
| --- | --- |
| M0 | Fork builds and upstream tests pass under new ids |
| M1 | Slots: two items in one mint, merged answers, ranges enforced; measurements set `MAX_SLOTS`, `MAX_CUTTING_SLOTS` |
| M2 | Armory: templates, items, royalties, vote and performance equip |
| M3 | Pool slots, observations, route context; Raid, Shield, Wall, Spy, Treaty templates |
| M4 | War: chest, siege, counter-strike, raze, bounties |
| M5 | Loot, forge, quests, seasons |
| M6 | App: indexer, API, site, bots; devnet |

## 9. Integration rulings (2026-10-08, after the first drafts)

Where a part disagrees with a ruling, the part is revised. Numbered for reference from the parts.

- **R1 Item money, token side.** A cutting item answers at most **one** delta per transfer, paid
  into its equip vault (`["equip", mint, slot]` under items). The token program does **not**
  compute royalties. `settle_equip` (04, permissionless, bounty) later pays the royalty to the
  item's royalty holding (02) and routes the rest to the item's destination (burn, war chest,
  partner, collector). So `MAX_CUTTING_SLOTS` token-side is at most `MAX_DELTAS` (3), less any
  delta a `Locked` slot answers on the same transfer.
- **R2 Item money, pool side.** The launchpad already answers 2 of 3 deltas per side on a buy. All
  Pool-item cuts on one side merge into **one** delta into `["pool-cuts", mint]` (items); each
  item records its share in its own `EquipState` in the same call; `settle_equip` pays from there.
  No royalty marks through the war chest, no `pay_royalties` in war.
- **R3 Raid state is owned by items.** Items are leaves (no CPIs). Pool items write the
  `RaidLedger` (`["raid-ledger", mint]`, owned by `hookwars_items`) directly; `hookwars_war` reads
  it. There is no `record_marks` CPI from the launchpad and no `war-caller` PDA.
- **R4 The raid mark.** One mark per `RaidLedger`: `(clock slot, recipient, rival, quote volume)`,
  written by the Raid pool half in `after_swap`, overwritten by the next swap of that pool. The
  token half consumes it on the first transfer into `recipient`'s holding in the same clock slot
  (the delivery: the DEX sends `after_swap` deltas only to other holdings, then delivers). No
  dependence on `Pool.swap_count` (the DEX increments it before `after_swap`, `swap.rs:331`, and
  Anchor writes the pool back only at the end of the instruction).
- **R5 Route.** The route reaches pool hooks in a new DEX-filled field `PoolHookArgs.route:
  RouteContext`, never in `hook_data` (a trader writes `hook_data` freely). A plain swap carries a
  one-hop route. Each hop's input is what the previous hop delivered.
- **R6 Pool items never set the LP fee** (it is the protocol's revenue on launch pools). Raid
  discounts come out of the target token's own creator and holder fees, capped at them.
- **R7 Params.** An item's parameters are a fixed array of `PARAM_FIELDS` `u32` fields. The armory
  checks floor and ceiling per field from the `Template` record; `hookwars_items` exposes
  `validate_params` and `combine_params` for template-specific rules; the forge rule uses
  `FORGE_GAIN_BPS` per field toward its ceiling, defined per template in 04.
- **R8 Locked slots.** The kit (and upstream Half-Life or tax_hook if used locked) keep upstream
  args and `HookReturn`; the token program passes the full 64 bytes and applies only the bytes in
  the slot's range. The kit's range is bytes 0..32 (hooks-v2 4.4). A mint with the kit has 32
  bytes left for item ranges.
- **R9 The kit changes in two places:** `init` accepts a mint whose kit is in a `Locked` slot
  (`programs/bordrless_kit/src/instructions/init.rs:92-96`), and R10.
- **R10 Sieges and the kit.** A kit with holder rewards refuses off-curve owners (hooks-v2 4.5), so
  a war chest cannot hold such a token. For v1, `siege` refuses a rival whose kit has holder
  rewards on (`SiegeTargetHasRewards`). Exempting war chests in the kit is open (D-7).
- **R11 Votes lock in place.** `set_vote_lock(amount, until)` on the token program, signed by the
  armory, stores a lock in `Holding.reserved`; a transfer may not take the holding below its
  locked amount. No escrow holdings.
- **R12 Equip at launch.** Non-`Locked` slots start empty at mint creation; the launchpad equips
  launch items through the armory **before** the supply `mint_to`; items never subscribe to mints
  on non-`Locked` slots, which keeps a companion launch inside call depth 5.
- **R13 Treaty payments** go to the partner's war chest, which streams to holders with the kit's
  `share` (never a direct reward-vault deposit, which a buy just before could capture).
- **R14 Season prize.** The DEX `fee_collector` points at `["prize-vault"]` under war;
  `split_protocol_fees` sends `SEASON_PRIZE_SHARE_BPS` to the last winner's chest and the rest to
  the protocol wallet. No DEX code change.
- **R16 Protocol payouts skip item slots.** Transfers out of protocol vaults (equip vaults,
  pool cuts, royalty holdings, war chests, treaty inboxes) prove their source with
  `ProtocolSource` and run only the `Locked` slot (01 section 3.3).
- **R17 Launch equip name.** The launchpad calls `hookwars_armory::equip_launch`, signed by
  `["armory-caller", mint]` under `<LAUNCH_ID>`, inside `prepare_launch` (03), before any supply.
- **R18 Item events are program logs.** Items are leaves, and `emit_cpi!` is a self-CPI, so
  `hookwars_items` callbacks emit with `emit!` (logs); the indexer decodes logs for items and
  self-CPI events for every other program (06).
- **R19 Composites** (08): one item, template 41, runs up to `MAX_MODULES` modules inside one
  `hookwars_items` callback; one slot, one call level, one cut into the equip vault with
  per-module destinations in `EquipState`, one royalty to the composite's owner; refusals AND,
  cuts, discounts and burns summed and capped, one fee override per side; each module has its own
  hook-data sub-range and may only read earlier modules' bytes. Module list in `["composite", item]`.
- **R20 The kit excludes protocol vaults instead of refusing them** (replaces 08's refusal and
  narrows R10). The kit treats as excluded owners (not settled, not capped, not counted, like the
  pool and the launch): (a) owners derivable from the mint: each `["equip", mint, slot]` and
  `["pool-cuts", mint]` under items, `["war-chest", mint]` and `["treaty-inbox", mint]` under war,
  checked by derivation; (b) the destination of a `transfer_from_protocol` (R16), which the token
  program marks in the kit's arguments after verifying the protocol source. Token-side cuts and
  royalty settlements therefore work on tokens with holder rewards on. Sieges of such tokens stay
  refused (R10) until war chests holding foreign kit tokens are covered by (a) for the foreign mint.
- **R21 Pool slots may burn.** `SlotBounds` gains `may_burn` for Pool and Composite-in-Pool slots;
  Sell Burn and Target Burn need it.
- **R22 Derived registry extras.** An item registry entry may be a PDA whose seeds include prefix
  account keys (for example `["holding", other_mint, source_owner]` under the token program), so
  items can read a holder's holding of another mint.
- **R23 Holder touch.** A holder may `touch` their own holding for an item's social payloads
  (Guild Tag, Rank Badge refresh); the item checks the authority is the owner.
- **R24 Item payouts** (`claim_loyalty`, `settle_referral`, `settle_equip`) leave protocol vaults
  only through `transfer_from_protocol`.
- **R25 to R30 (agents, 09):** R25 agents never operate a token's rules; R26 attribution through
  the agents `record` call after each instruction's own effects; R27 the three proof levels
  (declared, linked, attested) and what each proves; R28 the soulbound badge shape (template 42) and
  the armory accepting the agents caller for that shape only; R29 diplomat bonds forfeit only on a
  real rejection and ship only after review 1 H-1, M-1, M-2 are fixed; R30 broker fee is the
  royalty first, a war chest fee is opt-in. Full text in 09.
- **R31 to R36 (expansion, 10):** R31 item income belongs to the holder at claim time and claims are
  refused while an item is listed; R32 rent is a share of the royalty, never on top; R33 seasonal
  meta applies only to new equips, items and forges; R34 the template author's share comes out of
  item royalties, never on top; R35 the war-chest marker exclusion replaces R10's refusal if it fits
  the measured budgets; R36 no outcome transfers between communities. Full text in 10.
- **R37 to R45 (hook economy, 11):** fee waterfall order (protocol fee first, token side only; R37), access enforced at equip and never by failing trades (R38, R39), royalties of kit tokens with holder rewards paid to the agent key (R40), memo directives bound by hash to an on-chain Directive account (R41), material drops only from protocol-verified activity (R42), levels from protocol-only counters (R43), order book fully escrowed (R44), licence payments to the item holder (R45). Full text in 11.
- **R15 Events the app relies on** (06 section 9): every part emits the events 06 lists, with the
  names 06 uses unless the part already named them; 06 adopts the parts' names where they differ.

Open after integration:

| Id | Question | Recommendation |
| --- | --- | --- |
| D-7 | Exempt war chests in the kit so sieges can target holder-reward tokens | Later: needs the rival chest's mint in the kit's accounts; measure first |
| D-8 | Epoch byte wraps after 255 equips of one slot | Widen to 2 bytes only if a slot can plausibly see 255 equips (vote period bounds it) |
| D-9 | Timelock on season admin powers (score weights, loot tables) | Yes, the same timelock as the protocol's other admin setters |
| D-10 | First attestation verifiers (09) | Owner to name; timelocked set |
| D-11 | Use the Agent Credit rung on chain (09) | Later; indexer-only first |
| D-12 | Keep the one-time broker fee from war chests (09) | Opt-in per community, as specified |
