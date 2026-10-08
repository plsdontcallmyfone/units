# Hookwars spec 04: templates (`hookwars_items`)

Status: specification, 2026-10-08. Nothing here is built. Follows `00-overview.md`; where they
differ, 00 wins and this file is wrong. Upstream references are to Bordrless at `43688f3`.

## 1. What this program is

`hookwars_items` (`<ITEMS_ID>`) is **one program implementing every template** (00 D-2). An item
is a record in the armory (`Item` at `["item", item_mint]`, 02) holding a `template_id` and a
`params` byte string. When a slot holding an item runs, the caller invokes `hookwars_items`, passes
the `Item` account as the **first extra account**, and the program dispatches on
`Item.template_id`. So one deployed program, one audit and one hook signer per caller serve every
template and every item.

Templates mint parameters, never code (00 rule 4): loot and forging produce new `params` for a
registered template, always inside that template's ceilings.

## 2. Common machinery

### 2.1 Callers and the hook signer check

Upstream pattern: the caller signs every callback with `["hook-authority", hook_program]` under the
caller's id, and the hook compares the signer with that constant
(`crates/bordrless-hook/src/lib.rs:476`, `hook_signer`; checked as
`#[account(signer, address = TOKEN_HOOK_SIGNER)]` in `programs/half_life/src/lib.rs:466-470` and
`programs/tax_hook/src/lib.rs:265-268`).

`hookwars_items` accepts exactly two signers, as constants:

| Constant | Value | Signs |
| --- | --- | --- |
| `TOKEN_ITEMS_SIGNER` | `["hook-authority", <ITEMS_ID>]` under `<TOKEN_ID>` | token callbacks (`before_transfer`, `before_mint`, `before_burn`, and `touch`, 01) |
| `LAUNCH_ITEMS_SIGNER` | `["hook-authority", <ITEMS_ID>]` under `<LAUNCH_ID>` | pool-item callbacks forwarded by the launchpad's pool hook (03) |

A token callback signed by anything but `TOKEN_ITEMS_SIGNER`, or a pool callback signed by anything
but `LAUNCH_ITEMS_SIGNER`, fails with `BadHookSigner`. Pool items are never the DEX's pool hook
directly; the DEX's own signer is refused, so a pool someone opens on the DEX with
`hook_program = <ITEMS_ID>` cannot drive an item.

The armory calls `validate_params`, `manifest`, `combine_params` and `init_equip`, signed by
`PDA(["armory"], <ARMORY_ID>)` (`ARMORY_SIGNER`, a constant here; the name is 02's to confirm).

### 2.2 Which item, and is it really equipped

Every callback receives, after the caller's prefix accounts (upstream: 5 token prefix accounts,
`lib.rs:36`; 5 pool prefix accounts, `lib.rs:39`):

| Extra | Account | Writable |
| --- | --- | --- |
| 0 | `Item` (owned by `<ARMORY_ID>`) | no |
| 1 | `EquipState` at `["equip", item, mint, slot]` under `<ITEMS_ID>` | yes |
| 2.. | the template's own extras (section 3) | per template |

Checks on every callback, in order:

1. signer (2.1);
2. `Item` is owned by `<ARMORY_ID>` and has the `Item` discriminator (`WrongItem`);
3. the `Mint` in the prefix (token callbacks: prefix index 1; pool callbacks: `base_mint`, read from
   the prefix `base_mint` account, which the launchpad must pass as the `Mint` account itself) is
   owned by `<TOKEN_ID>`, and its slot table entry at `args.slot` (01) names this `Item` key and
   `<ITEMS_ID>` (`NotEquipped`). This is the cheap equip proof: one read of an account the caller
   already passes;
4. `EquipState` is at its seeds for `(item, mint, slot)` (`WrongEquipState`);
5. `Item.template_id` is a known template and `Item.params` decodes for it (`BadParams`).

The caller decides which item runs; the item still re-checks against the mint, so a client cannot
substitute another item's extras.

### 2.3 Registries

Upstream keys a hook's registry by mint: `["bordrless-hook-accounts", mint]`
(`lib.rs:490`). Several slots of one mint all run `<ITEMS_ID>`, so that key collides. Items publish
one registry per slot:

```
["bordrless-hook-accounts", mint, [slot]]   under <ITEMS_ID>
```

written by `init_equip` (2.6). Its first two entries are always the `Item` key and the
`EquipState` PDA; the template's extras follow. 01 must resolve slot registries with this seed
(Interfaces, I-01.3).

### 2.4 Hook data ranges

00 4.4: each slot owns `data_offset .. data_offset + data_len` of the 64 bytes. Byte 0 of a range is
the template's layout tag; all zeros means never stamped. The args carry the full 64 bytes of each
side (upstream `TokenHookArgs.source_hook_data`, `destination_hook_data`, `lib.rs:198-201`); an item
reads only its range and answers a full 64-byte array in which every byte outside its range equals
the input. 01 enforces this (I-01.2). An item that needs no bytes declares `data_len = 0` and never
answers hook data.

