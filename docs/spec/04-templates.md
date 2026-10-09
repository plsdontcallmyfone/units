# Hookwars spec 04: templates (`hookwars_items`)

Status: specification, 2026-10-08, revised after the integration rulings (00 section 9). Nothing
here is built. Follows `00-overview.md`; where they differ, 00 wins and this file is wrong.
Upstream references are to Bordrless at `43688f3`. Sibling names used here: `TokenSlotArgs`,
`SlotReturn`, `on_touch` (01); `Item`, `Template`, combine rules (02); `RouteContext`,
`ItemPoolContext`, `pool_before_swap` / `pool_after_swap` (03); `WarConfig`, `WarState`, seeds
`["war-signer"]`, `["war-chest", mint]` (05).

## 1. What this program is

`hookwars_items` (`<ITEMS_ID>`) is **one program implementing every template** (00 D-2). An item
is an armory record (`Item` at `["item", item_mint]`, 02) holding a `template_id` and
`params: [u32; PARAM_FIELDS]` (R7). When a slot holding an item runs, the caller invokes
`hookwars_items` and the program dispatches on `Item.template_id`. One deployed program, one audit
and one hook signer per caller serve every template and every item.

Templates mint parameters, never code (00 rule 4). Items are **leaves**: no callback makes a CPI
(R3). Everything an item must remember beyond a holding's range goes into accounts this program
owns: `EquipState` and `RaidLedger`.

## 2. Common machinery

### 2.1 Signers

Upstream pattern: the caller signs every callback with `["hook-authority", hook_program]` under its
own id, and the hook compares the signer with that constant (`crates/bordrless-hook/src/lib.rs:476`;
checked as `#[account(signer, address = TOKEN_HOOK_SIGNER)]` in
`programs/half_life/src/lib.rs:466-470` and `programs/tax_hook/src/lib.rs:265-268`).

| Constant | Value | Accepted for |
| --- | --- | --- |
| `TOKEN_ITEMS_SIGNER` | `["hook-authority", <ITEMS_ID>]` under `<TOKEN_ID>` | token slot callbacks (`before_transfer`, `on_touch`) |
| `LAUNCH_ITEMS_SIGNER` | `["hook-authority", <ITEMS_ID>]` under `<LAUNCH_ID>` | `pool_before_swap`, `pool_after_swap` forwarded by the launchpad (03 5.1) |
| `ARMORY_SIGNER` | the armory PDA that calls items (02 names it) | `init_equip`, `close_equip` |
| `WAR_SIGNER` | `["war-signer"]` under `<WAR_ID>` | the `authority` of a `touch` carrying a war payload (2.10) |

A callback with any other signer fails with `BadHookSigner`. The DEX's own pool-hook signer is
refused, so a pool someone opens on the DEX with `hook_program = <ITEMS_ID>` cannot drive an item.

### 2.2 Which item, and proof that it is equipped

Extras come from the item registry `["bordrless-hook-accounts", mint, item]` under `<ITEMS_ID>`
(00 4.3), written by `init_equip`. Every callback receives, after the caller's prefix:

| Extra | Account | Writable |
| --- | --- | --- |
| 0 | `Item` (owned by `<ARMORY_ID>`) | no |
| 1 | `EquipState` at `["equip", mint, slot]` under `<ITEMS_ID>` | yes |
| 2.. | the template's own extras (section 3) | per template |

Checks on every callback, in order:

1. signer (2.1);
2. `Item` is owned by `<ARMORY_ID>`, has the `Item` discriminator and its key equals the `item` in
   the args (`TokenSlotArgs.item`, 01; `ItemPoolContext.item`, 03) (`WrongItem`);
3. `EquipState` is at `["equip", mint, slot]` and `EquipState.item == item` (`NotEquipped`). The
   caller (token program or launchpad) only calls the item its slot table names, and the item
   re-checks against its own equip record, so a client cannot substitute another item's extras;
4. `Item.template_id` is known (`UnknownTemplate`) and `validate` of the params holds (`BadParams`).

### 2.3 Targets and roles: chosen when equipping, not in the item

Params are `u32` fields (R7), so they cannot name a mint. An item is a weapon with strengths; **what
it is aimed at is chosen by the community that equips it**. The equip carries:

```rust
pub struct EquipConfig {
    pub targets: Vec<Pubkey>,     // at most MAX_ITEM_TARGETS; meaning per template (section 3)
    pub role: u8,                 // 0 none; Tribute: 1 Pay, 2 Receive
}
```

It is part of the armory's equip proposal (vote) or launch equip (R12), passed to `init_equip`,
stored in `EquipState`, and fixed for as long as the item stays in that slot. Forging never touches
targets (they are not params), so a Raid forged from two Raids can be aimed anywhere when equipped.

### 2.4 Hook data ranges

00 4.4: the first byte of every item range is the token program's epoch byte; an item sees
`data_len - 1` bytes in `TokenSlotArgs.source_data` / `destination_data` (01) and answers the same
length in `SlotReturn` (01 3.1). All byte counts in section 3 are **without** the epoch byte; the
slot's `data_len` is that count plus one. The first byte an item sees is its layout tag; all zeros
means never stamped (or stale after a re-equip). Every template clears its bytes when the holding it
describes is emptied (upstream rule, `half_life/src/lib.rs:335-340`), so holdings stay closeable.

Timestamps in ranges are `u32` unix seconds (valid until 2106) to fit the budget; full `i64` times
stay in accounts.

**Budget with the kit.** The kit takes bytes 0..32 (R8, `hooks-v2.md` 4.4). Item ranges share the
other 32:

| Template | Bytes seen | `data_len` with epoch |
| --- | --- | --- |
| Raid | 11 | 12 |
| Shield | 6 | 7 |
| Half-Life | 5 | 6 |
| Wall, Spy, Treaty, Tribute, Transfer Fee, War orders | 0 | 0 |

Raid + Shield + Half-Life = 12 + 7 + 6 = 25 of 32: a kit token can carry all three with 7 bytes to
spare. Quest markers are not in any range: they live in 05's `QuestMark` PDA (05 9). Without the
kit, 64 bytes are free.

### 2.5 Money: token side (R1) and pool side (R2)

**Token side.** A cutting item answers at most **one** delta per transfer, into its **equip vault**:
`holding(mint, EquipState)` (the `EquipState` PDA owns it). The token program never computes
royalties. `MAX_CUTTING_SLOTS` token-side is at most `MAX_DELTAS` (3, upstream `lib.rs:43`) less any
delta the `Locked` slot answers on the same transfer.

**Pool side.** The launchpad already answers two of three deltas on a buy (creator fee, holder fee,
`hooks-v2.md` 5.4). All Pool-item cuts on one side are merged by the launchpad into **one** delta
into the pool-cuts holding `holding(BRIDGED_SOL, PoolCuts)`, `PoolCuts = ["pool-cuts", mint]` under
`<ITEMS_ID>`. Each item records its own share **in the same call**:
`EquipState.pool_owed += cut`. The item computes `cut` on `ItemPoolContext.quote_after_launch`
(03), the exact quote the launchpad then cuts, so what is recorded is what is applied; if the
launchpad refuses the answer the whole swap fails and nothing is recorded.

