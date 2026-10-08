# Hookwars spec 01: token slots

Status: specification, 2026-10-08. Nothing is built. Follows `00-overview.md` (names, seeds,
parameters, rules). Upstream reference: Bordrless `43688f3`, `docs/hooks-v2.md` sections 1 and 2,
and the code cited by `file:line` below (paths relative to the repository root).

This part covers the token program (`bordrless_token`, crate name kept per D-3) and the hook
protocol crate (`crates/bordrless-hook`). It replaces the single token hook of a Hookwars mint with
a **slot table**, defines how the token program calls and merges slot items, how each item is held
to its own range of the 64 hook-data bytes, how royalties are taken, and adds `set_slot_item` and
`touch`.

## 0. What stays exactly as upstream

- Holdings: `["holding", mint, owner]`, one per (mint, owner), `create_holding` idempotent, rent,
  delegation, freezing, `set_authority`, `update_metadata`
  (`programs/bordrless_token/src/instructions/holding.rs:10-150`).
- The **legacy hook** path: a mint whose `hook_program` is set (bridge-wrapped mints have none; a
  mint launched with the kit, Half-Life or a custom hook the upstream way). `HookCall`,
  `TokenHookArgs`, `HookReturn`, `apply_deltas`, `write_hook_data` and every upstream check apply to
  it unchanged (`programs/bordrless_token/src/hooks.rs:16-166`,
  `programs/bordrless_token/src/instructions/transfer.rs:55-171`,
  `programs/bordrless_token/src/instructions/holding.rs:187-221`).
- The safety rules of hooks-v2 section 1: an answer is read only when the callback and flags allow
  it, only from the hook program's own return data, return data is cleared before every call
  (`hooks.rs:90`), at most `MAX_DELTAS` (3) deltas per answer, no zero delta, no account twice
  (`crates/bordrless-hook/src/lib.rs:392-417`), every delta target an extra that is a writable,
  unfrozen holding of the mint and not one the instruction manages (`hooks.rs:122-148`).
- A hook never receives a user's signature: the prefix accounts are passed read-only and
  non-signing (`hooks.rs:66-70`); only this program's `["hook-authority", hook_program]` PDA signs.
- One hook signer per hook program, bump kept with the mint (`hooks.rs:36-59`,
  `programs/bordrless_token/src/state.rs:44-47`).
- Token prefix accounts: `hook_signer`, `mint`, `source`, `destination`, `authority`
  (`TOKEN_PREFIX_ACCOUNTS = 5`, `crates/bordrless-hook/src/lib.rs:36`). The owners travel in the
  arguments, and registries resolve them with `Seed::SourceOwner` / `Seed::DestinationOwner`
  (`lib.rs:496-507`). Note: `docs/architecture.md` lists seven prefix accounts; the code has five and
  wins.

