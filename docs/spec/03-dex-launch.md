# Hookwars spec 03: DEX, launchpad and companion changes

Status: specification, 2026-10-08. Nothing here is built. Follows `00-overview.md`; where this file
disagrees with it, this file is wrong, except for the conflicts listed in section 9, which the spec
lead must resolve in 00.

Upstream references are to Bordrless at `43688f3`. "hooks-v2" means `docs/hooks-v2.md`.

## 1. Scope

| Program | Change |
| --- | --- |
| `bordrless_swap` | an **observation ring** per pool; a **multi-hop** `swap_route`; a DEX-filled **route context** in every pool callback |
| `bordrless_launch` | `create_launch` creates the mint with a **slot table**; `LaunchConfig` carries slots instead of `custom_hook`; the launch pool hook **forwards to `Pool`-kind items**; war marks are forwarded to `hookwars_war` |
| `bordrless_companion` | `war_bps` in the split, paid to the token's war chest |

Everything else in these programs is upstream behaviour and keeps upstream's tests.

## 2. Two rules this part adds to the contract

1. **Items are leaves.** An item program (`hookwars_items`) never makes a CPI of its own on any
   callback. It reads accounts and answers. Everything an item wants done (a cut, a burn, a war
   mark) is applied by the program that called it. This is what keeps every path inside Solana's
   invoke height of 5 (section 7) and what stops an item from writing another token's war state.
2. **The route is the DEX's word, never the trader's.** Pool hooks read the route from a field the
   DEX fills (section 3.3), never from `hook_data`, which the trader controls.

## 3. DEX (`bordrless_swap`)

### 3.1 Observations

