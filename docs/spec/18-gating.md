# 18. Gating: scarce hooks, premium templates, the external gate

Owner decision (2026-10-10): hooks are scarce assets, so the economy runs on supply and demand and
not on free copies. Premium templates mint their items as Licensed. Other protocols' tokens may use
units hooks only while they hold a live licence (or own the item). There is no mint price: the
owner chose scarcity instead.

This part adds three things on top of 02 (armory), 11 (hook economy) and 14 (pass 4a access modes):

| Part | Where | What |
| --- | --- | --- |
| A. Supply | `hookwars_armory` (`Supply`, new instructions, checks in every create path) | A per-template cap, a minter rule and lifetime counters |
| B. Premium | `hookwars_armory` (existing `default_access` / `allowed_access`), app | A template whose items start Licensed and may never be Open |
| C. External gate | new program `hookwars_gate`, a restricted entry in `hookwars_items`, `hookwars_market` licences | Token-2022 mints of any protocol run units hooks as their transfer hook while licensed |

## 1. Supply (A)

### 1.1 Account

`Supply` at `["supply", template_id (u16 LE)]` under the armory, one per template that is tracked.

| Field | Meaning |
| --- | --- |
| `max_supply: u32` | Lifetime cap on new copies (`issued + drops`). `0` = uncapped (counted only) |
| `loot_reserve: u32` | Part of the cap kept for drops (loot and craft): author issuance stops at `max_supply - loot_reserve` |
| `issued: u32` | Copies made by authors: `create_item`, each module of `create_composite` / `mint_composite` |
| `drops: u32` | Copies made by `mint_loot` and `mint_crafted` |
| `forged: u32` | Items made by `forge` (a transformation, not counted against the cap) |
| `burned: u32` | Items burned by `forge` (two per forge) |
| `minter_rule: u8` | `AUTHOR_ONLY` 0: only `minter` issues; `OPEN_UNTIL_CAP` 1: anyone the template's authoring rule allows |
| `minter: Pubkey` | The wallet that issues under `AUTHOR_ONLY` |

Circulating supply is `issued + drops + forged - burned`. `minted` in the request is the lifetime
`issued + drops`; both are exposed by the API.

`Template.supply_flags` (taken from the first reserved byte, so the layout is unchanged) has bit 0
set once a `Supply` exists. A template without it (every template registered before this part, and
every test fixture) behaves exactly as before: uncapped, open as its `open_authoring` says.

### 1.2 Instructions

| Instruction | Who | Rule |
| --- | --- | --- |
| `init_supply(template_id, max_supply, loot_reserve, minter_rule, minter)` | admin, queued (L-1) | Once per template; `loot_reserve <= max_supply` when capped; sets the flag |
| `set_supply_cap(template_id, max_supply, loot_reserve)` | admin, queued | A cap may only go down (uncapped to any cap counts as down); never below `issued + drops`; `issued <= max_supply - loot_reserve` must still hold |
| `set_minter_rule(template_id, minter_rule)` | admin, queued | Switches the rule |
| `hand_over_minter(template_id, new_minter)` | the current `minter` | The author hands issuance to another wallet (a builder sells the right) |

### 1.3 Every create path

The `Supply` accounts a call needs come first in its remaining accounts (one per tracked template
it creates; any order). A tracked template whose `Supply` is missing is refused
(`SupplyMissing`), so the cap cannot be skipped by leaving the account out.

| Path | Supply effect | Rule checked |
| --- | --- | --- |
| `create_item` | `issued + 1` | `AUTHOR_ONLY`: the author is `minter` (who may then issue even when `open_authoring` is off). Cap: `issued < max_supply - loot_reserve` |
| `create_composite`, `mint_composite` | `issued + 1` for each module of a tracked template (and for the Composite template when tracked) | Same as `create_item`, per module |
| `fuse` | none: the burned components live on as the composite's modules | Components of tracked templates may be fused even when their template is closed to authoring |
| `mint_loot` | `drops + 1` | `issued + drops < max_supply`; when the cap is reached the call fails with `SupplyExhausted` |
| `mint_crafted` | `drops + 1` | Same; the craft transaction reverts and the crafter keeps the materials |
| `forge` | `forged + 1`, `burned + 2` | Never capped: net supply falls by one |

`SupplyChanged { template_id, max_supply, issued, drops, forged, burned }` is emitted after every
change, so the indexer and the economy panel read supply from events alone.

### 1.4 Loot and craft draw from the same cap

One cap per template keeps scarcity legible: there is never a second, hidden source of copies.
`loot_reserve` keeps drops possible after authors issue everything they may. When the whole cap is
spent, `mint_loot` fails: the war program's reveal of a roll that drew a capped-out template then
fails too. A raid (the swap) is never affected; only that roll's reveal. The loot table must stop
naming a template once its cap is spent (admin, timelocked, 05). Open item for the war lane: on
`SupplyExhausted` the reveal could fall back to the next table entry; recorded here, not built.

### 1.5 Existing items

`init_supply` on a template that already has items counts only new copies from then on. Mainnet
registers every template and creates its `Supply` in the same init plan, so nothing is uncounted
there. Devnet templates were registered before this part and stay uncapped until the admin queues
`init_supply`.

## 2. Premium templates (B)

No new program state. A premium template is one whose `default_access` is `LICENSED` and whose
`allowed_access` mask excludes `OPEN` (set with `set_template_economy`, queued). Its new items start
Licensed; their holder sets the licence terms with `set_access` (the app sends `create_item` and
`set_access` in one transaction, so a premium item is never listed without terms). The armory's
equip gate (pass 4a) refuses a Licensed item without a live licence at `equip_launch`, `propose` and
`execute`; this part adds tests for all three and for a holder trying to switch a premium item to
Open (`ModeNotAllowed`).

