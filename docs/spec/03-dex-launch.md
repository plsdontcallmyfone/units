# Hookwars spec 03: DEX, launchpad and companion changes

Status: specification, 2026-10-08, revised after the integration rulings (00 section 9). Nothing
here is built. Follows `00-overview.md`; where this file disagrees with it, this file is wrong.

Upstream references are to Bordrless at `43688f3`. "hooks-v2" means `docs/hooks-v2.md`.

## 1. Scope

| Program | Change |
| --- | --- |
| `bordrless_swap` | an **observation ring** per pool (price, quote volume, swap count); a **multi-hop** `swap_route`; a DEX-filled **route** in every pool callback (R5) |
| `bordrless_launch` | a two-step launch: `prepare_launch` creates the mint with its **slot table** and equips launch items through the armory before any supply exists (R12); `LaunchConfig` carries slots instead of `custom_hook`; the launch pool hook **forwards to `Pool`-kind items** and merges their cuts into one delta (R2) |
| `bordrless_companion` | `war_bps` in the split, paid to the token's war chest |

The DEX's code for protocol fees is unchanged; its `fee_collector` is pointed at war's prize vault by
configuration (R14, section 6.4). Everything else in these programs is upstream behaviour and keeps
upstream's tests.

## 2. Two rules this part relies on

1. **Items are leaves** (R3). An item program (`hookwars_items`) makes no CPI on any callback. It
   reads accounts, writes accounts it owns (`EquipState`, `RaidLedger`) and answers. Cuts and burns
   are applied by the DEX from the launchpad's merged answer. This keeps every swap path inside
   Solana's invoke height of 5 (section 8).
2. **The route is the DEX's word, never the trader's** (R5). Pool hooks read the route from a field
   the DEX fills (section 3.2), never from `hook_data`, which the trader controls.

## 3. DEX (`bordrless_swap`)

### 3.1 Observations

A new account per pool, `Observations` at `["obs", pool]` under the DEX, created by `create_pool` in
the same instruction as the pool (payer: the pool's creator). Every pool of the Hookwars deployment
therefore has one from its first swap; there are no pools from before (new program ids, 00
section 3), so there is no migration.

```rust
pub struct ObservationHeader {
    pub version: u8,
    pub bump: u8,
    pub pool: Pubkey,
    /// Price at the end of the last write, quote per base as Q64.64, virtual reserves included:
    /// (quote_reserve + virtual_quote) / (base_reserve + virtual_base).
    pub last_price_q64: u128,
    /// Time of the last accumulation.
    pub last_ts: i64,
    /// Next ring entry to write.
    pub index: u16,
    /// Entries filled (grows to OBS_RING_LEN, then the ring wraps).
    pub filled: u16,
}

pub struct Observation {
    pub ts: i64,
    /// Sum over time of last_price_q64 * seconds, wrapping (readers subtract, as Uniswap v2 does).
    pub price_cumulative: u128,
    /// `Pool.quote_volume` at `ts` (upstream `state.rs`, u128, both directions).
    pub quote_volume: u128,
    /// `Pool.swap_count` at `ts` (upstream `state.rs`, u64).
    pub swap_count: u64,
}
```

An entry is 48 bytes; the account is the header plus `OBS_RING_LEN` entries. `OBS_RING_LEN` is to
measure (rent against the longest window a relation item or a `Performance` rule may read).

**When it is written.** At the **start** of every instruction that can move the price (`swap`, each
hop of `swap_route`, `add_liquidity`, `remove_liquidity`, `finalize_curve`), before any reserve
changes:

1. `dt = now - last_ts`. If `dt > 0`, accumulate `last_price_q64 * dt` (wrapping) and, when the
   newest entry is older than `now`, write `{ now, cumulative, pool.quote_volume, pool.swap_count }`
   at `index`, advance `index` modulo `OBS_RING_LEN`, raise `filled` up to `OBS_RING_LEN`.
2. At the **end** of the same instruction, `last_price_q64` is set to the new price and
   `last_ts = now`.

The price that accrues for any second is the price at which the previous second ended, so a swap
never changes the price it is itself accumulated at. At most one entry is written per second.

**Reads** (pure functions in `bordrless_core`, mirrored in the SDK and pinned by test vectors as
upstream pins its fee math; used by items, the armory's `Performance` rule (02 section 6.7) and
`hookwars_war`):

```
window_read(obs, pool, now, window):
  require window >= MIN_TWAP_SECS                       else TwapWindowTooShort
  cum_now = newest.price_cumulative + last_price_q64 * (now - last_ts)
  e = newest entry with e.ts <= now - window            else ObservationTooOld
  twap        = (cum_now - e.price_cumulative) / (now - e.ts)
  quote_volume = pool.quote_volume - e.quote_volume
  swaps        = pool.swap_count - e.swap_count
```

The current `pool.quote_volume` and `pool.swap_count` are read from the `Pool` account, so a reader
passes both accounts. Neither read needs the DEX's signer (02 I-03-1). A reader that cannot read (a
new pool, too short a history) treats it as **no signal**: every template (04) and the
`Performance` rule (02) define no signal as no effect. A short history never blocks a trade.