A new account per pool, `Observations` at `["obs", pool]` under the DEX, created by `create_pool`
in the same instruction as the pool (payer: the pool's creator), so every pool of the Hookwars
deployment has one from its first swap. There is no migration path because there are no pools from
before: Hookwars is a new deployment with new program ids (00 section 3).

```rust
pub struct ObservationHeader {
    pub version: u8,
    pub bump: u8,
    pub pool: Pubkey,
    /// Price after the last write, quote per base as Q64.64, virtual reserves included
    /// (the same price `Swapped` implies: (quote_reserve + virtual_quote) / (base_reserve + virtual_base)).
    pub last_price_q64: u128,
    /// Time of the last accumulation.
    pub last_ts: i64,
    /// Next slot of the ring to write.
    pub index: u16,
    /// How many entries are filled (grows to OBS_RING_LEN, then the ring wraps).
    pub filled: u16,
}

pub struct Observation {
    pub ts: i64,
    /// Sum over time of last_price_q64 * seconds, wrapping (readers subtract, as Uniswap v2 does).
    pub price_cumulative: u128,
    /// The pool's `quote_volume` (upstream `state.rs` Pool, u128) at `ts`.
    pub quote_volume: u128,
}
```

Size: header plus `OBS_RING_LEN` entries of 40 bytes each; `OBS_RING_LEN` is to measure (rent
against the longest window a relation item may read).

**When it is written.** At the **start** of every instruction that can move the price
(`swap`, each hop of `swap_route`, `add_liquidity`, `remove_liquidity`, `finalize_curve`), before
any reserve changes:

1. `dt = now - last_ts`. If `dt > 0`: `cumulative += last_price_q64 * dt` (wrapping), and if the
   newest entry's `ts` is older than `now`, write a new entry `{ now, cumulative, pool.quote_volume }`
   at `index`, advance `index` modulo `OBS_RING_LEN`, raise `filled` up to `OBS_RING_LEN`.
2. At the **end** of the same instruction, `last_price_q64` is set to the new price and
   `last_ts = now`.

So the price that accrues for any second is the price at which the previous second ended. A swap
can never change the price it is itself accumulated at. At most one entry is written per second.

**Reading a TWAP** (a pure function in `bordrless_core`, used by items, `hookwars_war` and the
SDK, pinned to each other by test vectors as upstream pins its fee math):

```
twap(obs, now, window):
  require window >= MIN_TWAP_SECS                     else TwapWindowTooShort
  cum_now = cumulative of the newest entry + last_price_q64 * (now - last_ts)
  e = newest entry with e.ts <= now - window          else ObservationTooOld (history too short)
  price = (cum_now - e.price_cumulative) / (now - e.ts)
  volume = pool.quote_volume - e.quote_volume
```

A reader that cannot get a TWAP (a new pool, too short a history) must treat the read as "no
signal": every template in 04 is defined so that "no signal" means "no effect". A missing history
never blocks a trade.

**Manipulation.** A price held for one slot weighs one slot's time in a window of at least
`MIN_TWAP_SECS`. Moving a TWAP by `p` over a window `W` means holding the pool at `p * W / t`
above its price for `t` seconds, paying the LP fee (on a launch pool, Bordrless's) and the curve's
slippage both ways, and being arbitraged the whole time. A validator that leads several
consecutive slots can hold a price without arbitrage for those slots only, so `MIN_TWAP_SECS` must
be many leader rotations long. The value is an owner decision with this cost in front of it.

`Observations` is passed **writable** to every price-moving instruction (one more account on each
swap; measured in 07). Relation items take it **read-only** for other pools.

### 3.2 `PoolHookArgs.route`

Upstream `PoolHookArgs` (`crates/bordrless-hook/src/lib.rs:206`) gains one field, filled only by the
DEX:

```rust
pub struct RouteContext {
    /// The mint the trader started with (the first hop's input).
    pub route_input_mint: Pubkey,
    /// The mint the trader ends with (the last hop's output).
    pub route_output_mint: Pubkey,
    /// The pool of the first hop (so an item can check it is a launch pool of route_input_mint).
    pub first_pool: Pubkey,
    /// What the first hop took from the trader, in route_input_mint units.
    pub route_amount_in: u64,
    /// This hop, from 0.
    pub hop_index: u8,
    /// How many hops.
    pub hop_count: u8,
}
```

- A plain `swap` fills it as a **one-hop route**: `route_input_mint = in_mint`,
  `route_output_mint = out_mint`, `first_pool = pool`, `hop_index 0`, `hop_count 1`. It is never
  absent, so no item has a case where a route is "unknown".
- `hook_data` stays the trader's opaque bytes (`MAX_HOOK_DATA`, upstream `lib.rs:41`), forwarded as
  today. The launchpad and every Hookwars template ignore it.
- Why not in `hook_data` (as the design note suggested): any trader can put any bytes there in a
  plain `swap`, so a route read from it could be forged ("I came from X" with no X sold). A field
  the DEX fills cannot.

Liquidity callbacks and initialize carry a one-hop route of their own pool with `hop_count 0`
(meaning "not a swap").

`PoolHookArgs` is a shared type of `crates/bordrless-hook`; the change is not wire-compatible with
upstream, which is fine for a new deployment. Its serialized size grows by 106 bytes; the CPI
instruction data stays far under limits.

### 3.3 `swap_route`

```rust
pub struct SwapRouteArgs {
    /// The exact input of the first hop.
    pub amount_in: u64,
    /// The least the trader's final holding must gain.
    pub min_amount_out: u64,
    /// Per hop, in order.
    pub hops: Vec<HopArgs>,          // 1 ..= MAX_ROUTE_HOPS, else RouteTooLong / EmptyRoute
}

pub struct HopArgs {
    pub direction: u8,               // as SwapArgs
    pub accounts: u8,                // how many remaining accounts this hop takes
    pub in_hook_accounts: u8,
    pub out_hook_accounts: u8,
}
```

Accounts: `trader` (signer), `config`, `token_program`, `token_event_authority`, then the
remaining accounts as consecutive hop groups. A hop group is exactly what `swap` takes for one pool
(pool, `Observations`, base mint, quote mint, base vault, quote vault, trader input holding, trader
output holding, hook program or this program's id, hook signer or this program's id), followed by
that hop's token-hook slices and pool-hook extras, `accounts` in all.

Rules:

1. Hop `i`'s output mint must equal hop `i+1`'s input mint, and hop `i`'s trader output holding must
   be hop `i+1`'s trader input holding (`RouteBroken`).
2. No pool appears twice (`RoutePoolRepeated`): a route cannot be used to swap through one pool
   back and forth inside one instruction.
3. Hop `i+1`'s `amount_in` is what hop `i` **delivered**, measured on the holding (upstream's rule
   that amounts are measured, never assumed, `swap.rs:27`). A token hook's cut on the delivery is
   therefore never counted twice.
4. Each hop runs the whole of upstream `process_swap` (`swap.rs:107`), steps 1 to 7, with the route
   context of section 3.2. The hops are a loop **inside one instruction**, not CPIs, so a route
   is no deeper than a swap (section 7).
5. Only the last hop checks `min_amount_out` (`Slippage`); intermediate hops use 0, since the
   trader's bound is on what they end with.
6. Each hop emits its own `Swapped` (upstream `events.rs`, unchanged fields) and the instruction
   emits one `RouteSwapped { trader, route_input_mint, route_output_mint, amount_in,
   amount_out, hops: Vec<Pubkey /* pools */>, slot, ts }`.

`MAX_ROUTE_HOPS` is to measure; the account count per hop (a launch pool hop with a kit slice and
one Pool item is about 20 accounts) makes two hops the expected ceiling with the protocol lookup
table. The raid template (04) needs exactly two: rival to bridged SOL, bridged SOL to us.

New errors: `EmptyRoute`, `RouteTooLong`, `RouteBroken`, `RoutePoolRepeated`, `TwapWindowTooShort`,
`ObservationTooOld`, `WrongObservations`.

## 4. Launchpad: slot table at launch (`bordrless_launch`)

### 4.1 What replaces the single hook

Upstream decides the mint's one hook in `plan()` (`launch.rs:500-540`): the kit when a kit module
is on, else the `LaunchConfig`'s `custom_hook` (hooks-v2 5.8), else none. Hookwars replaces that
with a **slot table** written into the mint at creation (layout owned by 01).

```rust
pub struct SlotSpec {
    pub kind: u8,                 // 00 section 4.1
    pub bounds: SlotBounds,       // 01 defines the struct; it is fixed for ever
    pub equip_rule: u8,           // 00 section 4.2
    pub notice_secs: u32,         // MIN_NOTICE_SECS ..= MAX_NOTICE_SECS unless equip_rule is Locked
    pub data_len: u8,             // bytes of hook data this slot owns
    pub initial_item: Option<Pubkey>, // an armory Item, or None for an empty slot
    pub rule_params: Vec<u8>,     // the performance condition, decoded by the armory (02)
}
```

`CreateLaunchArgs` (`launch.rs:29`) gains `slots: Vec<SlotSpec>`. `LaunchConfig`
(`state.rs:169`) replaces `custom_hook` and `custom_hook_flags` (`state.rs:181-183`) with
`slots: Vec<SlotSpec>`; with a config, the inline slots must equal the config's (`ConfigMismatch`,
upstream's rule for rules and creator fee, hooks-v2 5.7). Listed configs and the author share are
unchanged.

### 4.2 Checks at launch (in `plan`, before anything is created)

| Check | Error |
| --- | --- |
| `slots.len() <= MAX_SLOTS` | `TooManySlots` |
| kinds known; at most one `Locked` slot, and it is slot 0 | `InvalidSlotKind` |
| equip rule known; `Locked` kind has the `Locked` rule | `InvalidEquipRule` |
| notice inside `MIN_NOTICE_SECS ..= MAX_NOTICE_SECS` for `Vote` and `Performance` | `NoticeOutOfBounds` |
| sum of `data_len` `<= 64`; ranges are laid out in slot order from byte 0 | `HookDataOverflow` |
| at most `MAX_CUTTING_SLOTS` slots whose bounds allow cuts | `TooManyCuttingSlots` |
| a `Pool` slot's bounds: per side, `max_cut_bps <= MAX_POOL_ITEM_CUT_BPS` (section 9, C2) | `SlotBoundsTooHigh` |
| each `initial_item` is an armory `Item` whose template fits the slot's kind and whose manifest is inside the slot's bounds (read through the armory client crate; 02 owns the check function) | `ItemDoesNotFit` |
| `Performance` slots have a non-empty `rule_params` the armory accepts | `InvalidRule` |

**The kit becomes slot 0, `Locked`.** When the rules install a kit module, the launchpad writes slot
0 as `{ kind: Locked, equip_rule: Locked, program: KIT_ID, data_len: 32 }`. The kit uses bytes
0..32 of the hook data today (`programs/bordrless_kit/src/state.rs:280`), so its range is 0..32 and
needs no change in the kit. The kit's own flags, registry and `kit-caller` init are unchanged
(hooks-v2 4.1). A kit slot is never an armory item and earns no royalty.

**Slot authority.** The mint is created with its slot authority set to the armory's
`SlotAuthority` PDA, `["slots", mint]` under `<ARMORY_ID>` (00 section 4.3, D-5). The upstream
`hook_authority` stays `None`: nobody, including the creator, can change the slot table's shape.

### 4.3 Order inside `create_launch`

Upstream order (`launch.rs:1055-1079`): plan, launch fee, mint, two launch holdings, supply, kit,
curve pool, pool registry, launch record, event. Hookwars keeps it and changes three steps:

- **create_mint** passes the slot table to the token program (01).
- **mint_supply**: no template subscribes to mints (04 rule; 01 refuses a `BEFORE_MINT` or
  `AFTER_MINT` flag on any non-`Locked` slot), so the supply `mint_to` calls no item.
- **write_pool_registry** (`launch.rs:926`) writes the launch pool's registry with upstream's four
  extras (`registry_list`, `launch.rs:187`: launch, quote holding, holder vault, kit config) plus,
  for Hookwars, at fixed indices 9 to 11:

| Index | Account | Why |
| --- | --- | --- |
| 9 | `WarChest`'s bridged-SOL holding, `["holding", BRIDGED_SOL, WarChest]` | the one destination of every Pool item's cut (section 5.3) |
| 10 | `WarState` `["war", mint]` under `<WAR_ID>` (writable) | where war marks land (section 5.4) |
| 11 | the launch pool's own `Observations` (read-only) | items read their own pool's TWAP |

then, per equipped `Pool`-kind slot, that item's program and its registry extras (04 defines each
template's list). Because items change, the pool registry must change with them: the launchpad
gains `refresh_pool_registry(mint)`, permissionless, which rewrites the pool registry from the
mint's current slot table and the items' registries. The armory calls it in the same transaction as
every equip (02); a swap with a stale registry fails in the DEX's account checks rather than running
a removed item.

`create_launch` creates the `WarChest` and `WarState` through a CPI to `hookwars_war::open_war`
(05), signed by the launch's PDA, after the pool exists. Depth: create_launch (1), war (2).

Upstream's custom-hook path (`check_custom_hook_accounts`, `launch.rs:415`) and its errors
(`CustomHookWithKitRules`, `HookRegistryMissing`, `HookExtrasMismatch`, ...) are removed; the
token-hook slice the launchpad passes on `mint_to`, the pool deposit and `graduate` becomes the
mint's slot slice (01 defines its layout: per slot, program, the token program's signer for it,
extras).