## 3. The external gate (C)

### 3.1 What it is

`hookwars_gate` implements the SPL Token-2022 transfer-hook interface. Any Token-2022 mint, from any
protocol, whose transfer hook program is the gate can bind up to `MAX_BINDINGS` (4) units items.
On every transfer Token-2022 calls the gate's `Execute`; the gate calls the items program's
restricted entry for each live binding.

### 3.2 Accounts

| Account | Seeds (gate) | Holds |
| --- | --- | --- |
| `GateConfig` | `["gate-config"]` | admin, `paused`, `require_holder_state_default` |
| `MintGate` | `["gate-mint", mint]` | the mint, its authority (the mint's transfer-hook authority at registration), `venue` (the AMM pool authority whose transfers count as buys and sells), `strict`, bindings |
| Extra account metas | `["extra-account-metas", mint]` | the TLV list Token-2022 reads to resolve `Execute`'s accounts |
| `HolderState` | `["gate-holder", mint, owner]` | 64 bytes of holder memory per wallet (Token-2022 accounts have none) |
| gate vault | `["gate-vault", mint]` | owns the units item holdings a mint deposited (the owned binding kind) |

A binding is `{ item, slot, kind, proof, targets, role, data_offset, data_bytes }`; `kind` is
`LICENCE` (proof = the market `License` at `["license", item, mint]`) or `OWNED` (proof = the gate
vault's holding of the item mint).

### 3.3 Instructions

| Instruction | Who | Does |
| --- | --- | --- |
| `init_config` | the deployer once | admin and defaults |
| `register_mint(venue, strict)` | the mint's transfer-hook authority (read from the Token-2022 `TransferHook` extension; its program must be the gate) | `MintGate` and the extra-account-metas account |
| `set_venue(venue, strict)` | the mint's gate authority | rewrites the metas |
| `bind(slot, kind, targets, role)` | the mint's gate authority | checks the item (below), assigns its holder-memory range, rewrites the metas |
| `unbind(slot)` | the mint's gate authority; or anyone once the binding has lapsed | frees the slot |
| `deposit_item` / `withdraw_item` | the mint's gate authority | moves an item into or out of the gate vault (withdraw only while unbound) |
| `open_holder(owner)` | anyone (pays rent) | creates a wallet's `HolderState` |
| `Execute` (SPL interface) | Token-2022, during a transfer | runs the bound items |

`bind` refuses: an item that may cut on the token side (`manifest.token_cuts()`; a transfer hook
has no authority over the tokens moved, so a cut cannot be taken), an item with no token-side
callback, an external template (its code is not the items program's), an `exclusive` item (the
gate does not take part in the armory's equipped count), a composite whose modules break any of
these, a range that does not fit the 64 bytes, and a proof that is not live at bind time.

### 3.4 Execute

1. The source account's Token-2022 `TransferHookAccount.transferring` flag must be set: only a real
   Token-2022 transfer reaches the bindings (a direct call cannot stamp holder memory).
2. The source and destination accounts' mint must be the mint; the `MintGate`, holder states and
   per-binding accounts must be at their derived or recorded addresses.
3. For each binding, in slot order: if its proof is not live now (licence expired or revoked, or
   the vault no longer holds the item) the binding is inert and allows the transfer. Otherwise the
   gate calls `hookwars_items::gate_before` signed by its `["items-signer"]` with the transfer as a
   `TokenSlotArgs` (balances reconstructed to their values before the transfer) and the binding's
   holder-memory range. A refusal fails the transfer; new holder bytes are written back.
4. When the mint is `strict` and a bound module keeps holder memory, a transfer to or from a wallet
   without a `HolderState` is refused (the venue is exempt). Projects create holder states next to
   token accounts; `open_holder` is permissionless.

### 3.5 Trust boundary

- The items program accepts `["items-signer"]` under the gate (`GATE_ITEMS_SIGNER`) only in
  `gate_before`. That entry never records a cut (a module that answers a cut fails the call), never
  touches an `EquipState`, a vault or the raid ledger, and runs the same template code as
  `before_transfer`.
- Template code finds the mint's buy-and-sell venue through `own_pool`, which now also accepts the
  mint's `MintGate` (owner the gate, address `["gate-mint", mint]`) and reads its `venue`.
- The market sells licences for a Token-2022 mint only when that mint's transfer hook program is the
  gate; payment and terms are unchanged.

### 3.6 Limits

- No cuts, no burns, no fee discounts on external tokens: only refusals, holder stamps and reads.
  Income comes from licence terms (protocol fee, template author share, holder).
- Pool-side templates do nothing on external tokens (no units pool runs them).
- War, relation and raid templates need units launch pools and war state: they bind but read
  nothing useful; the app lists the templates that make sense on external tokens.
- Holder memory needs a `HolderState` per wallet; a non-strict mint lets transfers through for
  wallets without one (their stamps are then skipped).
- Call depth: a DEX that calls Token-2022 adds two levels (gate, items); a router in front of a DEX
  reaches the runtime's limit of 5.

## 4. New owner settings

| Setting | Where | Meaning |
| --- | --- | --- |
| `max_supply` per template | `Supply` | The cap on new copies of each template |
| `loot_reserve` per template | `Supply` | Copies kept for loot and craft drops |
| `minter_rule`, `minter` per template | `Supply` | Who issues |
| premium set (`default_access`, `allowed_access`) | `Template` | Which templates start Licensed |
| `paused` | `GateConfig` | Stops new registrations and binds (never blocks transfers) |
| `require_holder_state_default` | `GateConfig` | Default `strict` for new registrations |

No values are set here; the TEST values in the suites are not decisions.