A mint uses **either** the legacy hook **or** a slot table, never both (section 1.4,
`MixedHookModes`).

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
    /// `SlotKind` (00 section 4.1): Fee 0, Reward 1, Defense 2, Relation 3, Pool 4, Locked 5.
    pub kind: u8,
    /// `EquipRule` (00 section 4.2), copied at creation so the rule is fixed with the bounds.
    /// The token program never acts on it; `hookwars_armory` does.
    pub equip_rule: u8,
    /// Fixed bounds.
    pub bounds: SlotBounds,
    /// First byte of this slot's range in `Holding.hook_data`; 0 with `data_len` 0.
    pub data_offset: u8,
    /// Length of the range, including the epoch byte (section 3.4); 0 when the slot keeps no data.
    pub data_len: u8,
    /// The equipped item (`Item` account of the armory, `["item", item_mint]`), or the default key
    /// for an empty slot. For a Locked slot: the default key (no item record).
    pub item: Pubkey,
    /// The program that runs for the equipped item (default key when empty).
    pub program: Pubkey,
    /// Which callbacks run and what they may answer (section 2.3). 0 when empty.
    pub flags: u16,
    /// The item's royalty share of its own cuts, in bps (copied from the item at equip).
    pub royalty_bps: u16,
    /// The holding that receives the royalty: `["holding", mint, PDA(["royalty", item], ARMORY)]`.
    /// Default key when `royalty_bps` is 0 or the slot cannot cut.
    pub royalty_holding: Pubkey,
    /// Bump of the caller's signer for `program`: this program's `["hook-authority", program]` for
    /// every kind but Pool; the launchpad's `["hook-authority", program]` under `<LAUNCH_ID>` for a
    /// Pool slot (03 calls Pool slots; this program never does).
    pub signer_bump: u8,
    /// Increments (wrapping, skipping 0) each time the item changes; tags this slot's data in every
    /// holding (section 3.4). Starts at 1.
    pub data_epoch: u8,
}
```

Bytes per `Slot` (Borsh, fixed): `kind 1 + equip_rule 1 + bounds 5 + data_offset 1 + data_len 1 +
item 32 + program 32 + flags 2 + royalty_bps 2 + royalty_holding 32 + signer_bump 1 + data_epoch 1
= 111`.

### 1.2 New `Mint` layout

Upstream fields keep their order and offsets (`state.rs:10-50`), so the documented `memcmp`
offsets (`decimals` 9, `supply` 10) still hold. New fields are appended after `reserved`:

```rust
pub struct Mint {
    // ... every upstream field, unchanged, through `reserved: [u8; 31]` ...
    /// The armory's `SlotAuthority` for this mint (`PDA(["slots", mint], <ARMORY_ID>)`), which
    /// alone may call `set_slot_item`; `None` when the table can never change.
    pub slot_authority: Option<Pubkey>,
    /// How many entries of `slots` are in use (0: no slot table).
    pub slot_count: u8,
    /// The table, in call order. Entries at `slot_count..` are zero.
    pub slots: [Slot; MAX_SLOTS],
}
```

Size: `Mint::LEN = 519 + 33 + 1 + 111 * MAX_SLOTS` bytes (upstream `Mint` is 519 bytes including the
8-byte discriminator: 511 from the fields of `state.rs:10-50` plus 8; the SDK's integration guide
`docs/integration/01-the-standard.md` lists the same 519). Every mint pays this size, slots or not; the extra rent per mint is to measure once
`MAX_SLOTS` is set. An alternative that keeps non-slot mints small (a separate `SlotTable` account)
was rejected by D-1 because it adds an account to every transfer.

### 1.3 Helpers

- `Mint::uses_slots() -> bool`: `slot_count > 0`.
- `Mint::locked_program() -> Option<Pubkey>`: the program of a Locked slot, if any (there is at
  most one, 1.4).
- `Mint::data_writer_ranges()`: the `(offset, len, epoch)` of every slot with `data_len > 0`.

### 1.4 Rules of a slot table (checked by `create_mint`)

| Rule | Error |
| --- | --- |
| `slot_count <= MAX_SLOTS` | `InvalidSlotTable` |
| A mint with `slot_count > 0` has `hook_program = None`, `hook_flags = 0`, `hook_authority = None` | `MixedHookModes` |
| `slot_authority` is `None` or exactly `PDA(["slots", mint], <ARMORY_ID>)` | `InvalidSlotAuthority` |
| `kind` is one of the six values; `equip_rule` is one of the three | `InvalidSlotTable` |
| A Locked slot has `equip_rule = Locked` and a non-default `program`; at most one Locked slot | `InvalidSlotTable` |
| If any slot has `equip_rule != Locked`, `slot_authority` is `Some` | `InvalidSlotAuthority` |
| `bounds.max_cut_bps > 0` only for kinds Fee, Reward, Relation (and Locked, whose program's flags decide); Defense and Pool slots have 0 at the token level (a Pool slot's swap bounds are 03's) | `InvalidSlotTable` |
| Sum of `max_cut_bps` over all slots `<= 10_000` | `InvalidSlotTable` |
| Number of slots with `max_cut_bps > 0` `<= MAX_CUTTING_SLOTS` | `TooManyCuttingSlots` |
| `may_write_data` implies `data_len >= 2` (epoch byte plus at least one byte); otherwise `data_len = 0` | `InvalidSlotTable` |
| A Locked slot that writes data takes the whole 64 bytes (`data_len = 64`, `data_offset = 0`, no epoch byte, section 3.4), so it is then the only data-writing slot | `InvalidSlotTable` |
| Pool slots have `data_len = 0` (the token program never calls them) | `InvalidSlotTable` |
| `data_offset` is assigned by the program in slot order (not passed): the ranges are packed from byte 0 and their sum is `<= HOOK_DATA_LEN` (64) | `InvalidSlotTable` |
| `may_answer_touch` only with `may_write_data` and kind Reward, Defense or Relation | `InvalidSlotTable` |

## 2. Calling slots

### 2.1 Which slots run

On every transfer, `mint_to` and `burn` of a slot mint, the token program walks `slots[0..slot_count]`
in index order and calls each slot that is:

- not empty (`item != default`, or a Locked slot with a program), and
- not kind Pool (Pool slots run from the launchpad's pool hook, 03), and
- subscribed to the callback by its `flags`.

Every called item sees the **same** pre-state: the original `amount`, the pre-operation balances and
its own range of each holding's pre-operation hook data. Items do not see each other's answers in
the `Before` phase. All answers are checked, then applied together (section 3). In the `After`
phase each subscribed item sees the post-state and the total cut.

### 2.2 Accounts: one slice per called slot

Upstream passes one optional `hook_program`, one optional `hook_signer` and the hook's extras as
`remaining_accounts` (`transfer.rs:43-48`, `swap/src/token.rs:11-46`). For a slot mint the two
named optional accounts are passed as this program's id (absent), and `remaining_accounts` is the
concatenation, in slot order, of one **slice** per slot that the operation calls:

```
slice(slot i) = [ program_i,            // must equal slots[i].program (WrongHookProgram)
                  hook_signer_i,        // ["hook-authority", program_i] at slots[i].signer_bump (BadHookSigner)
                  royalty_holding_i,    // only when slots[i].royalty_bps > 0 and max_cut_bps > 0 (WrongRoyaltyHolding)
                  extras_i ... ]        // the item's registry list, resolved by the client