## 5. Launchpad: forwarding to `Pool` items

### 5.1 Order inside `before_swap` and `after_swap`

Upstream callbacks: `launch/src/instructions/hooks.rs:128` (`before_swap`) and `:162`
(`after_swap`). Hookwars keeps them and appends the items:

1. **The launchpad's own rules, unchanged**: sniper LP fee (and the creator's one-time first buy),
   creator fee, holder fee, burn, computed by `bordrless_core::launch_before_swap` /
   `launch_after_swap` exactly as upstream. Call this answer `L`.
2. **Each equipped `Pool` slot, in slot order**: the launchpad invokes the item's `pool_before_swap`
   or `pool_after_swap` (names owned by 04), signing as `["hook-authority", <ITEMS_ID>]` under
   `<LAUNCH_ID>`, with:
   - the DEX's `PoolHookArgs` unchanged (including the route),
   - `ItemPoolContext { slot: u8, item: Pubkey, launch_answer: L (fee, cuts, burn), quote_after_launch: u64 }`
     (`quote_after_launch` is the quote side this callback acts on after `L`'s cuts),
   - accounts: the item signer, the five pool prefix accounts, then the item's own extras from the
     pool registry.
3. The launchpad reads each item's answer, checks it (section 5.2), merges it (section 5.3),
   collects war marks (section 5.4), and returns **one** `HookReturn` to the DEX.

### 5.2 What an item may answer to the launchpad

```rust
pub struct ItemPoolAnswer {
    /// Basis points of the launchpad's creator and holder fees on this side to waive (raids).
    pub discount_bps: u16,       // <= 10_000
    /// The item's own cut, in the quote, to the war chest.
    pub cut: u64,
    /// The item's royalty, as a part of `cut` (02 rule: royalty comes out of the cut).
    pub royalty: u64,            // <= cut * item.royalty_bps / 10_000, checked
    /// A burn of base, only on the sides where the slot allows burns.
    pub burn: u64,
    /// A war mark.
    pub mark: Option<WarMark>,   // 05 owns WarMark
}
```

Checks, each against the slot's fixed bounds (01) and the side:

| Check | Error |
| --- | --- |
| `cut <= quote_after_launch * slot.max_cut_bps / 10_000` | `ItemCutOutOfBounds` |
| `burn == 0` unless the slot allows burns on this side; burn is base, so only on a sell's input (before) or a buy's output (after), as upstream's burn | `ItemBurnNotAllowed` |
| `royalty <= cut * royalty_bps / 10_000` with `royalty_bps` read from the `Item` | `RoyaltyTooHigh` |
| `mark` only if the slot's kind is `Pool` and the template's manifest declares marks | `MarkNotAllowed` |
| the answer is the item program's own return data | `ForeignAnswer` |

### 5.3 Merging

- **LP fee.** Items never set the LP fee. On a launch pool the LP fee is Bordrless's revenue
  (hooks-v2 3.1, "The LP fee of a launch pool is Bordrless's"), and the sniper schedule protects
  every launch equally. The merged answer carries `L.lp_fee_bps` unchanged. A Shield that "raises
  the fee" does so as its own `cut`, which goes to the token's war chest, not to the protocol.
- **Discounts.** `discount = min(10_000, sum of discount_bps)`. The launchpad's creator fee and
  holder fee on this side are multiplied by `(10_000 - discount) / 10_000`, rounded down. The burn
  and the LP fee are never discounted. A raid is therefore paid by the token being raided **into**
  (its creator and holders give up their fee on raid buys), which is the community's choice when it
  equipped the Raid item.
- **Cuts.** Upstream's answer already uses up to two deltas per side (creator fee to index 6,
  holder fee to index 7, `hooks.rs:102-124`) of `MAX_DELTAS = 3` (`bordrless-hook/src/lib.rs:43`).
  All items' cuts on a side are therefore **summed into one delta** to registry index 9, the war
  chest's bridged-SOL holding. The sum must keep `L`'s cuts plus the items' cuts plus the burn below
  the side's amount (the DEX's `DeltaTooLarge`, `swap.rs:182`).