**`settle_equip(mint, slot)`**, permissionless, pays both:

```
token side:  b = equip vault balance
             royalty = floor(b * Item.royalty_bps / 10_000) -> holding(mint, RoyaltyOwner)
             bounty  = floor((b - royalty) * MAX_CRANK_BOUNTY_BPS / 10_000) -> sender's holding
             rest    -> the template's token-side destination (burn, collector)
pool side:   p = EquipState.pool_owed - EquipState.pool_settled
             royalty = floor(p * Item.royalty_bps / 10_000) -> holding(BRIDGED_SOL, RoyaltyOwner)
             bounty  = floor((p - royalty) * MAX_CRANK_BOUNTY_BPS / 10_000) -> sender
             rest    -> the template's pool-side destination (our war chest; partner's treaty inbox)
             pool_settled += p   (PoolCuts signs)
```

The royalty is exactly a share of what the item collected, never an extra charge (00 rule 1). The
DEX takes Bordrless's protocol share of every hook cut on the swap itself (`hooks-v2.md` 3.1), before
the cut reaches `PoolCuts`, so the protocol share is always first. `RoyaltyOwner` is
`["royalty", item]` under the armory (00 4.3).

An item can leave its slot only when settled: `close_equip` refuses while the equip vault holds a
balance or `pool_owed > pool_settled` (`VaultNotSettled`); the site puts `settle_equip` before every
`execute` of an unequip (02).

### 2.6 Entry points the armory calls

| Instruction | Args | Effect | Errors |
| --- | --- | --- | --- |
| `validate_params` | `template_id: u16`, `params: [u32; PARAM_FIELDS]` | none; template rules beyond per-field floor and ceiling (cross-field rules, section 3) | `BadParams` |
| `manifest` | `template_id`, `params` | return data: `Manifest` (2.7), stored in the `Item` at creation (02 1.6) | `BadParams` |
| `combine_params` | `template_id`, `field_max: [u32; PARAM_FIELDS]`, `a`, `b` | return data: the forged params | `NotForgeable`, `BadParams` |
| `init_equip` | `slot: u8`, `item: Pubkey`, `config: EquipConfig` | creates or resets `EquipState`, writes the item registry, creates the equip vault and, the first time, `RaidLedger` | `VaultNotSettled`, `BadTargets` |
| `close_equip` | `slot: u8` | marks `EquipState` empty | `VaultNotSettled` |

`validate_params`, `manifest` and `combine_params` touch no account but the signer, so a client may
simulate them as views. The per-field floor and ceiling live in the armory's `Template` (`field_min`,
`field_max`, 02 2.2); the armory checks them in one place, and this program adds only what a
per-field check cannot express.

### 2.7 Params and forging

`params: [u32; PARAM_FIELDS]`; fields a template does not use are 0. Each template below lists its
fields: index, meaning, floor and ceiling (by name, to set), and its **forge rule**:

| Rule | `combine(a, b)` with ceiling `c` and floor `f` |
| --- | --- |
| `TowardCeiling` | `m = max(a, b)`; `m + floor((c - m) * FORGE_GAIN_BPS / 10_000)` |
| `TowardFloor` | `m = min(a, b)`; `m - floor((m - f) * FORGE_GAIN_BPS / 10_000)` |
| `Keep` | `a`; both must be equal, else `NotForgeable` (modes and flags) |

`FORGE_GAIN_BPS` is the one forge constant (R7). Each forge closes a share of the remaining
distance to the ceiling, so levels have diminishing returns and never pass the ceiling. The results
are deterministic and symmetric in `(a, b)`. These three rules map onto 02's table as `Max`-based
(`TowardCeiling`), `Min`-based (`TowardFloor`) and `Keep` (see C3).

**The manifest** (16 bytes, Borsh, field order fixed; 02 stores it in the `Item`):

| # | Field | Type | Meaning |
| --- | --- | --- | --- |
| 1 | `kind` | u8 | slot kind, 00 4.1 |
| 2 | `token_flags` | u16 | upstream `token_flags` bits plus 01's `ANSWERS_TOUCH`; 0 when the item gets no token callbacks |
| 3 | `pool_flags` | u8 | bit 0 `pool_before_swap`, bit 1 `pool_after_swap`, bit 2 writes the raid mark; 0 when it gets no pool callbacks |
| 4 | `max_cut_buy_bps` | u16 | worst-case pool cut on a buy, from the params |
| 5 | `max_cut_sell_bps` | u16 | worst-case pool cut on a sell |
| 6 | `max_cut_transfer_bps` | u16 | worst-case token-side cut on a transfer |
| 7 | `max_discount_bps` | u16 | largest creator and holder fee discount it may ask for |
| 8 | `may_refuse` | bool | can fail a transfer (Wall, Transfer Fee with a wallet cap) |
| 9 | `may_burn` | bool | answers a burn |
| 10 | `data_bytes` | u8 | range bytes it uses, without the epoch byte (2.4) |
| 11 | `reads_other_pools` | u8 | foreign `Observations`, `Launch` or `Mint` accounts it reads |

1 + 2 + 1 + 2 + 2 + 2 + 2 + 1 + 1 + 1 + 1 = 16 bytes. 06's `ItemManifest.marks` is `pool_flags`
bit 2.

**Site sentence.** Each template in section 3 fixes one sentence the site prints for an item, with
`{field}` replaced by the field's value (bps shown as a percent, seconds as a duration) and
`{target}` by the equipped target's name. Nothing else is said about an item's behaviour.

### 2.8 The pool-item answer

The launchpad calls `pool_before_swap(args: PoolHookArgs, ctx: ItemPoolContext)` and
`pool_after_swap(...)` (03 5.1). `PoolHookArgs.route: RouteContext` is filled by the DEX (R5); the
launchpad passes it unchanged. An item answers:

```rust
pub struct ItemPoolAnswer {
    /// Basis points of the launchpad's creator and holder fees on this side to waive (R6).
    pub discount_bps: u16,
    /// This item's cut of ctx.quote_after_launch, merged by the launchpad into the one PoolCuts delta.
    pub cut: u64,
    /// Base to burn, only where the slot allows a burn on this side.
    pub burn: u64,
}
```

Items never set the LP fee (R6). Discounts are summed and capped by the launchpad, and come out of
the creator and holder fees only (03 5.3). No royalty field and no mark field: the royalty is taken
by `settle_equip` (2.5) and the mark is written by the item into `RaidLedger` directly (2.9).

### 2.9 `RaidLedger` and the raid mark (R3, R4)

Pool callbacks see the route but cannot write holdings; token callbacks write holdings but cannot
see the route. The bridge between them is `RaidLedger` at `["raid-ledger", mint]`, owned by
`hookwars_items`, written by pool items during swaps and read by `hookwars_war` (05 2.4).