```

Each instruction that runs slots gains one argument, `slot_accounts: Vec<u8>`: the number of
`extras_i` for each called slot, in order. The token program checks that the slices exactly cover
`remaining_accounts` (`SlotAccountsMismatch`). Slots the operation does not call (empty, Pool, or
not subscribed) have no slice and no entry. The same pattern exists upstream for the DEX, whose
instruction arguments say how many accounts each hook slice takes (hooks-v2 "The extra-accounts
registry", `swap/src/token.rs:11-46`).

Two slots may run the same program (two `hookwars_items` items): they get two slices, each with the
same program and signer accounts (one account lock each in the transaction). Account and byte cost
per slice is to measure (M1).

### 2.3 Slot flags

Slot `flags` reuse upstream `token_flags` bits (`crates/bordrless-hook/src/lib.rs:48-67`) and add one:

| Bit | Name | Meaning |
| --- | --- | --- |
| 0..5 | `BEFORE_TRANSFER` ... `AFTER_BURN` | as upstream |
| 6 | `TRANSFER_RETURNS_DELTA` | `before_transfer` may answer cuts |
| 7 | `WRITES_HOOK_DATA` | `before_*` and `on_touch` may answer this slot's range |
| 8 | `ANSWERS_TOUCH` (new) | `on_touch` runs (section 5) |

`slot_flags::ALL = (1 << 9) - 1`. Which flags each kind may carry (`SlotFlagsNotAllowed` otherwise,
checked at `set_slot_item` and at `create_mint` for a Locked slot):

| Kind | Allowed flags |
| --- | --- |
| Fee | transfer callbacks, `TRANSFER_RETURNS_DELTA` |
| Reward | all token callbacks, `TRANSFER_RETURNS_DELTA`, `WRITES_HOOK_DATA` (if `may_write_data`), `ANSWERS_TOUCH` (if `may_answer_touch`) |
| Defense | all token callbacks, `WRITES_HOOK_DATA` (if `may_write_data`), `ANSWERS_TOUCH` (if `may_answer_touch`); never `TRANSFER_RETURNS_DELTA` |
| Relation | as Reward |
| Pool | none at the token level (0) |
| Locked | `token_flags::ALL` (upstream meaning), never `ANSWERS_TOUCH` |

`TRANSFER_RETURNS_DELTA` additionally needs `bounds.max_cut_bps > 0`; `WRITES_HOOK_DATA` needs
`bounds.may_write_data`.

### 2.4 Two calling conventions

| Slot | Convention | Arguments | Answer |
| --- | --- | --- | --- |
| Locked | **legacy**, upstream unchanged | `TokenHookArgs` (`lib.rs:167-201`), full 64-byte hook data | `HookReturn` (`lib.rs:265-281`) |
| Fee, Reward, Defense, Relation | **slot** (new) | `TokenSlotArgs` (2.5): only the slot's range | `SlotReturn` (3.1) |

The legacy convention exists so the kit, Half-Life and `tax_hook` can sit in a Locked slot without
being rewritten (00 section 3: "kept"). The callback names are the same in both conventions
(`before_transfer`, `after_transfer`, `before_mint`, `after_mint`, `before_burn`, `after_burn`, with
upstream discriminators `lib.rs:100-112`); the slot convention adds `on_touch` (discriminator
`sha256("global:on_touch")[..8]`, computed into `discriminators::ON_TOUCH` when built). A program
written for the slot convention (`hookwars_items`, 04) implements those instruction names with
`TokenSlotArgs`; a legacy program is never put in a non-Locked slot, because its arguments would not
decode (and `set_slot_item` only accepts programs the armory registered as templates, 02).

### 2.5 `TokenSlotArgs`

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenSlotOp { Transfer, Mint, Burn, Touch }

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct TokenSlotArgs {
    pub op: TokenSlotOp,
    pub phase: Phase,                 // upstream enum
    /// Which slot of the mint is calling, and the item equipped in it. Trustworthy: only this
    /// program can sign with the hook signer the item receives as account 0.
    pub slot: u8,
    pub item: Pubkey,
    pub mint: Pubkey,
    pub source: Pubkey,               // the mint for a mint; the holding for a touch
    pub destination: Pubkey,          // the mint for a burn; the holding for a touch
    pub source_owner: Pubkey,         // default for a mint
    pub destination_owner: Pubkey,    // default for a burn
    /// Who signed: the source's owner or delegate; the mint authority; for a touch, the caller.
    pub authority: Pubkey,
    pub authority_is_delegate: bool,
    pub amount: u64,                  // 0 for a touch
    /// This slot's own cuts (After phase; 0 before).
    pub delta: u64,
    /// All slots' cuts together (After phase; 0 before).
    pub total_delta: u64,
    pub source_balance: u64,
    pub destination_balance: u64,
    pub decimals: u8,
    pub supply: u64,
    /// The source holding's bytes of this slot's range, without the epoch byte (`data_len - 1`
    /// bytes); all zeros when the range is stale or never stamped (3.4), or for a mint.
    pub source_data: Vec<u8>,
    /// The same for the destination; all zeros for a burn.
    pub destination_data: Vec<u8>,
    /// `touch` only: the caller's context bytes (at most `MAX_HOOK_DATA`, 256, `lib.rs:41`).
    pub context: Vec<u8>,
}
```

