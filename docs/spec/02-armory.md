# Hookwars spec 02: the armory (`hookwars_armory`)

Status: specification, 2026-10-08, revised after the integration rulings (00 section 9). Nothing
here is built. Follows `00-overview.md`; names, seeds and parameters are the overview's. Upstream
references are to Bordrless at `43688f3`.

The armory owns everything about **items**: which hook programs may be used (templates), the items
themselves (owned, tradable, supply-1 tokens), the royalty holdings and their claims, how a token's
slot changes what it runs (vote, performance, locked), equipping at launch, loot minting for
`hookwars_war`, and crafting (`forge`).

Who does what around it:

- the token program (01) runs token-side items and keeps vote locks; it **does not compute
  royalties** (R1);
- the launchpad (03) runs pool-side items and calls the armory to equip launch items (R12);
- the items program (04) implements the templates, keeps each equip's `EquipState` and equip vault,
  and **pays royalties** from them with `settle_equip` (R1, R2);
- the war program (05) calls `mint_loot`, signing as `["loot-signer"]`.

No transfer or swap reads or writes an armory account except the read-only `Item` that items take
as their first extra (04 section 2.2).

## 1. Design choices

1. **Params are a fixed array of `PARAM_FIELDS` `u32` fields (R7).** The armory checks each
   field's floor and ceiling from the `Template` record. Template-specific rules (a list must be
   sorted, two fields must agree) are the items program's `validate_params`; forging is the items
   program's `combine_params`, using each field's forge rule from 04 (`TowardCeiling`, `TowardFloor` with `FORGE_GAIN_BPS`, or `Keep`).
   The armory clamps whatever comes back to the template's floor and ceiling again, so a defect in
   `combine_params` can never produce an item above its ceiling. Where 04 writes params as
   `Vec<u8>`, the bytes are this array, little-endian, `4 * PARAM_FIELDS` long.
2. **Royalties are paid by items, claimed from the armory (R1, R2).** Cuts land in the item's
   equip vault (token side) or the mint's pool-cuts vault (pool side); `settle_equip` (04) pays
   `floor(balance * Item.royalty_bps / 10_000)` into the item's royalty holding of that cut mint.
   The armory creates those holdings, signs for them, and lets the item's current holder claim.
3. **Usage counters are not written per run in the armory.** An item equipped on many tokens would
   otherwise be one write-locked account shared by every transfer of all of them. Per-equip counts
   (`runs`, `collected`, `settled`) live in each `EquipState` (04); the armory keeps
   `equipped_count` (changed only by armory instructions) and `level` (changed only by `forge`).
4. **Votes lock tokens in place (R11).** The token program's `set_vote_lock`, signed by the armory,
   stores a lock in `Holding.reserved`. Tokens never move, so a vote never runs the token's own
   items (an exit fee, a lock or a max wallet would otherwise apply to a vote). Confirmed: this is
   what the armory uses; there are no escrow holdings.
5. **An item may be equipped on several tokens at once.** It is a licence to run a template with
   these parameters; its holder earns from every token that equips it. Only `forge` needs it
   unequipped everywhere.
6. **The manifest is computed once.** At creation the armory asks the items program for the item's
   `Manifest` (04 section 2.7) and stores it in the `Item`, so every compatibility check reads one
   account and a later template retirement never changes what an item promised.

## 2. Accounts

`F` is `PARAM_FIELDS` (00 section 6). Sizes are Anchor sizes: 8-byte discriminator plus Borsh
fields. `Manifest` is 04's struct (2.7): 1 + 2 + 1 + 2 + 2 + 2 + 2 + 1 + 1 + 1 + 1 = 16 bytes.

### 2.1 `ArmoryConfig` at `["config"]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `admin` | Pubkey | registers and retires templates; `<PROTOCOL_AUTHORITY>` at `init`; changed two-step (`propose_admin`, `accept_admin`) |
| `pending_admin` | Option<Pubkey> | |
| `items_minted` | u64 | the seed of the next item mint |
| `templates` | u16 | number registered |
| `reserved` | [u8; 64] | |

Size: 8 + 2 + 32 + 33 + 8 + 2 + 64 = 149. The admin can do nothing to an existing item, a royalty
balance, a vote or a slot. There is no pause.

### 2.2 Signers the armory uses

| PDA | Seeds | Signs |
| --- | --- | --- |
| `ArmorySigner` | `["armory"]` | every CPI into `hookwars_items` (`validate_params`, `manifest`, `combine_params`, `init_equip`, `close_equip`); 04 names it `ARMORY_SIGNER` and checks it (confirmed) |
| `Minter` | `["minter"]` | item mints' `mint_to`, then their authority is revoked |
| `SlotAuthority` | `["slots", mint]` (00 4.3) | `set_slot_item` and `set_vote_lock` on the token program |
| `RoyaltyOwner` | `["royalty", item]` (00 4.3) | royalty claims |