Every template clears its range when the holding it describes is emptied (upstream rule,
`half_life/src/lib.rs:335-340`), so holdings stay closeable.

### 2.5 Cuts, the equip vault and royalties

**One delta per cutting item.** Upstream allows `MAX_DELTAS = 3` per answer
(`lib.rs:43`), and the launchpad already uses two on a buy (creator fee to index 6, holder fee to
index 7, `docs/hooks-v2.md` 5.4). So every item sends **all** of its cut as **one** delta into its
**equip vault**:

```
EquipVaultOwner = PDA(["equip-vault", equip_state], <ITEMS_ID>)
equip vault     = holding(cut_mint, EquipVaultOwner)          (token program)
```

`cut_mint` is the mint the cut is taken in: the token itself for token callbacks, the side's mint
for pool callbacks (bridged SOL on the quote side, the token on the base side).

`settle_equip(equip_state, cut_mint)`, permissionless, splits the vault's balance:

1. `royalty = floor(balance * Item.royalty_bps / 10_000)` to the royalty holding
   `holding(cut_mint, RoyaltyOwner)` (00 4.3, `["royalty", item]` under armory);
2. the rest to the template's **destination** (section 3: burn, war chest, partner war chest, a
   named collector);
3. a crank bounty of `MAX_CRANK_BOUNTY_BPS` at most, from the rest, to the sender.

The royalty is therefore exactly a share of what the hook collected, never an extra charge
(00 rule 1, 2.2 of ideas DESIGN). Royalty first or protocol share first (ideas DECISIONS H4): the
DEX applies Bordrless's protocol share to cuts on the swap itself (`hooks-v2.md` 3.1), before
anything reaches the equip vault, so the protocol share is always first.

Equip vault holdings are program-owned PDAs. Delta recipients are credited without calling the
mint's hook (`hooks-v2.md` 1.6), so the kit's on-curve rule for destinations does not apply to them;
`settle_equip` must not transfer the token itself into another program-owned holding of a kit token
with holder rewards on (Interfaces, I-05.4).

### 2.6 Entry points the armory calls

All take no user funds. The first three touch no account except the armory signer, so a client may
also simulate them as views.

| Instruction | Args | Returns (return data, Borsh) | Errors |
| --- | --- | --- | --- |
| `validate_params` | `template_id: u16`, `ceilings: Vec<u8>`, `params: Vec<u8>` | nothing | `BadParams`, `AboveCeiling` |
| `manifest` | `template_id`, `params` | `Manifest` (2.7) | `BadParams` |
| `combine_params` | `template_id`, `ceilings`, `a: Vec<u8>`, `b: Vec<u8>` | `Vec<u8>` (the forged params) | `NotForgeable`, `BadParams` |
| `init_equip` | `slot: u8`, `data_offset: u8`, `data_len: u8` | nothing | creates `EquipState` and the slot registry (2.3), lights the equip vault holding of every cut mint the template names |
| `close_equip` | `slot` | nothing | refuses while any equip vault holds a balance (`VaultNotSettled`) |

`ceilings` are the template's ceilings as stored in the armory's `Template` account (02), encoded
as the template's `Ceilings` struct. Items never store ceilings themselves, so a ceiling is set once,
at template registration, by whoever registers the template under 00 rule 3.

### 2.7 Params and manifest encoding

`Item.params`: byte 0 is the params layout version (`1`), then the Borsh encoding of the template's
`Params` struct. Length at most `PARAMS_MAX_LEN` (to set, 02).

```rust
pub struct Manifest {
    pub kind: u8,                 // 00 4.1 slot kind
    pub token_flags: u16,         // upstream token_flags the item subscribes to
    pub pool_callbacks: u8,       // bit 0 before_swap, bit 1 after_swap
    pub max_cut_bps_buy: u16,     // worst case, from params
    pub max_cut_bps_sell: u16,
    pub max_cut_bps_transfer: u16,
    pub max_discount_bps: u16,    // creator fee discount it may ask for (pool items)
    pub may_refuse: bool,
    pub may_burn: bool,
    pub data_bytes: u8,           // hook-data bytes its range needs
    pub reads_other_pools: u8,    // how many foreign Observations / Mint accounts it reads
}
```

The armory compares the manifest with the slot's bounds before equipping (02).

### 2.8 The pool-item answer

The launchpad forwards upstream `PoolHookArgs` (`lib.rs:206-247`) plus `slot` and the route
(03). A pool item answers `ItemPoolReturn`:

```rust
pub struct ItemPoolReturn {
    pub base: HookReturn,               // upstream struct, lib.rs:265: at most ONE delta, to extra
                                        // index of its equip vault on the side being cut; burn as allowed
    pub creator_fee_discount_bps: u16,  // share of the launch's creator fee to waive on this swap
}
```

The launchpad merges item answers with its own (03): discounts add up and are capped at the
creator fee; the item deltas and the launchpad's deltas together must fit `MAX_DELTAS`. Because the
launchpad uses two deltas on a buy, **at most one pool item may cut on a buy** while the launch
takes both creator and holder fees; `MAX_CUTTING_SLOTS` on the pool side is to measure and decide
(00 parameters; Interfaces I-03.4).

### 2.9 Marks: how a pool item reaches per-holder state

Pool callbacks cannot see holdings' hook data or write it; token callbacks can, but cannot see the
route. Both run inside the same DEX `swap` instruction, in a fixed order (upstream `hooks-v2.md`
3.1, "A buy"):

```
1. DEX -> launchpad before_swap -> pool items (pool half)     route known; writes a Mark
2. DEX moves the input, runs the curve
3. DEX -> launchpad after_swap -> pool items
4. DEX delivers the output: token transfer pool vault -> recipient holding
       token program -> items before_transfer (token half)     reads the Mark, stamps hook data,
                                                               clears the Mark
```

**Pool slot marks, token slot stamps.** The Mark lives in the token's `WarState` (05), one per slot:

```rust
pub struct Mark {                 // in WarState.marks[slot]; all zero = none
    pub kind: u8,                 // 1 raid buy, 2 shield origin
    pub clock_slot: u64,          // Clock::slot when written
    pub pool_swap_count: u64,     // PoolHookArgs.swap_count of the swap that wrote it
    pub recipient: Pubkey,        // PoolHookArgs.recipient
    pub rival: u8,                // index into the item's rival list
    pub quote_in: u64,            // the swap's quote input after before_swap cuts (lamports)
}
```

The token half accepts a Mark only when all hold: `clock_slot == Clock::slot`;
`args.source_owner == launch pool`; `args.destination_owner == mark.recipient`;
`mark.pool_swap_count` equals the swap counter the token half reads from the `Pool` account (03 must
state whether `swap_count` is incremented before or after delivery, I-03.3); and the Mark is for its
own slot. It then zeroes the Mark in its answer path (it writes `WarState` directly, as a writable
extra). A Mark that is not consumed is dead after its slot ends and is overwritten by the next one;
it can never stamp a different swap, because the swap counter and recipient must both match.

`WarState` becomes writable on every launch-pool swap that runs a marking item. Every buy already
write-locks the `Launch` account (`hooks-v2.md` 5.4), so trades of one token are already serialized;
this adds no new contention between different tokens.

## 3. Templates

| Id | Template | Slot kind | Callbacks |
| --- | --- | --- | --- |
| 0 | none | | invalid |
| 1 | Raid | Pool (with token half) | `before_swap` (pool); `before_transfer` (token) |
| 2 | Shield | Pool (with token half) | `before_swap`, `after_swap` (pool); `before_transfer` (token) |
| 3 | Wall | Defense | `before_transfer` |
| 4 | Spy | Pool | `before_swap`, `after_swap` |
| 5 | Treaty | Relation (pool side) | `before_swap` |
| 6 | Tribute | Relation (pool side) | `before_swap` |
| 7 | Half-Life | Fee | `before_transfer` |
| 8 | Transfer Fee | Fee | `before_transfer` |

Counter-strike is **not** a hook: it is a war-chest instruction in `hookwars_war` triggered by
time-weighted price (05). Siege, raze and bounties are likewise 05.

A Pool-kind item with a token half needs the token program to call it on token transfers too and to
give its slot a data range (Interfaces I-01.4). Slot kinds that 00 4.1 marks as "token transfers"
only never receive pool callbacks.

Forge rule used below. For a parameter whose stronger value is higher:

```
forge_up(x, y, cap)   = min(cap, max(x, y) + floor(min(x, y) * FORGE_GAIN_BPS / 10_000))
```

and for one whose stronger value is lower:

```
forge_down(x, y, floor_) = max(floor_, min(x, y) - floor(... same gain on the difference to floor_ ...))
                         = max(floor_, min(x, y) - floor((max(x, y) - floor_) * FORGE_GAIN_BPS / 10_000))
```