- **Royalties** cannot be separate deltas (no delta is left). Each item's `royalty` is recorded as
  a **royalty mark** for `hookwars_war` (section 5.4): the war chest owes it, and
  `hookwars_war::pay_royalties` (05) moves it from the chest to each item's royalty holding
  (`["holding", BRIDGED_SOL, RoyaltyOwner]`, 00 section 4.3), permissionless. This replaces 00's
  "a royalty takes one delta" for pool items (section 9, C1).
- **Burns** add to `L.burn`.
- **Protocol share.** Unchanged. The DEX measures `cuts_in` and `cuts_out` (`swap.rs:261`,
  `:415`) and takes its share (`protocol_share_bps`, 25% today) of everything a hook cut, so item
  cuts pay Bordrless's share exactly like the creator fee. A launch whose items cut nothing pays
  nothing extra.
- **Ceiling.** Upstream caps creator + holder + burn per side by the launch config's
  `max_rules_fee_bps` (`launch.rs:220-245`). Item cuts are capped separately by each slot's
  bounds, which `create_launch` caps by `MAX_POOL_ITEM_CUT_BPS` per slot and side, and the sum over
  `Pool` slots by the same parameter times `MAX_CUTTING_SLOTS`. Both are shown on the token page.

### 5.4 War marks