### 2.3 `Template` at `["template", template_id: u16 le]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `id` | u16 | dense, from `ArmoryConfig.templates` |
| `program` | Pubkey | the hook program; `<ITEMS_ID>` for every template 04 defines |
| `code_hash` | [u8; 32] | sha256 of the verified build (`solana-verify get-executable-hash`), given by the admin; shown on the site |
| `deploy_slot` | Option<u64> | for an upgradeable program, the slot in its ProgramData header at registration; `None` if immutable |
| `kind` | u8 | 00 section 4.1: `Fee`, `Reward`, `Defense`, `Relation`, `Pool` or `War`; never `Locked` |
| `field_count` | u8 | fields used, at most `F` |
| `field_min` | [u32; F] | floor per field |
| `field_max` | [u32; F] | ceiling per field (04 names each as a per-template parameter, to set) |
| `open_authoring` | bool | anyone may `create_item` from it |
| `loot_enabled` | bool | `mint_loot` may mint it |
| `forge_enabled` | bool | `forge` accepts it |
| `max_level` | u8 | highest level `forge` may reach (per template, 04) |
| `loot_royalty_bps` | u16 | royalty of loot and forged items, at most `MAX_ROYALTY_BPS` |
| `status` | u8 | `Active` 0, `Retired` 1 |
| `name` | String (max 32) | |
| `registered_by` | Pubkey | |
| `created_at` | i64 | |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 2 + 32 + 32 + 9 + 1 + 1 + 8F + 1 + 1 + 1 + 1 + 2 + 1 + 36 + 32 + 8 + 32 = 202 + 8F.

### 2.4 `Item` at `["item", item_mint]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `item_mint` | Pubkey | the supply-1 mint that represents ownership |
| `template_id` | u16 | |
| `params` | [u32; F] | inside the template's floor and ceiling; unused fields 0 |
| `manifest` | Manifest | from the items program's `manifest` at creation (1.6) |
| `author` | Pubkey | `create_item`: the signer; loot: `["loot-signer"]` under `<WAR_ID>`; forge: the forger |
| `royalty_bps` | u16 | at most `MAX_ROYALTY_BPS`; fixed for life; read by `settle_equip` |
| `level` | u8 | 1 at creation; `forge` sets it |
| `source` | u8 | `Authored` 0, `Loot` 1, `Forged` 2 |
| `equipped_count` | u32 | slots currently running it, across all tokens |
| `royalty_owner_bump` | u8 | bump of `["royalty", item]` |
| `created_at` | i64 | |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 32 + 2 + 4F + 16 + 32 + 2 + 1 + 1 + 4 + 1 + 8 + 32 = 141 + 4F.

### 2.5 The item mint

A Bordrless-standard `Mint` (upstream `programs/bordrless_token/src/state.rs:10-50`) at the armory
PDA `["item-mint", items_minted: u64 le]`, created by CPI `create_mint` with `decimals 0`,
`max_supply 1`, `mint_authority = Some(Minter)`, no freeze, hook, slot or metadata authority, no
slots, name from the template, symbol `ITEM`, uri the site's item metadata route (06). The armory
then CPIs `mint_to` of 1 to the recipient and `set_authority(MintTokens, None)`. Upstream: a mint
address may be a PDA of the creating program (`docs/architecture.md`, "The token standard").

**Owner.** Whoever holds the one token: the holding `["holding", item_mint, owner]` with
`amount == 1` (upstream `Holding`, `state.rs:77-100`). Item mints have no slots, so moving an item
is a plain transfer. The armory never stores an owner.

### 2.6 Royalty holdings

`RoyaltyOwner` (`["royalty", item]`) is a system-owned address with no data. Its holding of each
cut mint, `["holding", cut_mint, RoyaltyOwner]`, receives the royalty from `settle_equip`. Cut
mints are the token itself (token-side items, and the base side of pool items) and bridged SOL (the
quote side of pool items). The armory creates every royalty holding the item can need before the
item is equipped (6.4, 7.1), so `settle_equip` never fails for a missing holding.

### 2.7 `SlotState` at `["slot-state", token_mint, slot: u8]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `mint`, `slot` | Pubkey, u8 | |
| `launch_item` | Option<Pubkey> | the item equipped by `equip_launch` (the target of a performance revert) |
| `rule` | Option<PerformanceRule> | 6.7; set at `equip_launch` for a `Performance` slot, never changed |
| `open_proposal` | Option<u64> | nonce of the proposal in `Open` or `Passed`; at most one per slot |
| `next_nonce` | u64 | |
| `condition_since` | Option<i64> | performance: when the condition was first seen true without a break |
| `last_check` | i64 | |
| `reserved` | [u8; 32] | |