`FORGE_GAIN_BPS` is a new parameter (to set, O; not yet in 00's table). Identity parameters
(rival lists, partners, collectors) are merged as stated per template or make the pair
`NotForgeable`. Combining is deterministic: the same `(a, b)` in either order gives the same result
(lists are sorted and deduplicated by key).

### 3.1 Raid (id 1)

Buyers who reach us by **selling a listed rival** get part of the creator fee waived, optionally pay
a toll to this item, and earn raid points and loot tickets in their holding.

**Params**

```rust
pub struct RaidParams {
    pub rivals: Vec<Pubkey>,      // rival mints, at most RAID_MAX_RIVALS; empty = any token
    pub discount_bps: u16,        // share of the creator fee waived on a raid buy
    pub toll_bps: u16,            // this item's own cut of the raid buy's quote input
    pub points_per_unit: u16,     // points per POINT_UNIT_LAMPORTS of quote_in
}
```

| Ceiling | Bounds |
| --- | --- |
| `RAID_MAX_RIVALS` | length of `rivals` |
| `RAID_MAX_DISCOUNT_BPS` | `discount_bps` (at most 10,000: a full waiver of the creator fee) |
| `RAID_MAX_TOLL_BPS` | `toll_bps` |
| `RAID_MAX_POINTS_PER_UNIT` | `points_per_unit` |

`POINT_UNIT_LAMPORTS` is a new parameter (to set, O). Loot tickets use 00's `LOOT_MIN_RAID_LAMPORTS`.

**Extras** (after Item, EquipState): `WarState` of the mint (w); the toll equip vault, a bridged-SOL
holding of `EquipVaultOwner` (w); the launch `Pool` (r, for the swap counter).

**Hook data range** (`RAID_DATA_BYTES` = 13):

| Bytes | Field |
| --- | --- |
| 0 | tag `0x01` |
| 1..3 | `season: u16`, the season the points belong to |
| 3..11 | `points: u64` |
| 11..13 | `tickets: u16`, loot tickets not yet rolled |

**`before_swap` (pool half)**

```
if args.direction != BUY: answer nothing
route = args.route (03 RouteContext); i = route.index
raid  = i >= 1
        and route.hops[i].input_mint == bridged SOL
        and route.hops[i-1].output_mint == bridged SOL
        and (rivals is empty or route.hops[i-1].input_mint in rivals)
        and route.hops[i-1].input_mint != our mint
if not raid: answer nothing
quote_in = args.amount_in
toll = toll_bps > 0 ? fee_amount(quote_in, toll_bps) : 0          // rounds up, upstream fee_amount
WarState.raid_inflow_add(rival = route.hops[i-1].input_mint, quote_in)   // 05 window accumulator
WarState.marks[slot] = Mark { kind: 1, clock_slot, pool_swap_count: args.swap_count,
                              recipient: args.recipient, rival: idx, quote_in: quote_in - toll }
answer ItemPoolReturn {
    base: { deltas: toll > 0 ? [Delta { amount: toll, account: <toll vault index> }] : [] },
    creator_fee_discount_bps: discount_bps }
```

**`before_transfer` (token half)**

```
if source == destination: nothing
mark = WarState.marks[slot]
if mark accepted (2.9):
    pts = (mark.quote_in / POINT_UNIT_LAMPORTS) * points_per_unit         // checked
    tix = mark.quote_in >= LOOT_MIN_RAID_LAMPORTS and WarState.war_active ? 1 : 0
    dest = read range(destination); if dest.season != WarState.season: dest = zero
    dest.points  = dest.points.saturating_add(pts)
    dest.tickets = dest.tickets.saturating_add(tix)
    zero WarState.marks[slot]
else if transfer between two holders (neither is the pool nor the launch nor a program vault):
    // points travel with tokens, pro rata, like Half-Life's age
    src = range(source) (season-checked)
    moved_pts = floor(src.points * amount / source_balance); same for tickets
    src -= moved; dest += moved
else if destination is the launch pool (a sell): 
    src.points  -= floor(src.points * amount / source_balance)   // points leave with the tokens
    src.tickets -= floor(src.tickets * amount / source_balance)
clear src range if source_balance == amount
answer source/destination hook data (own range only); no deltas
```

Raid points are spent by `claim_bounty` and tickets by `roll` in 05, both through `touch` (01).

**Abuse**

- Faking a route: the route is built by the DEX's multi-hop instruction, not the client, and hop
  `i`'s input must be exactly hop `i-1`'s delivered output (I-03.2). A raider cannot top up with
  outside SOL or sell dust X and claim a large raid.
- Wash raiding with one's own X: costs X's fees, our fees after discount, slippage on both pools,
  and the toll. Points are linear in SOL routed, so splitting gains nothing; loot needs
  `LOOT_MIN_RAID_LAMPORTS` per buy and an active war.
- Selling right after a raid destroys the points pro rata, so raid-and-dump keeps nothing.
- Sending points to a fresh wallet moves them, it does not copy them.

**Forge:** `rivals` = sorted union, refused if above `RAID_MAX_RIVALS` (`NotForgeable`); empty list
(any token) only with an empty list. `discount_bps`, `toll_bps`, `points_per_unit`: `forge_up`.

**Manifest:** kind Pool; token flags `BEFORE_TRANSFER | WRITES_HOOK_DATA`; pool callbacks
before_swap; `max_cut_bps_buy = toll_bps`; `max_discount_bps = discount_bps`; may_refuse false;
data_bytes 13.

### 3.2 Shield (id 2)

Holders whose tokens arrived **from a listed rival's route** within a window pay an extra cut when
they sell: it taxes hit-and-run raiders who rotate in and straight back out.

**Params**

```rust
pub struct ShieldParams {
    pub rivals: Vec<Pubkey>,      // at most SHIELD_MAX_RIVALS; empty = any token
    pub sell_cut_bps: u16,        // extra cut on a sell by a marked holder, in SOL
    pub window_secs: u32,         // how long an origin counts
    pub only_under_siege: bool,   // apply only while WarState.under_siege_until > now
}
```

Ceilings: `SHIELD_MAX_RIVALS`, `SHIELD_MAX_SELL_CUT_BPS`, `SHIELD_MAX_WINDOW_SECS`.

**Extras:** `WarState` (w); the equip vault in bridged SOL (w); the launch `Pool` (r); the actor's
holding of our mint (r), registry seed `["holding", mint, actor]` under `<TOKEN_ID>` (upstream
`Seed::Account` on the prefix `actor`).

**Hook data range** (`SHIELD_DATA_BYTES` = 10):

| Bytes | Field |
| --- | --- |
| 0 | tag `0x02` |
| 1 | `origin: u8`, index into `rivals` + 1 (0 = none) |
| 2..10 | `origin_at: i64`, weighted arrival time of the marked tokens |

**Stamping the origin.** `before_swap` on a buy runs the same route test as Raid and, if the buyer
came from a listed rival, writes `Mark { kind: 2, ... }` into `WarState.marks[slot]`. The token half
on delivery stamps `origin` and blends `origin_at` by weight with what the holding already had
(upstream `blend`, `half_life/src/lib.rs:127-134`). A plain buy into a marked holding blends the
time but keeps the origin; a transfer between holders carries origin and time like Half-Life.

**`after_swap` on a sell** (the SOL output is known here, `hooks-v2.md` 5.4):

```
h = range(actor holding)          // read-only account
marked = h.origin != 0 and now - h.origin_at < window_secs
         and (!only_under_siege or WarState.under_siege_until > now)
if marked: cut = fee_amount(args.amount_out, sell_cut_bps)   // from the SOL output
           answer { deltas: [Delta { amount: cut, account: <equip vault> }] }
```

`before_swap` on a sell answers nothing. Destination after `settle_equip`: our war chest (05).

**Abuse:** a raider can send the tokens to a fresh wallet before selling; origin travels with
tokens, so the fresh wallet is marked too. A raider who waits `window_secs` pays nothing, which is
the point (it defends against hit-and-run, not against holders).

**Forge:** rivals union as Raid; `sell_cut_bps`, `window_secs`: `forge_up`; `only_under_siege`: the
pair must agree, else `NotForgeable`.

**Manifest:** kind Pool; `max_cut_bps_sell = sell_cut_bps`; data_bytes 10; may_refuse false.

### 3.3 Wall (id 3)

A temporary max wallet while the token is **under siege** (05 sets `WarState.under_siege_until`
when another token's war chest spot-buys us). It slows accumulation of our supply by a rival chest
and by whales riding the siege.

**Params**

```rust
pub struct WallParams {
    pub max_wallet_bps: u16,      // of supply
}
```

Ceiling: `WALL_MIN_MAX_WALLET_BPS` (the tightest wall allowed; a stronger wall is a lower value).

**Extras:** `WarState` (r).

**`before_transfer`**

```
if WarState.under_siege_until <= now: answer nothing
if destination_owner in { launch pool, launch PDA, our war chest, any equip vault owner }: nothing
cap   = supply * max_wallet_bps / 10_000                 // u128, as tax_hook, lib.rs:151
after = destination_balance + amount
require(after <= cap, WallHolds)
```

No hook data, no cuts. It refuses, so `may_refuse = true`; a sell never fails on it (the
destination is the pool).

**Abuse:** a siege cannot last longer than 05 allows (`under_siege_until` is bounded there), so a
Wall cannot freeze buys indefinitely. The siege itself is not stopped by the Wall when the
attacker's chest is under the cap; the Wall bounds each wallet, not the sum.

**Forge:** `forge_down(a, b, WALL_MIN_MAX_WALLET_BPS)`.

**Manifest:** kind Defense; `BEFORE_TRANSFER`; may_refuse true; data_bytes 0.

### 3.4 Spy (id 4)

Our fees move with a **rival's time-weighted price**. Reads the rival pool's `Observations`
(03), never its spot price (00 rule 7).

**Params**

```rust
pub struct SpyParams {
    pub rival_pool: Pubkey,
    pub window_secs: u32,         // >= MIN_TWAP_SECS
    pub trigger_bps: u16,         // change between the last two windows that arms it
    pub mode: u8,                 // 1 Rivalry, 2 Momentum
    pub effect_bps: u16,          // Rivalry: sell cut; Momentum: creator fee discount on buys
}
```

Ceilings: `SPY_MAX_EFFECT_BPS`, `SPY_MIN_TRIGGER_BPS`, `SPY_MAX_WINDOW_SECS`; `window_secs >=
MIN_TWAP_SECS` always.

**Extras:** the rival pool's `Observations` at `["obs", rival_pool]` under `<SWAP_ID>` (r); the
equip vault in bridged SOL (w).

**Logic** (`twap(a, b)` is 03's helper over the ring):

```
now_w  = twap(now - window, now)
prev_w = twap(now - 2*window, now - window)
if ring cannot cover 2*window: answer nothing          // young pool or short ring
change_bps = (now_w - prev_w) * 10_000 / prev_w        // signed
Rivalry (rival up):    if change_bps >= trigger and SELL: after_swap cut = fee_amount(out, effect_bps)
Momentum (rival down): if change_bps <= -trigger and BUY: before_swap discount = effect_bps
```

**Abuse:** moving the rival's time-weighted price over `window_secs` costs holding a manipulated
price for that long on the rival's own pool, paying its fees both ways. The armory refuses a
`window_secs` below `MIN_TWAP_SECS`.

**Forge:** same `rival_pool` and `mode`, else `NotForgeable`; `effect_bps`: `forge_up`;
`trigger_bps`: `forge_down(.., SPY_MIN_TRIGGER_BPS)`; `window_secs`: the shorter of the two, never
below `MIN_TWAP_SECS`.

**Manifest:** kind Pool; `max_cut_bps_sell` or `max_discount_bps` = `effect_bps` by mode;
`reads_other_pools = 1`.

### 3.5 Treaty (id 5)

Two tokens pay each other's holders from their buys. **One** Treaty item is equipped by **both**
mints; it does nothing until both have equipped it.

**Params**

```rust
pub struct TreatyParams {
    pub mint_a: Pubkey,           // sorted: mint_a < mint_b
    pub mint_b: Pubkey,
    pub a_to_b_bps: u16,          // cut of A's buys (SOL input) to B
    pub b_to_a_bps: u16,
}
```

Ceiling: `TREATY_MAX_BPS` on each side.

**Extras:** the partner's `Mint` (r); the equip vault in bridged SOL (w).

**Verifying the partner cheaply:** the partner's `Mint` is passed read-only; the item scans its slot
table (at most `MAX_SLOTS` entries, 01) for an entry naming this `Item` key and `<ITEMS_ID>`. Found:
active. Not found: answer nothing. No extra account per treaty, no keeper, and either side leaves
by unequipping under its own rule and notice.

**`before_swap`, buy:** `cut = fee_amount(amount_in, my_side_bps)`; one delta to the equip vault.
Destination after `settle_equip`: the **partner's war chest**, tagged as treaty inflow, which 05
passes to the partner's kit through `kit.share` (streamed over upstream's hour, so a buyer cannot
sandwich it; `hooks-v2.md` 4.10). The item never deposits into the kit's reward vault directly:
upstream distributes direct deposits at once (`hooks-v2.md` 4.7), which a buy just before could
capture.

**Abuse:** a treaty minted without consent is inert: the partner never equips it. A side that stops
paying has unequipped, so the other side's item sees no partner and stops too.

**Forge:** `NotForgeable` (identity).

**Manifest:** kind Relation; `max_cut_bps_buy` = the larger side; `reads_other_pools = 1`
(a foreign `Mint`).

### 3.6 Tribute (id 6)

A one-way Treaty: `payer_mint` pays `bps` of its buys to `receiver_mint`'s holders. Active only when
both mints equip it (the receiver accepts). Anything granted back (for example a Raid discount for
the payer's holders) is a separate item the receiver equips: Tribute does not bundle it.

```rust
pub struct TributeParams { pub payer_mint: Pubkey, pub receiver_mint: Pubkey, pub bps: u16 }
```

Ceiling `TRIBUTE_MAX_BPS`. Logic as Treaty with one side zero; on the receiver's swaps it answers
nothing. **Forge:** `NotForgeable`.

### 3.7 Half-Life (id 7)

Upstream `half_life` as a template (`programs/half_life/README.md`; logic
`programs/half_life/src/lib.rs:276-356`), with its constants made parameters:

```rust
pub struct HalfLifeParams {
    pub max_fee_ppm: u32,         // upstream MAX_FEE_PPM, lib.rs:75
    pub half_life_secs: u32,      // upstream HALF_LIFE_SECS, lib.rs:77
    pub zero_after_halvings: u8,  // upstream ZERO_AFTER_HALVINGS, lib.rs:79
}
```

Ceilings: `HL_MAX_FEE_PPM`, `HL_MIN_HALF_LIFE_SECS`, `HL_MAX_HALF_LIFE_SECS`, `HL_MAX_HALVINGS`.

**Hook data range** (`HL_DATA_BYTES` = 9): byte 0 tag `0x07`; bytes 1..9 `since: i64`. (Upstream
used a 3-byte magic and 11 bytes, `lib.rs:83-121`; a range tag replaces the magic.)

**`before_transfer`:** upstream's function with three changes:

1. `fee_ppm(age)` uses the params (`lib.rs:87-99`);
2. exempt owners: the launch PDA, the launch pool, our war chest, every equip vault owner of this
   mint and the royalty owners (upstream exempted the launch, the pool and the furnace,
   `lib.rs:292-300`);
3. the fee is one delta to the item's equip vault in the token, not to a furnace;
   `settle_equip` burns the rest after the royalty (destination: burn). The "furnace not lit" rule
   becomes "equip vault not lit": `init_equip` lights it, so fees are never refused after equip.

**Forge:** `max_fee_ppm`: `forge_up`; `half_life_secs`: `forge_up` (a slower decay is stronger),
capped by `HL_MAX_HALF_LIFE_SECS`; `zero_after_halvings`: `forge_up`.

**Manifest:** kind Fee; `BEFORE_TRANSFER | TRANSFER_RETURNS_DELTA | WRITES_HOOK_DATA` (upstream
`FLAGS`, `lib.rs:68`); `max_cut_bps_transfer = ceil(max_fee_ppm / 100)`; may_refuse false.

### 3.8 Transfer Fee (id 8)

Upstream `tax_hook` as a template (`programs/tax_hook/src/lib.rs:127-165`).

```rust
pub struct TransferFeeParams {
    pub fee_bps: u16,             // upstream fee_bps, bounded 2,000 there (lib.rs:58-60)
    pub max_wallet_bps: u16,      // 0 = off
    pub collector: Pubkey,        // receives the rest after royalty
}
```

Ceilings: `TF_MAX_FEE_BPS`, `TF_MIN_MAX_WALLET_BPS`.

**`before_transfer`:** upstream's logic: the fee as one delta to the equip vault (not straight to
the collector, so the royalty can be taken), the wallet cap as upstream (`lib.rs:147-154`), exempt
owners as Half-Life plus the collector. Destination after `settle_equip`: the collector's holding.

**Forge:** same `collector`, else `NotForgeable`; `fee_bps`: `forge_up`; `max_wallet_bps`:
`forge_down(.., TF_MIN_MAX_WALLET_BPS)` when both are on.

**Manifest:** kind Fee; `max_cut_bps_transfer = fee_bps`; may_refuse = `max_wallet_bps > 0`.

## 4. Accounts of `hookwars_items`

| Account | Seeds | Fields |
| --- | --- | --- |
| `EquipState` | `["equip", item, mint, [slot]]` | `item`, `mint`, `slot`, `template_id`, `data_offset`, `data_len`, `equipped_at`, `runs: u64`, `collected: [u64; 2]` (token side, quote side), `settled: [u64; 2]`, `bump`, `vault_bump`, reserved |
| `EquipVaultOwner` | `["equip-vault", equip_state]` | none (system-owned; signs `settle_equip` transfers and burns) |
| slot registry | `["bordrless-hook-accounts", mint, [slot]]` | upstream `HookAccountList` (`lib.rs:533`) |

`runs` and `collected` are what the armory and the site read for an item's level and royalties (02,
06); they are counters, never promises.

## 5. Events

`ItemRan { item, mint, slot, template_id, cut, cut_mint, discount_bps, marked: bool }` on each
callback that does something; `EquipSettled { item, mint, cut_mint, royalty, destination, amount,
bounty }`; `RaidStamped { mint, owner, points, tickets, season }`; `ShieldTaken { mint, owner, cut }`.
Emitted by self-CPI (`emit_cpi!`), as upstream.

## 6. Errors

`BadHookSigner`, `WrongItem`, `NotEquipped`, `WrongEquipState`, `BadParams`, `AboveCeiling`,
`NotForgeable`, `WallHolds`, `WalletTooLarge`, `VaultNotSettled`, `UnknownTemplate`,
`MarkMismatch` (never returned to users; a mismatched Mark is ignored, not an error), `Overflow`.

## 7. Interfaces

### Needed from 01 (token slots)

- **I-01.1** `TokenHookArgs` gains `slot: u8` and the slot's `data_offset`, `data_len` (or the item
  reads them from the `Mint` it is passed; either works, 01 chooses).
