# Hookwars spec 01: token slots

Status: specification, 2026-10-08, revised to the integration rulings of `00-overview.md` section 9
(R1, R8, R11, R12; War kind; Pool slots with a token half; R15 events). Nothing is built. Follows
`00-overview.md` (names, seeds, parameters, rules). Upstream reference: Bordrless `43688f3`,
`docs/hooks-v2.md` sections 1 and 2, and the code cited by `file:line` below (paths relative to the
repository root).

This part covers the token program (`bordrless_token`, crate name kept per D-3) and the hook
protocol crate (`crates/bordrless-hook`). It replaces the single token hook of a Hookwars mint with
a **slot table**, defines how the token program calls and merges slot items, how each item is held
to its own range of the 64 hook-data bytes, where an item's cut goes, and adds `set_slot_item`,
`set_vote_lock` and `touch`.

## 0. What stays exactly as upstream

- Holdings: `["holding", mint, owner]`, one per (mint, owner), `create_holding` idempotent, rent,
  delegation, freezing, `set_authority`, `update_metadata`
  (`programs/bordrless_token/src/instructions/holding.rs:10-150`).
- The **legacy hook** path: a mint whose `hook_program` is set (a mint launched the upstream way
  with the kit, Half-Life or a custom hook). `HookCall`, `TokenHookArgs`, `HookReturn`,
  `apply_deltas`, `write_hook_data` and every upstream check apply to it unchanged
  (`programs/bordrless_token/src/hooks.rs:16-166`,
  `programs/bordrless_token/src/instructions/transfer.rs:55-171`,
  `programs/bordrless_token/src/instructions/holding.rs:187-221`). Bridge-wrapped and LP mints have
  no hook and no slots.
- The safety rules of hooks-v2 section 1: an answer is read only when the callback and flags allow
  it, only from the hook program's own return data, return data is cleared before every call
  (`hooks.rs:90`), at most `MAX_DELTAS` (3) deltas, no zero delta, no account twice
  (`crates/bordrless-hook/src/lib.rs:392-417`), every delta target an extra that is a writable,
  unfrozen holding of the mint and not one the instruction manages (`hooks.rs:122-148`).
- A hook never receives a user's signature: the prefix accounts are passed read-only and
  non-signing (`hooks.rs:66-70`); only this program's `["hook-authority", hook_program]` PDA signs.
- One hook signer per hook program, bump kept with the mint (`hooks.rs:36-59`,
  `programs/bordrless_token/src/state.rs:44-47`).
- Token prefix accounts: `hook_signer`, `mint`, `source`, `destination`, `authority`
  (`TOKEN_PREFIX_ACCOUNTS = 5`, `crates/bordrless-hook/src/lib.rs:36`). The owners travel in the
  arguments; registries resolve them with `Seed::SourceOwner` / `Seed::DestinationOwner`
  (`lib.rs:496-507`). `docs/architecture.md` lists seven prefix accounts; the code has five and wins.

A mint uses **either** the legacy hook **or** a slot table, never both (1.4, `MixedHookModes`).

## 1. The slot table in the mint (D-1)

### 1.1 `Slot`