```rust
pub struct RaidLedger {
    pub version: u8, pub bump: u8,
    pub mint: Pubkey,
    pub season_id: u32,                       // WarConfig.current_season when last rolled
    pub outbound_volume_season: u64,          // raid volume our raids brought into us this season (05 name)
    pub inbound: [RaidWindow; RAID_TABLE_LEN],// per rival whose holders were raided into us
    pub mark: Mark,
    pub reserved: [u8; 32],
}
pub struct RaidWindow { pub rival_mint: Pubkey, pub window_start: i64, pub volume: u64, pub prev_volume: u64 }
pub struct Mark {
    pub clock_slot: u64,                      // Clock::slot when written; 0 = none
    pub recipient: Pubkey,                    // PoolHookArgs.recipient
    pub rival: Pubkey,                        // RouteContext.route_input_mint
    pub quote_volume: u64,                    // PoolHookArgs.amount_in of the raid buy (bridged SOL)
    pub stamped_slots: u8,                    // bit i set once slot i's token half stamped it
}
```

**Windows (05's two-window rolling rule).** Adding `v` from `rival` at `now`, `W = RAID_WINDOW_SECS`:

```
e = entry for rival, else a free entry: one whose window ended (now >= window_start + 2*W),
    oldest window_start first; if none is free, the raid is not recorded in inbound (a flood of dust
    rivals cannot evict a live raid); the mark and points still happen
k = (now - e.window_start) / W                   // whole windows passed
if k == 1: e.prev_volume = e.volume; e.volume = 0
if k >= 2: e.prev_volume = 0;        e.volume = 0
e.window_start += k * W                          // new entry: window_start = now, both 0
e.volume += v                                    // checked
```

The rolling volume 05 reads at `t` is
`volume + prev_volume * (window_start + W - t) / W` (05 2.4).

**Season roll.** Every write first reads `WarConfig.current_season` (read-only extra) and, if it
differs from `season_id`, sets `season_id` and zeroes `outbound_volume_season`.

**The mark (R4).** Written in `pool_after_swap` by any marking item (Raid, Shield) that sees a raid
buy from one of its targets; overwritten by the next swap of that pool (which resets
`stamped_slots`). Two marking items on one swap write identical values. A token half consumes it on
the **first transfer into `recipient`'s holding in the same clock slot**, which is the delivery: the
DEX pays `after_swap` deltas only to other holdings, then delivers (R4). A token half accepts the
mark only when `clock_slot == Clock::slot`, `destination_owner == recipient`,
`source_owner == the launch pool` and its own bit in `stamped_slots` is clear; it then sets the bit.
No dependence on `Pool.swap_count`.

`RaidLedger` is writable on every launch-pool swap that runs a marking item. Every buy already
write-locks the `Launch` account (`hooks-v2.md` 5.4), so trades of one token are already serialized;
this adds no contention between tokens.

`init_raid_ledger(mint)`: permissionless, the payer pays rent; `init_equip` of a marking template
creates it if missing.

### 2.10 Touch payloads (01 `on_touch`)

`hookwars_war` changes a holding's Raid bytes only through `touch`, signed by `WAR_SIGNER` (05
sections 7 to 9). The token program forwards `TokenSlotArgs.context` and the caller as `authority`
(01). The Raid item decodes:

```rust
pub enum WarTouch {
    SpendRaidPoints { amount: u32 },   // claim_bounty (05 7), Raid quest (05 9)
    SpendTicket,                       // roll (05 8.2)
    AddTicket { amount: u16 },         // claim_quest (05 9)
}
```

Quest "once per period" markers are not in hook data: they live in 05's `QuestMark` PDA
`["quest", season, mint, owner]`.

Accepted only when `args.authority == WAR_SIGNER` and the token program reports it as a signer
(01); any other caller gets `NotWarSigner`. Behaviour in 3.1. Every other template answers a touch
with nothing.

## 3. Templates

| Id | Template | Slot kind (00 4.1) | Callbacks | Targets |
| --- | --- | --- | --- | --- |
| 0 | none | | | |
| 1 | Raid | Pool, with token half | `pool_before_swap`, `pool_after_swap`; `before_transfer`, `on_touch` | rival mints, 1..`RAID_MAX_RIVALS` |
| 2 | Shield | Pool, with token half | `pool_after_swap`; `before_transfer` | rival mints, 1..`SHIELD_MAX_RIVALS` |
| 3 | Wall | Defense | `before_transfer` | none |
| 4 | Spy | Pool | `pool_before_swap`, `pool_after_swap`; no token callbacks | one rival mint |
| 5 | Treaty | Relation, pool half only | `pool_before_swap`; no token callbacks | the partner mint |
| 6 | Tribute | Relation, pool half only | `pool_before_swap`; no token callbacks | the partner mint; role Pay or Receive |
| 7 | Half-Life | Fee | `before_transfer` | none |
| 8 | Transfer Fee | Fee | `before_transfer` | the collector |
| 9 | War orders | War | none | none |

A Pool-kind item with a token half needs its slot called on token transfers too, with a data range
(I-01.1). Under 00 4.1, a `Relation` item runs on token transfers and, when its template has a pool
half, on launch pool swaps through the launchpad. Treaty and Tribute have **only** a pool half: their
manifest's `token_flags` is 0, so the token program never calls them, and `pool_flags` is bit 0
(`pool_before_swap`). Spy is kind `Pool` with `pool_flags` bits 0 and 1 and `token_flags` 0. Fee,
Defense and War items never receive pool callbacks. Counter-strike, siege,
raze and bounties are `hookwars_war` instructions (05) configured by the War orders item; they are
not hooks.

### 3.1 Raid (id 1)

Buyers who reach us by **selling a targeted rival** pay less creator and holder fee, may pay a toll
to the item, and earn raid points and loot tickets in their holding.

**Fields**

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `discount_bps` | 0, `RAID_MAX_DISCOUNT_BPS` (at most 10,000) | `TowardCeiling` |
| 1 | `toll_bps` | 0, `RAID_MAX_TOLL_BPS` | `TowardCeiling` |
| 2 | `points_per_unit` | 0, `RAID_MAX_POINTS_PER_UNIT` | `TowardCeiling` |

Targets: 1 to `RAID_MAX_RIVALS` rival mints, each with a Hookwars launch. An empty list is refused
(`BadTargets`): otherwise anyone could launch a junk token, route through it and take the discount.

**Extras:** `RaidLedger` (w); `WarConfig` (r); for each target, its `Launch` at `["launch", rival]`
under `<LAUNCH_ID>` (r), fixed keys in the registry.

**Range** (11 bytes):

| Bytes | Field |
| --- | --- |
| 0 | tag `0x01` |
| 1..5 | `season_id: u32` |
| 5..9 | `raid_points: u32` |
| 9..11 | `tickets: u16` |

A range whose `season_id` is not the current season reads as `raid_points = 0` (tickets carry over;
05 decides whether tickets expire, C5).

**Raid test** (both pool callbacks):

```
r = args.route
raid = args.direction == BUY
       and r.hop_index >= 1
       and r.route_input_mint != our mint and r.route_input_mint != BRIDGED_SOL
       and r.route_input_mint in targets
       and r.first_pool == Launch(r.route_input_mint).pool       // the rival really sold on its own launch pool
```