- **I-01.2** The token program passes the full 64 bytes of each side and rejects an answer that
  changes bytes outside the item's range.
- **I-01.3** Per-slot extras resolved from `["bordrless-hook-accounts", mint, [slot]]` under the
  slot's program.
- **I-01.4** A `Pool`-kind slot also receives token callbacks (Raid and Shield token halves) and has
  a data range; its token answers carry hook data only, no deltas.
- **I-01.5** The `Mint` slot table is readable at a stable offset so an item can find
  `slots[i].item` and `slots[i].program` without the token crate's full deserializer (Treaty reads a
  foreign `Mint`).
- **I-01.6** `touch` calls the slot's item with `TokenOp::Touch` (new) and accepts hook data for the
  touched holding only; 05's `claim_bounty` and `roll` use it to clear Raid points and tickets.
- **I-01.7** Delta recipients may be holdings owned by `EquipVaultOwner` PDAs (credited without a
  hook call, upstream 1.6).

### Needed from 02 (armory)

- `Item { template_id: u16, params: Vec<u8>, royalty_bps: u16, owner, author, item_mint, ... }`.
- `Template { template_id, program == <ITEMS_ID>, ceilings: Vec<u8>, code_hash, ... }`.
- `ARMORY_SIGNER` seeds; the armory calls `init_equip` on equip and `close_equip` on unequip, and
  checks `Manifest` against slot bounds.
