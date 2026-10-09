# units spec 08: the arsenal and composite items

Status: specification, 2026-10-09. Nothing here is built. The product is named **units**; code
names stay `hookwars_*` until the global rename (CLAUDE.md). This part extends 04 (templates) and
02 (armory). Where it needs something the earlier parts do not provide, it says so in section 7
("What 00 and the other parts must add"); until 00 adopts those, they are proposals.

Every figure is **to set** (a named parameter in section 7, decided by the owner or measured) or
**to measure** (a LiteSVM test in 07). Spot only: every effect is a cut taken from a trade or a
transfer, a refusal, a waiver of the launch's own fees, a burn on a swap, bytes in a holding, or a
payout from a program-owned vault by a separate instruction. No bets, no outcome payouts, no lending,
no leverage. Naming follows 00 4.5.

## 1. What hooks can and cannot do (the box every template lives in)

From 01, 03, 04 and the M1 and M2 implementation notes:

| Capability | Code | Where | Limits |
| --- | --- | --- | --- |
| Token cut | `TC` | `before_transfer` (token side) | one delta per item into its equip vault (R1); slot `max_cut_bps`; at most `MAX_CUTTING_SLOTS` cutting slots |
| Pool cut | `PC` | `pool_before_swap` / `pool_after_swap` | `ItemPoolAnswer.cut` in the quote; all items on a side merged into one `PoolCuts` delta (R2); armory `max_pool_item_cut_bps` |
| Discount | `DS` | pool callbacks | `ItemPoolAnswer.discount_bps`, waives only the launch's creator and holder fees on that side (R6); armory `max_pool_item_discount_bps`; summed and capped by the launchpad |
| LP fee | none | | items never set it (R6) |
| Refuse | `RF` | any callback | declarative only (a failed CPI cannot be caught, 01 3.6); slot `may_refuse` |
| Burn | `BN` | pool callbacks only | `ItemPoolAnswer.burn` in base; a token callback never answers a burn (00, hooks-v2 0.6); needs a slot bound that allows it (R21, section 7) |
| Hook data | `HD` | token callbacks | own range only, `data_len - 1` bytes after the epoch byte (00 4.4); 32 bytes for items beside the kit, 64 without |
| Own market reads | `OR` | any | the token's own launch pool `Observations` (03 3.1) and `Launch`, read-only registry extras |
| Foreign reads | `FR` | any | other mints' `Observations`, `Launch`, `Mint`, `WarState`, or a holder's holding of another mint (R22); count toward armory `max_item_reads` |
| Raid mark | `WM` | `pool_after_swap` | writes `RaidLedger.mark` (04 2.9) |
| War touch | `WT` | `on_touch` from `WAR_SIGNER` | spends raid points or tickets (04 2.10) |
| Self touch | `ST` | `on_touch` from the holding's owner | social payloads (R23) |
| Clock | `CK` | any | `Clock::unix_timestamp`, `Clock::slot` |
| Payout | `PO` | separate items instruction | from an equip vault or `PoolCuts` through `transfer_from_protocol` (R16, R24); never inside a callback |

What a token callback knows: owners, balances, amount, supply, signer, delegate flag, its own range
of both holdings (01 `TokenSlotArgs`). Whether a transfer is a buy or a sell is known by comparing an
owner with the launch pool (read from the `Launch` extra, as Half-Life does). What a pool callback
knows: direction, amounts, reserves, the trader (`actor`), the recipient, the route (R5), and the
launch's fees on that side (03 `ItemPoolContext`). A pool callback reads a holder's bytes only by
reading the holding account (R22), never by writing it.

## 2. Composite items

### 2.1 What a composite is

A **composite** is one item whose behaviour is an ordered list of **modules**. Each module is a
registered template (marked `composable`, section 7) with its own parameter fields. All modules run
inside the **same** `hookwars_items` callback, as internal function calls, so a composite costs one
slot, one CPI level and one delta, whatever the number of modules.

Template id `41` is `Composite`. A composite is an `Item` like any other: one owner, one royalty,
one level, one equip.

### 2.2 Account and parameter layout

The `Item.params` array (`PARAM_FIELDS` = 11 `u32`, M2 note 1) is too small for a module list, so a
composite keeps its modules in its own account:

```rust
/// ["composite", item] under <ARMORY_ID>. Written once by `fuse` or `mint_composite`; changed only by `forge`.
pub struct CompositeItem {
    pub version: u8,
    pub bump: u8,
    pub item: Pubkey,
    pub modules: Vec<Module>,          // 1..=MAX_MODULES, executed in this order
    pub provenance: Vec<Pubkey>,       // items fused into it (display only, section 2.6)
}
pub struct Module {
    pub template_id: u16,              // a composable template (never 41, never War orders, never Locked)
    pub params: [u32; PARAM_FIELDS],   // the template's own fields, same floors, ceilings and rules
    pub target_start: u8,              // its targets are EquipConfig.targets[start .. start + count]
    pub target_count: u8,
    pub data_bytes: u8,                // its sub-range length (from the template's manifest)
    pub reads_module: u8,              // 0xFF none; else the index of an EARLIER module whose bytes it may read
}
```

`Item.template_id = 41`, `Item.params[0]` = module count, `Item.params[1]` = layout version, the
rest 0. `Item.manifest` is the **combined manifest** (2.4), computed at creation.

A module is 2 + 44 + 4 = 50 bytes. `MAX_MODULES` is **to measure** (compute of the worst
callback, registry size, transaction bytes); the account is sized for `MAX_MODULES` at creation.

### 2.3 Executing a callback

For every callback the composite subscribes to (the OR of its modules' flags):

```
answers = []
for i, m in modules (in order):
    if m's template does not subscribe to this callback: continue
    sub = this module's sub-range of the item's range (offsets in module order, 2.5)
    peer = sub-range of modules[m.reads_module] if set (read-only)
    a_i = run_template(m.template_id, m.params, targets[m.slice], args, sub, peer)
    if a_i refuses: fail the whole callback          // refusals are AND: any refusal refuses
    answers.push(a_i)
cut      = sum of a_i.cut          (each a_i computed on the SAME base amount, never compounded)
discount = sum of a_i.discount_bps
burn     = sum of a_i.burn
bytes    = concatenation of each module's new sub-range
check cut <= manifest max for this side, discount <= manifest max, burn allowed
record per-module shares: EquipState.module_owed_token[i] += a_i.cut (token side)
                          EquipState.module_owed_pool[i]  += a_i.cut (pool side)
answer ONE delta (token side, into the equip vault) or ONE ItemPoolAnswer (pool side)
```

Every module sees the pre-state, as slots do (01 3.2): a module never sees another module's cut.
`reads_module` is the only coupling: a later module may read (never write) an earlier module's
bytes, for example a Patience module reading a Streak module's count.

Raid marks: at most one module family writes the mark (2.7); if two modules would mark, they must
share the same target slice, and the composite writes the mark once.

War touches: routed to the one module that answers `WT` (at most one, 2.7). Self touches: the
payload carries the module index.

### 2.4 The combined manifest

Computed by the armory at creation and stored in `Item.manifest`; `check_fits` (02 6.1) checks it
against the slot like any item's.

| Manifest field | Combined as |
| --- | --- |
| `kind` | the composite's host kind (2.8) |
| `token_flags`, `pool_flags` | OR |
| `max_cut_buy_bps`, `max_cut_sell_bps`, `max_cut_transfer_bps` | sum of modules, capped at 10,000 |
| `max_discount_bps` | sum, capped at 10,000 |
| `may_refuse`, `may_burn` | OR |
| `data_bytes` | sum of module `data_bytes` |
| `reads_other_pools` | count of distinct foreign accounts after deduplicating identical reads |

A composite therefore advertises exactly what its worst case can do; the slot's bounds stay the
promise to holders.

### 2.5 Hook data inside a composite

The composite's range is one slot range (one epoch byte, 00 4.4). Inside it, modules get
consecutive sub-ranges in module order, each laid out exactly as the module's standalone template
(04 section 3, its own layout tag first). Total bytes = sum of module `data_bytes`, which must fit
the slot's `data_len - 1` (`CompositeBytes`). A re-equip changes the epoch byte, so every module's
bytes read as empty together.

### 2.6 Royalty, authorship and provenance

**One royalty, for the composite's owner.** A composite has one `royalty_bps` (at most
`MAX_ROYALTY_BPS`) paid by `settle_equip` on everything its modules collect, exactly as a single
item (04 2.5).

