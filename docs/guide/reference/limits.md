# Limits

Solana's limits shape units: 1,232 bytes per transaction, call depth 5, 64 trace entries, 64 account locks, 1,400,000 compute units. Every figure below was measured in the LiteSVM test suite; the test that measured it is named.

## Protocol limits (build values)

| Limit | Value | Set by |
| --- | --- | --- |
| Slots per token | 4 (kit + 3 items) | `budgets_launch.rs` |
| Cutting slots per transfer | 3 (2 when the locked slot cuts) | `budgets_launch.rs` |
| Route hops | 3 | `budgets.rs::route_budgets` |
| Modules per composite | 4 | provisional |
| Holder memory per holding | 64 bytes; kit uses 32 | token program |

## Trades on a launch pool

Kit in slot 0 with holder rewards and max wallet on. "With table" puts every account in one lookup table.

| Path | Bytes | With table | Trace | Depth | CU |
| --- | --- | --- | --- | --- | --- |
| Buy, kit only | 956 | 370 | 15 | 3 | 148,171 |
| Buy, kit + 3 pool items | 1,125 | 384 | 23 | 3 | 259,388 |
| Buy, kit + 3 token items | 1,224 | 390 | 18 | 3 | 230,475 |
| Buy and graduate, kit + 3 pool items | 1,391 | 464 | 37 | 4 | 388,884 |
| Route, 2 hops into a launch pool, 3 pool items | 1,303 | 407 | 29 | 3 | 332,413 |

Source: `programs/tests/tests/budgets_launch.rs`. Several paths exceed 1,232 bytes without a lookup table, so the app always uses the protocol table plus a per-token table.

## Launch

| Path | With table | Trace | Depth | CU |
| --- | --- | --- | --- | --- |
| `prepare_launch`, kit + 3 slots | 525 | 8 | 3 | 52,327 |
| `create_prepared_launch`, kit + 3 pool items | 545 | 44 | 4 | 359,908 |
| Companion slot launch, 1 pool item | 559 | 38 | 5 | 332,405 |

Sources: `budgets_launch.rs`, `e2e.rs`. The companion slot launch is at the call-depth limit.

## Wallet transfers and items

| Path | Depth | CU | Source |
| --- | --- | --- | --- |
| Transfer, 3 cutting slots | 2 | 62,856 | `budgets.rs` |
| Execute an equip (Transfer Fee) | 3 | 138,093 | `budgets.rs::armory_execute_and_forge` |
| Forge two Raid items | 3 | 159,054 | same |
| Settle a composite of 3 modules | 3 | 208,107 | `budgets_items.rs` |

## War

| Path | Depth | CU |
| --- | --- | --- |
| Siege | 4 | 312,139 |
| Counter-strike | 4 | 324,581 |
| Raze | 4 | 273,955 |
| Claim bounty | 4 | 117,702 |
| Roll loot | 3 | 63,689 |

Source: `programs/tests/tests/budgets_war.rs`.

## Account rent

| Account | Bytes | Rent (lamports) |
| --- | --- | --- |
| Slot mint | 1,009 | 7,913,520 |
| Pool with a 32-entry price ring | 2,043 | 15,110,160 |
| War state | 984 | 7,739,520 |

Sources: `budgets.rs::the_slot_table_size_and_rent`, the M3a notes, `budgets_war.rs`.