- `FORGE_GAIN_BPS`, `PARAMS_MAX_LEN` and every template ceiling named in section 3 live in
  `Template.ceilings` or 00's parameter table.

### Needed from 03 (DEX and launch)

- **I-03.1** `RouteContext { hops: Vec<RouteHop { pool, input_mint, output_mint }>, index: u8 }`
  passed to the launchpad in `PoolHookArgs.hook_data` and forwarded to pool items.
- **I-03.2** Hop `i`'s input amount equals hop `i-1`'s delivered output; no outside top-up.
- **I-03.3** Whether `Pool.swap_count` changes before or after output delivery, so the Mark check
  (2.9) compares the right value.
- **I-03.4** The launchpad forwards to pool items with `LAUNCH_ITEMS_SIGNER`, passes `slot`, the
  base `Mint` account, and merges `ItemPoolReturn` (discounts capped at the creator fee; total
  deltas within `MAX_DELTAS`).
- **I-03.5** `Observations` at `["obs", pool]` and a `twap(from, to)` helper (crate function) that
  returns none when the ring does not cover the range.

### Needed from 05 (war)

- `WarState` fields: `season: u16`, `war_active: bool`, `under_siege_until: i64`,
  `marks: [Mark; MAX_SLOTS]`, `raid_inflow` accumulators per rival mint with a window (written by
  Raid via an instruction-free field update, since items write `WarState` as a writable extra:
  `WarState` must be writable by `<ITEMS_ID>` for the `marks` and `raid_inflow` fields only, or be
  owned by `<ITEMS_ID>`; see conflict C1).