```rust
pub const MAX_SLOTS: usize = /* parameter MAX_SLOTS, to measure (M1) */;

/// Bounds a slot keeps for life. Set at `create_mint`, never changed.
#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SlotBounds {
    /// Most an item in this slot may cut from one transfer, in bps of the amount. 0: no cuts.
    pub max_cut_bps: u16,
    /// Whether an item may refuse operations (declared; see 3.6: not enforceable at run time).
    pub may_refuse: bool,
    /// Whether an item may write this slot's hook-data range.
    pub may_write_data: bool,
    /// Whether an item may answer `touch` (section 5).
    pub may_answer_touch: bool,
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slot {
    /// `SlotKind` (00 section 4.1): Fee 0, Reward 1, Defense 2, Relation 3, Pool 4, Locked 5, War 6.
    pub kind: u8,
    /// `EquipRule` (00 section 4.2), copied at creation so it is fixed with the bounds. The token
    /// program never acts on it; `hookwars_armory` does.
    pub equip_rule: u8,
    /// Fixed bounds.
    pub bounds: SlotBounds,
    /// First byte of this slot's range in `Holding.hook_data`; 0 with `data_len` 0.
    pub data_offset: u8,
    /// Length of the range. Item slots: including the epoch byte (3.4). Locked: the legacy program's
    /// own bytes, starting at 0, no epoch byte. 0 when the slot keeps no data.
    pub data_len: u8,
    /// The equipped item (armory `Item`, `["item", item_mint]`), or the default key when empty.
    /// Default for a Locked slot (no item record).
    pub item: Pubkey,
    /// The program that runs for the equipped item (default when empty).
    pub program: Pubkey,
    /// Token-side callbacks and answers (section 2.3). 0 when empty.
    pub flags: u16,
    /// Pool slots only: the upstream `pool_flags` the launchpad uses to call the item's pool half
    /// (03). 0 for every other kind. The token program stores it and never reads it.
    pub pool_flags: u16,
    /// The only holding a cut of this slot may credit (R1): `["holding", mint, PDA(["equip", mint,
    /// slot], <ITEMS_ID>)]`. Set at `create_mint` for every slot with `max_cut_bps > 0`; default
    /// otherwise. Fixed for life (it names the slot, not the item).
    pub equip_vault: Pubkey,
    /// Bump of this program's signer `["hook-authority", program]` (token callbacks).
    pub signer_bump: u8,
    /// Pool slots only: bump of the launchpad's signer `["hook-authority", program]` under
    /// `<LAUNCH_ID>` (03 calls the pool half with it). 0 otherwise.
    pub launch_signer_bump: u8,
    /// Increments (wrapping, skipping 0) each time the item changes; tags this slot's data in every
    /// holding (3.4). Starts at 1. Unused by Locked slots.
    pub data_epoch: u8,
}
```

Bytes per `Slot` (Borsh, fixed): `kind 1 + equip_rule 1 + bounds 5 + data_offset 1 + data_len 1 +
item 32 + program 32 + flags 2 + pool_flags 2 + equip_vault 32 + signer_bump 1 + launch_signer_bump 1
+ data_epoch 1 = 112`.

No royalty field: the token program does not compute or pay royalties (R1). The site shows an
item's royalty from its armory `Item` record (02), not from the mint.

### 1.2 New `Mint` layout

Upstream fields keep their order and offsets (`state.rs:10-50`), so the documented `memcmp`
offsets (`decimals` 9, `supply` 10) still hold. New fields are appended after `reserved`:

```rust
pub struct Mint {
    // ... every upstream field, unchanged, through `reserved: [u8; 31]` ...
    /// The armory's `SlotAuthority` for this mint (`PDA(["slots", mint], <ARMORY_ID>)`), which
    /// alone may call `set_slot_item` and `set_vote_lock`; `None` when the table never changes.
    pub slot_authority: Option<Pubkey>,
    /// How many entries of `slots` are in use (0: no slot table).
    pub slot_count: u8,
    /// The table, in call order. Entries at `slot_count..` are zero.
    pub slots: [Slot; MAX_SLOTS],
}
```

Size: `Mint::LEN = 519 + 33 + 1 + 112 * MAX_SLOTS` bytes (M1 built 113 per slot, see M1 notes;
measured: 1,005 bytes at `MAX_SLOTS` 4, rent 7,885,680 lamports against 4,503,120 upstream (R21 adds `may_burn`: 1,009 bytes, rent 7,913,520, measured on branch r20),
`budgets.rs::the_slot_table_size_and_rent`) (upstream `Mint` is 519 bytes including
the 8-byte discriminator: 511 from the fields of `state.rs:10-50` plus 8; the SDK's integration
guide `docs/integration/01-the-standard.md` lists the same 519). Every mint pays this size, slots or
not; the extra rent per mint is to measure once `MAX_SLOTS` is set. A separate `SlotTable` account
was rejected by D-1 because it adds an account to every transfer.

### 1.3 Helpers

- `Mint::uses_slots() -> bool`: `slot_count > 0`.
- `Mint::locked_slot() -> Option<(u8, &Slot)>`: the Locked slot, if any (at most one, 1.4).
- `Mint::token_called_slots(op, phase)`: the slots this program calls for `op` and `phase` (2.1).

### 1.4 Rules of a slot table (checked by `create_mint`)

| Rule | Error |
| --- | --- |
| `slot_count <= MAX_SLOTS` | `InvalidSlotTable` |
| A mint with `slot_count > 0` has `hook_program = None`, `hook_flags = 0`, `hook_authority = None` | `MixedHookModes` |
| `slot_authority` is `None` or exactly `PDA(["slots", mint], <ARMORY_ID>)` | `InvalidSlotAuthority` |
| `kind` is one of the seven values; `equip_rule` one of the three | `InvalidSlotTable` |
| A Locked slot has `equip_rule = Locked`, a non-default `program`; at most one Locked slot | `InvalidSlotTable` |
| A non-Locked slot may have `equip_rule = Locked`: it starts empty (R12), the armory equips it once at launch and never again (02); kind and rule pairs 02 does not allow are refused | `InvalidSlotTable` |
| If any non-Locked slot exists, `slot_authority` is `Some` | `InvalidSlotAuthority` |
| `bounds.max_cut_bps > 0` only for kinds Fee, Reward, Relation, and Locked when its flags have `TRANSFER_RETURNS_DELTA`; Defense, Pool and War slots have 0 at the token level (a Pool slot's swap cuts are 03's, under `MAX_POOL_ITEM_CUT_BPS`) | `InvalidSlotTable` |
| Sum of `max_cut_bps` over all slots `<= 10_000` | `InvalidSlotTable` |
| Item slots with `max_cut_bps > 0` number at most `MAX_CUTTING_SLOTS`, and `MAX_CUTTING_SLOTS + (1 if the Locked slot may answer deltas) <= MAX_DELTAS` (R1) | `TooManyCuttingSlots` |
| `may_write_data` implies `data_len >= 2` for an item slot (epoch byte plus at least one byte); otherwise `data_len = 0` | `InvalidSlotTable` |
| A Locked slot's range starts at byte 0 and has no epoch byte (the kit: `data_len = 32`, R8) | `InvalidSlotTable` |
| War slots have `data_len = 0` and no token flags ever | `InvalidSlotTable` |
| Pool slots may have a range (a template with a token half, such as Raid, stamps points) | |
| `data_offset` is assigned by the program in slot order (not passed): the Locked range first (from byte 0), then item ranges packed after it; total `<= HOOK_DATA_LEN` (64). With the kit, 32 bytes are left for item ranges (R8) | `InvalidSlotTable` |
| `may_answer_touch` only with `may_write_data` and kind Reward, Defense, Relation or Pool | `InvalidSlotTable` |

Every cutting slot's `equip_vault` is derived and stored here; the holding itself is created by the
items program when the first item is equipped (04), and `set_slot_item` checks it exists (4.2).

## 2. Calling slots

### 2.1 Which slots run

On every transfer and `burn` of a slot mint, and on `mint_to` for the Locked slot only, the token
program walks `slots[0..slot_count]` in index order and calls each slot that is:

- not empty (an item is equipped, or it is the Locked slot), and
- not kind War (no callbacks, ever), and
- subscribed to the callback by its token `flags`.

Pool slots are called here **only** for the token callbacks their `flags` name (the token half of a
template such as Raid); their pool callbacks are the launchpad's (03).

**No mint callbacks for item slots (R12).** Item slots (every kind but Locked) may not carry
`BEFORE_MINT` or `AFTER_MINT`, so `mint_to` never calls them. This keeps a companion launch's
supply `mint_to` inside call depth 5 (section 8).

Every called item sees the **same** pre-state: the original `amount`, the pre-operation balances and
its own range of each holding's pre-operation hook data. Items do not see each other's answers in
the `Before` phase. All answers are checked, then applied together (section 3). In the `After`
phase each subscribed item sees the post-state and the total cut.

### 2.2 Accounts: one slice per called slot

Upstream passes one optional `hook_program`, one optional `hook_signer` and the hook's extras as
`remaining_accounts` (`transfer.rs:43-48`, `programs/bordrless_swap/src/token.rs:11-46`). For a slot
mint the two named optional accounts are passed as this program's id (absent), and
`remaining_accounts` is the concatenation, in slot order, of one **slice** per slot the operation
calls:

```
item slot i:   [ program_i,         // must equal slots[i].program (WrongHookProgram)
                 hook_signer_i,     // ["hook-authority", program_i] at slots[i].signer_bump (BadHookSigner)
                 extras_i ... ]     // the item's registry list (section 7); includes the equip vault
                                    // when the slot cuts
Locked slot:   [ program, hook_signer, extras ... ]   // as upstream
```

Each instruction that runs slots gains one argument, `slot_accounts: Vec<u8>`: the number of
`extras_i` for each called slot, in order. The token program checks that the slices exactly cover
`remaining_accounts` (`SlotAccountsMismatch`). Slots the operation does not call have no slice and
no entry. The same pattern exists upstream for the DEX, whose instruction arguments say how many
accounts each hook slice takes (hooks-v2 "The extra-accounts registry").

Two slots may run the same program (two `hookwars_items` items): they get two slices, each with the
same program and signer accounts. Account and byte cost per slice is to measure (M1).

### 2.3 Slot flags

Token `flags` reuse upstream `token_flags` bits (`crates/bordrless-hook/src/lib.rs:48-67`) and add
one:

| Bit | Name | Meaning |
| --- | --- | --- |
| 0..5 | `BEFORE_TRANSFER` ... `AFTER_BURN` | as upstream |
| 6 | `TRANSFER_RETURNS_DELTA` | `before_transfer` may answer a cut |
| 7 | `WRITES_HOOK_DATA` | `before_*` and `on_touch` may answer this slot's range |
| 8 | `ANSWERS_TOUCH` (new) | `on_touch` runs (section 5) |

`slot_flags::ALL = (1 << 9) - 1`. Allowed per kind (`SlotFlagsNotAllowed` otherwise, checked at
`set_slot_item`, and at `create_mint` for the Locked slot):

| Kind | Allowed token flags |
| --- | --- |
| Fee | transfer callbacks, `TRANSFER_RETURNS_DELTA` |
| Reward | transfer and burn callbacks, `TRANSFER_RETURNS_DELTA`, `WRITES_HOOK_DATA` (if `may_write_data`), `ANSWERS_TOUCH` (if `may_answer_touch`) |
| Defense | transfer and burn callbacks, `WRITES_HOOK_DATA` (if `may_write_data`), `ANSWERS_TOUCH` (if `may_answer_touch`); never `TRANSFER_RETURNS_DELTA` |
| Relation | as Reward |
| Pool | transfer callbacks, `WRITES_HOOK_DATA` (if `may_write_data`), `ANSWERS_TOUCH` (if `may_answer_touch`); never `TRANSFER_RETURNS_DELTA` (pool-side cuts are 03's, R2) |
| War | none (0) |
| Locked | `token_flags::ALL` (upstream meaning, mint callbacks included), never `ANSWERS_TOUCH` |

No item slot may carry `BEFORE_MINT` or `AFTER_MINT` (R12). `TRANSFER_RETURNS_DELTA` additionally
needs `bounds.max_cut_bps > 0`; `WRITES_HOOK_DATA` needs `bounds.may_write_data`.

### 2.4 Two calling conventions

| Slot | Convention | Arguments | Answer |
| --- | --- | --- | --- |
| Locked | **legacy**, upstream | `TokenHookArgs` (`lib.rs:167-201`) with the **full** 64 bytes of each holding (R8) | `HookReturn` (`lib.rs:265-281`); only the bytes inside the slot's range are applied |
| Fee, Reward, Defense, Relation, Pool | **slot** (new) | `TokenSlotArgs` (2.5): only the slot's range | `SlotReturn` (3.1) |

The legacy convention lets the kit (and Half-Life or `tax_hook`, if used locked) sit in a Locked
slot without being rewritten; the kit's only change is its `init` check (R9, interfaces). The
callback names are the same in both conventions (`before_transfer`, `after_transfer`,
`before_mint`, `after_mint`, `before_burn`, `after_burn`, upstream discriminators `lib.rs:100-112`);
the slot convention adds `on_touch` (`discriminators::ON_TOUCH = sha256("global:on_touch")[..8]`,
computed when built). A slot-convention program (`hookwars_items`, 04) implements those names with
`TokenSlotArgs`. A legacy program is never put in a non-Locked slot (`set_slot_item` is signed only
by the armory, which equips only registered templates, 02).

### 2.5 `TokenSlotArgs`

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenSlotOp { Transfer, Burn, Touch }

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct TokenSlotArgs {
    pub op: TokenSlotOp,
    pub phase: Phase,                 // upstream enum
    /// Which slot of the mint is calling, and the item equipped in it. Trustworthy: only this
    /// program can sign with the hook signer the item receives as account 0.
    pub slot: u8,
    pub item: Pubkey,
    pub mint: Pubkey,
    pub source: Pubkey,               // the holding for a touch
    pub destination: Pubkey,          // the mint for a burn; the holding for a touch
    pub source_owner: Pubkey,
    pub destination_owner: Pubkey,    // default for a burn
    /// Who signed: the source's owner or delegate; for a touch, the caller.
    pub authority: Pubkey,
    pub authority_is_delegate: bool,
    pub amount: u64,                  // 0 for a touch
    /// This slot's own cut (After phase; 0 before).
    pub delta: u64,
    /// All slots' cuts together (After phase; 0 before).
    pub total_delta: u64,
    pub source_balance: u64,
    pub destination_balance: u64,
    pub decimals: u8,
    pub supply: u64,
    /// The source holding's bytes of this slot's range, without the epoch byte (`data_len - 1`
    /// bytes); all zeros when the range is stale or never stamped (3.4).
    pub source_data: Vec<u8>,
    /// The same for the destination; all zeros for a burn.
    pub destination_data: Vec<u8>,
    /// `touch` only: the caller's payload (at most `MAX_HOOK_DATA`, 256, `lib.rs:41`); empty otherwise.
    pub payload: Vec<u8>,
}
```

## 3. Answers and how they merge

### 3.1 `SlotReturn`

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotReturn {
    /// At most ONE cut (R1), before_transfer only. Its account index must name this slot's
    /// `equip_vault` among the slot's extras.
    pub deltas: Vec<Delta>,
    /// New bytes for this slot's range in the source holding (`data_len - 1` bytes).
    pub source_data: Option<Vec<u8>>,
    /// New bytes for this slot's range in the destination holding (`data_len - 1` bytes).
    pub destination_data: Option<Vec<u8>>,
}
```

What each callback may answer (others refused, `UnsupportedHookReturn`), the slot analogue of
`Allowed::token` (`lib.rs:320-342`):

| Callback | deltas | source_data | destination_data |
| --- | --- | --- | --- |
| `before_transfer` | at most 1, with `TRANSFER_RETURNS_DELTA` | with `WRITES_HOOK_DATA` | with `WRITES_HOOK_DATA` |
| `before_burn` | no | with `WRITES_HOOK_DATA` | no |
| `on_touch` | no | with `WRITES_HOOK_DATA` (the touched holding) | no |
| every `after_*` | nothing is read | | |

A data field must be exactly `data_len - 1` bytes (`SlotDataLength`). Return data is read only when
it is the slot program's own and non-empty, as upstream (`lib.rs:444-460`), and is cleared before
each slot's call, so one item's answer can never be read as the next one's.

### 3.2 Checks per slot, then across slots

For each called item slot `i` in the `Before` phase, with `cut_i` its delta (0 if none):

1. Upstream answer rules (no zero delta) and **at most one delta** (`TooManyDeltas`).
2. The delta's target is the slot's `equip_vault` (`WrongEquipVault`), a writable, unfrozen
   holding of the mint (`InvalidDeltaAccount`, extends `hooks.rs:122-148`).
3. `cut_i <= floor(amount * bounds_i.max_cut_bps / 10_000)` (`SlotCutExceeded`).

For the Locked slot (legacy answer): upstream checks (`Allowed::token(op, phase, flags)`, at most 3
deltas, targets as upstream), then its cut is bound by its `max_cut_bps` like any slot.

Across slots:
- total deltas of all slots on one transfer `<= MAX_DELTAS` (`TooManyDeltas`), which the static
  rule of 1.4 already guarantees and the program checks again;
- `sum cut <= amount` (`DeltaTooLarge`, as upstream `transfer.rs:115`);
- all delta targets distinct across slots (`InvalidDeltaAccount`).

The destination receives `amount - sum cut`. All checks run before any balance or byte is written
(upstream "All are checked before any is written", `hooks.rs:118-148`).

### 3.3 Where an item's cut goes (R1)

A cutting item answers at most **one** delta, paid into its slot's **equip vault**
(`["holding", mint, PDA(["equip", mint, slot], <ITEMS_ID>)]`). The token program does **not**
compute or pay royalties. `settle_equip` in `hookwars_items` (04, permissionless, with a bounty)
later pays the royalty to the item's royalty holding (02) and routes the rest to the item's
destination (burn, war chest, partner, collector). The equip vault belongs to the slot, so a cut
taken under one item and settled after a re-equip is settled by the items program's own records
(04, `EquipState`), never by the token program.

Consequences: `MAX_CUTTING_SLOTS <= MAX_DELTAS` (3), less one when the Locked slot may answer
deltas on the same transfer; a trader never pays more than the cut; royalty logic is entirely off
the transfer path.

**Protocol payouts skip item slots (R16).** A transfer out of a protocol vault (an equip vault, the
`PoolCuts` holding, a `RoyaltyOwner` holding, a war chest, a treaty inbox) must not be cut by the
token's own items, or a Half-Life-style exit fee would tax royalty claims and settlements. Such a
transfer passes `ProtocolSource { program, seeds }` in its arguments; the token program derives
`create_program_address(seeds, program)`, checks it equals the source holding's owner and that
`program` is one of `<ITEMS_ID>`, `<ARMORY_ID>`, `<WAR_ID>`, and then calls **only** the `Locked`
slot (whose upstream rules, such as the kit's excluded owners, still apply). Any other transfer
calls every slot. Error `NotProtocolSource`. Its compute cost is to measure (07).

### 3.4 Hook-data ranges and the epoch byte

- Each slot with `data_len > 0` owns bytes `[data_offset, data_offset + data_len)` of every
  holding's `hook_data`.
- **Item ranges: byte `data_offset` is the epoch byte, owned by the token program** (00 section 4.4).
  The item sees and answers only the `data_len - 1` bytes after it; its own layout tag is the first
  byte it sees.
- When the token program reads an item range: if the epoch byte equals `slots[i].data_epoch` it
  passes the bytes, otherwise (0, or an older epoch) it passes zeros: the range is **stale or never
  stamped**.
- When it writes an item's answer: all-zero bytes clear the whole range including the epoch byte;
  anything else writes `data_epoch` into the epoch byte and the bytes after it.
- `set_slot_item` increments `data_epoch` (wrapping 255 to 1). A previous item's data is then stale
  in every holding at once, without touching any holding.
- **Locked range (R8):** no epoch byte, starts at byte 0. The legacy program receives the full 64
  bytes (as upstream) so its layout is unchanged; the token program writes back **only** the bytes
  inside `[0, data_len)` of its answer and ignores the rest. The kit's range is bytes 0..32
  (hooks-v2 4.4), leaving 32 bytes for item ranges.

Known limit (D-8): after 255 equips of one slot an old epoch value comes back. The armory's vote
period bounds how often a slot can change; a 2-byte epoch is the fallback.

### 3.5 Applying

In order: credit every slot's delta (each into its equip vault; the Locked slot's into its own
targets), move the balances (source loses `amount`, destination gains `amount - sum cut`), write
every answered range (source and destination, per slot, 3.4), run the `After` callbacks, emit the
event. Same order as upstream `transfer.rs:104-169`.

### 3.6 Refusals

An item refuses by failing its callback; the whole transaction fails, nothing moves (any failing
CPI). The token program cannot catch a failed CPI, so `bounds.may_refuse = false` is **not
enforceable at run time**: the armory (02) and the template tests (04) must check it before an item
is equipped in such a slot, and the site shows it. Cut bounds, range bounds, the one-delta rule and
vote locks are enforced on every operation.

## 4. Instructions

### 4.1 `create_mint` (changed)

`CreateMintArgs` (`programs/bordrless_token/src/instructions/mint.rs:12-37`) gains:

```rust
pub slot_authority: Option<Pubkey>,
pub slots: Vec<SlotInit>,

pub struct SlotInit {
    pub kind: u8,
    pub equip_rule: u8,
    pub bounds: SlotBounds,
    /// Item slots: range length including the epoch byte, or 0. Locked: the legacy program's range
    /// length from byte 0 (the kit: 32), or 0.
    pub data_len: u8,
    /// Locked slot only: the program and its upstream token_flags. Every other slot is created
    /// empty and equipped by the armory (R12).
    pub locked_program: Option<Pubkey>,
    pub locked_flags: u16,
}
```

Checks: upstream `create_mint` checks (`mint.rs:82-104`), then every rule of 1.4. The program
assigns `data_offset` (Locked first from 0, then item ranges in slot order), derives and stores each
cutting slot's `equip_vault`, sets `data_epoch = 1`, and for a Locked slot stores `program`,
`flags = locked_flags` and `signer_bump` from `Mint::hook_signer(&program)` (`state.rs:61-65`).

**Non-Locked slots start empty (R12).** The launchpad equips launch items through the armory
before the supply `mint_to` (03); `mint_to` calls only the Locked slot in any case.

Event `SlotsInitialized` (section 6), emitted right after the upstream `MintCreated`.

### 4.2 `set_slot_item` (new)

Accounts:

| Account | |
| --- | --- |
| `slot_authority` | signer: `PDA(["slots", mint], <ARMORY_ID>)` by CPI from the armory; equal to `mint.slot_authority` (`InvalidSlotAuthority`) |
| `mint` | writable |
| `program` | the new item's program (executable), or this program's id to empty the slot |
| `equip_vault` | the slot's equip vault holding when the slot cuts (must exist), else this program's id |
| event authority, this program | |

Arguments: `slot: u8`, `item: Pubkey` (default to empty), `flags: u16`, `pool_flags: u16`.

| Check | Error |
| --- | --- |
| `slot < slot_count` | `SlotIndexOutOfRange` |
| `kind != Locked` | `SlotLocked` |
| emptying: `item` default, both flags 0 | `InvalidSlotItem` |
| otherwise: `program` executable, not this program, not `<SWAP_ID>`, `<LAUNCH_ID>`, `<BRIDGE_ID>`, `<ARMORY_ID>`, `<WAR_ID>` | `InvalidSlotItem` |
| `flags` allowed for the kind and bounds (2.3); `pool_flags` 0 unless kind Pool; War: both 0 | `SlotFlagsNotAllowed` |
| when `flags` has `TRANSFER_RETURNS_DELTA`: `equip_vault` equals `slots[slot].equip_vault` and is an existing holding of the mint | `WrongEquipVault` |

The token program does not check that the program is a registered template or that the item's
manifest fits the bounds: the armory (02) does, and it is the only signer. The bounds are enforced
at every operation regardless (3.2).

Effects: writes `item`, `program`, `flags`, `pool_flags`, `signer_bump`
(`Mint::hook_signer(&program).1`), `launch_signer_bump` for a Pool slot
(`hook_signer(&<LAUNCH_ID>, &program).1`, `lib.rs:476-479`), increments `data_epoch`. Event
`SlotEquipped`. No holding is touched (3.4).

### 4.3 `set_vote_lock` (new, R11)

Votes lock tokens in place: no escrow holding, no transfer.

`Holding.reserved: [u8; 16]` (`state.rs:98-99`, always zero upstream) becomes two fields of the same
total size, so `Holding::LEN` and every upstream offset are unchanged:

```rust
pub struct Holding {
    // ... upstream fields through `hook_data: [u8; 64]` ...
    /// Tokens the armory has locked for a vote; transfers and burns may not take `amount` below it
    /// while `vote_lock_until` is in the future.
    pub vote_locked: u64,       // was reserved[0..8], little-endian
    /// Unix time the lock ends; 0 with no lock.
    pub vote_lock_until: i64,   // was reserved[8..16], little-endian
}
```

Accounts: `slot_authority` (signer, `PDA(["slots", mint], <ARMORY_ID>)`, equal to
`mint.slot_authority`, `InvalidSlotAuthority`), `mint`, `holding` (writable, of `mint`,
`MintMismatch`), event authority, this program.

Arguments: `amount: u64`, `until: i64`. The armory computes the lock that should stand (for
example the largest of the holder's open votes) and sets it; `amount = 0` or a past `until` clears
it.

Checks: `amount <= holding.amount` (`VoteLockExceedsBalance`); `holding` not frozen (`Frozen`).
Event `VoteLockSet { mint, holding, owner, amount, until, ts }`.

Enforcement in `transfer` and `burn` (slot mints and legacy mints alike, since the field is zero for
every holding the armory never touched): when `now < vote_lock_until`,
`source.amount - amount >= vote_locked`, else `VoteLocked`. Delegates are bound the same way.
`close_holding` needs `amount == 0`, which a live lock already prevents.

### 4.4 `transfer`, `mint_to`, `burn` (changed)

- New argument `slot_accounts: Vec<u8>` (2.2), appended after `amount`. Empty for a legacy mint.
- Legacy mint (`hook_program` set): upstream path; `slot_accounts` must be empty
  (`SlotAccountsMismatch`); the vote lock check of 4.3 applies.
- Slot mint: the named `hook_program` and `hook_signer` accounts are this program's id; slots run
  as in sections 2 and 3. The Locked slot runs with the legacy convention inside the same walk.
- `mint_to` calls only the Locked slot (R12); `mint_to` and `burn` never take cuts (upstream:
  only `before_transfer` answers deltas, `lib.rs:320-342`).
- Prefix per slot call: `[slot hook_signer, mint, source, destination, authority]`, as upstream
  (`hooks.rs:66-70`, `mint.rs:175-180`, `transfer.rs:212-217`).

### 4.5 `close_holding` (rule restated)

Unchanged in spirit (`holding.rs:167-185`): an empty holding closes unless a hook still keeps data
in it. For a slot mint, an item range "keeps data" only when its epoch byte equals the slot's
current `data_epoch` and any of its bytes is non-zero; stale ranges do not block. The Locked range
blocks when any byte in `[0, data_len)` is non-zero (`HookDataNotEmpty`).

### 4.6 `write_hook_data` (restricted)

Kept for legacy mints as upstream. For a slot mint it is allowed only for the Locked slot's
program (the kit's `claim` uses it, `programs/bordrless_kit/src/instructions/claim.rs:119`); the
token program writes only the bytes inside the Locked range (R8). For any other slot it is refused
(`HookDataNotWritable`): item slots change data only through their answers and `touch`.

### 4.7 `set_hook` and `set_authority(Hook)`

Refused for a slot mint (`MixedHookModes`): its `hook_authority` is `None` by rule 1.4.

## 5. `touch(holding, slot_index, payload)` (new)

Lets a program (the war program's bounty and quest claims, 05) have one slot's item update one
holding's range with no transfer.

Accounts:

| Account | |
| --- | --- |
| `caller` | signer: any account; reaches the item as `TokenSlotArgs.authority` and as the prefix `authority` |
| `mint` | read-only |
| `holding` | writable, of `mint` (`MintMismatch`) |
| `program`, `hook_signer` | the slot's program and signer, checked as in 2.2 |
| event authority, this program | |
| remaining | the slot's `extras` |

Arguments: `slot_index: u8`, `payload: Vec<u8>` (at most `MAX_HOOK_DATA`), `extra_count: u8`.

| Check | Error |
| --- | --- |
| `slot_index < slot_count`, slot not empty | `SlotIndexOutOfRange`, `SlotEmpty` |
| kind not Locked or War; slot has `ANSWERS_TOUCH` | `TouchNotSupported` |
| holding not frozen | `Frozen` |
| `payload.len() <= MAX_HOOK_DATA` | `PayloadTooLong` |
| extras exactly `extra_count` | `SlotAccountsMismatch` |

Call: `on_touch` with `TokenSlotArgs { op: Touch, phase: Before, slot: slot_index, item, mint,
source: holding, destination: holding, source_owner: holding.owner, destination_owner:
holding.owner, authority: caller, amount: 0, source_balance: holding.amount, destination_balance:
holding.amount, source_data: range, destination_data: range, payload, .. }`.

Answer: `source_data` only (3.1). The item decides whether to act by checking `args.authority`
(for example: only the war program's signer may reset raid points) and `args.payload`.
`args.authority` is trustworthy: the token program verified it as a signer, and only the token
program can sign the hook signer the item receives.

When the answer writes the range, the token program emits the upstream event `HookDataWritten
{ mint, holding, owner, data }` (`events.rs:63-70`) with the holding's full 64 bytes after the write.
No `after_touch`.

## 6. Events

| Event | Fields | Notes |
| --- | --- | --- |
| `Transferred` | upstream fields (`events.rs:47-61`) plus `slot_cuts: Vec<SlotCut>` | `deltas` keeps listing every credit as `DeltaApplied` |
| `SlotCut` (struct) | `slot: u8, item: Pubkey, cut: u64` | one per slot that cut; `item` default for the Locked slot |
| `SlotsInitialized` | `mint, slot_authority, slots: Vec<SlotInfo>` | `SlotInfo` = kind, equip_rule, bounds, data_offset, data_len, equip_vault, locked program; emitted after `MintCreated` |
| `SlotEquipped` | `mint, slot: u8, old_item, new_item, program, flags, pool_flags, data_epoch, ts` | the one name for an item change (empties included) |
| `VoteLockSet` | `mint, holding, owner, amount, until, ts` | |
| `HookDataWritten` | upstream (`events.rs:63-70`) | also emitted by `touch` when the item writes |
| `Minted`, `Burned`, `MintCreated` | unchanged | upstream indexers keep reading them |

## 7. Registries: one program, many items

Upstream registry: `["bordrless-hook-accounts", mint_or_pool]` under the hook program
(`lib.rs:490-493`). `hookwars_items` may run several items on one mint, so an item slot's registry
is `["bordrless-hook-accounts", mint, item]` under the item's program (00 section 4.3; `item` = the
armory `Item` account key). A cutting item's list includes its slot's equip vault holding. The
Locked slot keeps the upstream two-seed registry. Clients resolve each called slot's list with the
slot prefix (5 accounts) and the owners, as upstream `HookAccountList::resolve` (`lib.rs:571-608`).
A new helper `slot_accounts_address(program, mint, item)` joins `hook_accounts_address`;
`write_registry` (`lib.rs:612-`) is reused with the three-seed signer.

## 8. Call depth

Invoke height counts the top-level instruction as 1; Solana's limit is 5 (hooks-v2 section 6).
Items are leaf programs: a slot item makes no CPI (04), so a slot call adds exactly one level.

| Path | Deepest call | Upstream reference | Measured |
| --- | --- | --- | --- |
| wallet transfer | token 1, item 2 | 2 | 2 (`budgets.rs`, 1 to 3 slots) |
| DEX swap delivery | DEX 1, token 2, item 3 | 3 (hooks-v2 6 table) | 3 (`budgets.rs`, buy and sell, 1 to 3 slots, ordinary pool) |
| launch supply `mint_to` | launch 1, token 2, Locked kit 3; item slots not called (R12) | | to measure |
| graduation top-up and reserve burn | launch 1, DEX 2, token 3, item 4 | 4 | to measure |
| companion `create_launch` then `mint_to` | companion 1, launch 2, token 3, Locked kit 4; items not called (R12) | 5 for `create_launch` from a companion (`docs/companions.md:75`) | to measure; must stay at most 5 |
| armory equip at launch | launch 1, armory 2, token `set_slot_item` 3 (from a companion: 4) | | to measure |
| armory vote | armory 1, token `set_vote_lock` 2 | | to measure |
| war `touch` | war 1, token 2, item 3 | | to measure |

Bytes, account locks, trace entries and compute per extra slice are to measure in M1 and recorded
in 07; `MAX_SLOTS` and `MAX_CUTTING_SLOTS` are set from them.

## 9. Errors (new)

`InvalidSlotTable`, `InvalidSlotAuthority`, `MixedHookModes`, `TooManyCuttingSlots`,
`SlotIndexOutOfRange`, `SlotLocked`, `SlotEmpty`, `InvalidSlotItem`, `SlotFlagsNotAllowed`,
`WrongEquipVault`, `SlotCutExceeded`, `SlotDataLength`, `SlotAccountsMismatch`,
`TouchNotSupported`, `PayloadTooLong`, `VoteLocked`, `VoteLockExceedsBalance`. Upstream errors
keep their codes; new ones are appended after `NotHookAuthority` (`error.rs:61`). Existing errors
reused: `TooManyDeltas`, `ZeroDelta`, `InvalidDeltaAccount`, `DeltaTooLarge`,
`UnsupportedHookReturn`, `WrongHookProgram`, `BadHookSigner`, `HookDataNotEmpty`,
`HookDataNotWritable`, `Frozen`, `MintMismatch`.

Open at this level: whether non-slot mints should keep the upstream size (a separate table) if
`MAX_SLOTS * 112` bytes of rent on every mint proves too much (D-1, revisit after M1).

## 10. Tests

Upstream suites that must still pass, changed only by the added `slot_accounts` argument in
builders (legacy mints): `programs/tests/tests/token.rs`, `token_hooks.rs`, `hook_signers.rs`,
`hook_authority.rs`, `half_life.rs`, `kit.rs`, `kit_money.rs`, `launch.rs`, `launch_rules.rs`,
`launch_configs.rs`, `launch_money.rs`, `swap.rs`, `swap_hooks.rs`, `bridge.rs`, `companion.rs`,
`marketplace.rs`, `vectors.rs`.

New LiteSVM tests (M1), with `hook_tester` extended to the slot convention:

- the table rules of 1.4, one test per error;
- two cutting slots plus a Locked Half-Life: one delta each into the right equip vault
  (`WrongEquipVault` otherwise), per-slot bound (`SlotCutExceeded`), total `MAX_DELTAS`
  (`TooManyDeltas`), sum bound (`DeltaTooLarge`);
- a slot answering more than one delta (`TooManyDeltas`) or a field its flags do not allow
  (`UnsupportedHookReturn`);
- ranges: wrong length (`SlotDataLength`); two item ranges never overlap after a sequence of
  transfers; stale data after `set_slot_item` reads as zeros and does not block close;
- the kit in a Locked slot with range 0..32 next to an item range at 32..: the kit's answer bytes
  outside 0..32 are ignored; `write_hook_data` from the kit writes only 0..32;
- `mint_to` on a slot mint calls the Locked slot and no item slot;
- `set_slot_item` and `set_vote_lock` signed by anything but the armory PDA (`InvalidSlotAuthority`);
- vote locks: a transfer or burn below the locked amount before `until` fails (`VoteLocked`), the
  same after `until` succeeds; a lock above the balance (`VoteLockExceedsBalance`); a delegate is
  bound;
- `touch` by an unexpected caller: the item ignores it; `touch` on a slot without `ANSWERS_TOUCH`;
  `HookDataWritten` emitted on a write;
- a Pool slot with a token half stamps a range on transfer and is not called on `mint_to`;
- a War slot is never called;
- a forged callback (an item called by anything but the token program's signer) is refused by the
  item, as upstream `half_life.rs` "a forged callback is refused";
- measurements: bytes, account locks, trace entries, compute per slice; call depth per path in 8.

## 11. Interfaces

### Required from 02 (`hookwars_armory`)

- `SlotAuthority` = `PDA(["slots", mint], <ARMORY_ID>)`; the armory signs `set_slot_item` and
  `set_vote_lock` with it by CPI and is the only program that does.
- Before `set_slot_item` the armory checks: the program is a registered template program
  (immutable or managed, 00 rule 3), the item's manifest fits the slot's kind and `bounds` (it reads
  `Mint.slots[slot]`), `may_refuse` honoured, the item's data bytes `<= data_len - 1`, no mint
  callbacks, and the equip rule (vote, performance, notice). It passes `item`, `program`, `flags`,
  `pool_flags` from its `Item` and `Template` records.
- The armory computes each holder's standing vote lock and calls `set_vote_lock(amount, until)`; it
  clears locks when votes end.
- The armory triggers the item program's registry creation at `["bordrless-hook-accounts", mint,
  item]` when equipping.

### Required from 03 (launchpad, DEX)

- The launchpad creates a Hookwars mint with `slots` (non-Locked slots empty), `slot_authority =
  PDA(["slots", mint], <ARMORY_ID>)`, `hook_program = None`, `hook_authority = None`, and equips
  launch items through the armory **before** the supply `mint_to` (R12).
- The DEX's token side (`programs/bordrless_swap/src/token.rs:11-46`) carries, for a slot mint, the
  concatenated slot slices and `slot_accounts` instead of one hook slice; the client builds them
  per side.
- The launchpad calls Pool slots' pool halves itself, reading `Mint.slots` (kind Pool, `program`,
  `pool_flags`, `launch_signer_bump`, `item`); the token program calls only their token halves.
- Kit change (R9): `init`'s check `mint.hook_program == Some(KIT_ID)`
  (`programs/bordrless_kit/src/instructions/init.rs:92-96`) accepts a slot mint whose Locked slot
  runs the kit with the same flags and range 0..32.

### Required from 04 (`hookwars_items`)

- Implements `before_transfer`, `after_transfer`, `before_burn`, `after_burn`, `on_touch` taking
  `TokenSlotArgs` and answering `SlotReturn` through return data; never `before_mint` or
  `after_mint` (R12).
- Verifies account 0 is the token program's `["hook-authority", <ITEMS_ID>]` signer (canonical bump,
  as upstream hooks do) and that `args.item` matches `Mint.slots[args.slot].item`.
- Makes no CPI from a slot callback.
- A cutting template answers at most one delta, into its slot's equip vault
  `["holding", mint, PDA(["equip", mint, slot], <ITEMS_ID>)]`, and creates that holding when the
  first item is equipped; `settle_equip` pays royalties and destinations from it (R1).
- Declares each template's range as `data_len - 1` visible bytes, its first visible byte being its
  layout tag.
- Publishes its per-(mint, item) registry at `["bordrless-hook-accounts", mint, item]`.

### Required from 05 (`hookwars_war`)

- Calls `touch(holding, slot_index, payload)` with its own signer PDA as `caller`; the template's
  `on_touch` accepts point resets only from that caller.

## M1 implementation notes (2026-10-08)

Built in `programs/bordrless_token`, `crates/bordrless-hook`, test-only `programs/slot_tester` and
`programs/armory_stub`, tests `programs/tests/tests/slots.rs` and `budgets.rs`. Where the build
differs from the text above, the build is what exists; each difference and why:

1. **No `slot_accounts` argument (2.2, 4.4).** Each `Slot` keeps `extra_count: u8`, the number of
   extras its item takes, fixed when it is equipped (`set_slot_item` gains `extra_count`; a
   Locked slot's comes from `SlotInit.locked_extra_count`). The slices are then determined by the
   mint alone, so `transfer`, `mint_to` and `burn` keep their upstream instruction layout and the
   DEX, launchpad, kit, bridge and companion call them unchanged (the DEX passes a slot mint's
   slices as its token-hook extras). `Slot` is 113 bytes. An item whose registry changes length
   after it is equipped makes its slot's operations fail (`SlotAccountsMismatch`) until the armory
   re-equips it.
2. **`create_mint` is unchanged; `create_slot_mint(args, slot_authority, slots)` is new** (4.1),
   so the four upstream callers of `create_mint` are untouched.
3. **R16 is its own instruction**, `transfer_from_protocol(amount, program, seeds)`, with the
   `Transfer` accounts. On a legacy mint it runs the legacy hook as `transfer` does.
4. **Account names.** `set_slot_item` and `touch` call the item's program account `item_program`:
   Anchor's `#[event_cpi]` already adds an account named `program`.
5. **Mint size and the SBF stack.** At 1,005 bytes the `Mint` overflowed two upstream frames at
   build time (`cargo build-sbf` reported 6,528 bytes in the DEX `CreatePool` accounts and 6,272
   in the kit `Init` accounts, against 4,096); the runtime symptom was a corrupted `create_pool`
   argument (`HookDataTooLong`). The DEX `CreatePool` and the kit `Init` now box their `Mint`
   accounts. No other frame overflows at this size; a larger `MAX_SLOTS` must be rebuilt and
   checked for the same warning.
6. **`MAX_SLOTS` = 4 and `MAX_CUTTING_SLOTS` = 3 are the build values** (`constants.rs`) for the
   measurements below; they remain parameters (00 section 6).
7. **Answer errors.** Slot answers use their own `SlotAnswerError` (adds `DataLength`, mapped to
   `SlotDataLength`) so the shared `AnswerError` and the DEX's mapping are unchanged.
8. **War rule.** A War slot needs `data_len` 0, no cut, no data and no touch; `may_refuse` is not
   checked (it is declarative).
9. **Test doubles.** `hook_tester` cannot also take the slot convention (the callback names are
   the same, the argument types differ), so `slot_tester` is a separate test-only item program.
   `armory_stub` is declared at `<ARMORY_ID>` and forwards a token instruction signed as
   `["slots", mint]`; it is never deployed, and the real armory (M2) takes the same id.
   `hook_tester` stands in for a Locked legacy hook; the kit itself in a Locked slot needs the
   R9 change to its `init`, which is 03's.
10. **Not tested, unreachable:** `DeltaTooLarge` across slots, since the table rule caps the sum of
    `max_cut_bps` at 10,000.

### Measured (M1, `programs/tests/tests/budgets.rs`)

Ordinary DEX pool (no pool hook), each item answering a 1-unit cut, `slot_tester` items (a test
item that rewrites a 4,000-byte script account on every call, so its compute is an upper bound
for a leaf item, not a template's figure). "With table" puts every account the instruction
names in one lookup table. 0 slots is a plain upstream mint.

| Path | Cutting slots | Keys | v0 bytes | With table | Trace | Height | CU |
| --- | --- | --- | --- | --- | --- | --- | --- |
| transfer | 0 | 7 | 363 | 273 | 3 | 2 | 12,277 |
| transfer | 1 | 11 | 495 | 281 | 4 | 2 | 30,028 |
| transfer | 2 | 13 | 563 | 287 | 5 | 2 | 46,363 |
| transfer | 3 | 15 | 631 | 293 | 6 | 2 | 62,856 |
| buy | 0 | 14 | 609 | 302 | 7 | 3 | 51,032 |
| buy | 1 | 18 | 741 | 310 | 8 | 3 | 70,848 |
| buy | 2 | 20 | 809 | 316 | 9 | 3 | 88,689 |
| buy | 3 | 22 | 877 | 322 | 10 | 3 | 106,733 |
| sell | 0 | 14 | 609 | 302 | 7 | 3 | 51,036 |
| sell | 1 | 18 | 741 | 310 | 8 | 3 | 70,892 |
| sell | 2 | 20 | 809 | 316 | 9 | 3 | 88,732 |
| sell | 3 | 22 | 877 | 322 | 10 | 3 | 106,775 |

Each further slot of the same item program adds 2 keys (its script and its equip vault; the
program and signer repeat), 68 v0 bytes without a table and 6 with one, 1 trace entry, no
height, and with this test item 16,335 and 16,493 compute units on a transfer (the second and
third slot) and 17,841 and 18,044 on a buy; the first slot also pays the slot path's fixed cost
(transfer 12,277 to 30,028).

**Proposal.** `MAX_SLOTS` = 4 and `MAX_CUTTING_SLOTS` = 3 (2 when the Locked slot may cut): three
cutting slots fit a plain DEX buy at 877 bytes without a table and 322 with one, 10 trace entries
and height 3. Before either is fixed, M3 must measure the same three slots on a **launch pool**
swap and on buy-and-graduate, where upstream already uses about 1,200 of 1,232 bytes for a launch
with one custom hook (`programs/half_life/README.md`).