Each hop's input is what the previous hop delivered (R5), so the SOL entering our pool is the
proceeds of the rival sale; no outside top-up is possible.

**`pool_before_swap`:** if raid, answer `{ discount_bps, cut: fee_amount(ctx.quote_after_launch,
toll_bps), burn: 0 }` (upstream `fee_amount` rounds up) and `EquipState.pool_owed += cut`.
Otherwise answer nothing.

**`pool_after_swap`:** if raid: roll the season; `inbound.add(route_input_mint, args.amount_in)`;
`outbound_volume_season += args.amount_in`; write the mark
`{ clock_slot, recipient: args.recipient, rival, quote_volume: args.amount_in, stamped_slots: 0 }`.

**`before_transfer`** (token half):

```
if source == destination: nothing
d = dest range (season-checked), s = source range (season-checked)
if mark accepted (2.9):
    pts = (mark.quote_volume / POINT_UNIT_LAMPORTS) * points_per_unit         // saturating u32
    tik = mark.quote_volume >= LOOT_MIN_RAID_LAMPORTS and WarConfig.current_season > 0 ? 1 : 0
    d.raid_points += pts; d.tickets += tik (saturating); d.season_id = current
    set stamped bit; emit RaidMarked { mint, rival: mark.rival, trader: recipient,
                                        volume: mark.quote_volume, points: pts, loot_ticket: tik == 1 }
else if both owners are holders (neither the launch pool, the launch PDA nor a program vault):
    moved = floor(s.raid_points * amount / source_balance); same for tickets
    s -= moved; d += moved
else if destination_owner == launch pool (a sell):
    s.raid_points -= floor(s.raid_points * amount / source_balance); same for tickets
clear s when source_balance == amount
answer source_data / destination_data
```

**`on_touch`** (`WAR_SIGNER` only, 2.10):

| Payload | Effect | Error |
| --- | --- | --- |
| `SpendRaidPoints { amount }` | `raid_points -= amount` for the current season | `NotEnoughPoints` |
| `SpendTicket` | `tickets -= 1` | `NoTicket` |
| `AddTicket { amount }` | `tickets += amount` (saturating) | none |

**Abuse**

- Faking a route: the DEX fills `route` (R5); `hook_data` is never read. The rival's own launch pool
  must be the first hop, so the trader really sold the rival there and paid its fees.
- Wash raiding with one's own rival tokens: costs the rival's fees, our fees after discount,
  slippage on both pools and the toll. Points are linear in SOL routed; splitting gains nothing;
  tickets need `LOOT_MIN_RAID_LAMPORTS` per buy and a running season.
- Raid then dump: a sell destroys points and tickets pro rata.
- Fresh wallets: points and tickets move with tokens, never copy. Quests cannot be farmed by moving
  tokens: their markers are 05's `QuestMark` PDA per owner, and the Raid quest spends points.
- Sieges built on raid volume (05) refuse rivals whose kit has holder rewards on (R10): a war chest,
  off curve, cannot hold that token. Raiding such a rival is still allowed; only the siege is not.

**Manifest:** `kind` Pool; `token_flags` `BEFORE_TRANSFER | WRITES_HOOK_DATA | ANSWERS_TOUCH`;
`pool_flags` bits 0, 1, 2; `max_cut_buy_bps` = `toll_bps`; `max_cut_sell_bps` 0;
`max_cut_transfer_bps` 0; `max_discount_bps` = `discount_bps`; `may_refuse` false; `may_burn` false;
`data_bytes` 11; `reads_other_pools` = number of targets.

**Site sentence:** "Buyers who sold {target} pay {discount_bps} less creator and holder fee, pay a
{toll_bps} toll, and earn {points_per_unit} raid points per unit of SOL raided."

### 3.2 Shield (id 2)

Holders whose tokens arrived **from a targeted rival's raid route** within a window pay an extra
cut when they sell: a toll on raiders who rotate in and straight back out.

**Fields**

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `sell_cut_bps` | 0, `SHIELD_MAX_SELL_CUT_BPS` | `TowardCeiling` |
| 1 | `window_secs` | `SHIELD_MIN_WINDOW_SECS`, `SHIELD_MAX_WINDOW_SECS` | `TowardCeiling` |
| 2 | `only_under_siege` | 0, 1 | `Keep` |

Targets: 1 to `SHIELD_MAX_RIVALS` rival mints with launches.

**Extras:** `RaidLedger` (w); `WarConfig` (r); `WarState` at `["war", mint]` under `<WAR_ID>` (r);
the target `Launch` accounts (r); the actor's holding `["holding", mint, actor]` under `<TOKEN_ID>`
(r, upstream `Seed::Account` on the prefix actor).

**Range** (6 bytes): 0 tag `0x02`; 1 `origin: u8` (target index + 1, 0 none); 2..6 `origin_at: u32`.

**`pool_after_swap`, buy:** the Raid test (3.1) against Shield's targets; if it holds, write the
mark as Raid does (identical values if both are equipped) and roll `inbound` the same way only if no
Raid item is equipped in the mint (the item reads the mint's slot table from `ItemPoolContext`, 03;
one writer per swap, so volume is never counted twice).

**`before_transfer`:** on an accepted mark, `origin = index(mark.rival) + 1` and `origin_at` blended
by weight with what the holding held (upstream `blend`, `half_life/src/lib.rs:127-134`). Between
holders, origin and time move with the tokens like Half-Life's age. Cleared when emptied.

**`pool_after_swap`, sell:**

```
h = Shield bytes of the actor's holding (offset from the mint's slot table; stale epoch reads empty)
marked = h.origin != 0 and now - h.origin_at < window_secs
         and (only_under_siege == 0 or WarState.under_siege_until > now)
if marked: cut = fee_amount(ctx.quote_after_launch, sell_cut_bps); EquipState.pool_owed += cut
           answer { cut }
```

Pool-side destination: our war chest.

**Abuse:** sending to a fresh wallet first carries the origin. Waiting out `window_secs` pays
nothing, which is intended: it taxes hit-and-run, not holders.

**Manifest:** `kind` Pool; `token_flags` `BEFORE_TRANSFER | WRITES_HOOK_DATA`; `pool_flags` bits
1, 2; `max_cut_sell_bps` = `sell_cut_bps`; other cuts 0; `max_discount_bps` 0; `may_refuse` false;
`may_burn` false; `data_bytes` 6; `reads_other_pools` = number of targets.

**Site sentence:** "Holders who came from {target} pay {sell_cut_bps} extra when they sell within
{window_secs}" followed, when `only_under_siege` is 1, by " while we are under siege".

### 3.3 Wall (id 3)

A temporary max wallet while the token is **under siege** (05 sets `WarState.under_siege_until` and
the attacker). It caps wallets and the attacking war chest.

**Fields**

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `max_wallet_bps` | `WALL_MIN_MAX_WALLET_BPS`, 10,000 | `TowardFloor` (a tighter wall is stronger) |

**Extras:** `WarState` (r).

**`before_transfer`:**