## 3. Answers and how they merge

### 3.1 `SlotReturn`

```rust
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct SlotReturn {
    /// Cuts from the amount (before_transfer only). Account indices are into this slot's callback
    /// list (prefix 5, then extras_i). At most MAX_DELTAS, or MAX_DELTAS - 1 when the slot has a
    /// royalty (3.3).
    pub deltas: Vec<Delta>,
    /// New bytes for this slot's range in the source holding (data_len - 1 bytes).
    pub source_data: Option<Vec<u8>>,
    /// New bytes for this slot's range in the destination holding (data_len - 1 bytes).
    pub destination_data: Option<Vec<u8>>,
}
```

What each callback may answer (others refused, `UnsupportedHookReturn`), the slot analogue of
`Allowed::token` (`lib.rs:320-342`):

| Callback | deltas | source_data | destination_data |
| --- | --- | --- | --- |
| `before_transfer` | with `TRANSFER_RETURNS_DELTA` | with `WRITES_HOOK_DATA` | with `WRITES_HOOK_DATA` |
| `before_mint` | no | no | with `WRITES_HOOK_DATA` |
| `before_burn` | no | with `WRITES_HOOK_DATA` | no |
| `on_touch` | no | with `WRITES_HOOK_DATA` (the touched holding) | no |
| every `after_*` | nothing is read | | |

A data field must be exactly `data_len - 1` bytes (`SlotDataLength`). Return data is read only when
it is the slot program's own and non-empty, as upstream (`lib.rs:444-460`). Return data is cleared
before each slot's call, so one item's answer can never be read as the next one's.

### 3.2 Checks per slot, then across slots

For each called slot `i` in the `Before` phase, with `cut_i = sum(deltas_i)` (checked arithmetic):

1. Upstream answer rules: at most the delta budget (3.3), no zero delta, no index twice
   (`TooManyDeltas`, `ZeroDelta`, `InvalidDeltaAccount`).
2. `cut_i <= floor(amount * bounds_i.max_cut_bps / 10_000)` (`SlotCutExceeded`).
3. Every delta target is an extra of slice `i`, a writable unfrozen holding of the mint, not the
   source, the destination, or any slice's `royalty_holding` (`InvalidDeltaAccount`; extends
   `hooks.rs:122-148`). Targets are unique across all slots of the operation, so two slots never
   credit the same holding in one call to `apply_deltas` (`InvalidDeltaAccount`).

Across slots: `sum_i cut_i <= amount` (`DeltaTooLarge`, as upstream `transfer.rs:115`). The
destination receives `amount - sum_i cut_i`. All checks run before any balance or byte is written
(upstream "All are checked before any is written", `hooks.rs:118-148`).

### 3.3 Royalty: computed by the token program

Recommendation adopted: the token program computes and pays the royalty itself, from the slot's
copied `royalty_bps`, so an item can never skip or shrink it, and a trader never pays more than the
item's cut:

```
royalty_i = floor(cut_i * royalty_bps_i / 10_000)
```