**Manipulation.** A price held for one slot weighs one slot's time in a window of at least
`MIN_TWAP_SECS`. Moving a TWAP by `p` over a window `W` means holding the pool `p * W / t` away from
its price for `t` seconds, paying the LP fee (on a launch pool, Bordrless's) and the curve's
slippage both ways while being arbitraged. A leader of several consecutive slots can hold a price
without arbitrage for those slots only, so `MIN_TWAP_SECS` must span many leader rotations (owner
decision with this cost in front of it). Volume and swap-count windows can be inflated by wash
trading at the trader's own fees; a `Performance` rule reading them is a revert to the launch item,
never a payment, so inflating them buys nothing but the revert's timing.

`Observations` is passed **writable** to every price-moving instruction (one more account on each
swap; measured in 07). Relation items, the armory and war read other pools' observations
**read-only**.

### 3.2 `PoolHookArgs.route` (R5)

Upstream `PoolHookArgs` (`crates/bordrless-hook/src/lib.rs:206`) gains one field, filled only by the
DEX:

```rust
pub struct RouteContext {
    /// The mint the trader started with (the first hop's input).
    pub route_input_mint: Pubkey,
    /// The mint the trader ends with (the last hop's output).
    pub route_output_mint: Pubkey,
    /// The pool of the first hop (an item checks it is the launch pool of a rival).
    pub first_pool: Pubkey,
    /// What the first hop took from the trader, in route_input_mint units.
    pub route_amount_in: u64,
    /// This hop, from 0.
    pub hop_index: u8,
    /// How many hops; 0 on liquidity and initialize callbacks (not a swap).
    pub hop_count: u8,
}
```

- A plain `swap` fills a **one-hop route**: `route_input_mint = in_mint`,
  `route_output_mint = out_mint`, `first_pool = pool`, `hop_index 0`, `hop_count 1`. The route is
  never absent, so no item has an "unknown route" case.
- `hook_data` stays the trader's opaque bytes (`MAX_HOOK_DATA`, upstream `lib.rs:41`), forwarded as
  today; the launchpad and every Hookwars template ignore it, because any trader can write any bytes
  there and a route read from it could be forged.

The change to the shared crate type is not wire-compatible with upstream, which is fine for a new
deployment. It adds 106 bytes to each pool callback's instruction data.

### 3.3 `swap_route`

```rust
pub struct SwapRouteArgs {
    /// Exact input of the first hop.
    pub amount_in: u64,
    /// The least the trader's final holding must gain.
    pub min_amount_out: u64,
    /// Per hop, in order; 1 ..= MAX_ROUTE_HOPS (EmptyRoute, RouteTooLong).
    pub hops: Vec<HopArgs>,
}

pub struct HopArgs {
    pub direction: u8,               // as SwapArgs
    pub accounts: u8,                // remaining accounts this hop takes
    pub in_hook_accounts: u8,
    pub out_hook_accounts: u8,
}
```

Accounts: `trader` (signer), `config`, `token_program`, `token_event_authority`, event authority and
program, then the remaining accounts as consecutive hop groups. A hop group is what `swap` takes for
one pool (pool, `Observations`, base mint, quote mint, base vault, quote vault, trader input
holding, trader output holding, hook program or this program's id, hook signer or this program's
id) followed by that hop's token-hook slices (01 section 2.2) and pool-hook extras, `accounts` in
all.

Rules:

1. Hop `i`'s output mint equals hop `i+1`'s input mint, and hop `i`'s trader output holding is hop
   `i+1`'s trader input holding (`RouteBroken`).
2. No pool appears twice (`RoutePoolRepeated`).
3. Hop `i+1`'s `amount_in` is what hop `i` **delivered**, measured on the holding (upstream rule,
   `swap.rs:27`), so a token hook's cut on a delivery is never counted twice.
4. Each hop runs the whole of upstream `process_swap` (`swap.rs:107`), steps 1 to 7, with the route
   of section 3.2. Hops are a loop **inside one instruction**, not CPIs: a route is no deeper than a
   swap (section 8).
5. Only the last hop checks `min_amount_out` (`Slippage`); intermediate hops use 0.
6. Each hop emits its own `Swapped` (with its `route`, section 9) and the instruction emits
   `RouteSwapped` once.

`MAX_ROUTE_HOPS` is to measure: a launch-pool hop with a kit slice and one Pool item is on the order
of twenty accounts, so two hops with the protocol lookup table is the expected ceiling. The Raid
template needs exactly two: rival to bridged SOL, bridged SOL to us.

New DEX errors: `EmptyRoute`, `RouteTooLong`, `RouteBroken`, `RoutePoolRepeated`,
`TwapWindowTooShort`, `ObservationTooOld`, `WrongObservations`.

## 4. Launchpad: the slot table and launch items (`bordrless_launch`)

### 4.1 What replaces the single hook