Items cannot write `WarState` (rule 1 of section 2; `WarState` is owned by `<WAR_ID>`). An item
returns marks; the launchpad delivers them:

- After merging, if any item answered a mark or a royalty, the launchpad CPIs
  `hookwars_war::record_marks(mint, marks: Vec<WarMark>, royalties: Vec<(Pubkey /* item */, u64)>)`,
  signing with `["war-caller", mint]` under `<LAUNCH_ID>` (a PDA with no other power, the pattern of
  upstream's `kit-caller`, hooks-v2 4.1). `hookwars_war` accepts marks only from that signer for that
  mint.
- Depth: DEX `swap` (1), launchpad `before_swap`/`after_swap` (2), `hookwars_war::record_marks`
  (3). The item calls (also 3) have returned before.
- Marks recorded in `before_swap` describe the swap now running. Every mark carries the pool's
  `swap_count` **after** this swap's increment (upstream increments it at `swap.rs:331`, before
  `after_swap`; in `before_swap` the launchpad uses `args.swap_count + 1`), the route context and
  the recipient. The token-side half of the Raid template (04) reads `WarState` on the delivery
  transfer, which happens after `after_swap` (`swap.rs:400`), and stamps the holding only when the
  mark's `swap_count` equals the pool's current one and the mark's recipient is the transfer's
  destination owner. A stale or foreign mark stamps nothing.