- War chest destination holdings for Shield, Spy, Raid toll and Treaty/Tribute inflow, with a
  treaty inflow tag and a crank that calls `kit.share`.

## 8. Conflicts and assumptions for the integrator

- **C1 `WarState` ownership.** Only the owning program can write an account. Items must write
  `marks` and `raid_inflow` during swaps. Options: `WarState` split in two (`WarMarks` at
  `["war-marks", mint]` owned by `<ITEMS_ID>`, read by 05), or items CPI into `hookwars_war` (one
  more call level on a buy: DEX 1, launchpad 2, item 3, war 4; inside 5 but to measure).
  Recommendation: `WarMarks` owned by items.
- **C2 Pool-side cutting slots.** With the launchpad's two deltas on a buy, only one pool item can
  cut per buy; `MAX_CUTTING_SLOTS` must be stated per side, not as one number.
- **C3 Kit tokens and war chests.** With kit holder rewards on, a transfer of that token to a
  program-owned owner is refused (`hooks-v2.md` 4.5, `DestinationNotAllowed`). A siege buying a
  kit token with rewards on into a war chest PDA fails. 05 must restrict sieges to targets without
  that kit rule, or the kit rule must exempt war chests.
- **C4 Hook data ranges.** Raid 13 + Shield 10 + Half-Life 9 = 32 bytes; the kit (Locked slot)
  already uses 32 (`hooks-v2.md` 4.4). A kit token with all three items fits 64 exactly; any more
  does not.
- **C5 New parameters** not in 00's table: `FORGE_GAIN_BPS`, `POINT_UNIT_LAMPORTS`,
  `PARAMS_MAX_LEN`, and every per-template ceiling in section 3.
- **C6 `TokenOp::Touch`** is a new upstream enum value (01).