```
if WarState.under_siege_until <= now: nothing
capped = destination_owner.is_on_curve()                       // wallets (upstream kit syscall, hooks-v2 4.5)
         or destination_owner == WarState.siege_by_chest      // the attacking war chest
if !capped: nothing                                           // pool, launch, vaults, royalty owners
cap = supply * max_wallet_bps / 10_000                        // u128, as tax_hook lib.rs:151
require(destination_balance + amount <= cap, WallHolds)
```

No data, no cuts. A sell never fails on it (the destination is the pool). `siege_by_chest` is a field
05 must keep (C4).

**Abuse:** a siege cannot outlast 05's bound on `under_siege_until`, so a Wall cannot freeze buys
indefinitely. It bounds each wallet, not the sum.

**Manifest:** `kind` Defense; `token_flags` `BEFORE_TRANSFER`; `pool_flags` 0; all cuts 0;
`max_discount_bps` 0; `may_refuse` true; `may_burn` false; `data_bytes` 0; `reads_other_pools` 0.

**Site sentence:** "While we are under siege, no wallet may hold more than {max_wallet_bps} of
supply."

### 3.4 Spy (id 4)

Our fees move with a **rival's time-weighted price**, read from its `Observations` (03), never spot
(00 rule 7).

**Fields**

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `mode` | 1 Rivalry, 2 Momentum | `Keep` |
| 1 | `window_secs` | `MIN_TWAP_SECS`, `SPY_MAX_WINDOW_SECS` | `TowardFloor` (reacts sooner) |
| 2 | `trigger_bps` | `SPY_MIN_TRIGGER_BPS`, `SPY_MAX_TRIGGER_BPS` | `TowardFloor` |
| 3 | `effect_bps` | 0, `SPY_MAX_EFFECT_BPS` | `TowardCeiling` |

Target: one rival mint. **Extras:** the rival `Launch` (r); `["obs", rival_pool]` under `<SWAP_ID>` (r).

```
now_w  = twap(now - window, now); prev_w = twap(now - 2*window, now - window)   // 03 helper
if either is none (ring too short): nothing
change_bps = (now_w - prev_w) * 10_000 / prev_w                                // signed
Rivalry,  change_bps >=  trigger, SELL, pool_after_swap: cut = fee_amount(quote_after_launch, effect_bps)
Momentum, change_bps <= -trigger, BUY,  pool_before_swap: discount_bps = effect_bps
```

Pool-side destination: our war chest. **Abuse:** moving the rival's time-weighted price costs holding
a manipulated price on the rival's own pool for `window_secs`, paying its fees both ways.

**Callbacks:** `pool_before_swap` (Momentum discount) and `pool_after_swap` (Rivalry cut); never a
token callback.

**Manifest:** `kind` Pool; `token_flags` 0; `pool_flags` bits 0, 1; `max_cut_sell_bps` =
`effect_bps` in Rivalry, else 0; `max_discount_bps` = `effect_bps` in Momentum, else 0; other cuts 0;
`may_refuse` false; `may_burn` false; `data_bytes` 0; `reads_other_pools` 1.

**Site sentence:** Rivalry: "When {target} rises {trigger_bps} over {window_secs}, sellers pay
{effect_bps} extra." Momentum: "When {target} falls {trigger_bps} over {window_secs}, buyers pay
{effect_bps} less creator and holder fee."

### 3.5 Treaty (id 5)

Two tokens pay each other from their buys. **One** Treaty item is equipped by **both** mints, each
targeting the other; it does nothing until both have.

**Fields**

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `low_to_high_bps` | 0, `TREATY_MAX_BPS` | not forgeable |
| 1 | `high_to_low_bps` | 0, `TREATY_MAX_BPS` | not forgeable |
| 2 | `returns_captured` | 0, 1 | not forgeable |

"Low" is the numerically smaller mint key of the pair. `returns_captured = 1` lets 05's
`return_captured` send captured holdings back under this treaty (05 6.4).