**No royalty to module or component authors.** Justification:
1. Templates are registered by the protocol (02 4.1), so a module has no third-party author to pay.
2. A composite made by `fuse` **burns** its component items; their owners were paid by whoever
   bought or forged them, on the open market, and give up the item when it is fused.
3. Splitting one cut among N authors makes `settle_equip` O(N) and lets royalties stack beyond
   `MAX_ROYALTY_BPS`; one royalty keeps both bounded.

`CompositeItem.provenance` lists the fused items so the site can show where a composite came from.

### 2.7 What cannot be composed

| Not composable | Why |
| --- | --- |
| Composite inside a composite | one level only; keeps execution and settlement flat |
| War orders (9) | War kind has no callbacks (00 4.1) |
| Locked slot programs (kit, upstream Half-Life or tax_hook) | not items |
| Two modules answering war touches (`WT`) | a war touch must reach exactly one module |
| Two marking modules (`WM`) with different targets | one mark per swap (04 2.9) |
| A refusing module in a composite meant for a slot without `may_refuse` | caught by `check_fits` on the combined manifest |
| A burning module where no slot bound allows a burn | R21 |
| Modules whose bytes, reads or targets sum past the limits | `CompositeBytes`, `TooManyReads`, `BadTargets` |
| Token-side cutting modules on a mint whose kit has holder rewards | section 3.3 |

### 2.8 Host kinds

A composite is placed in a slot of kind K; every module must be hostable in K:

| Slot kind | May host modules of kind |
| --- | --- |
| Fee | Fee |
| Reward | Reward, Fee |
| Defense | Defense |
| Relation | Relation (token half and pool half) |
| Pool | Pool, and the token halves of Pool modules; plus Fee, Defense and Reward modules (01: a Pool slot is also called on token transfers with a data range) |

A Pool slot is therefore the general host for mixed composites, within its bounds.

### 2.9 Making, forging and equipping composites

| Instruction | Who | Effect |
| --- | --- | --- |
| `fuse(items: Vec<Pubkey>, royalty_bps)` | the holder of every listed item | burns the items (none may be equipped or hold unclaimed royalties; 02 10 order applies), creates one composite item whose modules are their templates and params in the given order; the caller is the new item's author |
| `mint_composite(preset_id, params)` | as `create_item` (02), when the preset's templates are all `open_authoring` | creates a composite from a registered **preset** (section 5) |
| `mint_loot` (02) | war only | may mint a composite preset if the season loot table lists it |
| `forge(a, b)` | holder of both | both must have the **same module sequence** (template ids in order); each module's fields combine by its template's forge rule (04 2.7); level = max + 1, capped at the minimum `max_level` of its modules; both burned, one minted |

There is no `unfuse`: fusing is one-way, so components cannot be duplicated or farmed.

At creation (`fuse`, `mint_composite`) the armory runs 2.10. At equip it runs `check_fits` with the
combined manifest, the kit rule (3.3) and the target count.

### 2.10 Validation at creation

In order, each with its error:

1. `1 <= modules.len() <= MAX_MODULES` (`TooManyModules`).
2. Every template exists, is `composable`, is not 41 or 9 (`NotComposable`).
3. All modules hostable in one kind (2.8) (`KindMismatch`); that kind is the composite's.
4. Each module's params pass the template's per-field floors and ceilings and `validate_params`
   (`BadParams`).
5. Capability conflicts (matrix 3.2) (`ModuleConflict`).
6. `sum data_bytes <= ITEM_DATA_MAX` where `ITEM_DATA_MAX` is the largest item range a slot table
   can give (`CompositeBytes`); the exact fit is checked again at equip.
7. Target slices are disjoint and `sum target_count <= MAX_ITEM_TARGETS` (`BadTargets`).
8. `reads_module` points only to an earlier module whose template exports bytes (`BadModuleRead`).

### 2.11 Settlement

`settle_equip` (04 2.5) gains a per-module loop: for each module `i` with
`module_owed_token[i] > 0` or `module_owed_pool[i] > module_settled_pool[i]`, the royalty is taken
from the module's share first, then the bounty, then the rest goes to **that module's** destination.
`EquipState` grows by `MAX_MODULES * 24` bytes (three `u64` per module); the size of `settle_equip`
for a full composite is to measure.

### 2.12 Budget (to measure in 07)

| Path | To measure |
| --- | --- |
| Transfer through a composite of 1 to `MAX_MODULES` modules | CU, keys, bytes with the per-mint table |
| Buy and sell through a Pool composite of 1 to `MAX_MODULES` modules | same |
| `fuse`, `forge` of composites | CU, trace |
| `settle_equip` of a full composite | CU, trace, keys (one destination per module) |

`MAX_MODULES` is the largest count for which the worst callback stays inside the measured CU and
byte ceilings with the slot table's other slots full.

## 3. Compatibility model

### 3.1 Capability classes per template

Every template declares its capability set (section 4 lists it per template): `TC`, `PC` with sides,
`DS` with sides, `RF`, `BN`, `HD` bytes, `OR`, `FR` count, `WM`, `WT`, `ST`, `CK`, `PO`.

### 3.2 Matrix (inside one composite, and across slots of one mint)

| Pair | Inside one composite | Across two slots of one mint |
| --- | --- | --- |
| `TC` + `TC` | allowed; summed into one delta | allowed; one delta each, counted against `MAX_CUTTING_SLOTS` |
| `PC` + `PC` (same side) | allowed; summed, cap `max_pool_item_cut_bps` | allowed; merged by the launchpad (R2) under the same cap |
| `DS` + `DS` (same side) | allowed; summed, cap `max_pool_item_discount_bps` | allowed; summed and capped by the launchpad (03 5.3) |
| `PC` + `DS` (same side) | allowed; a discount waives launch fees, a cut adds the item's own; both shown | allowed |
| `RF` + anything | allowed; any refusal refuses | allowed |
| `BN` + `BN` | allowed; summed, the slot's burn bound caps the total | allowed |
| `HD` + `HD` | allowed; sub-ranges, sum must fit | allowed; separate ranges by the slot table |
| `FR` + `FR` | allowed; deduplicated, sum `<= max_item_reads` | allowed; each slot counts its own |
| `WM` + `WM` | only with the same target slice | allowed (identical writes, 04 2.9) |
| `WT` + `WT` | **conflict** | **conflict**: at most one Raid-family slot per mint (the war program touches one slot) |
| `ST` + `ST` | allowed; payload names the module | allowed; payload names the slot |
| `PO` + `PO` | allowed; one claim instruction per module | allowed |