Upstream picks the mint's one hook in `plan()` (`launch.rs:500-540`): the kit when a kit module is
on, else the `LaunchConfig`'s `custom_hook` (hooks-v2 5.8), else none. Hookwars replaces that with a
**slot table** (layout owned by 01 section 1, created by 01's `create_mint` with `SlotInit`).

```rust
pub struct SlotSpec {
    pub kind: u8,                 // 00 section 4.1
    pub equip_rule: u8,           // 00 section 4.2
    pub bounds: SlotBounds,       // 01; fixed for ever
    pub notice_secs: u32,         // MIN_NOTICE_SECS ..= MAX_NOTICE_SECS for Vote and Performance
    pub data_len: u8,             // bytes of hook data this slot owns
    pub launch_item: Option<Pubkey>, // an armory Item equipped before the supply exists (R12)
    pub rule_data: Vec<u8>,       // Performance condition, decoded by the armory (02 section 6.7)
}
```

`LaunchConfig` (`state.rs:169`) replaces `custom_hook` and `custom_hook_flags` (`state.rs:181-183`)
with `slots: Vec<SlotSpec>`; a launch from a config must pass the config's slots unchanged
(`ConfigMismatch`, upstream's rule for rules and creator fee, hooks-v2 5.7). Listed configs and the
author share are unchanged. Upstream's custom-hook path (`check_custom_hook_accounts`,
`launch.rs:415`, and its errors) is removed; the safety rule it carried (who may upgrade a hook,
hooks-v2 section 7) moves to template registration (02).

**The kit is slot 0, `Locked`** (R8). When the rules install a kit module, the launchpad writes slot
0 as `{ kind: Locked, equip_rule: Locked, locked_program: KIT_ID, data_len: 32 }`. The kit keeps
bytes 0..32 (`programs/bordrless_kit/src/state.rs:280`); items share the other 32. The kit's own
flags, registry and `kit-caller` init are unchanged except for R9 (its `init` accepts a mint whose
kit is in a Locked slot, 02 and 01).

**Slot authority.** `slot_authority = PDA(["slots", mint], <ARMORY_ID>)` whenever any slot is not
`Locked` (01 section 1.4); upstream's `hook_authority` stays `None`.

### 4.2 Checks (in `prepare_launch`, repeated by `create_launch` against the mint)

| Check | Error |
| --- | --- |
| `slots.len() <= MAX_SLOTS` | `TooManySlots` |
| kinds known; at most one `Locked` slot, and it is slot 0 | `InvalidSlotKind` |
| equip rule known; a `Locked` kind has the `Locked` rule | `InvalidEquipRule` |
| notice within `MIN_NOTICE_SECS ..= MAX_NOTICE_SECS` for `Vote` and `Performance` | `NoticeOutOfBounds` |
| sum of `data_len <= 64` (01 assigns offsets in slot order) | `HookDataOverflow` |
| token-side cutting slots `<= MAX_CUTTING_SLOTS` (R1) | `TooManyCuttingSlots` |
| a `Pool` slot's `max_cut_bps` (applied per side, cut and burn together) `<= MAX_POOL_ITEM_CUT_BPS`, and the sum over `Pool` slots `<= MAX_POOL_ITEM_CUT_BPS` | `SlotBoundsTooHigh` |
| a `Performance` slot has `rule_data` the armory accepts and a `launch_item` | `InvalidRule` |
| each `launch_item` fits its slot (the armory's check, run by the armory in 4.3 step 3) | the armory's errors (02) |

### 4.3 Two steps: `prepare_launch`, then `create_launch` (R12)

Upstream does everything in one `create_launch` (`launch.rs:1055-1079`: plan, launch fee, mint,
launch holdings, supply, kit, curve pool, pool registry, record, event), and its own launch with a
custom hook already uses about 1,200 of the 1,232 bytes (half_life README, measured). Equipping
items before the supply (R12) adds the armory, the items program and each item's accounts, which
cannot fit in that transaction. Hookwars splits it, the way upstream's Half-Life flow already splits
`prepare`, launch and `light` into three transactions (`programs/half_life/README.md`, "Launching
with it").

**Transaction 1, `prepare_launch(args: PrepareLaunchArgs { name, symbol, uri, slots })`**, signed by
the creator and the new mint keypair:

1. Check section 4.2.
2. CPI token `create_mint` (01 section 4.1) with the slot table, `slot_authority`, and
   `mint_authority = PDA(["launch-mint", mint], <LAUNCH_ID>)` (a PDA with no other power, so only
   `create_launch` can ever mint the supply). Metadata authority as upstream.
3. For each slot with a `launch_item`, CPI `hookwars_armory::equip_launch(slot, item)` (02 must
   add it; I-02 below), signed by `PDA(["armory-caller", mint], <LAUNCH_ID>)`. The armory checks the
   item fits the slot, records it as the slot's **launch item** (the target of a `Performance`
   revert), CPIs token `set_slot_item` signed as `SlotAuthority` (01 section 4.2), and CPIs
   `hookwars_items::init_equip` (04 section 2.6), which creates `EquipState`, the item registry and
   the equip vault holdings.
4. Write `PreparedLaunch` at `["prepared", mint]` under `<LAUNCH_ID>`: `creator`, `slots_hash`,
   `prepared_at`. Event `LaunchPrepared { mint, creator, slots }`.

**Transaction 2, `create_launch(args: CreateLaunchArgs)`**, signed by the creator and the mint
keypair: upstream's steps with three changes.

- The mint already exists: `create_mint` is skipped; the mint's slot table must hash to
  `PreparedLaunch.slots_hash` and `PreparedLaunch.creator` must be the signer (`NotPrepared`,
  `WrongCreator`). Only the mint keypair can call it (it signs), so nobody can launch someone else's
  prepared mint.
- **Supply:** `mint_to` signed by `["launch-mint", mint]`, then the mint authority is revoked (as
  upstream revokes it at the end, hooks-v2 5.3). Items never subscribe to mints on non-`Locked`
  slots (R12, 01), so `mint_to` calls only a `Locked` slot that subscribes (upstream kit: it does
  not).
- **Pool registry** (`write_pool_registry`, `launch.rs:926`): upstream's four extras
  (`registry_list`, `launch.rs:187`: launch, quote holding, holder vault, kit config) at indices 5 to
  8, then Hookwars' fixed extras at 9 to 12, then each equipped `Pool` slot's item extras (section
  5.5).

| Index | Account | Why |
| --- | --- | --- |
| 9 | the `PoolCuts` holding of the quote: `holding(BRIDGED_SOL, PDA(["pool-cuts", mint], <ITEMS_ID>))` (writable) | the single delta of all Pool-item cuts on a side (R2) |
| 10 | the launch pool's `Observations` (read-only) | items read their own pool's TWAP |
| 11 | the launchpad's event authority | `emit_cpi!` of `PoolItemCuts` (section 9) |
| 12 | the launchpad program | likewise |

`create_launch` creates the `PoolCuts` quote holding (token `create_holding`, payer the creator)
before the registry is written.

**Transaction 3, `hookwars_war::init_war(mint)`** (05 section 5), when the launch has a `War` slot:
permissionless, sent by the site right after transaction 2. Not a CPI from `create_launch`, so
`create_launch`'s size and depth stay upstream's plus the registry entries.

**Between the steps.** After transaction 1 the mint has no supply and only the launch program can
mint it; after transaction 2 trading is open with every launch item already equipped, so no trade
ever runs without its items. A war chest that does not exist yet (before transaction 3) has nothing
to spend and receives nothing: the companion's `claim_fees` and the Raid template's settlement wait
for its holding (05).

**Depth of the two steps.**

| Path | Height |
| --- | --- |
| `prepare_launch` (1), token `create_mint` (2) | 2 |
| `prepare_launch` (1), armory `equip_launch` (2), token `set_slot_item` (3) | 3 |
| `prepare_launch` (1), armory (2), items `init_equip` (3), token `create_holding` (4) | 4 |
| the same from a companion | 5, the limit |
| `create_launch` (1), DEX `create_pool` (2), token deposit transfer (3), slot programs (4) | 4 |
| the same from a companion | 5, the limit (upstream reaches 5 there with the kit, `docs/companions.md:75`) |

The deposit transfer moves the launch's reserve into the pool vault and runs token-side items; it
fits only because items are leaves (section 2). Every row is measured in 07.

A companion launches by calling `prepare_launch` and then `create_launch`, in two transactions,
signing as its creator PDA (upstream `docs/companions.md`, "The companion is the launch's creator").

### 4.4 `refresh_pool_registry(mint)`

The pool registry names the equipped Pool items' accounts, which change on every equip. New,
permissionless: rewrites the pool registry from the mint's current slot table and each Pool item's
registry (04 section 2.3). The armory calls it in the same transaction as every equip and every
`Performance` revert of a Pool slot (I-02 below). A swap built from a stale registry fails in the
DEX's and the launchpad's account checks (`StaleRegistry`) instead of running a removed item.

## 5. Launchpad: forwarding to `Pool` items

### 5.1 Order inside `before_swap` and `after_swap`

Upstream callbacks: `programs/bordrless_launch/src/instructions/hooks.rs:128` (`before_swap`) and
`:162` (`after_swap`). Hookwars keeps them and appends the items:

1. **The launchpad's own rules, unchanged**: sniper LP fee (and the creator's one-time first buy),
   creator fee, holder fee, burn, from `bordrless_core::launch_before_swap` / `launch_after_swap`.
   Call this answer `L`.
2. **Each equipped `Pool` slot, in slot order**, whose template subscribes to this callback (04's
   `pool_callbacks` bits): the launchpad invokes `hookwars_items`'s `before_swap` or `after_swap`
   (04 section 2.1), signing as `LAUNCH_ITEMS_SIGNER` = `["hook-authority", <ITEMS_ID>]` under
   `<LAUNCH_ID>`, with:
   - args: the DEX's `PoolHookArgs` unchanged (with the route) and
     `ItemPoolContext { slot: u8, launch_fee_bps: u16 /* L's creator + holder fee rate this side */,
     launch_cut: u64, side_amount: u64 }`, where `side_amount` is what this callback acts on after
     `L`'s cuts (before: the input; after: the output it was told);
   - accounts: the signer, the five pool prefix accounts (with `base_mint` the `Mint` itself, 04
     section 2.2 check 3), then the item's extras (`Item`, `EquipState`, the template's extras, which
     include `RaidLedger` writable for templates that keep raid state).