**Extras:** the partner's `Mint` (r); the partner's `EquipState` for the slot holding this item (r;
the client finds the slot from the partner's slot table).

**Active when:** the partner's `EquipState.item == this item`, its `targets[0] == our mint`, and the
partner's slot table entry at that slot names this item (01 layout). One read each, no keeper.

**`pool_before_swap`, buy:** `cut = fee_amount(ctx.quote_after_launch, my_side_bps)`;
`EquipState.pool_owed += cut`. Pool-side destination: the **partner's treaty inbox**
`holding(BRIDGED_SOL, ["treaty-inbox", partner])` under `<WAR_ID>`, which 05 streams to the
partner's holders with the kit's `share` (R13). Never a direct deposit into a kit reward vault:
upstream distributes direct deposits at once (`hooks-v2.md` 4.7), which a buy just before could
capture.

**Callbacks:** `pool_before_swap` only; never a token callback, never `pool_after_swap`.

**Equip check (`init_equip`):** the partner must have the kit in a `Locked` slot with holder rewards
on, since its treaty inbox is streamed through the kit's `share` (05); else `BadTargets`.

**Abuse:** a treaty nobody accepted is inert. A side that stops paying has unequipped, so the
other side sees no partner and stops too.

**Manifest:** `kind` Relation; `token_flags` 0; `pool_flags` bit 0; `max_cut_buy_bps` = the larger
of fields 0 and 1; other cuts 0; `max_discount_bps` 0; `may_refuse` false; `may_burn` false;
`data_bytes` 0; `reads_other_pools` 1 (the partner `Mint`).

**Site sentence:** "Every buy of us sends {my_side_bps} to {target}'s holders, and every buy of
{target} sends {their_side_bps} to ours" (the two sides read from fields 0 and 1 by key order),
followed, when `returns_captured` is 1, by "; captured bags are returned while it holds".

### 3.6 Tribute (id 6)

A one-way Treaty. Field 0 `bps` (0, `TRIBUTE_MAX_BPS`; not forgeable). Both mints equip the same
item targeting each other, the payer with role `Pay`, the receiver with role `Receive`; active only
when both roles are present. On the payer's buys, as Treaty; on the receiver's swaps, nothing.
Anything granted back (a Raid discount for the payer's holders, for example) is a separate item.

**Callbacks:** `pool_before_swap` only, and only the payer's equip answers; never a token callback.
The receiver needs the kit with holder rewards on, as Treaty.

**Manifest:** `kind` Relation; `token_flags` 0; `pool_flags` bit 0; `max_cut_buy_bps` = `bps` (payer
side); other cuts 0; `max_discount_bps` 0; `may_refuse` false; `may_burn` false; `data_bytes` 0;
`reads_other_pools` 1.

**Site sentence:** payer: "Every buy of us sends {bps} to {target}'s holders as tribute." Receiver:
"{target} pays us {bps} of every buy of it as tribute."

### 3.7 Half-Life (id 7)

Upstream `half_life` as a template (`programs/half_life/README.md`, logic
`programs/half_life/src/lib.rs:276-356`), its constants made fields:

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `max_fee_ppm` (upstream `MAX_FEE_PPM`, `lib.rs:75`) | 0, `HL_MAX_FEE_PPM` | `TowardCeiling` |
| 1 | `half_life_secs` (`HALF_LIFE_SECS`, `lib.rs:77`) | `HL_MIN_HALF_LIFE_SECS`, `HL_MAX_HALF_LIFE_SECS` | `TowardCeiling` |
| 2 | `zero_after_halvings` (`ZERO_AFTER_HALVINGS`, `lib.rs:79`) | 1, `HL_MAX_HALVINGS` | `TowardCeiling` |

**Range** (5 bytes): 0 tag `0x07`; 1..5 `since: u32` (upstream used a 3-byte magic and an `i64`,
`lib.rs:83-121`).

**`before_transfer`:** upstream's function with three changes: `fee_ppm(age)` uses the fields
(`lib.rs:87-99`); exempt owners are the launch PDA, the launch pool, our war chest, `PoolCuts`,
every `EquipState` of the mint and the royalty owners (upstream exempted launch, pool and furnace,
`lib.rs:292-300`); the fee is one delta into the equip vault, not a furnace. Token-side destination:
burn (`settle_equip` burns what is left after the royalty and bounty). Fees are never refused for a
missing furnace: `init_equip` creates the vault.

**Manifest:** `kind` Fee; `token_flags` `BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA |
WRITES_HOOK_DATA` (upstream `FLAGS`, `lib.rs:68`); `pool_flags` 0; `max_cut_transfer_bps` =
`ceil(max_fee_ppm / 100)`; pool cuts 0; `max_discount_bps` 0; `may_refuse` false; `may_burn` false
(the burn happens in `settle_equip`, not in an answer); `data_bytes` 5; `reads_other_pools` 0.

**Site sentence:** "Selling or sending costs {max_fee_ppm} of the tokens moved, halving every
{half_life_secs} held, and nothing after {zero_after_halvings} halvings; buys are free; fees are
burned."

### 3.8 Transfer Fee (id 8)

Upstream `tax_hook` as a template (`programs/tax_hook/src/lib.rs:127-165`).

| # | Field | Floor, ceiling | Forge |
| --- | --- | --- | --- |
| 0 | `fee_bps` | 0, `TF_MAX_FEE_BPS` | `TowardCeiling` |
| 1 | `max_wallet_bps` | 0 (off) or `TF_MIN_MAX_WALLET_BPS`, 10,000 | `TowardFloor` when both are on, else `Keep` |

Target: the collector. Cross-field rule (`validate_params`): `max_wallet_bps` is 0 or at least
`TF_MIN_MAX_WALLET_BPS`. **`before_transfer`:** upstream's fee as one delta into the equip vault and
upstream's wallet cap (`lib.rs:147-154`); exempt owners as Half-Life plus the collector. Token-side
destination: the collector's holding.

**Manifest:** `kind` Fee; `token_flags` `BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA`; `pool_flags` 0;
`max_cut_transfer_bps` = `fee_bps`; pool cuts 0; `max_discount_bps` 0; `may_refuse` =
`max_wallet_bps > 0`; `may_burn` false; `data_bytes` 0; `reads_other_pools` 0.

**Site sentence:** "Every transfer pays {fee_bps} to {target}" followed, when `max_wallet_bps` is
above 0, by "; no wallet may hold more than {max_wallet_bps} of supply".

### 3.9 War orders (id 9)

Kind `War` (00 4.1, value 6): **no callbacks**; the token program and launchpad never call it.
`hookwars_war` reads its fields from the `Item` in the mint's War slot (05). Equipping it by vote is
how a community sets its war policy.

| # | Field | Floor, ceiling | Forge | Used by 05 |
| --- | --- | --- | --- | --- |
| 0 | `siege_threshold` (units of `SIEGE_UNIT_LAMPORTS`) | `WAR_MIN_SIEGE_THRESHOLD`, `WAR_MAX_SIEGE_THRESHOLD` | not forgeable | `siege`: rolling inbound volume from the rival must reach it |
| 1 | `siege_spend_bps` | 0, `SIEGE_MAX_SPEND_BPS` | not forgeable | `siege` spend cap, of the chest |
| 2 | `siege_twap_secs` | `MIN_TWAP_SECS`, `WAR_MAX_TWAP_SECS` | not forgeable | `siege` premium check |
| 3 | `counter_drop_bps` | `WAR_MIN_COUNTER_DROP_BPS`, `WAR_MAX_COUNTER_DROP_BPS` | not forgeable | `counter_strike` trigger |
| 4 | `counter_short_secs` | `MIN_TWAP_SECS`, `WAR_MAX_TWAP_SECS` | not forgeable | trigger window, short TWAP |
| 5 | `counter_long_secs` | `MIN_TWAP_SECS`, `WAR_MAX_TWAP_SECS` | not forgeable | trigger window, long TWAP |
| 6 | `counter_interval_secs` | `COUNTER_STRIKE_MIN_INTERVAL_SECS`, `WAR_MAX_INTERVAL_SECS` | not forgeable | spacing |
| 7 | `counter_spend_bps` | 0, `COUNTER_STRIKE_MAX_SPEND_BPS` | not forgeable | spend cap per call |
| 8 | `raze_enabled` | 0, 1 | not forgeable | `raze` allowed |
| 9 | `bounty_rate` (lamports per raid point) | 0, `WAR_MAX_BOUNTY_RATE` | not forgeable | `claim_bounty` |
| 10 | `crank_bounty_bps` | 0, `MAX_CRANK_BOUNTY_BPS` | not forgeable | war step bounties |

Cross-field rule: `counter_short_secs < counter_long_secs`. War orders are policy, not a weapon, so
the template is not forgeable (02 `forge_enabled = false`); loot may still mint them.

**Manifest:** `kind` War; `token_flags` 0; `pool_flags` 0; all cuts 0; `max_discount_bps` 0;
`may_refuse` false; `may_burn` false; `data_bytes` 0; `reads_other_pools` 0.

**Site sentence:** "Siege a rival once {siege_threshold} of its holders' SOL has raided us, spending
up to {siege_spend_bps} of the chest; counter-strike when our price falls {counter_drop_bps} between
{counter_short_secs} and {counter_long_secs}, at most every {counter_interval_secs}, spending up to
{counter_spend_bps}; bounties pay {bounty_rate} per raid point" followed, when `raze_enabled` is 1, by
"; captured bags may be razed".

## 4. Accounts of `hookwars_items`

| Account | Seeds | Fields |
| --- | --- | --- |
| `EquipState` (also owns the token-side equip vault) | `["equip", mint, slot]` | `mint`, `slot`, `item` (default = empty), `template_id`, `config: EquipConfig`, `equipped_at`, `runs: u64`, `collected_token: u64`, `pool_owed: u64`, `pool_settled: u64`, `bump`, reserved |
| `PoolCuts` (owns the pool-side holding) | `["pool-cuts", mint]` | none (system-owned signer) |
| `RaidLedger` | `["raid-ledger", mint]` | 2.9 |
| item registry | `["bordrless-hook-accounts", mint, item]` | upstream `HookAccountList` (`lib.rs:533`) |

`runs`, `collected_token` and `pool_owed` are what 02 and 06 read for an item's record; counters,
never promises.

## 5. Events

Callbacks run inside other programs' CPIs, so they log with `emit!` (program log, no extra call
level); top-level instructions use `emit_cpi!` as upstream.

| Event | Where | Fields |
| --- | --- | --- |
| `RaidMarked` | Raid token half, on stamping | `mint, rival, trader, volume, points, loot_ticket` (06's name and fields) |
| `ShieldTaken` | Shield sell | `mint, owner, cut` |
| `ItemCut` | any pool or token cut | `mint, slot, item, side, amount` |
| `EquipSettled` | `settle_equip` | `mint, slot, item, royalty_token, royalty_quote, destination, amount_token, amount_quote, bounty` |
| `EquipInitialized`, `EquipClosed` | `init_equip`, `close_equip` | `mint, slot, item, config` |

## 6. Errors

`BadHookSigner`, `WrongItem`, `NotEquipped`, `UnknownTemplate`, `BadParams`, `BadTargets`,
`NotForgeable`, `WallHolds`, `WalletTooLarge`, `VaultNotSettled`, `NotWarSigner`, `NotEnoughPoints`,
`NoTicket`, `Overflow`.

## 7. Interfaces

### From 01 (token slots)

- **I-01.1** A `Pool`-kind slot also receives `before_transfer` and `on_touch` with a data range;
  its token answers carry data only, no deltas.
- **I-01.2** `TokenSlotArgs.authority` for a touch is the caller and the token program verifies it
  signed, so the Raid item can trust `authority == WAR_SIGNER`.
- **I-01.3** The `Mint` slot table at a stable offset (Shield and Treaty read a slot's offset, epoch
  and item from a `Mint` they are passed).

### From 02 (armory)

- `Item.params: [u32; PARAM_FIELDS]`, `Template.field_min` / `field_max`, `forge_enabled`; the
  equip proposal and launch equip carry `EquipConfig` and pass it to `init_equip`.

### From 03 (DEX and launch)

- `PoolHookArgs.route: RouteContext` (R5) with `route_input_mint`, `first_pool`, `hop_index`.
- `ItemPoolContext { slot, item, quote_after_launch, ... }` and the merge of `ItemPoolAnswer`
  (`discount_bps`, `cut`, `burn`) into one `PoolCuts` delta per side (R2).
- `["obs", pool]` and `twap(from, to)` returning none when the ring does not cover the range.

### From 05 (war)

- `WarConfig.current_season`; `WarState.under_siege_until` and `WarState.siege_by_chest`.
- `["treaty-inbox", mint]` under `<WAR_ID>`, whose bridged-SOL holding 05 streams to holders.
- The Raid range layout (3.1), `WarTouch` payloads (2.10) and War orders fields (3.9).
- Quest markers in 05's `QuestMark` PDA; no quest state in any range.

## 8. Conflicts and assumptions for the integrator

- **C1** 03 section 5.2 still has `royalty` and `mark` in its answer and pays item cuts to the war
  chest; R2 and R3 replace both: the answer here is `{ discount_bps, cut, burn }`, cuts go to
  `PoolCuts`, royalties are paid by `settle_equip`, marks are written into `RaidLedger`.
- **C2** Targets are per equip (`EquipConfig`), not params: 02's proposal and launch equip must carry
  them, and `MAX_ITEM_TARGETS` joins 00's parameters.
- **C3** Forge rules: this file uses `TowardCeiling` / `TowardFloor` with `FORGE_GAIN_BPS` (R7); 02's
  table has `Max`, `Min`, `SumCapped`, `MaxPlusStep`, `Keep`. 02 should replace `MaxPlusStep` and
  `SumCapped` with the two rules here.
- **C4** 05 must store the attacking chest (`siege_by_chest`) next to `under_siege_until` for Wall,
  and add the `["treaty-inbox", mint]` seed to 00 4.3.
- **C5** 05 decides whether tickets expire with the season (Raid keeps them). `Hold` is dropped
  (05 9); quest markers are 05's `QuestMark` PDA.
- **C6** New parameters for 00 section 6: `MAX_ITEM_TARGETS`, `RAID_TABLE_LEN` and
  `RAID_WINDOW_SECS` (with 05), `POINT_UNIT_LAMPORTS`, `SIEGE_UNIT_LAMPORTS`, and every per-template
  floor and ceiling in section 3. `PARAM_FIELDS` is at least 11 (War orders).
- **C7** Events from callbacks are program logs (`emit!`); 06's indexer must decode logs as well as
  self-CPI events.

## M3b items implementation notes (branch m3bi, 2026-10-09)

Built in `programs/hookwars_items` (`engine.rs`, `equip.rs`, `settle.rs`, `templates/<name>.rs`),
`crates/hookwars-common` (raid ledger, war touch, composites, wave A shapes), the armory (composites,
`settle_bounty_bps`, the Performance reader) and the war program (integration). Where the build
differs from the text above:

1. **Accounts of a callback.** Token callbacks: the token prefix (signer, mint, source,
   destination, authority), then the registry extras. Pool callbacks: the launchpad's items signer
   (`["hook-authority", items]` under the launchpad), pool, base mint (the token), quote mint,
   actor, then **the same registry extras**. Registry: `Item`, `EquipState`, the equip vault when
   the item cuts on the token side, the `CompositeItem` for a composite, then each module's own
   extras (`templates::extras`). One registry serves both sides.
2. **Own extras per template, as built.** Raid: `RaidLedger` (w), `WarConfig`, our `Launch`, each
   target's `Launch`. Shield: `RaidLedger` (w), `WarConfig`, our `WarState`, our `Launch`, each
   target's `Launch`. Wall: our `WarState`. Spy: per target its `Launch` and its launch pool (the
   pool address derived with the policy LP fee and checked against the `Launch`; the ring is in
   the pool, 03 M3a notes). Treaty and Tribute: per target its `Mint` and its four `EquipState`s
   (the partner's slot is found in its slot table). Half-Life, Transfer Fee, Launch Decay, Max
   Transaction, Dust Guard: our `Launch`. Size Tiers, Side Skew, Sell Burn, War orders: none.
3. **Shield reads no holding from the pool side.** Its token half writes a `SellMark` (clock slot,
   seller, a bit per slot) into the `RaidLedger` on the seller's transfer into the pool, and
   `pool_after_swap` cuts from it. Shield writes the raid mark as Raid does but never counts
   inbound volume, so a swap is never counted twice; Shield alone does not feed siege volume.
4. **`RaidLedger` gains `sell_mark`** before `reserved`; its type lives in
   `hookwars_common::raid` (the war program reads the same type).
5. **Points and tickets parameters.** `POINT_UNIT_LAMPORTS` and `LOOT_MIN_RAID_LAMPORTS` are war
   parameters (`WarParams.point_unit_lamports`, `loot_min_raid_lamports`), read by the Raid item from
   `WarConfig` with `current_season` and `raid_window_secs`. Without a `WarConfig`: season 0, no
   points, no tickets.
6. **Quote-side fee templates subscribe to both pool callbacks.** Size Tiers, Side Skew and Launch
   Decay cut on a buy's `pool_before_swap` and a sell's `pool_after_swap` (the launchpad's side
   rule, 03 section 5.2); 08 names `pool_before_swap` only.
7. **Sell Burn answers in `pool_before_swap` on a sell** (base side), not `pool_after_swap`.
8. **Destinations at settlement.** Token side: Half-Life burns, Transfer Fee pays its collector.
   Pool side: Raid, Shield, Spy, Size Tiers, Side Skew, Launch Decay pay our war chest's bridged-SOL
   holding; Treaty and Tribute the partner's treaty inbox. Destination holdings must exist (the
   settler creates only its own); the bounty rate is the armory's `settle_bounty_bps`.
9. **`EquipState` v2** keeps unsettled cuts per module (`token_unsettled`, `pool_unsettled`,
   `MAX_MODULES` each); `pool_owed` and `pool_settled` stay cumulative.
10. **`init_raid_ledger` is a permissionless instruction**; `init_equip` does not create the ledger
    (its accounts come from the armory). A launch of a token with a marking item should send it.
11. **Composites (R19).** Authored with the armory's `create_composite(modules, royalty_bps)` from
    open templates (each module's `Template` passed in order); validation and the combined manifest
    are `hookwars_common::composite::validate_modules` (08 2.10, with `ITEM_DATA_MAX` 63). Not built
    yet: `fuse` (burning component items), presets, forging composites (the Composite template is
    not forgeable), `reads_module` reads (validated, no template reads another module's bytes yet).
12. **Not built in M3b:** the Treaty equip check that the partner runs the kit with holder rewards
    (the partner's inbox streams through the kit's `share`), the Shield `only_under_siege` read uses
    our `WarState` only; holder touches (R23) answer nothing (no wave A template takes them).
13. **War integration.** `hookwars_war::foreign` now re-exports the armory's `Item`, `Template`,
    `ForgeCounter` and the shared `RaidLedger`; TWAPs read the pool account's ring
    (`siege`, `counter_strike` lost their observations account). The war suites still run their CPI
    targets on `war_items_stub` and `war_armory_stub`, with accounts written in the real layouts.
14. **Measured** (`programs/tests/tests/budgets_items.rs::items_budgets`, LiteSVM, legacy
    transactions without lookup tables; a transfer row also carries an idempotent
    `create_holding`; pool rows are called through the test-only `launch_stub`, which adds its own
    call level and compute, so the launchpad's real path is measured by the launchpad branch):

    | Path | Keys | Bytes | Trace | Height | CU |
    | --- | --- | --- | --- | --- | --- |
    | transfer, Half-Life | 15 | 643 | 5 | 2 | 45,712 |
    | transfer, Transfer Fee | 15 | 643 | 5 | 2 | 48,568 |
    | transfer, Max Transaction | 14 | 610 | 5 | 2 | 49,885 |
    | transfer, Dust Guard | 14 | 610 | 5 | 2 | 46,621 |
    | `pool_before_swap` buy, Side Skew | 11 | 896 | 3 | 2 | 17,927 |
    | `pool_before_swap` buy, Size Tiers | 11 | 896 | 3 | 2 | 19,434 |
    | `pool_before_swap` buy, Launch Decay | 12 | 929 | 3 | 2 | 17,278 |
    | `pool_before_swap` raid buy, Raid | 15 | 1,028 | 3 | 2 | 19,596 |
    | `pool_after_swap` raid buy (mark, inbound), Raid | 15 | 1,028 | 3 | 2 | 29,738 |
    | delivery transfer stamping raid points, Raid | 17 | 709 | 5 | 2 | 62,586 |
    | `create_composite`, 3 modules | 17 | 841 | 15 | 3 | 123,389 |
    | `pool_before_swap` buy, composite of 3 | 14 | 995 | 3 | 2 | 23,182 |
    | transfer, composite of 3 (Half-Life module cuts) | 16 | 676 | 5 | 2 | 54,929 |
    | `settle_equip`, composite of 3 (2 quote payouts, 1 burn) | 21 | 820 | 22 | 3 | 208,107 |

    `MAX_MODULES` is provisionally 4 (`hookwars_common::MAX_MODULES`); the composite of 3 adds about
    5,000 CU on a pool callback over one module, so 4 fits with room; to confirm on the launchpad's
    real path.
15. **Token program changes (M3b):** a Pool slot may cut on the token side (`kind_may_cut` and
    `allowed_flags` include Pool, so composites with Fee modules fit a Pool slot, 08 2.8).
16. **Armory changes (M3b):** template ids may be sparse (any known template id, registered once);
    `EquipCtx.new_composite`; `create_composite`; `ArmoryParams.settle_bounty_bps` (the config's
    `reserved` shrinks by 2); `CheckPerformance` reads the ring from the pool account and lost its
    `observations` account.

## Integration notes (branch integ, 2026-10-09)

Changed by Hookwars: what the integration worker changed in the items program and the shared
crate, with the security review 2 findings each change answers.

- **Pool answers are always set** (H-A). `engine::pool` sets return data for every answer, the
  default too. The launchpad also reads no return data, or empty data from the item's program, as
  the default answer, and still refuses data from any other program.
- **Launch accounts are exempt token-side** (task 3). `token_before` answers nothing (no cut, no
  refusal, no stamp) when the source owner is the off-curve launch PDA of the mint: the deposit and
  the graduation top-up.
- **Stray vault tokens are burned at settlement** (M-A). `settle_equip` burns what the equip vault
  holds beyond the recorded `token_unsettled`, whenever the vault is passed. A donation can no
  longer keep `close_equip` from passing.
- **A missing destination holding is skipped** (L-C). That module keeps its counters (and its
  royalty and bounty stay unpaid), the other modules settle. Anyone can create the holding and
  settle again.
- **Raid season volume is net** (M-B). A raid buy counts in whole point units
  (`amount_in / point_unit_lamports * point_unit_lamports`). When raid points leave holders (sold
  back into the pool, or dropped when a holder empties into a non-holder), their volume
  (`points * point_unit_lamports / points_per_unit`) comes out of `outbound_volume_season`. A raid
  bought and sold back counts nothing. The inbound windows (`add_inbound`) stay gross. The war
  program reads `season_volume` unchanged, so it needs no code change; the score is now net raid
  volume. A Raid item with `points_per_unit` 0 stamps no points and cannot be netted, so the
  template's floor for that field should be at least 1 (owner decision).
- **Spy reads over at least `MIN_TWAP_SECS`** (L-A). Spy's extras gain the armory config (last);
  both window reads use `max(window_secs, min_twap_secs)` with `min_twap_secs` as the floor.
- **Shield** (L-B). The sell mark names who signed the sell (`authority`: the owner, or a delegate
  selling the owner's tokens), which is the swap's actor. The raid origin travels with tokens to
  every destination except the launch's own accounts, off-curve owners included.
- **Sell Burn's rate is in the manifest** (L-E). `max_cut_sell_bps` is the burn rate, so the
  armory's `max_pool_item_cut_bps` caps it at every equip, alone and summed in composites.