Template-specific conflicts are listed under each template ("Compatibility").

### 3.3 The kit with holder rewards

With holder rewards on, the kit refuses holdings whose owner is off the curve (hooks-v2 4.5), so a
token-side cut into an equip vault (owned by the `EquipState` PDA) would make every such transfer
fail. Until a kit change exempts equip vaults (R20, section 7), the armory refuses to equip an item
or composite with any token-side cut (`TC`) on a mint whose kit has holder rewards on
(`KitRewardsTokenCut`, read with the kit's `holder_rewards_on` helper merged in `kitcomp`). Pool-side
cuts (`PC`, in bridged SOL into `PoolCuts`) are unaffected and are the recommended form for such
tokens; every Fee template below that charges on trades has a pool-side form for this reason.

### 3.4 Validation at equip

`check_fits` (02 6.1) on the combined manifest, plus: the kit rule (3.3); `WT` at most once per mint
(3.2); `BN` only where the slot allows burns (R21); target count; and the mint's whole slot table
re-summed (`MAX_CUTTING_SLOTS`, bytes).

## 4. The arsenal

Ids 1 to 9 are 04's. New ids start at 10. Every parameter's floor and ceiling is a named parameter
**to set** (`<TEMPLATE>_MIN_<FIELD>`, `<TEMPLATE>_MAX_<FIELD>` unless named otherwise). Bytes are
without the epoch byte. "Pool" in callbacks means the launch pool through the launchpad (03 5.1).
Every template below is `composable` unless it says otherwise.

Common helpers used in pseudocode:
- `is_buy` / `is_sell` on the token side: source owner or destination owner equals the launch pool
  (read from the `Launch` extra, as Half-Life, 04 3.7).
- `twap(obs, w)`: 03's reader over window `w`, returns "no signal" when the ring does not cover `w`;
  every template treats "no signal" as "no effect".
- `day(t) = t / 86_400`, `hour(t) = (t / 3_600) mod 24` (UTC).

### 4.1 Family: Fee

**10 Size Tiers** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. Capabilities: `PC` both sides.
- Fields: `t1_lamports`, `t2_lamports` (tier bounds, `t1 < t2`), `cut1_bps`, `cut2_bps`, `cut3_bps` (`SIZE_MAX_CUT_BPS`).
- Bytes 0. Extras: none.
- Behaviour: `q = quote side of the trade; cut_bps = q < t1 ? cut1 : q < t2 ? cut2 : cut3; cut = q * cut_bps / 10_000`.
- Sentence: "Trades under {t1} pay {cut1}, up to {t2} pay {cut2}, larger trades pay {cut3}."
- Forge: bounds `Keep`, cuts `TowardCeiling`.
- Nearest: Impact Fee (14) charges by pool depth, this by absolute size; Sell Ladder (37) by share of the seller's own position.
- Abuse: splitting a trade into small ones avoids high tiers; each split still pays the launch's fees, so splitting costs what the tier saves at most. Destination: war chest or collector (equip config).

**11 Side Skew** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides.
- Fields: `buy_cut_bps`, `sell_cut_bps` (`SKEW_MAX_CUT_BPS`).
- Behaviour: `cut = side_amount * (is_buy ? buy_cut : sell_cut) / 10_000`.
- Sentence: "Buys pay {buy_cut_bps}, sells pay {sell_cut_bps}."
- Forge: `TowardCeiling` both.
- Nearest: Size Tiers splits by size, this by direction only; the launch's own creator fee is symmetric.
- Abuse: none beyond the fee; capped per side.

**12 Launch Decay** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides. Extras: `Launch` (created_at).
- Fields: `start_cut_bps`, `end_cut_bps` (`<=` start), `decay_secs` (`DECAY_MIN_SECS`, `DECAY_MAX_SECS`).
- Behaviour: `f = min(1, (now - created_at) / decay_secs); cut_bps = start - (start - end) * f`.
- Sentence: "Trades pay {start_cut_bps} at launch, falling to {end_cut_bps} over {decay_secs}."
- Forge: `start` `TowardCeiling`, `end` `Keep`, `decay_secs` `TowardCeiling`.
- Nearest: upstream's sniper fee changes the LP fee for 30 seconds and belongs to the protocol (R6); this is the item's own cut, chosen duration, chosen destination. Half-Life (7) decays by holder age, not token age.
- Abuse: none; deterministic from launch time.

**13 Velocity Fee** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides; `OR` (own `Observations`).
- Fields: `window_secs` (`>= MIN_TWAP_SECS`), `swaps_threshold`, `cut_per_excess_bps`, `max_cut_bps`.
- Behaviour: `n = swap_count(now) - swap_count(now - window)` from observations; `excess = max(0, n - threshold); cut_bps = min(max_cut, excess * cut_per_excess)`.
- Sentence: "When more than {swaps_threshold} trades happen in {window_secs}, each extra trade adds {cut_per_excess_bps}, up to {max_cut_bps}."
- Forge: threshold `TowardCeiling` (gentler), cuts `TowardCeiling`, window `Keep`.
- Nearest: Volatility Fee (15) reacts to price movement, this to trade count.
- Abuse: an attacker can raise others' fees by trading often, paying the fee themselves each time; bounded by `max_cut_bps`.

**14 Impact Fee** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides.
- Fields: `cut_per_impact_bps` (bps of cut per 100 bps of price impact), `max_cut_bps`.
- Behaviour: `impact_bps = expected price move of this trade from the pool's reserves and virtual reserves in PoolHookArgs; cut_bps = min(max_cut, impact_bps * cut_per_impact / 100)`.
- Sentence: "Trades that move the price pay {cut_per_impact_bps} per 1% moved, up to {max_cut_bps}."
- Forge: `TowardCeiling` both.
- Nearest: Size Tiers (10) uses absolute size; this scales with depth, so the same trade pays less as liquidity grows.
- Abuse: splitting trades lowers impact per trade; same trade-off as 10.

**15 Volatility Fee** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides; `OR`.
- Fields: `short_secs`, `long_secs` (`short < long`, both `>= MIN_TWAP_SECS`), `trigger_bps`, `cut_bps`.
- Behaviour: `s = twap(short); l = twap(long); dev = abs(s - l) * 10_000 / l; cut = dev >= trigger ? cut_bps : 0`.
- Sentence: "When the price moves more than {trigger_bps} from its average, trades pay {cut_bps}."
- Forge: trigger `TowardFloor` (fires sooner), cut `TowardCeiling`, windows `Keep`.
- Nearest: Dump Brake (21) fires only on drops and only on sells; this on any deviation, both sides.
- Abuse: TWAP windows resist one-block spikes (00 rule 7).

**16 Rush Hour** (Fee, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides; `CK`.
- Fields: `start_hour` (0..23), `hours` (1..24), `inside_cut_bps`, `outside_cut_bps`.
- Behaviour: `h = hour(now); inside = (h - start_hour) mod 24 < hours; cut_bps = inside ? inside_cut : outside_cut`.
- Sentence: "From {start_hour}:00 UTC for {hours} hours trades pay {inside_cut_bps}; otherwise {outside_cut_bps}."
- Forge: hours and start `Keep`, cuts `TowardCeiling`.
- Nearest: no other template uses the clock's hour.
- Abuse: traders time their trades, which is the intent.

### 4.2 Family: Defense

**17 Cooldown** (Defense)
- Callbacks: `before_transfer`. `RF`, `HD` 5 bytes (tag, `last_buy: u32`).
- Fields: `cooldown_secs` (`COOLDOWN_MAX_SECS`).
- Behaviour: on a buy, stamp `last_buy = now` in the destination. On any transfer out of a wallet: refuse if `now < last_buy + cooldown`. Receiving into a holding that already has a stamp keeps the later stamp.
- Sentence: "After buying, a wallet waits {cooldown_secs} before selling or sending."
- Forge: `TowardCeiling`.
- Nearest: Half-Life (7) charges early exits; this refuses them; the kit's early-buyer lock applies only to the opening window.
- Abuse: a holder cannot reset the stamp by moving tokens (the send is refused). Holdings with no stamp (received by a send before the item) are not locked.

**18 Daily Sell Cap** (Defense)
- Callbacks: `before_transfer`. `RF`, `HD` 11 bytes (tag, `day: u16`, `base: u64`).
- Fields: `cap_bps` (share of the day's starting balance that may leave per day, `CAP_MIN_BPS`).
- Behaviour: on a transfer out: if `day(now) != day` then `day = day(now); base = source_balance` (before the transfer). Keep `spent = base - source_balance_after` implicitly via the balance; refuse if `base - (source_balance - amount) > base * cap_bps / 10_000`. Receiving raises `base` by the amount received the same day.
- Sentence: "A wallet may sell or send at most {cap_bps} of its holding per day."
- Forge: `TowardFloor` (tighter is stronger).
- Nearest: Max Transaction (19) caps one transfer by supply; this caps a day by the holder's own balance; Sell Ladder (37) charges instead of refusing.
- Abuse: splitting across wallets multiplies the cap; the cap still binds each wallet's own balance.

**19 Max Transaction** (Defense)
- Callbacks: `before_transfer`. `RF`. Bytes 0.
- Fields: `max_tx_bps` (of supply).
- Behaviour: refuse if `amount > supply * max_tx_bps / 10_000`, except transfers whose source or destination is the launch pool's vault during graduation (exempt owners as Half-Life's).
- Sentence: "No single transfer may move more than {max_tx_bps} of the supply."
- Forge: `TowardFloor`.
- Nearest: Wall (3) caps a wallet's total; this caps one transfer.
- Abuse: split transfers; intended to slow, not stop.

**20 Flash Guard** (Defense)
- Callbacks: `before_transfer`. `RF`, `HD` 5 bytes (tag, `buy_slot_low: u32`).
- Fields: `min_slots` (at least 1).
- Behaviour: on a buy stamp the low 32 bits of `Clock::slot`. On a sell from that holding refuse if `slot_low(now) - buy_slot_low < min_slots` (wrapping arithmetic).
- Sentence: "Tokens cannot be sold within {min_slots} slots of being bought."
- Forge: `TowardCeiling`.
- Nearest: Cooldown (17) is in seconds and covers sends; this targets same-block round trips (sandwich legs) only.
- Abuse: a sandwich's back leg from the same wallet fails; from a fresh wallet it needs a send first, which this does not stop, so pair with Cooldown in a composite.

**21 Dump Brake** (Defense, Pool kind)
- Callbacks: `pool_before_swap`. `PC` sell side; `OR`.
- Fields: `short_secs`, `long_secs`, `drop_bps`, `sell_cut_bps`.
- Behaviour: `if is_sell and twap(short) < twap(long) * (10_000 - drop_bps) / 10_000: cut = side_amount * sell_cut / 10_000`.
- Sentence: "While the price is more than {drop_bps} under its average, sells pay {sell_cut_bps}."
- Forge: `drop_bps` `TowardFloor`, cut `TowardCeiling`, windows `Keep`.
- Nearest: Volatility Fee (15) is two-sided and direction-blind; the war program's counter-strike buys back on a drop, this slows the sellers.
- Abuse: TWAP based; a dumper cannot avoid it by spiking the price for one block.

**22 Dust Guard** (Defense)
- Callbacks: `before_transfer`. `RF`. Bytes 0.
- Fields: `min_amount` (raw units).
- Behaviour: refuse a wallet-to-wallet transfer with `amount < min_amount` (buys and sells exempt).
- Sentence: "Wallet sends under {min_amount} are refused."
- Forge: `Keep`.
- Nearest: no other template filters by a minimum; it stops dusting spam that would fill holders' feeds.
- Abuse: none; holders can always send their whole holding (an exact empty-out is exempt).

**23 Guest List** (Defense, Pool kind)
- Callbacks: `pool_before_swap`. `RF` buy side; `FR` 1 (the buyer's holding of the target mint, R22).
- Fields: `min_hold` (raw units of the target), `open_after_secs` (from launch).
- Targets: one mint.
- Behaviour: `if is_buy and now < created_at + open_after and holding(target, actor).amount < min_hold: refuse`.
- Sentence: "For the first {open_after_secs}, only holders of at least {min_hold} {target} can buy."
- Forge: `min_hold` `TowardFloor`, window `Keep`.
- Nearest: Ally Pass (27) gives a discount to the same holders forever; this gates the opening only.
- Abuse: a buyer can borrow nothing on chain here (no lending); they must actually hold the target at the moment of the buy.

### 4.3 Family: Reward

**24 Loyalty Pot** (Reward, Pool kind with token half)
- Callbacks: `pool_before_swap` (sell side cut), `before_transfer` (stamp), `on_touch` (`ST` claim marker). `PC` sell side, `HD` 5 bytes (tag, `joined_epoch: u16`, `claimed_epoch: u16`), `PO`.
- Fields: `sell_cut_bps`, `epoch_secs` (`LOYALTY_MIN_EPOCH_SECS`).
- Behaviour: sells pay `sell_cut` into the module's pool share (the pot). Receiving tokens sets `joined_epoch = current epoch` (dilution rule as Half-Life: receiving resets eligibility for that epoch). At an epoch boundary the pot accrued in the last epoch becomes `payable[e]`. `claim_loyalty(holding)` (items instruction, holder signs) pays `payable[e] * balance / supply_eligible` when `joined_epoch < e` and `claimed_epoch < e`, then touches `claimed_epoch = e`. Unclaimed `payable` rolls into the next epoch.
- `supply_eligible` = supply minus the pool, launch and protocol vault balances (read at claim). Sum of balances `<=` that supply, so claims never exceed the pot.
- Sentence: "Sells pay {sell_cut_bps} into a pot; every {epoch_secs}, wallets that held the whole period can claim their share."
- Forge: cut `TowardCeiling`, epoch `Keep`.
- Nearest: Holder Stream (25) pays every holder continuously through the kit; this pays only those who held a full epoch, by claim.
- Abuse: buying just before the boundary does not qualify (joined in that epoch); moving tokens to a fresh wallet resets eligibility.

**25 Holder Stream** (Reward, Pool kind)
- Callbacks: `pool_before_swap`. `PC` both sides.
- Fields: `buy_cut_bps`, `sell_cut_bps`.
- Destination: the token's war chest treaty inbox path, streamed to holders by the kit's `share` (R13 machinery); requires the kit with holder rewards.
- Sentence: "{buy_cut_bps} of buys and {sell_cut_bps} of sells are streamed to all holders."
- Forge: `TowardCeiling`.
- Nearest: the kit's holder rewards are fixed at launch; this one is equippable and swappable; Loyalty Pot (24) is age-gated.
- Abuse: streaming (kit `share` over time) means a buy just before cannot capture a lump (R13).

**26 Streak** (Reward, token half; exports bytes)
- Callbacks: `before_transfer`. `HD` 6 bytes (tag, `last_day: u16`, `streak: u16`, `flags: u8`).
- Fields: `max_streak` (`STREAK_MAX`).
- Behaviour: a buy on a day `d`: if `last_day == d - 1` then `streak += 1` else if `last_day != d` then `streak = 1`; `last_day = d`. Any sell or send out: `streak = 0`.
- Sentence: "Buy on consecutive days without selling to build a streak, up to {max_streak}."
- Forge: `TowardCeiling`.
- Nearest: Rank Badge (35) counts volume; this counts consistency. On its own it pays nothing; it exports bytes for Patience (40) or Ally-style perks in a composite (`reads_module`).
- Abuse: one dust buy per day keeps a streak; pair with a minimum in the reading module.

### 4.4 Family: Relation

**27 Ally Pass** (Relation, Pool kind)
- Callbacks: `pool_before_swap`. `DS` buy side; `FR` 1 (buyer's holding of the ally).
- Fields: `min_hold`, `discount_bps`.
- Targets: one ally mint.
- Behaviour: `if is_buy and holding(ally, actor).amount >= min_hold: discount = discount_bps`.
- Sentence: "Holders of at least {min_hold} {target} pay {discount_bps} less on buys."
- Forge: `min_hold` `TowardFloor`, discount `TowardCeiling`.
- Nearest: Raid (1) discounts buyers who sold a rival in the route; this discounts holders of an ally without selling anything; Treaty (5) moves money between the two tokens.
- Abuse: one holding can be used by many buys; the discount only waives this token's own launch fees.

**28 Embargo** (Relation, Pool kind)
- Callbacks: `pool_before_swap`. `PC` buy side.
- Fields: `cut_bps`.
- Targets: 1..`EMBARGO_MAX_TARGETS` mints.
- Behaviour: `if is_buy and route.route_input_mint in targets: cut = side_amount * cut_bps / 10_000`.
- Sentence: "Buyers who arrive by selling {target} pay {cut_bps} extra."
- Forge: `TowardCeiling`.
- Nearest: the mirror of Raid (1): Raid rewards those arriving from a rival, Embargo charges them; Shield (2) charges raiders when they leave.
- Abuse: routing through an intermediate token hides the input only if the trader leaves the route instruction, which the route context makes visible (R5); a separate prior swap is not seen, by design.

### 4.5 Family: War

**29 Mercenary** (War, Pool kind with token half)
- Callbacks: `pool_after_swap` (mark), `before_transfer` (stamp). `WM`, `HD` shares Raid's layout (it is a Raid-family module), `WT`.
- Fields: `points_per_unit` (`MERC_MAX_POINTS_PER_UNIT`, below Raid's ceiling).
- Targets: none (any route input other than the quote counts).
- Behaviour: as Raid's mark and stamp (04 3.1) but untargeted, points only, no discount, no toll.
- Sentence: "Any buyer arriving from another token earns {points_per_unit} raid points per unit."
- Forge: `TowardCeiling`.
- Nearest: Raid (1) is aimed and gives discounts; Mercenary rewards any inflow with points only.
- Compatibility: a Raid-family module (`WT`): not with Raid in the same mint (3.2).
- Abuse: route through a junk token to farm points; points only pay bounties from the chest at the chest's own posted rate and caps (05), so the chest controls the cost.

**30 Garrison** (War, Pool kind)
- Callbacks: `pool_before_swap`. `DS` buy side; reads own `WarState`.
- Fields: `discount_bps`.
- Behaviour: `if is_buy and WarState.under_siege_until > now: discount = discount_bps`.
- Sentence: "While under siege, buyers pay {discount_bps} less."
- Forge: `TowardCeiling`.
- Nearest: Shield (2) charges raiders leaving; Wall (3) caps wallets; Garrison rallies defenders with cheaper entries.
- Abuse: the besieger can buy cheaper too; the discount waives only this token's launch fees.

**31 War Levy** (War, Pool kind)
- Callbacks: `pool_before_swap`. `PC` sell side; reads own `RaidLedger`.
- Fields: `trigger_lamports` (rolling inbound raid volume that counts as "active"), `sell_cut_bps`.
- Destination: own war chest.
- Behaviour: `active = any RaidLedger.inbound rolling volume >= trigger; if is_sell and active: cut = side_amount * sell_cut / 10_000`.
- Sentence: "While a raid of at least {trigger_lamports} is under way, sells pay {sell_cut_bps} to the war chest."
- Forge: trigger `TowardFloor`, cut `TowardCeiling`.
- Nearest: Shield (2) charges only arrivals from the raider; Levy charges every seller during a raid, to fund defense.
- Abuse: a rival can trigger the levy on us by raiding; the levy funds our own chest, so it costs the rival and pays us.

### 4.6 Family: Burn and supply

**32 Sell Burn** (Burn, Pool kind)
- Callbacks: `pool_after_swap`. `BN` sell side (base burned from what the seller sends, before it enters the pool).
- Fields: `burn_bps`.
- Behaviour: `if is_sell: burn = base_amount * burn_bps / 10_000`.
- Sentence: "{burn_bps} of every sell is burned."
- Forge: `TowardCeiling`.
- Nearest: the kit's burn rule is fixed at launch and both-sided; this is equippable and sell-only; Target Burn (33) stops at a supply.
- Compatibility: needs a burn bound (R21).
- Abuse: none beyond the fee.

**33 Target Burn** (Burn, Pool kind)
- Callbacks: `pool_after_swap`. `BN` both sides.
- Fields: `burn_bps`, `target_supply_bps` (of the launch's initial supply).
- Behaviour: `target = initial_supply * target_supply_bps / 10_000; if supply > target: burn = min(base_amount * burn_bps / 10_000, supply - target)`.
- Sentence: "{burn_bps} of every trade is burned until the supply reaches {target_supply_bps} of the start."
- Forge: burn `TowardCeiling`, target `Keep`.
- Nearest: Sell Burn (32) never stops and is sell-only.
- Abuse: none; it ends by itself.

**34 Gift Ember** (Burn, Fee kind)
- Callbacks: `before_transfer`. `TC` on wallet-to-wallet transfers only.
- Fields: `cut_bps`.
- Destination: burn (`settle_equip` burns the vault's balance; a token callback never answers a burn directly).
- Behaviour: `if not is_buy and not is_sell: cut = amount * cut_bps / 10_000`.
- Sentence: "{cut_bps} of every wallet-to-wallet send is burned."
- Forge: `TowardCeiling`.
- Nearest: Transfer Fee (8) charges every transfer to a collector; this charges only sends, and burns.
- Compatibility: `TC`, so not on a kit-with-holder-rewards mint (3.3).
- Abuse: none.

### 4.7 Family: Social and game

**35 Rank Badge** (Social, token half; exports bytes)
- Callbacks: `before_transfer`. `HD` 6 bytes (tag, `volume_units: u32`, `rank: u8`).
- Fields: `unit_lamports`, `rank_step` (units per rank), `max_rank`.
- Behaviour: on a buy, `volume_units += amount_in_quote / unit_lamports` (from the Raid-style mark when present, else the token amount times the TWAP price), `rank = min(max_rank, volume_units / rank_step)`. Sends move nothing (a badge is earned, never transferred); emptying the holding clears it.
- Sentence: "Every {unit_lamports} bought earns a unit; each {rank_step} units is a rank, up to {max_rank}."
- Forge: `unit_lamports` `TowardFloor`, others `Keep`.
- Nearest: Streak (26) counts days, this counts volume. Pays nothing alone; exported for perks.
- Abuse: wash buys raise rank but pay every fee each time.

**36 Referral** (Social, Pool kind)
- Callbacks: `pool_before_swap`. `PC` buy side; `FR` 1 (`["referred", mint, actor]`, R22); `PO`.
- Fields: `cut_bps` (paid by referred buyers, out of the item's share).
- Setup: `set_referrer(mint, referrer)` (items instruction, the buyer signs once, buyer pays rent) creates `Referred { referrer, owed }` at `["referred", mint, buyer]`.
- Behaviour: `if is_buy and Referred exists: cut = side_amount * cut_bps / 10_000; Referred.owed += cut` (the module's pool share). `settle_referral(referred)` (permissionless, bounty) pays `owed` to the referrer from `PoolCuts`.
- Sentence: "Buyers who name a referrer send {cut_bps} of their buys to that referrer."
- Forge: `TowardCeiling`.
- Nearest: no other template pays a person chosen by the trader.
- Abuse: self-referral pays oneself one's own cut, a net zero; the launch's fees are unchanged.

**37 Sell Ladder** (Fee, token half)
- Callbacks: `before_transfer`. `TC` on sells.
- Fields: `step_bps` (share of the seller's balance per step), `cut_per_step_bps`, `max_cut_bps`.
- Behaviour: `if is_sell: share = amount * 10_000 / source_balance; cut_bps = min(max_cut, (share / step) * cut_per_step); cut = amount * cut_bps / 10_000`.
- Sentence: "Selling a bigger share of your holding at once costs more: {cut_per_step_bps} per {step_bps} sold, up to {max_cut_bps}."
- Forge: `TowardCeiling` cuts, `step` `Keep`.
- Nearest: Size Tiers (10) by absolute size; Daily Sell Cap (18) refuses; this charges by share of position.
- Compatibility: `TC` (3.3).
- Abuse: splitting sells lowers each share; Daily Sell Cap in a composite closes it.

**38 First Blood** (Social, Pool kind)
- Callbacks: `pool_before_swap`. `DS` buy side; state in `EquipState` (`last_day: u32`).
- Fields: `discount_bps`, `min_lamports`.
- Behaviour: `if is_buy and side_amount >= min_lamports and day(now) != last_day: discount = discount_bps; last_day = day(now)`.
- Sentence: "The first buy of at least {min_lamports} each day pays {discount_bps} less."
- Forge: discount `TowardCeiling`, minimum `Keep`.
- Nearest: Rush Hour (16) changes fees by hour for everyone; this rewards one buyer per day.
- Abuse: bots race for it; the prize is bounded to one waived fee per day.

**39 Guild Tag** (Social, token half)
- Callbacks: `before_transfer` (clears on empty), `on_touch` (`ST`). `HD` 3 bytes (tag, `guild: u16`).
- Fields: none beyond a version field.
- Behaviour: `touch` signed by the holding's owner with `SetGuild { guild }` writes it; nothing else changes it. No money effect.
- Sentence: "Holders can wear a guild tag."
- Forge: `Keep`.
- Nearest: Rank Badge (35) is earned; a guild is chosen. The indexer builds guild boards (06).
- Abuse: none; cosmetic.

**40 Patience** (Reward, Pool kind)
- Callbacks: `pool_before_swap`. `DS` sell side; reads the seller's holding (R22) for an age or a streak.
- Fields: `min_age_secs`, `discount_bps`, `source` (1 own age stamp, 2 a Streak module via `reads_module`, 3 Half-Life age in the same mint).
- Behaviour: `if is_sell and age(seller) >= min_age: discount = discount_bps`.
- Sentence: "Wallets that held for {min_age_secs} pay {discount_bps} less when they sell."
- Forge: age `TowardFloor`, discount `TowardCeiling`, source `Keep`.
- Nearest: Half-Life (7) charges early sellers; Patience waives launch fees for late ones; both can run together.
- Abuse: age travels with tokens and blends on receive (Half-Life's rule), so a fresh wallet cannot borrow age.

**41 Composite**: section 2. Not composable itself.

### 4.8 Composite presets

Registered by the protocol (`register_preset(id, modules, name)`, timelock as every admin setter).
Parameters are chosen per minted item within each module's ceilings; the preset fixes the order.

| Preset | Modules (in order) | Host kind | Idea |
| --- | --- | --- | --- |
| Diamond Hands | Cooldown (17), Daily Sell Cap (18), Patience (40), Loyalty Pot (24) | Pool | hold longer, sell slower, get paid for staying |
| Fortress | Wall (3), Max Transaction (19), Flash Guard (20), Garrison (30) | Pool | hard to dump, hard to sandwich, cheap to defend |
| Warband | Raid (1), Dump Brake (21), War Levy (31) | Pool | raid out, brake dumps, fund the chest when raided |
| Mercenary Camp | Mercenary (29), Rank Badge (35), First Blood (38) | Pool | open-door points with status and a daily prize |
| Furnace | Sell Burn (32), Target Burn (33), Gift Ember (34) | Pool | supply goes down on every path, until the target |
| Fair Start | Launch Decay (12), Flash Guard (20), Max Transaction (19), Guest List (23) | Pool | a gentle, gated opening |
| Steady Hands | Volatility Fee (15), Velocity Fee (13), Impact Fee (14) | Pool | calm markets pay little, frantic ones pay more |
| Alliance Gate | Ally Pass (27), Embargo (28) | Pool (Relation modules hosted) | friends in cheap, rivals in dear |
| Streak Club | Streak (26), Patience (40, source 2) | Pool | daily buyers become patient sellers |
| Rush Hour Raid | Rush Hour (16), Raid (1) | Pool | raids priced by the clock |

Every preset is checked by 2.10 at registration; a preset that fails (bytes, conflicts) is refused.

## 5. Build waves and tests

### 5.1 Waves

| Wave | Templates | Why first |
| --- | --- | --- |
| A (with M3b) | Side Skew (11), Size Tiers (10), Launch Decay (12), Max Transaction (19), Dust Guard (22), Sell Burn (32) | no foreign reads, no bytes or simple ones; exercise pool cut, refusal and pool burn paths |
| B | Cooldown (17), Flash Guard (20), Daily Sell Cap (18), Streak (26), Rank Badge (35), Guild Tag (39) | hook-data templates; exercise ranges and self touch |
| C | Velocity (13), Volatility (15), Impact (14), Dump Brake (21), Rush Hour (16) | need M3a observations and route context |
| D | Ally Pass (27), Guest List (23), Embargo (28), Patience (40), Referral (36) | foreign holding reads (R22) and the referral account |
| E | Mercenary (29), Garrison (30), War Levy (31), Target Burn (33), Gift Ember (34), Loyalty Pot (24), Holder Stream (25), Sell Ladder (37), First Blood (38) | need war state, payout instructions or kit streaming |
| F | Composite (41), `fuse`, `forge` of composites, presets | needs every module above |

### 5.2 Tests per template

Each template gets, in `programs/tests/tests/templates_<name>.rs`:
1. **Behaviour**: every branch of its pseudocode with exact amounts (cut, discount, burn, bytes).
2. **Abuse**: the case named under "Abuse", showing the bound holds (for example, Flash Guard refuses
   a same-slot round trip; Loyalty Pot pays nothing to a wallet that joined this epoch; Referral
   self-referral nets zero).
3. **Forge**: `combine_params` follows the stated rules and never passes a ceiling.
4. **Invariants** of 07 section 4 after every step (supply, cuts, vaults, hook data, votes).
5. **Kit**: for `TC` templates, equip is refused on a kit-with-holder-rewards mint (3.3).

Composites (`composites.rs`): every 2.10 refusal; execution equals running the modules as separate
slots for cuts, discounts and burns (same base amount, no compounding); per-module settlement exact;
`reads_module` read-only; forge of equal and unequal sequences; each preset equips and trades.

Budgets (`budgets.rs`): one row per template on transfer, buy and sell; composites at 1 to
`MAX_MODULES` modules (2.12).

## 6. Template list

| Id | Name | Family | Slot kind |
| --- | --- | --- | --- |
| 1 | Raid | War | Pool + token half |
| 2 | Shield | War | Pool + token half |
| 3 | Wall | Defense | Defense |
| 4 | Spy | Relation | Pool |
| 5 | Treaty | Relation | Relation (pool half) |
| 6 | Tribute | Relation | Relation (pool half) |
| 7 | Half-Life | Fee | Fee |
| 8 | Transfer Fee | Fee | Fee |
| 9 | War orders | War | War |
| 10 | Size Tiers | Fee | Pool |
| 11 | Side Skew | Fee | Pool |
| 12 | Launch Decay | Fee | Pool |
| 13 | Velocity Fee | Fee | Pool |
| 14 | Impact Fee | Fee | Pool |
| 15 | Volatility Fee | Fee | Pool |
| 16 | Rush Hour | Fee | Pool |
| 17 | Cooldown | Defense | Defense |
| 18 | Daily Sell Cap | Defense | Defense |
| 19 | Max Transaction | Defense | Defense |
| 20 | Flash Guard | Defense | Defense |
| 21 | Dump Brake | Defense | Pool |
| 22 | Dust Guard | Defense | Defense |
| 23 | Guest List | Defense | Pool |
| 24 | Loyalty Pot | Reward | Pool + token half |
| 25 | Holder Stream | Reward | Pool |
| 26 | Streak | Reward | Reward (token) |
| 27 | Ally Pass | Relation | Pool |
| 28 | Embargo | Relation | Pool |
| 29 | Mercenary | War | Pool + token half |
| 30 | Garrison | War | Pool |
| 31 | War Levy | War | Pool |
| 32 | Sell Burn | Burn | Pool |
| 33 | Target Burn | Burn | Pool |
| 34 | Gift Ember | Burn | Fee |
| 35 | Rank Badge | Social | Reward (token) |
| 36 | Referral | Social | Pool |
| 37 | Sell Ladder | Fee | Fee |
| 38 | First Blood | Social | Pool |
| 39 | Guild Tag | Social | Reward (token) |
| 40 | Patience | Reward | Pool |
| 41 | Composite | Utility | host kind (2.8) |

40 templates plus Composite.

## 7. What 00 and the other parts must add

**Parameters (00 section 6), all to set:** `MAX_MODULES` (M), `ITEM_DATA_MAX` (layout), every
template's field floors and ceilings named in section 4 (O), `EMBARGO_MAX_TARGETS`,
`LOYALTY_MIN_EPOCH_SECS`, `STREAK_MAX`, `MERC_MAX_POINTS_PER_UNIT`, `COOLDOWN_MAX_SECS`,
`DECAY_MIN_SECS`, `DECAY_MAX_SECS`, `CAP_MIN_BPS`.

**Seeds (00 4.3):** `CompositeItem` `["composite", item]` under armory; `Referred`
`["referred", mint, holder]` under items; `Preset` `["preset", id: u16 le]` under armory.

**Rulings to add (proposed):**
- **R19 Composites.** One item, one slot, one CPI level, one delta; modules run in order on the same
  pre-state; refusals AND; cuts, discounts and burns summed per side and capped by the combined
  manifest; per-module accounting in `EquipState`; one royalty; `fuse` is one-way.
- **R20 Kit and token-side cuts.** On a mint whose kit has holder rewards on, the armory refuses
  items with token-side cuts (`KitRewardsTokenCut`), until the kit exempts the equip-vault owners of
  its own mint (derivable from `["equip", mint, slot]` for each slot; a kit change, measured first).
- **R21 Pool burns need a bound.** `SlotBounds` (01) gains `may_burn` for Pool slots; today no slot
  allows a burn (M2 note 4), so burn templates (32, 33) are refused until it lands.
- **R22 Reads of a holder's other holdings.** `init_equip` writes registry extras of the PDA kind
  (upstream `HookAccountList`: seeds may name prefix accounts), so an item can read
  `["holding", other_mint, actor]` (pool side) or `["holding", other_mint, source_owner]` (token
  side) and `["referred", mint, actor]`. Reads count toward `max_item_reads`.
- **R23 Self touches.** `touch` signed by the holding's owner reaches items with `ANSWERS_TOUCH`
  for social payloads only (`SetGuild`, Loyalty claim markers); war payloads still need `WAR_SIGNER`
  (04 2.10). Each item checks which authority a payload needs.
- **R24 Item payouts.** Items instructions that pay people (`claim_loyalty`, `settle_referral`)
  pay from the equip vault or `PoolCuts` through `transfer_from_protocol` (R16), never from a
  callback, and each checks solvency against the module's recorded share.
- **Template flag `composable`** in the armory's `Template` (02 2.2), set at registration.

## R20 to R24 implementation notes (branch r20)

- **R20, the Locked slot runs last.** The token program calls the item slots first and then the
  Locked slot, telling it the items' total cut as `TokenHookArgs.delta` in the `Before` phase
  (upstream always passed 0 there). The kit uses it: a holder destination receives
  `amount - delta`, so settle, max wallet, the early-buyer lock and `eligible` count what really
  arrives; on a holder-to-holder transfer `eligible` falls by the cut, which leaves for an equip
  vault. Upstream single-hook mints still pass 0, so nothing changes for them. Upstream Locked
  hooks (Half-Life, tax_hook) ignore `delta` in `Before`.
- **R20 (b), the encoding.** In `transfer_from_protocol` the Locked slot is told `authority =
  PROTOCOL_TRANSFER_MARKER`, the token program's PDA `["protocol-transfer"]`
  (`HHwFz2okVQVyTE3HLsKSM2a7u4Y2DW729MbmkSeNoWEG`). No ordinary transfer can carry it: it is a PDA
  the token program never signs for. The kit reads it as "verified protocol source": the source is
  excluded, and an off-curve destination is excluded (a royalty holding, a partner's treaty
  inbox); a wallet destination is an ordinary holder. No argument layout changed.
- **R20 (a).** `KitConfig::is_excluded` now also covers the owners derivable from the mint: every
  `["equip", mint, slot]` for `slot < MAX_SLOTS` and `["pool-cuts", mint]` under the items id,
  `["war-chest", mint]` and `["treaty-inbox", mint]` under the war id. The derivations run only for
  off-curve owners; a wallet costs one curve check. `is_excluded_basic` keeps the upstream three.
  The mirror and `claim` use the same function, so off chain and on chain agree.
- **Invariant this relies on (R24).** A protocol program moves what its PDAs hold only through
  `transfer_from_protocol` (or burns from a derived vault). A royalty owner or a partner inbox that
  sent through an ordinary transfer would be treated as a holder on that send. The armory's
  `claim_royalty` already uses `transfer_from_protocol`; M3b's `settle_equip` and M4's treaty
  streaming must too.
- **Any other program address stays refused** on a token with holder rewards (upstream section 0
  item 5: no second pool), tested.
- **R21.** `SlotBounds.may_burn` (a new last field, so `Slot` and `Mint` grow by one byte per
  slot; `SlotInfo` in `SlotsInitialized` gains `may_burn` after `may_answer_touch`, which 06's
  decoder must follow). `create_slot_mint` refuses it on any slot but `Pool`
  (`InvalidSlotTable`). The armory's fit check is `burn_fits(manifest.may_burn,
  bounds.may_burn)`. The run-time enforcement of a pool item's burn is the launchpad's forwarding
  (M3b).
- **R22** needed no token-program change: extras are passed by count (`extra_count`), so a
  derived extra is resolved by the client like any other. The SDK work is the app branch's.
- **R23** needed no token-program change: `touch` already accepts any signer and tells the item
  `authority` and `source_owner`; the item decides (tested with an item that acts only for the
  owner).
- **Test stand-ins:** `items_stub` (new, test-only, at the items id) and `armory_stub::as_pda`
  forward a token instruction signing as any PDA of their id, standing in for `settle_equip` and
  `claim_royalty` until M3b. `items_stub-keypair.json` is a copy of the items keypair.

## Arsenal waves B and C notes (2026-10-09, branch arsenal1)

Built: Cooldown (17), Flash Guard (20), Daily Sell Cap (18), Streak (26), Rank Badge (35), Guild
Tag (39), Velocity Fee (13), Impact Fee (14), Volatility Fee (15), Dump Brake (21), Rush Hour (16),
each a new file in `programs/hookwars_items/src/templates/`, with its shape, validation and manifest
in `hookwars-common` and one suite `programs/tests/tests/templates_<name>.rs` (behaviour, abuse,
forge, invariants after every step) plus `budgets_arsenal1.rs`. Full suite on server B: 405 passed,
0 failed, 2 ignored (the Studio fixtures), with `app_vectors.rs` removed in the server copy only.

Where the build differs from section 4:

1. **`init_equip` target rule.** `check_module_targets` in `equip.rs` refuses any template it does
   not name, so the eleven ids were added to its no-target arm (one arm, nothing else in `equip.rs`).
   Templates 43 to 45 (Coalition, Boss, Rivalry) are not named there either, so they cannot be
   equipped yet; that belongs to the expansion lane.
2. **Kinds.** Streak, Rank Badge and Guild Tag are slot kind `Reward` (there is no Social kind).
   Cooldown, Flash Guard and Daily Sell Cap are `Defense`; Velocity, Impact, Volatility, Rush Hour
   and Dump Brake are `Pool`.
3. **Quote side.** As wave A, the pool templates cut on a buy's `pool_before_swap` (input) and a
   sell's `pool_after_swap` (output). Dump Brake therefore answers on a sell's `after` callback,
   not `pool_before_swap`, and its manifest names `AFTER_SWAP` only.
4. **Own-ring extras.** Velocity, Volatility, Dump Brake and Rank Badge read the token's own ring:
   extras are our `Launch` and our launch pool (`launch_pool_address`), and the pool must be the one
   the `Launch` names. Every read uses `MIN_TWAP_SECS` as the floor (rule 7); any read error is no
   signal and no effect. Cooldown, Flash Guard, Daily Sell Cap and Streak take our `Launch` only
   (buy and sell detection); Impact Fee, Rush Hour and Guild Tag take none.
5. **Velocity Fee** counts swaps from the ring entry at or before `now - window_secs` to the pool's
   current `swap_count`.
6. **Impact Fee.** A buy is measured at its `before` callback from the reserves before the swap and
   the quote in; a sell at its `after` callback, where the DEX passes the reserves after the swap,
   from `(x - amount_in, y + amount_out)` before against `(x, y)` after. Virtual reserves count.
7. **Rank Badge** values a buy as the tokens bought times the time-weighted price over
   `MIN_TWAP_SECS` (not the raid mark, which only Raid writes); with no signal the buy earns no
   units. A stamp is written only once a buy earns at least one unit.
8. **Daily Sell Cap** raises a same-day `base` by the amount received before other slots' cuts (an
   upper bound). Transfers out of the pool and protocol accounts are never capped.
9. **Flash Guard** refuses sells only; sends pass (as section 4.2 says, Cooldown covers them).
10. **Guild Tag** has one field (`version`, Keep); its payload is `GuildTouch::SetGuild { guild }`
    (Borsh), only the holding's owner may send it (`NotHolder`), and `guild = 0` removes the tag.
11. **Emptying a holding** clears the stamp of Cooldown, Flash Guard, Daily Sell Cap, Streak, Rank
    Badge and Guild Tag, so the holding can close.
12. **New errors** (appended to `ItemsError`): `CooldownActive`, `FlashSellTooSoon`,
    `DailyCapExceeded`, `NotHolder`.
13. **Test harness.** `programs/tests/src/arsenal1.rs` (new): a slot mint with Pool, Defense (12
    bytes), Reward (7 bytes, touch) and Relation slots; a fake launch pool with a ring at the real
    launch pool address; a delegate written onto a pool holding so suites can move tokens out of a
    pool address; pool callbacks carrying reserves. TEST floors and ceilings for the eleven ids are in
    `src/armory.rs` `test_schema`.
14. **Engine dependency.** These templates follow the engine at `ed4fa47`: a module that answers
    nothing returns a default answer. Security review 2 H-A (a silent pool item reverting the swap)
    is the integration lane's fix; none of these templates depends on how it is fixed.

Measured (server B, `budgets_arsenal1.rs`, every account in one lookup table):

| Template | Path | Keys | v0 bytes (with table) | Trace | Height | CU |
| --- | --- | --- | --- | --- | --- | --- |
| 17 Cooldown | buy delivery | 12 | 528 (283) | 4 | 2 | 35,340 |
| 20 Flash Guard | buy delivery | 12 | 528 (283) | 4 | 2 | 33,822 |
| 18 Daily Sell Cap | buy delivery | 12 | 528 (283) | 4 | 2 | 32,244 |
| 26 Streak | buy delivery | 12 | 528 (283) | 4 | 2 | 33,709 |
| 35 Rank Badge | buy delivery, ring read | 13 | 561 (285) | 4 | 2 | 43,304 |
| 39 Guild Tag | buy delivery | 11 | 495 (281) | 4 | 2 | 29,696 |
| 13 Velocity Fee | pool callback through `launch_stub` | 12 | 930 (685) | 3 | 2 | 18,711 |
| 14 Impact Fee | pool callback through `launch_stub` | 11 | 896 (682) | 3 | 2 | 15,719 |
| 15 Volatility Fee | pool callback through `launch_stub` | 12 | 930 (685) | 3 | 2 | 27,720 |
| 16 Rush Hour | pool callback through `launch_stub` | 11 | 896 (682) | 3 | 2 | 16,741 |
| 21 Dump Brake | pool callback through `launch_stub` | 12 | 930 (685) | 3 | 2 | 23,307 |

Transfer rows are one `transfer` with the slot's registry slice; pool rows include the stub's own
call level and its instruction data (the forwarded `PoolHookArgs`).