3. The launchpad reads each answer (section 5.2), merges them (section 5.3) and returns **one**
   `HookReturn` to the DEX, then emits `PoolItemCuts` (section 9).

### 5.2 What an item answers

```rust
pub struct ItemPoolAnswer {
    /// Basis points of the launchpad's creator and holder fees on this side to waive (raids).
    pub discount_bps: u16,       // <= the template's max_discount_bps (04), <= 10_000
    /// The item's cut, in the quote, into the PoolCuts holding.
    pub cut: u64,
    /// A burn of base, where the side's input or output is base.
    pub burn: u64,
}
```

The item has already added its `cut` to its own `EquipState.collected[1]` in the same call (R2);
the launchpad checks it did (`collected` read before and after, `CutNotRecorded`).

| Check | Error |
| --- | --- |
| `cut` only where the side being acted on is the quote: a buy's `before_swap` (input) or a sell's `after_swap` (output) | `ItemCutWrongSide` |
| `burn` only where it is base: a sell's `before_swap` or a buy's `after_swap` (upstream's burn sides) | `ItemBurnWrongSide` |
| `cut + burn <= side_amount * slot.bounds.max_cut_bps / 10_000` | `ItemCutOutOfBounds` |
| the answer is the item program's own return data | `ForeignAnswer` |

### 5.3 Merging (R2, R6)

- **LP fee.** Items never set it (R6). On a launch pool the LP fee is Bordrless's revenue (hooks-v2
  3.1, "The LP fee of a launch pool is Bordrless's") and the sniper schedule protects every launch
  alike. The merged answer carries `L.lp_fee_bps` unchanged. A Shield that charges more does so as
  its own `cut`.