`royalty_i` is taken **out of** slot `i`'s deltas, in delta order: each delta is reduced until the
royalty is covered; a delta reduced to 0 is dropped. The royalty is credited to
`royalty_holding_i` (checked: equal to `slots[i].royalty_holding`, a writable holding of the mint,
else `WrongRoyaltyHolding`). `royalty_bps <= MAX_ROYALTY_BPS` is checked at `set_slot_item`
(`RoyaltyTooHigh`).

Delta budget consequence: a slot with `royalty_bps > 0` may answer at most `MAX_DELTAS - 1` (2)
deltas, so that with the royalty credit it never exceeds the upstream `MAX_DELTAS` (3) credits per
cutting item (00 section 6, `MAX_CUTTING_SLOTS`). A transfer's total credits are therefore at most
`MAX_DELTAS * MAX_CUTTING_SLOTS`; the account and compute cost of that is to measure (M1).

The royalty is a token-side credit, in the mint being transferred. Bordrless's protocol share on a
launch pool is taken by the DEX in SOL from the swap's measured cuts (hooks-v2 section 3.1); the
royalty is part of the measured cut, so the order of the two shares is decision H4, resolved in 03.

### 3.4 Hook-data ranges and the epoch byte

- Each slot with `data_len > 0` owns bytes `[data_offset, data_offset + data_len)` of every
  holding's `hook_data`.
- **Byte `data_offset` is the epoch byte, owned by the token program.** The item sees and answers
  only the `data_len - 1` bytes after it. The item's own layout tag (00 section 4.4, "byte 0 of each
  range is a layout tag") is therefore the first byte the item sees.
- When the token program reads a range for an item: if the epoch byte equals `slots[i].data_epoch`,
  it passes the bytes; otherwise (0, or an older epoch) it passes zeros: the range is **stale or
  never stamped**.
- When it writes an item's answer: all-zero bytes clear the whole range including the epoch byte;
  anything else writes `data_epoch` into the epoch byte and the bytes after it.
- `set_slot_item` increments `data_epoch` (wrapping 255 to 1). Data a previous item left behind is
  then stale everywhere at once, without touching any holding.
- A Locked slot that writes data owns all 64 bytes with **no** epoch byte, exactly as upstream (it
  is never re-equipped, so there is nothing to tag).

Known limit: after 255 equips of one slot an old epoch value comes back, and data left by an item
255 equips ago would read as current. The armory's notice period bounds how often a slot can change
(`MIN_NOTICE_SECS`); whether a 2-byte epoch is worth one more byte per range is open (section 9).

### 3.5 Applying

In order: credit every slot's (reduced) deltas, then every royalty; move the balances (source loses
`amount`, destination gains `amount - sum cut`); write every answered range (source and destination,
per slot); then the `After` callbacks; then the event. Same order as upstream
`transfer.rs:104-169`.

### 3.6 Refusals

An item refuses by failing its callback; the whole transaction fails, nothing moves (upstream
behaviour of any failing CPI). The token program cannot catch a failed CPI, so `bounds.may_refuse =
false` is **not enforceable at run time**. It is a declaration that the armory (02) and the template
tests (04) must check before an item can be equipped in such a slot, and that the site shows. The
cut bounds, the range bounds and the delta rules are enforced on every operation.

## 4. Instructions

### 4.1 `create_mint` (changed)

`CreateMintArgs` (`mint.rs:12-37`) gains:

```rust
pub slot_authority: Option<Pubkey>,
pub slots: Vec<SlotInit>,

pub struct SlotInit {
    pub kind: u8,
    pub equip_rule: u8,
    pub bounds: SlotBounds,
    /// Range length including the epoch byte (Locked: 0 or 64).
    pub data_len: u8,
    /// Locked slots only: the program and its flags (upstream token_flags). Every other slot is
    /// created empty and filled by the armory through set_slot_item (4.2).
    pub locked_program: Option<Pubkey>,
    pub locked_flags: u16,
}
```

Checks: upstream `create_mint` checks (`mint.rs:82-104`), then every rule of 1.4. `data_offset` is
assigned in slot order; `data_epoch` starts at 1; a Locked slot gets `signer_bump` from
`Mint::hook_signer(&program)` (`state.rs:61-65`). Non-Locked slots are created **empty**: initial
items are equipped by the armory right after (02, 03), because only the armory's `SlotAuthority`
may put an item in a slot. Event `SlotTableCreated` (section 6).

Consequence for launches (03): every non-Locked slot must be equipped, or deliberately left empty,
**before** the supply `mint_to`, since `mint_to` runs the slots subscribed to mints.

### 4.2 `set_slot_item` (new)

Accounts:

| Account | |
| --- | --- |
| `slot_authority` | signer: `PDA(["slots", mint], <ARMORY_ID>)`, signing by CPI from the armory; must equal `mint.slot_authority` (`InvalidSlotAuthority`) |
| `mint` | writable |
| `program` | the new item's program (executable), or this program's id to empty the slot |
| `royalty_holding` | the item's royalty holding of this mint, or this program's id when `royalty_bps` is 0 |
| event authority, this program | |

Arguments: `slot: u8`, `item: Pubkey` (default to empty), `flags: u16`, `royalty_bps: u16`.

Checks, in order:

| Check | Error |
| --- | --- |
| `slot < slot_count` | `SlotIndexOutOfRange` |
| `kind != Locked` | `SlotLocked` |
| emptying: `item` default, `flags` 0, `royalty_bps` 0 | `InvalidSlotItem` |
| otherwise: `program` executable, not this program, not `<SWAP_ID>`, `<LAUNCH_ID>`, `<BRIDGE_ID>` | `InvalidSlotItem` |
| `flags` allowed for the kind and bounds (2.3) | `SlotFlagsNotAllowed` |
| `royalty_bps <= MAX_ROYALTY_BPS`; `royalty_bps > 0` only if `max_cut_bps > 0` | `RoyaltyTooHigh` |
| `royalty_holding` is `["holding", mint, PDA(["royalty", item], <ARMORY_ID>)]` and exists | `WrongRoyaltyHolding` |

The token program does **not** check the item's manifest against the slot's bounds or that the
program is a registered template: that is the armory's job (02), which is the only signer. The
token program enforces the bounds at every operation regardless (3.2), so a wrong manifest can never
take more than the slot allows.

Effects: writes `item`, `program`, `flags`, `royalty_bps`, `royalty_holding`, `signer_bump`
(`Mint::hook_signer(&program).1`, or the launchpad's signer bump for a Pool slot:
`hook_signer(&<LAUNCH_ID>, &program).1`, `lib.rs:476-479`), increments `data_epoch`. Event
`SlotItemSet`. No holding is touched (3.4).

The item's extra-accounts registry for this mint is created by the item's program (04) when the
armory equips it, at the seed in section 7.

### 4.3 `transfer`, `mint_to`, `burn` (changed)

- New argument `slot_accounts: Vec<u8>` (2.2), appended after `amount`. Empty for a legacy mint.
- Legacy mint (`hook_program` set): upstream path, unchanged; `slot_accounts` must be empty
  (`SlotAccountsMismatch`).
- Slot mint: the named `hook_program` and `hook_signer` accounts are this program's id; slots run as
  in sections 2 and 3. A Locked slot runs with the legacy convention inside the same walk, its slice
  being `[program, signer, extras]` and its answer a `HookReturn` checked with
  `Allowed::token(op, phase, locked_flags)`, and its full-range hook data written as upstream.
- `mint_to` and `burn` never take cuts (upstream: only `before_transfer` answers deltas,
  `lib.rs:320-342`).
- Prefix per slot call: `[slot hook_signer, mint, source, destination, authority]`, as upstream
  (`hooks.rs:66-70`, `mint.rs:175-180`, `transfer.rs:212-217`).

### 4.4 `close_holding` (rule restated)

Unchanged in spirit (`holding.rs:167-185`): an empty holding closes unless a hook still keeps data
in it. For a slot mint, a range "keeps data" only when its epoch byte equals the slot's current
`data_epoch` and any byte is non-zero; stale ranges do not block (`HookDataNotEmpty`). A Locked
slot that writes data uses the upstream all-64-bytes-zero rule. On close, the account's bytes go
away with it, stale ranges included.

### 4.5 `write_hook_data` (restricted)