`PerformanceRule` is 1 + 4 + 4 + 1 + 2 + 4 = 16 bytes (6.7). Size: 8 + 2 + 32 + 1 + 33 + 17 + 9 + 8 +
9 + 8 + 32 = 159.

Created by `equip_launch` for every `Vote` and `Performance` slot (so the launch item and the rule
are recorded at launch), never for `Locked` slots.

### 2.8 `Proposal` at `["proposal", token_mint, slot: u8, nonce: u64 le]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `mint`, `slot`, `nonce` | Pubkey, u8, u64 | |
| `proposer` | Pubkey | pays the rent, refunded at `close_proposal` |
| `item` | Option<Pubkey> | the item to equip; `None` empties the slot (only if the slot's bounds allow it, 01) |
| `created_at`, `vote_end` | i64, i64 | `vote_end = created_at + VOTE_PERIOD_SECS` |
| `executable_at` | i64 | `vote_end + slot.notice_secs` |
| `votes_for`, `votes_against` | u64, u64 | locked token amounts |
| `status` | u8 | `Open` 0, `Passed` 1, `Failed` 2, `Executed` 3, `Cancelled` 4 |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 32 + 1 + 8 + 32 + 33 + 8 + 8 + 8 + 8 + 8 + 1 + 32 = 189.

### 2.9 `VoteLock` at `["vote", proposal, voter]`

| Field | Type | Meaning |
| --- | --- | --- |
| `proposal`, `voter` | Pubkey, Pubkey | |
| `amount` | u64 | votes cast |
| `support` | bool | |
| `bump` | u8 | |

Size: 82. Its existence stops a second vote by the same voter on the same proposal. Closed after
the proposal is final; rent to the voter.

### 2.10 `ForgeCounter` at `["forges", wallet]`

`{ wallet: Pubkey, count: u64, bump: u8 }`, size 49, incremented by every `forge` the wallet signs.
05 reads it for the `Forge` quest.

## 3. Templates

### 3.1 `register_template(args)`

Accounts: `admin` (signer, `== config.admin`), `payer` (signer, mut), `config` (mut), `template`
(init, `["template", config.templates]`), `program` (executable), `programdata` (its ProgramData
address, as upstream passes it), `armory_signer`, `<ITEMS_ID>` when `program` is it,
`system_program`.

Args: `code_hash`, `kind`, `field_count`, `field_min`, `field_max`, `open_authoring`,
`loot_enabled`, `forge_enabled`, `max_level`, `loot_royalty_bps`, `name`.

Checks, in order:

1. Upgrade authority, rule 3: upstream's `check_hook_authority`
   (`programs/bordrless_launch/src/instructions/launch_config.rs:88-140`) against
   `HOOK_UPGRADE_AUTHORITIES` holding `<MANAGED_HOOK_KEY>` and `<PROTOCOL_AUTHORITY>` (upstream
   `constants.rs:80-83`). Else `TemplateUpgradeable` / `ProgramDataMissing`.
2. `program` is none of `<TOKEN_ID>`, `<SWAP_ID>`, `<LAUNCH_ID>`, `<KIT_ID>`, `<BRIDGE_ID>`,
   `<COMPANION_ID>`, `<ARMORY_ID>`, `<WAR_ID>`, the system program, the default key (upstream
   `PROTOCOL_PROGRAMS`, `constants.rs:97`, extended). Else `InvalidTemplateProgram`.
3. `kind` is `Fee`, `Reward`, `Defense`, `Relation`, `Pool` or `War`. Else `InvalidKind`.
4. `field_count <= F`; every used field has `field_min <= field_max`; unused fields are 0. Else
   `InvalidSchema`.
5. `loot_royalty_bps <= MAX_ROYALTY_BPS` (`RoyaltyTooHigh`); `max_level >= 1` (`InvalidSchema`).
6. `deploy_slot` is read from the ProgramData header for an upgradeable program, `None` otherwise.

Event `TemplateRegistered { template_id, program, code_hash, deploy_slot, kind, field_count,
field_min, field_max, name, ts }`.

### 3.2 `retire_template(template_id)`

Admin only. `status = Retired`: no new item from it and no new equip of its items. Items already
equipped keep running. Event `TemplateRetired { template_id, ts }`.

### 3.3 Staleness

Every equip (`execute`, `equip_launch`, a performance revert) re-reads the template program's
ProgramData. If `deploy_slot` is `Some(s)` and the header slot is not `s`, the code changed since
registration: `TemplateChanged`. A revert to the launch item is the one exception: it is allowed,
because the launch item is what holders bought into and the alternative is leaving the current item
in place. The site shows "upgraded since registration" from the same read (06).

## 4. Items

### 4.1 Validation shared by every creation path

For template `T` and params `p`:

1. `T.status == Active` (`TemplateClosed`);
2. every used field `T.field_min[i] <= p[i] <= T.field_max[i]`, unused fields 0
   (`ParamOutOfRange`);
3. CPI `validate_params(T.id, ceilings, p)` into `<ITEMS_ID>`, signed by `ArmorySigner`
   (`BadParams`, `AboveCeiling` from 04 propagate);
4. CPI `manifest(T.id, p)`; the returned `Manifest.kind` must equal `T.kind` (`KindMismatch`).

`ceilings` passed to 04 is `T.field_max` as little-endian bytes.

### 4.2 `create_item(template_id, params, royalty_bps)`

Accounts: `author` (signer, mut, pays), `config` (mut), `template`, `minter`, `item_mint` (mut,
`["item-mint", config.items_minted]`), `item` (init), `recipient_holding` (mut, created here),
`armory_signer`, `<ITEMS_ID>`, the token program and its event authority, `system_program`.

Checks: `T.open_authoring` (`TemplateClosed`); 4.1; `royalty_bps <= MAX_ROYALTY_BPS`
(`RoyaltyTooHigh`).

Effects: create the item mint (2.5), mint 1 to the author, revoke the mint authority, write `Item`
(`level 1`, `source Authored`, the manifest), `items_minted += 1`. Event `ItemCreated { item,
item_mint, template_id, params, manifest, author, royalty_bps, level, source, ts }`.

### 4.3 `mint_loot(owner, template_id, params)` (CPI only)

Accounts: `loot_signer` (signer: `["loot-signer"]` under `<WAR_ID>`; the armory hard-codes
`<WAR_ID>`, as the kit hard-codes `LAUNCH_ID`, upstream hooks-v2 section 4.1), `payer` (signer,
mut), `owner`, then as `create_item`.

Checks: `loot_signer` is that PDA at its canonical bump; nothing else may call it (`NotLootSigner`);
`T.loot_enabled` (`TemplateClosed`); 4.1. 05 chooses `params` from its loot table and randomness
(05 checks them against `Template` when the table is set, and the armory checks them again here).

Effects: as `create_item`, with `royalty_bps = T.loot_royalty_bps`, `source Loot`,
`author = loot_signer`, the item token to `owner`'s holding. Events `ItemCreated` and
`LootMinted { item, owner, template_id, params, ts }`.

## 5. Royalties

### 5.1 How they accrue (R1, R2; restated so 02 and 04 agree)

- Token side: a cutting item answers at most one delta per transfer, into its equip vault
  (the holding of `token_mint` owned by `["equip", mint, slot]` under items, 01 section 3.3).
- Pool side: all Pool-item cuts on one side of a swap merge into one delta into
  `["pool-cuts", mint]` (items); each item's share is recorded in its `EquipState`.
- `settle_equip(equip_state, cut_mint)` (04 section 2.5), permissionless with a bounty, pays
  `floor(balance * Item.royalty_bps / 10_000)` into `["holding", cut_mint, RoyaltyOwner]` and the
  rest to the template's destination.

The token program does not compute or credit royalties. The protocol share is taken by the DEX on
the swap before anything reaches a vault (04 2.5, `hooks-v2.md` 3.1), so it is always first
(DECISIONS H4 resolved).

### 5.2 `claim_royalty(amount)`

Accounts: `claimant` (signer), `item`, `item_holding` (`["holding", item.item_mint, claimant]`),
`royalty_owner` (`["royalty", item]`), `cut_mint`, `royalty_holding` (mut,
`["holding", cut_mint, royalty_owner]`), `destination` (mut, the claimant's holding of `cut_mint`,
created if missing), the token program, its event authority, and, when `cut_mint` has slots, the
slot accounts 01 requires for a transfer (with I-01-4 they are passed but no item runs).

Checks: `item_holding.mint == item.item_mint`, `owner == claimant`, `amount == 1`, not frozen
(`NotItemOwner`); `amount <= royalty_holding.amount` (`InsufficientRoyalty`); `u64::MAX` claims all.

Effects: CPI transfer signed by `royalty_owner`. When `cut_mint` is bridged SOL the site appends
the bridge's `unwrap_sol` (upstream `bridge.unwrapSolAbove`). Event `RoyaltyClaimed { item,
cut_mint, claimant, amount, ts }`.

Whoever holds the item at claim time takes everything accrued, including under an earlier holder.
The site shows unclaimed and unsettled royalties on the item's listing (06). How many claims fit one
transaction: to measure (07).

## 6. Equip rules

A slot's kind, bounds, equip rule, notice and data range are in the mint (01) and never change
(00 rule 2). The armory decides when its item changes and signs the change as `SlotAuthority`, the
only key 01 accepts for `set_slot_item`.

### 6.1 Compatibility (`check_fits`, run at `equip_launch`, `propose` and `execute`)

For item `I` (manifest `M`) and slot `S` of token `T`:

| Check | Error |
| --- | --- |
| `S.kind != Locked`, `S.equip_rule != Locked` (except `equip_launch`, which fills any non-`Locked`-kind slot once) | `SlotLocked` |
| template of `I` `Active` and not stale (3.3) | `TemplateClosed`, `TemplateChanged` |
| `M.kind == S.kind` | `KindMismatch` |
| `M.max_cut_bps_buy`, `_sell`, `_transfer` each `<=` the slot's bound for that side | `OverBounds` |
| `M.max_discount_bps <=` the slot's discount bound (Pool) | `OverBounds` |
| `M.may_refuse`, `M.may_burn` only where the slot allows | `OverBounds` |
| `M.data_bytes + 1 <= S.data_len` (the epoch byte, 00 4.4) | `DataRangeTooSmall` |
| `M.reads_other_pools <=` the slot's read bound | `OverBounds` |
| `M.token_flags` has no mint callback (R12) | `OverBounds` |
| `I` is not already in another slot of `T` | `AlreadyEquipped` |

`check_fits` is also published in the armory's client crate, so 03 runs the same function in
`create_launch`'s plan (03 4.2, `ItemDoesNotFit`). Emptying a slot needs only the first check and
the slot's `may_be_empty` bound.

### 6.2 `propose(slot, item: Option<Pubkey>)`

Accounts: `proposer` (signer, mut), `token_mint`, `slot_state` (mut), `proposal` (init, nonce
`slot_state.next_nonce`), `item` and its `template` and `programdata` (when `Some`),
`system_program`.

Checks: 6.1; `slot_state.open_proposal` is `None` (`ProposalOpen`); the rule is `Vote` or
`Performance` (`SlotLocked`).

Effects: write the proposal, `open_proposal = Some(nonce)`, `next_nonce += 1`. Event
`ProposalCreated { mint, slot, nonce, proposer, item, vote_end, executable_at }`.

### 6.3 `vote(support, amount)`

Accounts: `voter` (signer, mut), `proposal` (mut), `vote_lock` (init), `holding` (mut, the voter's
holding of the token), `token_mint`, `slot_authority`, the token program, `system_program`.

Checks: `now < vote_end` (`VotingClosed`); `0 < amount <= holding.amount` (`InvalidAmount`);
`holding.owner == voter`.

Effects: CPI `set_vote_lock(amount, until = proposal.vote_end)` (R11, I-01-3), signed by the voter
and `SlotAuthority`. The token program keeps the larger amount and the later `until` of overlapping
locks, so one lock can back votes on several slots' proposals; transfers and burns that would take
the holding below the locked amount before `until` fail. No slot item runs. Add `amount` to
`votes_for` or `votes_against`. Event `VoteLocked { proposal, voter, support, amount, until }`.

Votes cannot be flash-borrowed: the lock outlives the transaction.

### 6.4 `finalize` and `execute`

`finalize(proposal)` (anyone, `now >= vote_end`):

```
eligible = token_mint.supply - pool_base_vault.amount - launch_holding.amount
passed   = votes_for > votes_against
           and (votes_for + votes_against) * 10_000 >= eligible * VOTE_QUORUM_BPS