### 5.5 Accounts and registry

The launchpad's `HookCallback` accounts (`hooks.rs:49-81`) gain indices 9 to 11 (section 4.3) and
the item programs with their extras as remaining accounts, located by the slot table. New errors:
`WrongItemProgram`, `ItemAccountsMissing`, `StaleRegistry`.

## 6. Companion (`bordrless_companion`)

### 6.1 The split

`Split` (`programs/bordrless_companion/src/state.rs:9`) gains `war_bps: u16`. Its `valid()` becomes
`buyback_bps + holders_bps + beneficiary_bps + war_bps == 10_000`, with `war_bps <= WAR_BPS_MAX`
(`WarBpsTooHigh`). `Companion` gains `paid_war_total: u64` (from `reserved`).

### 6.2 Paying the chest

`claim_fees` (`steps.rs:246`) splits what it claimed as today and sends the `war_bps` part
**directly** to the war chest's bridged-SOL holding in the same step (a token-program transfer
signed by the creator PDA; bridged SOL has no hook, so depth is companion (1), token (2)). There is
no `pending_war`: the chest is the account that holds it. Event `WarFunded { companion, mint,
amount, total }`. The bounty is taken before the split, as upstream.

### 6.3 Call depth when a companion launches

`docs/companions.md:75`: `create_launch` reaches invoke height 4 and, from the companion, 5, which is
Solana's limit. With slots:

- the supply `mint_to` calls no item (section 4.3);
- the pool deposit (companion (1), launch (2), DEX `create_pool` (3), token transfer (4), slot items
  (5)) reaches 5, the same height as upstream's kit callback on that transfer. It works only
  because items are leaves (section 2, rule 1). Any item that made a CPI there would break every
  companion launch;
- `open_war` from `create_launch` is at height 3 under a companion.

