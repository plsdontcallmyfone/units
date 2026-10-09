# Hookwars spec 07: budgets, tests and the security checklist

Status: specification, 2026-10-08. Every figure in this file is **to measure**: a LiteSVM test
named here produces it, asserts a ceiling, and the measured value is written next to the test name
when it first passes. Nothing here is an estimate.

## 1. Mainnet limits that bind

From upstream `docs/hooks-v2.md` section 6: 64 instruction-trace entries, invoke height 5, 1,232
bytes per transaction, 64 account locks. Upstream's own measurement: a launch with one custom hook
and the longest metadata is about 1,200 of 1,232 bytes (`programs/half_life/README.md`, "Launching
with it"). Hookwars adds accounts to every hooked path, so per-mint lookup tables (06) are not
optional.

## 2. Call depth per path

Height 1 is the top-level instruction. Items are leaves (00 R3): they make no CPIs.

| Path | Chain | Height to measure | Upstream reference |
| --- | --- | --- | --- |
| Wallet transfer, slots equipped | token, item | 2 | upstream 2 |
| Buy on a launch pool | DEX, token (input), item; DEX, launchpad (pool hook), pool item; DEX, token (delivery), item | 3 | upstream 3 |
| Sell on a launch pool | as buy | 3 | upstream 3 |
| Multi-hop `swap_route` | hops loop inside one DEX instruction (03) | 3 | new |
| Buy and graduate | launchpad `graduate`, DEX, token, item | 4 | upstream 4 |
| `create_launch` with slots | launchpad, armory (equip), token `set_slot_item`; launchpad, token `mint_to` (Locked slot only) | 4 | upstream 4 |
| Companion launch | companion, launchpad, then as above | 5 | upstream 5 (`docs/companions.md`, "Call depth") |
| `siege` / `counter_strike` / `raze` | war, DEX, token, item | 4 | new |
| `claim_bounty` / `reveal` / `claim_quest` | war, token `touch`, item | 3 | new |
| `reveal` minting loot | war, armory `mint_loot`, token | 3 | new |
| `execute` equip | armory, token `set_slot_item` | 2 | new |
| `forge` | armory, token burn x2 and mint | 2 | new |
| `settle_equip` | items, token transfers | 2 | new |

**Failure rule:** if any path measures above 5, the part that owns it changes the design (the
companion launch is the one at the limit; 00 R12 is what keeps it there).

## 3. Transaction budgets to measure

For each, the test records keys, v0 bytes with the protocol and per-mint lookup tables, trace
entries, call height and compute units, and asserts a ceiling.

| Path | Variants | Test |
| --- | --- | --- |
| Buy with SOL in | 0, 1, 2, 3 cutting slots; kit on and off; Raid pool item on | `budgets.rs::buy_*` |
| Sell with SOL out | same | `budgets.rs::sell_*` |
| Buy and graduate | with the maximum slot set | `budgets.rs::buy_graduate_max` |
| Raid buy via `swap_route` | 2 hops up to `MAX_ROUTE_HOPS` | `budgets.rs::route_*` |
| Wallet transfer | 0 to `MAX_SLOTS` slots | `budgets.rs::transfer_*` |
| `create_launch` | Plain; kit + 1 item; kit + max items; companion | `budgets.rs::launch_*` |
| `siege`, `counter_strike`, `raze` | kit off on the rival (R10) | `budgets.rs::war_*` |
| `settle_equip` | token side, pool side | `budgets.rs::settle_*` |
| `execute` equip with registry creation | each template | `budgets.rs::equip_*` |
| `roll` and `reveal` | with the chosen oracle (D-4) | `budgets.rs::loot_*` |

Measured in M1 (`budgets.rs::transfers_buys_and_sells_with_zero_to_three_cutting_slots`, ordinary
pool, `slot_tester` items; full table in `01-token-slots.md`, M1 notes): wallet transfer with 3
cutting slots 631 v0 bytes (293 with a table), 6 trace entries, height 2, 62,856 CU; DEX buy with 3
cutting slots 877 bytes (322 with a table), 10 trace entries, height 3, 106,733 CU. Mint with the
slot table: 1,005 bytes, rent 7,885,680 lamports (upstream 519 bytes, 4,503,120). Launch-pool paths
are still to measure (M3).

Measured in M3a (spec 03, "M3a implementation notes"): buy and sell with the observation ring
54,933 and 54,937 CU (keys and bytes unchanged); `swap_route` 2 hops 19 keys, 787 v0 bytes (325
with a table), 13 trace, height 3, 105,903 CU; 3 hops 24 keys, 960 bytes (343), 18 trace, height
3, 154,746 CU; 2 hops delivering a 3-cutting-slot mint 27 keys, 1,055 bytes (345), 16 trace,
height 3, 161,616 CU; pool account with a 32-entry ring 2,043 bytes, rent 15,110,160 lamports.

These measurements set `MAX_SLOTS`, `MAX_CUTTING_SLOTS`, `MAX_ROUTE_HOPS`, `OBS_RING_LEN`,
`PARAM_FIELDS`, `MAX_CAPTURED`, `RAID_TABLE_LEN` and `LOOT_TABLE_LEN` (00 section 6, set by M).

## 4. Invariants every suite checks after every step

Money is conserved and nothing is stranded:

1. **Token supply:** `mint.supply` equals the sum of all holdings of the mint, after every
   transfer, burn, forge and siege (upstream's event post-balances let the test sum them).
2. **Cuts:** for every transfer, `amount = delivered + sum(deltas)`; no delta exceeds its slot's
   bound; the sum of a transfer's cuts never exceeds the amount (01).
3. **Equip vaults and pool cuts:** the vault balance equals the sum of unsettled shares recorded in
   the `EquipState` accounts that point at it (R1, R2).
4. **Royalties:** after `settle_equip`, royalty paid equals `floor(cut * royalty_bps / 10000)` per
   item record, and the rest reached its destination.
5. **War chest:** never spends more than its balance less rent; every spend is one of `siege`,
   `counter_strike`, `claim_bounty`, treaty streaming; each within its parameter.
6. **Pools:** upstream's reserve rule holds: the quote vault holds `quote_reserve +
   protocol_fees_quote` and the base vault `base_reserve` after every swap (hooks-v2 3.1).
7. **Hook data:** an item never changes a byte outside its range; the epoch byte changes on every
   equip; a holding with all-zero data can close.
8. **Votes:** a holding never goes below its vote lock (R11).

## 5. Test plan per milestone

New files live in `programs/tests/tests/`; upstream suites stay and must stay green unless a test
asserts a behaviour Hookwars changed on purpose, in which case it is edited with a `Changed by
Hookwars` note.

| Milestone | Suites | Must show |
| --- | --- | --- |
| M0 | all upstream suites | green under new program ids |
| M1 | `slots.rs`, `budgets.rs` (transfer, buy, sell) | answers merged, ranges enforced, epoch byte, Locked kit masking, bounds refusals, measurements recorded |
| M2 | `armory.rs`, `votes.rs`, `royalties.rs` | template registration refusals (upgradeable program, changed code hash), item creation, vote lock in place, quorum, notice, execute, performance revert, royalty settle and claim by the current holder after a sale |
| M3 | `observations.rs`, `route.rs`, `pool_items.rs`, `templates_*.rs` | TWAP over the ring, a one-block spike does not move a `MIN_TWAP_SECS` read, route context unforgeable, raid mark consumed at delivery only, Shield, Wall, Spy, Treaty (needs both sides), Tribute |
| M4 | `war.rs`, `siege.rs`, `counter_strike.rs` | chest funding from `war_bps`, siege due and not due, `SiegeTargetHasRewards`, sandwich bound (attacker net <= 0 under the companion's argument), raze rate limit, peace return, bounty spend through `touch` |
| M5 | `loot.rs`, `forge.rs`, `quests.rs`, `seasons.rs` | roll then reveal with a stub oracle, never the slot hash; forge burns two, mints one within ceilings; quest once per period, markers move by max on transfer; season submit, challenge, finalize in O(1); prize split |
| M6 | site checks (06), devnet drill | every page renders against devnet, empty states, no monospace, no invented figures |

## 6. Security checklist (before any devnet deploy)

| Area | Check |
| --- | --- |
| Signers | every callback checks the caller's hook signer (`["hook-authority", program]`) and that the item is in the mint's slot table (04) |
| Slot authority | only `["slots", mint]` under the armory can call `set_slot_item` and `set_vote_lock` |
| Loot | only `["loot-signer"]` under war can call `mint_loot` |
| Code under items | template program upgrade authority is none, `<MANAGED_HOOK_KEY>` or `<PROTOCOL_AUTHORITY>`; code hash checked at equip |
| Refusals | `may_refuse` is declarative (a failed CPI cannot be caught); templates that declare `false` are fuzzed in `templates_*.rs` to never refuse |
| Price reads | every rival read is a TWAP over at least `MIN_TWAP_SECS`; reader returns "no signal" rather than reverting |
| Route | route context set by the DEX only; `hook_data` never trusted for it |
| Cranks | every permissionless instruction checks it is due and caps its bounty at `MAX_CRANK_BOUNTY_BPS` |
| Swaps from chests | output no worse than the pool's own quote less fees and the slippage parameter; reference price that only falls (companion rule) |
| Admin | every admin setter (templates, season weights, loot tables, parameters) behind the protocol timelock (D-9) |
| Upgrades | upgrade authorities move to a multisig behind a timelock before mainnet (upstream 4.14) |
| Keys | program keypairs backed up off the repo before any build that could delete `target/` |

## 7. Toolchain and where the suites run

Upstream pins Linux x86_64: Agave 4.3.0, platform-tools v1.57, Anchor CLI 1.2.0, host rustc
>= 1.97.1 (`scripts/solana/toolchain.sh`, `rust-toolchain.toml`). This machine is macOS arm64 with
a machine-wide Agave 3.1.12 used by other sessions and limited free disk. The suites run either:

1. in a Linux x86_64 environment (a container or a remote box) with the pinned toolchain, or
2. on this Mac with the pinned versions installed **side by side** under a Hookwars-only
   directory, never replacing `~/.local/share/solana/install/active_release`.

Option 1 is recommended (reproducible builds match upstream's `solana-verify` image). Owner
decision before M0.