- **Discounts.** `discount = min(10_000, sum of discount_bps)`. The launchpad's creator fee and holder
  fee on this side are multiplied by `(10_000 - discount) / 10_000`, rounded down; the burn and the
  LP fee never are. A raid discount is paid by the creator and holders of the token raided into,
  capped at their own fees (R6), which is what its holders accepted when the item was equipped.
- **Cuts.** Upstream already answers up to two deltas per side (creator fee to index 6, holder fee to
  index 7, `hooks.rs:102-124`) of `MAX_DELTAS = 3` (`crates/bordrless-hook/src/lib.rs:43`). All
  items' cuts on a side are summed into **one** delta to index 9, the `PoolCuts` quote holding (R2).
  The sum is capped again by `MAX_POOL_ITEM_CUT_BPS` of the side (`PoolCutsTooHigh`), and `L`'s
  cuts, the items' cuts and the burns together must stay below the side's amount (the DEX's
  `DeltaTooLarge`, `swap.rs:182`).
- **Royalties** are not the launchpad's business: each item's share sits in its `EquipState`, and
  `settle_equip` (04) later pays the royalty and the item's destination from the `PoolCuts` holding
  (R2). There are no royalty marks and no war-chest royalty path.
- **Burns** add to `L.burn`.
- **Protocol share.** Unchanged. The DEX measures `cuts_in` and `cuts_out` (`swap.rs:261`, `:415`)
  and takes `protocol_share_bps` of everything a hook cut, so item cuts pay Bordrless's share
  exactly like the creator fee. A launch whose items cut nothing pays nothing more.

### 5.4 Raid marks (R3, R4)

The launchpad delivers no marks and holds no war state. The Raid template's pool half writes the
token's `RaidLedger` (`["raid-ledger", mint]`, owned by `hookwars_items`) itself, as a leaf, in
`after_swap`: one mark `(clock slot, recipient, rival, quote volume)`, overwritten by the next swap of
that pool. The token half consumes it on the first transfer into `recipient`'s holding in the same
clock slot, which is the DEX's delivery (after `after_swap`, the DEX sends deltas only to other
holdings, `swap.rs:379-409`, then delivers). Nothing depends on `Pool.swap_count`. 03's part is only
to pass `RaidLedger` writable among the item's extras (from the item's registry) and to call the
Raid item's `after_swap` on buys. `hookwars_war` reads the ledger (05).

### 5.5 Accounts

The launchpad's `HookCallback` accounts (`hooks.rs:49-81`) gain indices 9 to 12 (section 4.3).
After them, per equipped `Pool` slot in slot order: `<ITEMS_ID>`, `LAUNCH_ITEMS_SIGNER`, then the
item's registry extras. Each slot's extras count comes from the slot table and the item registry,
so the launchpad splits them without arguments. New errors: `WrongItemProgram`,
`ItemAccountsMissing`, `StaleRegistry`, `CutNotRecorded`, `ItemCutWrongSide`, `ItemBurnWrongSide`,
`ItemCutOutOfBounds`, `PoolCutsTooHigh`, `ForeignAnswer`.

## 6. Companion and protocol fees

### 6.1 The split

`Split` (`programs/bordrless_companion/src/state.rs:9`) gains `war_bps: u16`. `valid()` becomes
`buyback_bps + holders_bps + beneficiary_bps + war_bps == 10_000`, with `war_bps <= WAR_BPS_MAX`
(`WarBpsTooHigh`). `Companion` gains `paid_war_total: u64` (from `reserved`).

### 6.2 Paying the chest