Kept for legacy mints exactly as upstream. For a slot mint it is allowed only for the Locked slot's
program (the kit's `claim` uses it, `programs/bordrless_kit/src/instructions/claim.rs:119`),
writing all 64 bytes; for any other slot it is refused (`HookDataNotWritable`). Slot items change
data only through their answers and `touch`.

### 4.6 `set_hook` and `set_authority(Hook)`

Refused for a slot mint (`MixedHookModes`): its `hook_authority` is `None` by rule 1.4.

## 5. `touch` (new)

Lets a program (the war program's bounty and quest claims, 05) have one slot's item update one
holding's range with no transfer.

Accounts:

| Account | |
| --- | --- |
| `caller` | signer: any account; reaches the item as `TokenSlotArgs.authority` and the prefix `authority` |
| `mint` | read-only |
| `holding` | writable, of `mint` (`MintMismatch`) |
| `program`, `hook_signer` | the slot's program and signer, checked as in 2.2 |
| event authority, this program | |
| remaining | the slot's `extras` |

Arguments: `slot: u8`, `context: Vec<u8>` (at most `MAX_HOOK_DATA`), `extra_count: u8`.

Checks: slot in range and not empty (`SlotEmpty`), kind not Pool or Locked, slot has
`ANSWERS_TOUCH` (`TouchNotSupported`), holding not frozen (`Frozen`), extras exactly `extra_count`
(`SlotAccountsMismatch`).

Call: `on_touch` with `TokenSlotArgs { op: Touch, phase: Before, slot, item, mint, source: holding,
destination: holding, source_owner: holding.owner, destination_owner: holding.owner, authority:
caller, amount: 0, source_balance: holding.amount, destination_balance: holding.amount,
source_data: range, destination_data: range, context, .. }`.

Answer: `source_data` only (3.1). The item decides whether to act by checking `args.authority`
(for example: only the war program's PDA may reset raid points) and `args.context`. `args.authority`
is trustworthy because the token program verified it as a signer and only the token program can
sign the hook signer the item receives. No `after_touch`.

Event `Touched`.

## 6. Events

| Event | Fields | Notes |
| --- | --- | --- |
| `Transferred` | upstream fields (`events.rs:47-61`) plus `slot_cuts: Vec<SlotCut>` | `deltas` keeps listing every credit, royalties included, as `DeltaApplied` |
| `SlotCut` (struct) | `slot: u8, item: Pubkey, cut: u64, royalty: u64` | one per slot that cut |
| `SlotTableCreated` | `mint, slot_authority, slots: Vec<SlotInfo>` | `SlotInfo` = kind, equip_rule, bounds, data_offset, data_len, locked program |
| `SlotItemSet` | `mint, slot, old_item, new_item, program, flags, royalty_bps, data_epoch, ts` | |
| `Touched` | `mint, holding, owner, slot, item, caller, wrote: bool, ts` | hook data is not in events, as upstream |
| `Minted`, `Burned` | unchanged | |

`MintCreated` is unchanged (its layout stays readable by upstream indexers); the slot table is in
`SlotTableCreated`, emitted right after it.

## 7. Registries: one program, many items

Upstream registry: `["bordrless-hook-accounts", mint_or_pool]` under the hook program
(`lib.rs:490-493`). One program per mint could only publish one list per mint, but `hookwars_items`
may run several items on the same mint.

Proposed seed for slot items: `["bordrless-hook-accounts", mint, item]` under the item's program
(`item` = the armory `Item` account key). A Locked slot keeps the upstream two-seed registry. The
client resolves each called slot's list with the slot prefix (5 accounts) and the owners, as
upstream `HookAccountList::resolve` (`lib.rs:571-608`). A new helper
`slot_accounts_address(program, mint, item)` joins `hook_accounts_address`. `write_registry`
(`lib.rs:612-`) is reused with the three-seed signer.

## 8. Call depth

Invoke height counts the top-level instruction as 1; Solana's limit is 5 (hooks-v2 section 6).
Items are leaf programs: a slot item must make no CPI (04 must honour this), so a slot call adds
exactly one level.

| Path | Height of the deepest item call | Upstream reference | Measured |
| --- | --- | --- | --- |
| wallet transfer | token 1, item 2 | 2 | to measure |
| DEX swap delivery | DEX 1, token 2, item 3 | 3 (hooks-v2 6 table) | to measure |
| launch supply `mint_to` | launch 1, token 2, item 3 | | to measure |
| graduation top-up | launch 1, DEX 2, token 3, item 4 | 4 | to measure |
| companion `create_launch` then `mint_to` | companion 1, launch 2, token 3, item 4 | 5 for `create_launch` from a companion (`docs/companions.md:75`) | to measure; must stay at most 5 |
| armory equip at launch | launch 1, armory 2, token `set_slot_item` 3 (companion: 4) | | to measure |
| war `touch` | war 1, token 2, item 3 | | to measure |

Bytes, account locks, trace entries and compute per extra slice are to measure in M1 and recorded in
07; `MAX_SLOTS` and `MAX_CUTTING_SLOTS` are set from them.

## 9. Errors (new)

`InvalidSlotTable`, `InvalidSlotAuthority`, `MixedHookModes`, `TooManyCuttingSlots`,
`SlotIndexOutOfRange`, `SlotLocked`, `SlotEmpty`, `InvalidSlotItem`, `SlotFlagsNotAllowed`,
`RoyaltyTooHigh`, `WrongRoyaltyHolding`, `SlotCutExceeded`, `SlotDataLength`,
`SlotAccountsMismatch`, `TouchNotSupported`. Upstream errors keep their codes; new ones are appended
after `NotHookAuthority` (`error.rs:61`) so upstream codes do not move.

Open at this level:
- Epoch width: 1 byte (current) or 2 (3.4).
- Whether a non-slot mint should keep the upstream size (a `version` 2 mint with a separate table)
  if `MAX_SLOTS * 111` bytes of rent on every mint proves too much (D-1 revisit after M1).

## 10. Tests

Upstream suites that must still pass, unchanged except for the added `slot_accounts` argument in
builders (legacy mints): `programs/tests/tests/token.rs`, `token_hooks.rs`, `hook_signers.rs`,
`hook_authority.rs`, `half_life.rs`, `kit.rs`, `kit_money.rs`, `launch.rs`, `launch_rules.rs`,
`launch_configs.rs`, `launch_money.rs`, `swap.rs`, `swap_hooks.rs`, `bridge.rs`, `companion.rs`,
`marketplace.rs`, `vectors.rs`.

New LiteSVM tests (M1), with `hook_tester` extended to the slot convention:

- table rules of 1.4, one test per error;
- two cutting slots: each bound enforced (`SlotCutExceeded`), sum enforced (`DeltaTooLarge`),
  royalty taken out of the cut, never on top; 3-delta budget with royalty;
- a slot writing outside its length (`SlotDataLength`); two slots' ranges never overlap after a
  sequence of transfers; stale data after `set_slot_item` reads as zeros and does not block close;
- a slot answering a field its flags do not allow (`UnsupportedHookReturn`);
- `set_slot_item` signed by anything but the armory PDA (`InvalidSlotAuthority`);
- `touch` by an unexpected caller: the item ignores it; `touch` on a slot without `ANSWERS_TOUCH`;
- a forged callback (an item called by someone other than the token program's signer) refused by
  the item (as upstream `half_life.rs` "a forged callback is refused");
- Locked slot running the kit with the legacy convention next to an item slot;
- measurements: bytes, account locks, trace entries, compute per slice; call depth per path in 8.

## 11. Interfaces

### Required from 02 (`hookwars_armory`)

- `SlotAuthority` = `PDA(["slots", mint], <ARMORY_ID>)`; the armory signs `set_slot_item` with it by
  CPI and is the only program that does.
- Before calling `set_slot_item` the armory checks: the program is a registered template program
  (immutable or managed, 00 rule 3), the item's manifest fits the slot's `bounds` and kind (reads
  `Mint.slots[slot]`), `may_refuse` honoured, the item's data bytes `<= data_len - 1`, and the
  equip rule (vote, performance, notice).
- The armory passes, from the `Item` account: `item` (its key), `program`, `flags`, `royalty_bps`.
- The armory creates the item's royalty holding (`create_holding` of
  `PDA(["royalty", item], <ARMORY_ID>)` for the mint) before equipping an item with a royalty.
- The armory triggers the item program's registry creation at `["bordrless-hook-accounts", mint,
  item]` (section 7) when equipping.

### Required from 03 (launchpad, DEX)

- The launchpad creates a Hookwars mint with `slots` and `slot_authority = PDA(["slots", mint],
  <ARMORY_ID>)`, `hook_program = None`, `hook_authority = None`, and equips initial items through
  the armory **before** the supply `mint_to`.
- The DEX's token side (`swap/src/token.rs:11-46`) carries, for a slot mint, the concatenated slot
  slices and `slot_accounts` instead of one hook slice; the client builds them per side.
- The launchpad calls Pool slots itself, reading `Mint.slots` (kind Pool, `program`, `flags`,
  `signer_bump` computed for the launchpad's signer, `item`); the token program never calls them.
- The kit's `init` check `mint.hook_program == Some(KIT_ID)`
  (`programs/bordrless_kit/src/instructions/init.rs:92-96`) must accept a slot mint whose Locked slot
  runs the kit with the same flags. This is a one-check change to the kit (see conflicts).

### Required from 04 (`hookwars_items`)

- Implements `before_transfer`, `after_transfer`, `before_mint`, `after_mint`, `before_burn`,
  `after_burn`, `on_touch` taking `TokenSlotArgs` and answering `SlotReturn` through return data.
- Verifies account 0 is the token program's `["hook-authority", <ITEMS_ID>]` signer (canonical bump,
  as upstream hooks do), and that `args.item` is an item of its own template whose `Item` account it
  reads, matching `Mint.slots[args.slot].item`.
- Makes no CPI from a slot callback.
- Each template declares its range length as `data_len - 1` visible bytes (the epoch byte is the
  token program's), its first visible byte being its layout tag.
- Publishes its per-(mint, item) registry at `["bordrless-hook-accounts", mint, item]`.

### Required from 05 (`hookwars_war`)

- Uses `touch` with its own PDA as `caller`; the template's `on_touch` accepts only that caller for
  point resets.