Mitigation beyond the leaf rule, to decide in M1 from measurement: a companion launch may also be
created with `initial_item: None` in every non-`Locked` slot and equip after the launch lands (the
armory's equip in the next transaction). The cost is a window with no items; it is not needed if the
measured height is 5 as computed.

## 7. Call depth

| Path | Height | Notes |
| --- | --- | --- |
| `swap`: token transfer to a token item | DEX 1, token 2, item 3 | as upstream's kit |
| `swap`: launch hook to a Pool item | DEX 1, launch 2, item 3 | new |
| `swap`: launch hook to `record_marks` | DEX 1, launch 2, war 3 | new |
| `swap_route` | as `swap` (hops are a loop, not CPIs) | new |
| `graduate`: reserve top-up transfer to token items | launch 1, token 2, item 3 | upstream passes the hook slice (graduate.rs header) |
| `graduate`: `finalize_curve`, LP mint | launch 1, DEX 2, token 3 | LP mints have no slots |
| `create_launch` pool deposit to items | launch 1, DEX 2, token 3, item 4 | |
| companion `create_launch` deposit to items | 5 | the limit; section 6.3 |
| companion `claim_fees` to the chest | companion 1, token 2 | |

Every row is to be measured in LiteSVM (07) and asserted as a ceiling, as upstream does for its
paths (hooks-v2 section 6).

## 8. Events, errors, compatibility

New events: `RouteSwapped` (DEX), `ObservationsCreated { pool, len }` (DEX), `PoolItemsAnswered {
launch, swap_count, discount_bps, items_cut, items_burn, marks }` (launchpad),
`PoolRegistryRefreshed { mint, items }` (launchpad), `WarFunded` (companion). `Swapped`,
`LaunchCreated` and every other upstream event keep their fields; `LaunchCreated` gains `slots`.

Upstream tests that must still pass unchanged in meaning: `swap.rs`, `swap_hooks.rs`,
`launch.rs`, `launch_money.rs`, `launch_rules.rs`, `protocol_fees_sol.rs`, `companion*.rs`,
`vectors.rs` (with the one-hop route added to the expected `PoolHookArgs`). Tests that change:
`launch_configs.rs`, `hook_authority.rs`, `studio_template.rs` (the custom-hook path is replaced by
slots; their safety assertions move to slot and item tests). New tests listed in 07: observation
math against a reference, TWAP under a one-slot spike, route forgery through `hook_data` refused,
route rules, item answers out of bounds, discount merge, the single merged delta, royalty marks,
stale registry, stale marks, depth ceilings.

## 9. Interfaces and conflicts for the other parts

Needs from **01** (token slots): the slot table type and `SlotBounds` (max cut per side, may
refuse, may burn, marks allowed); `create_mint` taking the table and the slot authority; the slot
slice layout the launchpad passes on transfers; refusal of mint callbacks on non-`Locked` slots.

Needs from **02** (armory): `SlotAuthority` = `["slots", mint]` under `<ARMORY_ID>`; a client
function `check_fits(item, slot_spec)`; `rule_params` validation; the armory calls
`refresh_pool_registry` in every equip transaction.

Needs from **04** (templates): callback names `pool_before_swap`, `pool_after_swap` taking
`(PoolHookArgs, ItemPoolContext)` and returning `ItemPoolAnswer`; each template's pool-registry
extras; the Raid token half reading `WarState` marks as section 5.4 describes; no template sets
mint flags or makes a CPI.

Needs from **05** (war): `open_war(mint)` callable by the launchpad's PDA; `record_marks` accepting
only `["war-caller", mint]` under `<LAUNCH_ID>`; `WarMark` (must carry swap_count, recipient,
route_input_mint, first_pool, quote amount, kind); `pay_royalties`; the war chest's bridged-SOL
holding address.

Conflicts for the spec lead (00):

- **C1, royalties as deltas.** 00 section 6 says "a royalty takes one delta per cutting item". On a
  launch pool only one delta per side is free (upstream uses two of three), so pool-item royalties
  are recorded as royalty marks and paid from the war chest (section 5.3). Either accept this for
  pool items, or raise `MAX_DELTAS` in the fork (each delta is one more token transfer CPI and
  trace entries; measure first).
- **C2, a missing parameter.** `MAX_POOL_ITEM_CUT_BPS` (per slot and side ceiling on a Pool item's
  cut) is not in 00's table. Add it (owner decision).
- **C3, route location.** The brief said route context in `hook_data`; this part puts it in a new
  `PoolHookArgs.route` field because `hook_data` can be forged by any trader (section 3.2). The
  hook crate type change must be agreed with 01.
- **C4, items never set the LP fee** on launch pools (section 5.3). If the owner wants Raid to lower
  the LP fee, the protocol gives up its own revenue on raid buys; that is an owner decision.
- **C5, `WarState` writes** go through `record_marks` from the launchpad, never from items, which
  05 must reflect (00 section 4.3 lists `WarState` under war, consistent).
- **C6, `Observations` per pool** (00 seeds table has `["obs", pool]` under swap, consistent); the
  account adds one writable account to every swap and liquidity change.