`claim_fees` (`steps.rs:246`) splits what it claimed as today and moves the `war_bps` part
**directly** to the war chest's bridged-SOL holding, `holding(BRIDGED_SOL, ["war-chest", mint] under
<WAR_ID>)`, in the same step (a token transfer signed by the creator PDA; bridged SOL has no hook,
so the height is companion 1, token 2). The bounty is taken before the split, as upstream. If the
chest's holding does not exist yet (before `init_war`), the war part stays in the creator's holding
as `pending_war` and is moved by the next `claim_fees` after it exists (`ChestNotReady` is never an
error, so other shares are never blocked). Event `CompanionWarFunded { companion, mint, amount,
total }`; 05's `record_funding` counts it on the war side.

### 6.3 Launching from a companion

The companion calls `prepare_launch` and `create_launch` in two transactions (section 4.3), both at
height 5 at most. `create` (upstream `create.rs`) gains `war_bps` in its `Split` argument and the
companion's creation event carries it.

### 6.4 Protocol fees and the season prize (R14)

No DEX code changes. The DEX config's `fee_collector` is set to `PDA(["prize-vault"], <WAR_ID>)`:

- `collect_protocol_fees_sol` (permissionless, `pool.rs:675`) unwraps a pool's
  `protocol_fees_quote` and moves the lamports to `fee_collector` (address-checked); the prize vault
  is a system-owned PDA, so it receives lamports like a wallet;
- the admin path `collect_protocol_fees` (`pool.rs:578`) pays into the collector's **holding of the
  quote**, `holding(BRIDGED_SOL, prize-vault)`. That holding must exist (anyone may create it,
  upstream `create_holding` is permissionless) and 05's `split_protocol_fees` must sweep both the
  lamports and that holding.

## 7. Sieges and the kit (R10)

Nothing in 03: the restriction that `siege` refuses a rival whose kit has holder rewards on
(`SiegeTargetHasRewards`) lives in 05.

## 8. Call depth

| Path | Height | Notes |
| --- | --- | --- |
| `swap`: delivery transfer to token-side items | DEX 1, token 2, items 3 | as upstream's kit |
| `swap`: launch hook to Pool items | DEX 1, launch 2, items 3 | new; items write `EquipState`, `RaidLedger` without CPI |
| `swap`: launch hook `emit_cpi!` | DEX 1, launch 2, launch self-CPI 3 | `PoolItemCuts` |
| `swap_route` | as `swap` | hops are a loop |
| `graduate`: reserve top-up to token-side items | launch 1, token 2, items 3 | upstream passes the slice (graduate.rs header) |
| `graduate`: `finalize_curve`, LP mint | launch 1, DEX 2, token 3 | LP mints have no slots |
| `prepare_launch` | up to 4; 5 from a companion | section 4.3 |
| `create_launch` deposit | 4; 5 from a companion | section 4.3 |
| companion `claim_fees` to the chest | companion 1, token 2 | |

Every row is measured in LiteSVM (07) and asserted as a ceiling, as upstream does (hooks-v2
section 6).

## 9. Events (R15, 06 section 9)

| Event | Program | Fields |
| --- | --- | --- |
| `Swapped` (extended) | DEX | upstream fields (`events.rs`) plus `route: RouteContext` |
| `RouteSwapped` (new) | DEX | `trader, route_input_mint, route_output_mint, amount_in, amount_out, pools: Vec<Pubkey>, slot, ts` |
| `ObservationsCreated` (new) | DEX | `pool, observations, len` |
| `PoolItemCuts` (new) | launchpad | `launch, pool, mint, side (0 input, 1 output), discount_bps, cuts: Vec<{ slot, item, amount }>, burns: Vec<{ slot, item, amount }>, pool_cuts_delta, slot, ts`; one per callback in which any item answered. The DEX's `Swapped.deltas_in` / `deltas_out` show the one merged `PoolCuts` delta; this event attributes it to slots |
| `LaunchPrepared` (new) | launchpad | `mint, creator, slots: Vec<SlotSpec>` |
| `LaunchCreated` (extended) | launchpad | upstream fields plus `slots` (the table as created), `war_chest: Pubkey` (`["war-chest", mint]`, whether or not opened yet), `war_bps: u16` (0 unless the creator is a companion's creator PDA; the companion passes its `Companion` account as an optional account so the launchpad can read it) |
| `PoolRegistryRefreshed` (new) | launchpad | `mint, pool, items: Vec<{ slot, item }>` |
| `CompanionWarFunded` (new) | companion | `companion, mint, amount, total` |
| companion creation event (extended) | companion | plus `war_bps` |

Slot-level events (`SlotTableCreated`, `SlotEquipped`) are 01's and 02's.

## 10. Upstream tests

Unchanged in meaning: `swap.rs`, `swap_hooks.rs`, `launch_money.rs`, `launch_rules.rs`,
`protocol_fees_sol.rs`, `companion*.rs`, `vectors.rs` (with the one-hop route in the expected
`PoolHookArgs`). Changed: `launch.rs` and `launch_configs.rs` (two-step launch, slots instead of the
custom hook), `hook_authority.rs` and `studio_template.rs` (their safety assertions move to template
registration and slot tests). New tests (07): observation math against a reference, reads under a
one-slot spike, route forgery through `hook_data` ignored, route rules, item answers on the wrong
side or out of bounds, the discount merge, the single `PoolCuts` delta and `CutNotRecorded`, stale
registry, the raid mark consumed only by the same-slot delivery, both depth ceilings from a
companion, `pending_war` before `init_war`.

## 11. Interfaces with the other parts

From **01**: `create_mint` with `slot_authority` and `SlotInit` (incl. `locked_program`); `SlotBounds`
with `max_cut_bps` (03 applies it per side to Pool slots, cut and burn together); the slot slice
layout; refusal of mint subscriptions on non-`Locked` slots; `set_slot_item` callable during
`prepare_launch`.

From **02**: `equip_launch(slot, item)` accepting only `PDA(["armory-caller", mint], <LAUNCH_ID>)`,
recording the launch item, then `set_slot_item` and `init_equip` (I-02); `rule_data` validation; the
armory calls `refresh_pool_registry` with every equip and `Performance` revert of a Pool slot. This
replaces 02's I-03-2 ("create_launch writes each slot's launch item through 01"): only the armory's
`SlotAuthority` may write an item into a slot, so the launchpad goes through the armory.

From **04**: pool callbacks `before_swap` / `after_swap` taking `(PoolHookArgs, ItemPoolContext)`
and answering `ItemPoolAnswer`; `max_discount_bps` per template; each item adds its cut to
`EquipState.collected[1]` in the call; Raid's pool half writes the R4 mark in `after_swap` (04's
current `Mark` in `WarState` with `pool_swap_count` is superseded by R3 and R4); the item registry
seed (00 section 4.3); `init_equip` creates the `PoolCuts` accounting it needs, while 03 creates the
`PoolCuts` quote holding.

From **05**: `init_war(mint)` as the third launch transaction; the chest's bridged-SOL holding
address; `record_funding` counting companion transfers; `split_protocol_fees` sweeping both the
prize vault's lamports and its quote holding (6.4); the siege restriction (R10).

## Kit and companion implementation notes (2026-10-09, branch kitcomp)

Built in `programs/bordrless_kit` (new `src/setup.rs`, `init`), `programs/bordrless_companion`
(state, constants, error, events, `create`, `claim_fees`, client) and the tests. Where the build
differs from the text above:

1. **R9, the kit's mint check is one function**, `bordrless_kit::setup::mint_setup_ok(mint, modules,
   supply)`, used by `init`. It accepts the upstream single-hook mint unchanged, or a slot mint with
   no single hook and no hook authority whose `Locked` slot is the kit with exactly the kit's flags,
   range offset 0 and two extras. The range length is `kit_data_len(flags)`: 32 when the kit keeps
   hook data (holder rewards or the early-buyer lock), else 0, because the token program refuses a
   Locked range for a program that never writes (`slot_table.rs`, the `writes` rule).
2. **R10 helper**, `bordrless_kit::setup::holder_rewards_on(mint_key, mint, kit_config)`: true when
   the kit runs on the mint (single hook or Locked slot) and the given `KitConfig` belongs to that
   mint and has holder rewards on. War reads the rival's `KitConfig` at `KitConfig::address(mint)`
   (owned by the kit) and passes `None` when it has none. Kit transfer rules are unchanged.
3. **Companion `war_bps`.** `Split` gains `war_bps` as a fourth part; the four still add up to
   10,000. `Companion` gains `war_total` and its `reserved` shrinks from 64 to 54 bytes, so
   `Companion::LEN` is unchanged. `claim_fees` pays `floor(rest * war_bps / 10,000)` (rest = claimed
   less the bounty) at once, as bridged SOL, from the creator's holding into the holding of
   `PDA(["war-chest", mint], WAR_ID)`, creating that holding when missing (the cranker pays its
   rent), and emits `CompanionWarFunded { companion, mint, war_chest, amount, war_total }`. Rounding
   still goes to buybacks when there are any, else to the beneficiary. The client always passes the
   war accounts (two more keys than upstream).
4. **`WAR_BPS_MAX` is an owner value still to set** (00 section 6). The build uses the structural
   bound, 10,000, and `create` refuses a larger `war_bps` with `WarShareTooHigh`. `WAR_ID` is a
   constant in the companion (`5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2`), not a crate
   dependency, as upstream does for the kit's `COMPANION_ID`.
5. **Presets.** The companion has no on-chain templates; every test split sets `war_bps: 0` except
   the two new war tests.

Measured (LiteSVM, `programs/tests/tests/companion.rs`, legacy transactions without lookup tables):

| Path | Bytes | CU | Height |
| --- | --- | --- | --- |
| `claim_fees` with a war share (war holding created) | 873 | 191,390 | 4 |
| companion launch (upstream path, unchanged) | 1,309 | 311,801 | 5 |
| companion launch, v0 with the protocol table extended | 1,142 | 401,782 | 5 |

Tests added: kit unit tests `setup::*` (5), `kit.rs`
`every_module_set_runs_exactly_its_rules_in_a_locked_slot` and
`in_a_locked_slot_the_kit_never_writes_bytes_32_to_64`, `kit_money.rs`
`a_seeded_walk_with_the_kit_in_a_locked_slot_keeps_the_vault_solvent`, `companion.rs`
`a_claim_pays_the_war_chest_its_share` and `no_war_share_no_war_payment_and_the_split_still_adds_up`.
Suite: 203 passed, 0 failed, 2 ignored (the Studio fixtures).

## M3a implementation notes (2026-10-09)

Built on branch `m3a`: `crates/bordrless-core/src/observations.rs`, `crates/bordrless-hook`
(`RouteContext`, `PoolHookArgs.route`), `programs/bordrless_swap` (`obs.rs`, `run_hop`,
`swap_route`), tests `programs/tests/tests/observations.rs`, `route.rs` and `budgets.rs`
(`route_budgets`). Where the build differs from sections 3.1 to 3.3, the build is what exists:

1. **The ring lives in the pool account, not at `["obs", pool]`.** A separate account added one
   key to every price-moving transaction, and upstream's `create_launch` with a custom hook and the
   site's longest metadata measured **1,233 bytes**, over Solana's 1,232 (`half_life.rs`,
   `prepare_launch_then_light_fit_mainnet_limits`); it also shifted account positions that
   upstream's launch, kit and hook-signer tests index. The ring is now the tail of the pool
   account, from offset `Pool::LEN` (`bordrless_swap::obs::POOL_ACCOUNT_LEN`): `create_pool`
   allocates the pool with it and fills it after the first deposit, Anchor writes back only the
   `Pool` fields, and every instruction keeps upstream's account list. A reader passes the pool
   account alone, which it needs for `quote_volume` and `swap_count` anyway
   (`bordrless_swap::obs::window_read`, or `bordrless_core::observations::window_read` on
   `obs::ring_of(data)`). Consequences: no `["obs", pool]` seed (00 4.3 lists it; to drop at
   integration), no `WrongObservations` binding case beyond a malformed ring, and no pre-funded
   address to tolerate. There is **no `ObservationsCreated` event**: its self-CPI made upstream's
   `create_launch` with every module 46 trace entries against its asserted ceiling of 45
   (`runtime.rs`); the ring's length and spacing are in its header, and every pool has one.
2. **A running cumulative in the header, entries spaced by `OBS_SPACING_SECS`.** Section 3.1
   writes an entry every second a price moves; an active pool would then cover only
   `OBS_RING_LEN` seconds. The header keeps the running `cumulative`; an entry is written when the
   newest is at least `OBS_SPACING_SECS` old, so a ring covers at least
   `OBS_RING_LEN * OBS_SPACING_SECS` seconds. Section 3.1's rule is the case spacing = 1. Both are
   stored in the ring's header, so readers never need the DEX's constants.
3. **Where it is written.** At the start (`accumulate`, before any reserve moves, with the pool's
   counters before the instruction) and the end (`set_price`) of `swap`, each hop of
   `swap_route`, `add_liquidity`, `remove_liquidity` and `finalize_curve`. `collect_protocol_fees`
   moves no reserve and does not write it.
4. **Price.** Q64.64 quote per base from effective reserves; totals above 64 bits are scaled down
   together (`price_q64`). Cumulative sums wrap; a read is exact while the true difference over the
   window stays below 2^128, i.e. while the integer price times the window stays below 2^64.
5. **`swap_route` hop groups** start with 9 fixed accounts (`HOP_FIXED_ACCOUNTS`: pool, base mint,
   quote mint, base vault, quote vault, trader base holding, trader quote holding, hook program or
   this program's id, hook signer or this program's id), then the input slice, the output slice and
   the pool-hook extras; `HopArgs.accounts` counts all of them. `SwapRouteArgs` also carries
   `hook_data`, forwarded to every hop's pool hook (never read for the route). A slot mint's slices
   are its slot extras, as for a plain `swap` (01 M1 notes).
6. **One hop function.** `swap` and every hop of `swap_route` run `run_hop` (upstream's steps 1 to 7
   unchanged), which also writes the pool back before returning, so a later hop or the caller reads
   it as it stands. A plain swap fills the one-hop route itself; a route fills each hop's.
7. **Build values (TEST, to set):** `OBS_RING_LEN` 32, `OBS_SPACING_SECS` 30, `MIN_TWAP_SECS` 300,
   `MAX_ROUTE_HOPS` 3. Measurements below.

### Measured (M3a, `budgets.rs`, `observations.rs`, upstream `runtime.rs`, `half_life.rs`, `launch_configs.rs`)

| Path | Keys | v0 bytes | With table | Trace | Height | CU |
| --- | --- | --- | --- | --- | --- | --- |
| buy, plain pool, 0 slots (ring written) | 14 | 609 | 302 | 7 | 3 | 54,933 (M1: 51,032) |
| sell, plain pool, 0 slots | 14 | 609 | 302 | 7 | 3 | 54,937 (M1: 51,036) |
| buy, 3 cutting slots | 22 | 877 | 322 | 10 | 3 | 110,655 (M1: 106,733) |
| `swap_route`, 2 hops, plain pools | 19 | 787 | 325 | 13 | 3 | 105,903 |
| `swap_route`, 3 hops, plain pools | 24 | 960 | 343 | 18 | 3 | 154,746 |
| `swap_route`, 2 hops delivering a 3-cutting-slot mint | 27 | 1,055 | 345 | 16 | 3 | 161,616 |
| upstream `create_launch`, Half-Life, longest metadata | 32 | 1,200 | | 38 | 4 | 310,420 |
| upstream `create_launch`, every module (protocol table) | 32 | 1,043 | | 45 | 4 | 344,723 |
| upstream buy and graduate, every rule (protocol table) | 35 | 970 | | 43 | 4 | 347,569 |

Writing the ring costs about 3,900 compute units per price-moving instruction and no keys or
bytes. The pool account grows from 411 to 2,043 bytes with a 32-entry ring (1,632 bytes, 48 per
entry): rent 15,110,160 lamports against upstream's 3,751,440. A one-second spike of a buy of a
fifth of the pool's quote side moved a 301-second read by 44,942,040,237,162,311 (Q64.64) against
the spike's 8,014,837,288,195,843,276, about 1/178 of it (`observations.rs`).

**Proposals.** `MAX_ROUTE_HOPS` = 3 on plain pools (960 bytes without a table, 343 with); a
launch-pool hop with the launchpad's accounts and Pool items is larger, so M3b measures two
launch-pool hops (the Raid case) before the value is fixed, and keeps 2 if three do not fit.
`OBS_RING_LEN` = 32 with `OBS_SPACING_SECS` = 30 covers at least 16 minutes, enough for
`MIN_TWAP_SECS` = 300 with room for one wrap of the window; a longer `MIN_TWAP_SECS` (the owner's
decision, section 3.1) needs `OBS_RING_LEN * OBS_SPACING_SECS` above it, paid in pool rent at
48 bytes per entry.