```

Read live at `finalize`, not snapshotted: the pool vault and the launch reserve are the two holders
upstream excludes from eligible supply (hooks-v2 section 0, item 4). The accounts add the `Launch`
(`["launch", mint]` under `<LAUNCH_ID>`), its pool's base vault and the launch PDA's holding, bound
through the `Launch` account. Sets `Passed` or `Failed`; on `Failed`, `open_proposal = None`. Event
`ProposalResolved { proposal, status, votes_for, votes_against, eligible, ts }`.

`execute(proposal)` (anyone; `status == Passed`, `now >= executable_at`, else `NotExecutable`):

1. 6.1 again.
2. **Unequip the old item**: CPI `close_equip(slot)` into items, signed by `ArmorySigner`. It
   refuses while any of the old item's vaults holds a balance (`VaultNotSettled`, 04), so the
   sender settles first (section 10). Old item `equipped_count -= 1`.
3. **Equip the new item**: create its royalty holdings for every cut mint its manifest can cut in
   (2.6; the sender pays); CPI `init_equip(slot, data_offset, data_len)` into items (creates the
   `EquipState`, the slot registry `["bordrless-hook-accounts", mint, slot]` and the equip vault
   holdings); new item `equipped_count += 1`.
4. CPI `set_slot_item` (I-01-1) signed by `SlotAuthority`.
5. For a `Pool` slot, CPI the launchpad's `refresh_pool_registry(mint)` (03 4.3).
6. `status = Executed`, `open_proposal = None`. Event `EquipApplied { mint, slot, old_item,
   new_item, by: Vote, ts }` (01 also emits `SlotEquipped`).

`fail_stale(proposal)` (anyone): a `Passed` proposal whose 6.1 checks now fail becomes `Failed`;
`open_proposal = None`; event `ProposalResolved`.

### 6.5 `cancel(proposal)`

The proposer, while `Open` with no votes: `Cancelled`, `open_proposal = None`, `ProposalResolved`.

### 6.6 `close_proposal`, `close_vote`

Anyone, once the proposal is `Failed`, `Executed` or `Cancelled`: rent back to the proposer, and to
each voter for their `VoteLock`, with event `VoteUnlocked { proposal, voter, amount }`. The lock in
the token program expires by time on its own; `VoteUnlocked` marks when the armory stops counting
it.

### 6.7 Performance rule

00 4.2: a `Performance` slot accepts votes like `Vote`, and reverts to its launch item when its
condition holds for its hold time.

`PerformanceRule` (16 bytes), decoded from the launch's `rule_params` (03) and validated by
`equip_launch` (`InvalidRule`):

| Field | Type | Meaning |
| --- | --- | --- |
| `metric` | u8 | `QuoteVolume` 0, `Twap` 1, `SwapCount` 2 |
| `window_secs` | u32 | the window measured, at least `MIN_TWAP_SECS` |
| `base_window_secs` | u32 | the longer window it is compared with |
| `op` | u8 | `Below` 0, `Above` 1 |
| `ratio_bps` | u16 | condition: `rate(window) op rate(base_window) * ratio_bps / 10_000`, rates per second |
| `hold_secs` | u32 | how long it must hold |

All three metrics come from the pool's observation ring (03), which must carry cumulative quote
volume and swap count beside the price accumulator (I-03-1).

`check_performance(slot)` (anyone; it moves no funds and pays no bounty): reads the launch, its
pool and its observations; sets or clears `condition_since` (event `PerformanceCondition { mint,
slot, true_since }`). If `now - condition_since >= hold_secs` and the slot does not already run its
launch item: the `execute` steps 2 to 5 with the launch item, any open proposal `Cancelled`, events
`PerformanceReverted { mint, slot, from_item, to_item: launch_item, ts }` and `EquipApplied { by:
Performance }`. A revert has no notice.

If the old item's vaults are not settled, the revert fails at `close_equip`; the cranking client
sends `settle_equip` first in the same transaction (section 10).

## 7. Launch

### 7.1 `equip_launch(slots: Vec<LaunchEquip>)` (R12)

Called by `create_launch` (03) **before the supply `mint_to`**, signed by the launchpad PDA
`["armory-caller", mint]` under `<LAUNCH_ID>` (the armory hard-codes `<LAUNCH_ID>`; that PDA signs
nothing else, like upstream's `["kit-caller", mint]`).

`LaunchEquip { slot: u8, item: Option<Pubkey>, rule_params: Vec<u8> }`, one per non-`Locked` slot
of the new mint.

Accounts: `launch_caller` (signer), `payer` (signer, mut), `token_mint`, `slot_authority`, then per
entry: `slot_state` (init for `Vote` and `Performance` slots), and when `item` is `Some`: the
`Item`, its `Template`, its program's `programdata`, the royalty holdings to create, the items
program's `EquipState`, registry and equip vault accounts (04's `init_equip` list); then
`armory_signer`, `<ITEMS_ID>`, the token program, `system_program`.

Checks: the signer (`NotLaunchCaller`); the mint was created in this launch (its supply is 0 and its
slots are all empty, `NotFreshMint`); per entry, 6.1 (the `Locked` rule check skipped: a slot whose
rule is `Locked` but whose kind is not is filled here once and never again); for a `Performance`
slot, `rule_params` decodes to a `PerformanceRule` with `window_secs >= MIN_TWAP_SECS` and
`base_window_secs > window_secs` (`InvalidRule`).

Effects per entry with an item: royalty holdings, `init_equip`, `set_slot_item`,
`equipped_count += 1`, and `SlotState { launch_item: Some(item), rule }`. Entries with `None` only
write `SlotState` (for `Vote` and `Performance`). Event `EquipApplied { by: Launch }` per item.

The launch pool does not exist yet at this point, so the launchpad writes the pool registry itself
when it creates the pool (03 4.3); `equip_launch` does not call `refresh_pool_registry`.

Depth: create_launch 1, armory 2, token `set_slot_item` / items `init_equip` / token
`create_holding` 3; from a companion launch, 4. To measure (07).

### 7.2 Locked slots and the kit

The kit (`<KIT_ID>`) is only ever placed in a `Locked` slot by 03 at mint creation; it is not an
item, earns no royalty, and the armory never sees it. `propose` and `check_performance` refuse a
`Locked` slot (`SlotLocked`).

## 8. War slots

Kind `War` (6) runs no callbacks; its item's parameters configure `hookwars_war` (siege threshold,
counter-strike trigger, bounty rate; 05 section 6 reads `Item` and `Template`). It is equipped and
voted on like any other slot. Its manifest has no cuts, no flags and `data_bytes 0`, so `init_equip`
creates an `EquipState` and no vault holdings, and no royalty is ever settled for it.

## 9. Forge

### 9.1 `forge(item_a, item_b)`

Accounts: `forger` (signer, mut), `config` (mut), `forge_counter` (init_if_needed), `item_a`,
`item_b` (mut, closed), their mints (mut), the forger's holdings of them (mut), `template`,
`minter`, the new `item_mint` and `item`, the forger's holding of the new mint, `armory_signer`,
`<ITEMS_ID>`, the token program, its event authority, `system_program`.

| Check | Error |
| --- | --- |
| `item_a != item_b`; both held by the forger (amount 1, not frozen) | `NotItemOwner` |
| same `template_id`; template `Active` and `forge_enabled` | `TemplateMismatch`, `TemplateClosed` |
| `equipped_count == 0` on both | `ItemEquipped` |
| `max(level_a, level_b) + 1 <= template.max_level` | `MaxLevel` |

Effects:

1. CPI `combine_params(T.id, ceilings, a.params, b.params)` into items, signed by `ArmorySigner`.
   04 applies `FORGE_GAIN_BPS` per field toward its ceiling (04 section 3, `forge_up` and
   `forge_down`) and merges or refuses identity fields (`NotForgeable`). The result is
   deterministic and symmetric in `a` and `b`.
2. The armory clamps every field of the result to `field_min..=field_max`, then runs 4.1 on it.
3. Burn both item tokens (token `burn` by the forger; item mints have no slots) and close both
   `Item` accounts, rent to the forger.
4. New item: `level = max(level_a, level_b) + 1`, `royalty_bps = max(royalty_a, royalty_b)` (both
   already at most `MAX_ROYALTY_BPS`), `source Forged`, `author = forger`, manifest from 4.1.
5. `forge_counter.count += 1`.
6. Events `ItemCreated` and `Forged { burned: [item_a, item_b], item, template_id, params, level,
   forger, ts }`.

Royalties already sitting in the burned items' royalty holdings become unclaimable once their
`Item` accounts are closed: `claim_royalty` needs the `Item`. Section 10 is the order the site
follows.

## 10. Settle and claim order (for the site and any client)

Money moves through two stages before anyone can take it, and some armory steps refuse while money
is still in the first stage. The order is:

1. **`settle_equip`** (04) for every vault of the item: its equip vault per cut mint and, for a Pool
   item, its share of `["pool-cuts", mint]`. This moves the royalty into the item's royalty holdings.
2. **`claim_royalty`** (5.2) for every royalty holding with a balance. When the cut mint is bridged
   SOL, unwrap in the same transaction.
3. Only then:
   - **`forge`**: an item's royalty holdings are lost with its `Item`. The site settles and claims
     both items' royalties in the transactions before the forge and refuses to build a forge while
     either has an unsettled or unclaimed balance it can read.
   - **`execute`** or a performance revert that removes an item: `close_equip` refuses an unsettled
     vault (`VaultNotSettled`), so the site prepends `settle_equip` for the outgoing item. Claiming
     can wait: royalty holdings outlive the equip.
   - **Selling or sending an item**: the site shows unsettled and unclaimed royalty on the listing,
     and offers settle and claim first, since the buyer would receive them.

## 11. Call depth

| Path | Height (top = 1) |
| --- | --- |
| `create_item` | armory 1, items `validate_params` / `manifest` 2, token `create_mint` / `mint_to` / `set_authority` 2 |
| `mint_loot` | war 1, armory 2, items or token 3 |
| `forge` | armory 1, items `combine_params` / `validate_params` / `manifest` 2, token `burn` / `create_mint` / `mint_to` 2 |
| `vote` | armory 1, token `set_vote_lock` 2 |
| `execute`, performance revert | armory 1, items `close_equip` / `init_equip` 2, token `set_slot_item` / `create_holding` 2, launchpad `refresh_pool_registry` 2 |
| `equip_launch` | launch 1, armory 2, items / token 3; companion launch 4 |
| `claim_royalty` | armory 1, token `transfer` 2 (no item runs, I-01-4) |

All within Solana's 5. Compute, bytes and trace entries for each: to measure (07).

## 12. Errors

`NotAdmin`, `TemplateUpgradeable`, `ProgramDataMissing`, `InvalidTemplateProgram`, `InvalidKind`,
`InvalidSchema`, `RoyaltyTooHigh`, `TemplateClosed`, `TemplateChanged`, `TemplateMismatch`,
`ParamOutOfRange`, `KindMismatch`, `NotLootSigner`, `NotLaunchCaller`, `NotFreshMint`,
`InvalidRule`, `NotItemOwner`, `InsufficientRoyalty`, `SlotLocked`, `OverBounds`,
`DataRangeTooSmall`, `AlreadyEquipped`, `ProposalOpen`, `VotingClosed`, `InvalidAmount`,
`NotExecutable`, `ItemEquipped`, `MaxLevel`. Errors from 04 (`BadParams`, `AboveCeiling`,
`NotForgeable`, `VaultNotSettled`) and 01 propagate unchanged.

## 13. Events (final names)

All by self-CPI (`emit_cpi!`), as upstream.

| Event | Fields |
| --- | --- |
| `TemplateRegistered` | `template_id, program, code_hash, deploy_slot, kind, field_count, field_min, field_max, name, ts` |
| `TemplateRetired` | `template_id, ts` |
| `ItemCreated` | `item, item_mint, template_id, params, manifest, author, royalty_bps, level, source, ts` |
| `LootMinted` | `item, owner, template_id, params, ts` |
| `Forged` | `burned: [Pubkey; 2], item, template_id, params, level, forger, ts` |
| `RoyaltyClaimed` | `item, cut_mint, claimant, amount, ts` |
| `ProposalCreated` | `mint, slot, nonce, proposer, item, vote_end, executable_at` |
| `VoteLocked` | `proposal, voter, support, amount, until` |
| `VoteUnlocked` | `proposal, voter, amount` |
| `ProposalResolved` | `proposal, status, votes_for, votes_against, eligible, ts` |
| `EquipApplied` | `mint, slot, old_item, new_item, by: Launch / Vote / Performance, ts` |
| `PerformanceCondition` | `mint, slot, true_since: Option<i64>` |
| `PerformanceReverted` | `mint, slot, from_item, to_item, ts` |

## 14. Interfaces

### From 01 (token program)

- **I-01-1** `set_slot_item(slot: u8, item: Pubkey, flags: u16)` with accounts `slot_authority`
  (signer, `["slots", mint]` under `<ARMORY_ID>`), `mint`, `program`. No royalty fields (R1): the
  token program neither stores nor computes a royalty. 01 bumps the slot's epoch byte (00 4.4).
- **I-01-2** The slot record exposes `kind`, `equip_rule`, `notice_secs`, `data_offset`,
  `data_len`, `item`, `program` and `bounds` (cut per side, discount, may refuse, may burn, pool
  reads, may be empty), fixed at launch.
- **I-01-3** `set_vote_lock(amount, until)` on a holding, signed by its owner and the mint's
  `SlotAuthority`, stored in `Holding.reserved` (upstream 16 bytes, `state.rs:98`); transfers and
  burns below the locked amount before `until` fail (`VoteLocked`). Confirmed as R11.
- **I-01-4** A transfer out of a royalty holding does not run the token's slot items: the token
  program skips them when the source owner equals
  `create_program_address(["royalty", item, bump], <ARMORY_ID>)` for the `item` and `bump` passed
  with the transfer, as upstream Half-Life exempts the launch PDA and its furnace. Without it, a
  Half-Life-style exit fee would tax royalty claims.
- **I-01-5** Item mints are created with no slots and no slot authority.

### From 03 (DEX and launch)

- **I-03-1** The observation ring carries cumulative price, quote volume and swap count.
- **I-03-2** `create_launch` calls `equip_launch` (7.1) before the supply `mint_to`, signing as
  `["armory-caller", mint]`, and runs `check_fits` from the armory's client crate in its plan.
- **I-03-3** `refresh_pool_registry(mint)` is callable by the armory in every equip of a `Pool`
  slot.

### From 04 (templates)

- **I-04-1** `validate_params`, `manifest`, `combine_params`, `init_equip`, `close_equip`, all
  checking `ArmorySigner` = `["armory"]` under `<ARMORY_ID>` (confirmed name).
- **I-04-2** `settle_equip` pays royalties into `["holding", cut_mint, ["royalty", item]]`; it reads
  `Item.royalty_bps`.
- **I-04-3** Per template: kind, field count, each field's floor and ceiling as named parameters,
  `max_level`, `loot_enabled`, `forge_enabled`, `open_authoring`.
- **I-04-4** Settled: `EquipState` is at `["equip", mint, slot]` under `<ITEMS_ID>` (00 4.3, 04
  section 2); `execute` and `equip_launch` derive it there.

### From 05 (war)

- **I-05-1** `mint_loot(owner, template_id, params)` is called only by `hookwars_war`, signing as
  `["loot-signer"]`.
- **I-05-2** 05 reads `Item`, `Template` and `ForgeCounter` (2.10) directly.
