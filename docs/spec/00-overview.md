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

## 3. Programs

| Program | Status | Role |
| --- | --- | --- |
| `bordrless_token` | changed | mints, holdings, transfers; **slots** replace the single hook; `touch` |
| `bordrless_swap` | changed | DEX; **observation ring** per pool; **route context** for multi-hop swaps |
| `bordrless_launch` | changed | launchpad; launches create a slot table; its pool hook **forwards to pool-kind items** |
| `bordrless_companion` | changed | gains `war_bps` in its split, paid to the token's war chest |
| `bordrless_kit` | kept | launch rules; equippable only in a locked slot (it keeps its own accounting) |
| `bordrless_bridge` | kept | unchanged |
| `half_life`, `tax_hook` | kept | kept as programs; their behaviour is also offered as templates in `hookwars_items` |
| `hook_tester` | kept | test-only |
| `hookwars_armory` | new | items, royalties, equip rules, loot minting, crafting |
| `hookwars_items` | new | one program implementing every template; behaviour chosen by the item's template id and parameters |
| `hookwars_war` | new | war chests, war state, siege, counter-strike, raze, bounties, loot tickets, quests, seasons |

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
| `Relation` | 3 | token transfers | cuts, hook data; reads other pools |
| `Pool` | 4 | launch pool swaps, through the launchpad | fee override, cuts, burn (within the launchpad's own rules and the slot's bounds), war-state marks |
| `Locked` | 5 | as the kit does today | whatever the equipped program's flags allow; never re-equipped |

### 4.2 Equip rules

| Rule | Value | Meaning |
| --- | --- | --- |
| `Locked` | 0 | the launch item stays for ever |
| `Vote` | 1 | holders vote by locking tokens; equips after `notice_secs` |
| `Performance` | 2 | reverts to the launch item when its on-chain condition holds; anyone cranks |

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
| hook signer | token, swap, launch | upstream `["hook-authority", program]`, unchanged |

### 4.4 Hook data

Upstream: 64 bytes per holding, written only by the mint's hook. Hookwars splits them into
**ranges**, one per slot, fixed when the slot is created (`data_offset`, `data_len`); an item may
read and write only its own range; the token program enforces it. Templates declare how many
bytes they need (04). Byte 0 of each range is a layout tag; a range of all zeros means
"never stamped". The holding can be closed only when all 64 bytes are zero (upstream rule).

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
