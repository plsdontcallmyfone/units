# Hookwars spec 02: the armory (`hookwars_armory`)

Status: specification, 2026-10-08. Nothing here is built. Follows `00-overview.md`; names, seeds
and parameters are the overview's. Upstream references are to Bordrless at `43688f3`.

The armory owns everything about **items**: which hook programs may be used (templates), the items
themselves (owned, tradable, supply-1 tokens), their royalties, how a token's slot changes what it
runs (vote, performance, locked), loot minting for `hookwars_war`, and crafting (`forge`).

The token program (01) runs items on transfers; the launchpad (03) runs pool items on swaps; the
items program (04) implements the templates; the war program (05) calls `mint_loot`. The armory is
called by none of them on the hot path: no transfer or swap reads or writes an armory account.

## 1. Design choices

1. **Params are a fixed array of `u32` fields checked by the armory itself.** A template declares
   how many fields it uses, each field's `min` and `max` (the ceiling), and a **combine rule** per
   field for `forge`. The armory validates params, mints loot and forges with no CPI into the items
   program, so `mint_loot` (called from war) and `forge` stay shallow (section 9) and a template's
   ceiling is enforced in one audited place. The cost: a template's parameters must be expressible
   as bounded integers. Every template in 04 is (bps, seconds, counts, indices), so this holds.
   The rejected alternative, a `validate_params` view in the items program, would add a call level
   to every loot reveal and make the ceiling depend on code the armory cannot read.
2. **Usage counters are not written on chain per run.** An item equipped on many tokens would
   otherwise be a write-locked account shared by every transfer of every one of them. `runs` and
   `collected` per item come from the token program's events (indexer, 06); what the chain keeps is
   the royalty balances themselves, `equipped_count` (changed only by armory instructions) and
   `level` (changed only by `forge`).
3. **Voting locks tokens in place.** Moving tokens into an escrow would run the token's own slot
   items on the vote (a Half-Life-style exit fee, the kit's locks and max wallet would all apply to
   a vote). Instead the token program keeps a vote lock in the holding (section 6.3, interface I-01-3).
   Recommended over escrow; escrow is the fallback if 01 cannot add the lock.
4. **An item may be equipped on several tokens at once.** It is a licence to run a template with
   these parameters, not a physical object; its owner earns from every token that equips it. Only
   `forge` needs it unequipped everywhere.

## 2. Accounts

`F` below is `PARAM_FIELDS`, a layout constant (the number of `u32` fields an item carries). It is
**to set** in M2 from the largest template in 04 and must be added to 00 section 6. Sizes are
Anchor sizes: 8-byte discriminator plus the Borsh fields.

### 2.1 `ArmoryConfig` at `["config"]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version` | u8 | layout version |
| `bump` | u8 | |
| `admin` | Pubkey | registers and retires templates; set to `<PROTOCOL_AUTHORITY>` at `init`, changeable by `set_admin` (two-step: `propose_admin`, `accept_admin`) |
| `pending_admin` | Option<Pubkey> | |
| `items_minted` | u64 | counter; the seed of the next item mint |
| `templates` | u16 | number registered |
| `reserved` | [u8; 64] | |

Size: 8 + 1 + 1 + 32 + 33 + 8 + 2 + 64 = 149.

The admin can do nothing to an existing item, a royalty balance, a vote or a slot. There is no
pause.

### 2.2 `Template` at `["template", template_id: u16 le]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `id` | u16 | template id (dense, from `ArmoryConfig.templates`) |
| `program` | Pubkey | the hook program that runs it (usually `<ITEMS_ID>`; `half_life` and `tax_hook` behaviours may also be registered against their own programs) |
| `code_hash` | [u8; 32] | sha256 of the verified build, given by the admin (`solana-verify get-executable-hash`); a statement, shown on the site |
| `deploy_slot` | Option<u64> | for an upgradeable program, the slot in its ProgramData header at registration; `None` for an immutable one |
| `kind` | u8 | slot kind (00 section 4.1); `Locked` is never a template kind |
| `flags` | u16 | what items of it may do: `MAY_REFUSE` 1, `MAY_BURN` 2, `MAY_SET_FEE` 4, `WRITES_DATA` 8, `READS_POOLS` 16, `TAKES_CUTS` 32 |
| `data_len` | u8 | hook-data bytes an item of it needs (its range, 00 section 4.4), at most 64 |
| `field_count` | u8 | fields used, at most `F` |
| `field_min` | [u32; F] | lowest value per field |
| `field_max` | [u32; F] | ceiling per field |
| `combine` | [u8; F] | forge rule per field (section 8) |
| `cut_fields` | [u8; 2] | index of the field holding the max cut in bps on the buy side and on the sell side; `0xFF` when the template takes no cut on that side |
| `pool_reads` | u8 | number of other pools an item may read (0 unless `READS_POOLS`) |
| `open_authoring` | bool | anyone may `create_item` from it |
| `loot_enabled` | bool | `mint_loot` may mint it |
| `forge_enabled` | bool | `forge` accepts it |
| `max_level` | u8 | highest level `forge` may reach (per template, set in 04) |
| `loot_royalty_bps` | u16 | royalty of loot and forged items, at most `MAX_ROYALTY_BPS` |
| `status` | u8 | `Active` 0, `Retired` 1 |
| `name` | String (max 32) | |
| `registered_by` | Pubkey | the admin key that registered it |
| `created_at` | i64 | |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 2 + 32 + 32 + 9 + 1 + 2 + 1 + 1 + 4F + 4F + F + 2 + 1 + 1 + 1 + 1 + 1 + 2 + 1 + 36
+ 32 + 8 + 32 = 207 + 9F.

### 2.3 `Item` at `["item", item_mint]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `item_mint` | Pubkey | the supply-1 mint that represents ownership |
| `template_id` | u16 | |
| `params` | [u32; F] | inside the template's `field_min..=field_max`; unused fields are 0 |
| `author` | Pubkey | who created it (`create_item`: the signer; loot: the war program's caller PDA; forge: the forger) |
| `royalty_bps` | u16 | at most `MAX_ROYALTY_BPS`; fixed for life |
| `level` | u8 | 1 at creation; `forge` sets it |
| `source` | u8 | `Authored` 0, `Loot` 1, `Forged` 2 |
| `cut_buy_bps`, `cut_sell_bps` | u16, u16 | the manifest's max cut per side, copied from `params[cut_fields[..]]` at creation (0 when `0xFF`) |
| `flags` | u16 | the template's flags at creation |
| `data_len` | u8 | the template's `data_len` |
| `pool_reads` | u8 | the template's `pool_reads` |
| `equipped_count` | u32 | slots currently running it, across all tokens |
| `royalty_owner_bump` | u8 | bump of `["royalty", item]`, so the token program derives the royalty owner with `create_program_address` |
| `created_at` | i64 | |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 32 + 2 + 4F + 32 + 2 + 1 + 1 + 4 + 2 + 1 + 1 + 4 + 1 + 8 + 32 = 133 + 4F.

The manifest (cuts, flags, `data_len`, `pool_reads`) is copied into the item so a compatibility
check reads one account, and so a later template retirement never changes what an item promised.

### 2.4 The item mint

A Bordrless-standard `Mint` (upstream `programs/bordrless_token/src/state.rs:10-50`) at the
armory PDA `["item-mint", items_minted: u64 le]`, created by CPI `create_mint` with
`decimals 0`, `max_supply 1`, `mint_authority = Some(MintAuthority)`, no freeze, hook or metadata
authority, `hook_program None`, name from the template's name, symbol `ITEM`, uri the site's item
metadata route (06). The armory then CPIs `mint_to` of 1 to the recipient's holding and
`set_authority(MintTokens, None)`. Upstream: "A mint address is any signer: a keypair, or a PDA of
the creating program" (`docs/architecture.md`, "The token standard").

`MintAuthority` is the armory PDA `["minter"]`. It signs nothing else.

**Owner.** The owner of an item is whoever holds its one token: the holding
`["holding", item_mint, owner]` with `amount == 1` (upstream `Holding`, `state.rs:77-100`).
Transfers are ordinary token transfers; the item mint has no hook, so they cost what a plain
transfer costs. The armory never stores an owner.

### 2.5 `RoyaltyOwner` at `["royalty", item]`

A system-owned address with no data, signed for only by the armory. Its holding of each token the
item runs on, `["holding", token_mint, RoyaltyOwner]`, receives the item's royalty (01 delivers it
as a delta: a token-level delta credits a holding without calling the hook for it, upstream
hooks-v2 section 1.6). One royalty holding per (item, token).

### 2.6 `SlotState` at `["slot-state", token_mint, slot: u8]`

Created lazily by the first `propose` or `check_performance` on that slot (payer: the caller), so
`create_launch` stays the size it is.

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `mint`, `slot` | Pubkey, u8 | |
| `open_proposal` | Option<u64> | nonce of the proposal in `Open` or `Passed` state; at most one per slot |
| `next_nonce` | u64 | |
| `condition_since` | Option<i64> | performance rule: when its condition was first seen true without a break |
| `last_check` | i64 | |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 32 + 1 + 9 + 8 + 9 + 8 + 32 = 109.

### 2.7 `Proposal` at `["proposal", token_mint, slot: u8, nonce: u64 le]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8, u8 | |
| `mint`, `slot`, `nonce` | Pubkey, u8, u64 | |
| `proposer` | Pubkey | pays the rent, refunded at `close_proposal` |
| `item` | Option<Pubkey> | the item to equip; `None` empties the slot (only if the slot's bounds allow an empty slot, 01) |
| `created_at`, `vote_end` | i64, i64 | `vote_end = created_at + VOTE_PERIOD_SECS` |
| `executable_at` | i64 | `vote_end + slot.notice_secs` (the slot's notice, 01) |
| `votes_for`, `votes_against` | u64, u64 | locked token amounts |
| `status` | u8 | `Open` 0, `Passed` 1, `Failed` 2, `Executed` 3, `Cancelled` 4 |
| `reserved` | [u8; 32] | |

Size: 8 + 2 + 32 + 1 + 8 + 32 + 33 + 8 + 8 + 8 + 8 + 8 + 1 + 32 = 189.

### 2.8 `VoteLock` at `["vote", proposal, voter]`

| Field | Type | Meaning |
| --- | --- | --- |
| `proposal`, `voter` | Pubkey, Pubkey | |
| `amount` | u64 | votes cast (the locked amount counted) |
| `support` | bool | |
| `bump` | u8 | |

Size: 8 + 32 + 32 + 8 + 1 + 1 = 82. Its existence is what stops a second vote by the same voter on
the same proposal. Closed after the proposal is final; rent to the voter.

## 3. Templates

### 3.1 `register_template(args: RegisterTemplateArgs)`

Accounts: `admin` (signer, `== config.admin`), `payer` (signer, mut), `config` (mut), `template`
(init, `["template", config.templates]`), `program` (the hook program, executable),
`programdata` (its ProgramData address, as upstream passes it), `system_program`.

Args: `code_hash`, `kind`, `flags`, `data_len`, `field_count`, `field_min`, `field_max`,
`combine`, `cut_fields`, `pool_reads`, `open_authoring`, `loot_enabled`, `forge_enabled`,
`max_level`, `loot_royalty_bps`, `name`.

Checks, in order:

1. Upgrade authority, rule 3: exactly upstream's `check_hook_authority`
   (`programs/bordrless_launch/src/instructions/launch_config.rs:88-140`) against
   `HOOK_UPGRADE_AUTHORITIES` with `<MANAGED_HOOK_KEY>` and `<PROTOCOL_AUTHORITY>`
   (upstream `constants.rs:80-83`). Else `TemplateUpgradeable` / `ProgramDataMissing`.
2. `program` is not `<TOKEN_ID>`, `<SWAP_ID>`, `<LAUNCH_ID>`, `<KIT_ID>`, `<BRIDGE_ID>`,
   `<COMPANION_ID>`, `<ARMORY_ID>`, `<WAR_ID>`, the system program or the default key
   (upstream `PROTOCOL_PROGRAMS`, `constants.rs:97`, extended). Else `InvalidTemplateProgram`.
3. `kind` is `Fee`, `Reward`, `Defense`, `Relation` or `Pool`. Else `InvalidKind`.
4. `flags` has no unknown bit; `MAY_REFUSE` only with kind `Defense` or `Pool`; `MAY_BURN` and
   `MAY_SET_FEE` only with kind `Pool`; `READS_POOLS` only with `Relation` or `Pool`. Else
   `InvalidFlags`.
5. `data_len <= 64`, and `WRITES_DATA` set exactly when `data_len > 0`. Else `InvalidDataLen`.
6. `field_count <= F`; for every used field `field_min <= field_max`; unused fields are zero.
   `combine[i]` is a known rule (section 8). Else `InvalidSchema`.
7. `cut_fields` name used fields (or `0xFF`), set exactly when `TAKES_CUTS`; the `field_max` of a
   cut field is at most 10,000 bps. Else `InvalidCutField`.
8. `loot_royalty_bps <= MAX_ROYALTY_BPS`; `max_level >= 1`. Else `RoyaltyTooHigh` / `InvalidSchema`.
9. `deploy_slot` is read from the ProgramData header for an upgradeable program, `None` otherwise.

Event `TemplateRegistered { template_id, program, code_hash, deploy_slot, kind, flags, data_len,
field_count, field_min, field_max, combine, cut_fields, name, ts }`.

### 3.2 `retire_template(template_id)`

Admin only. Sets `status = Retired`: no new item from it (`create_item`, `mint_loot`, `forge`) and
no new equip of its items. Items already equipped keep running; nothing a token runs changes.
Event `TemplateRetired`.

### 3.3 Staleness

Every instruction that equips an item (`execute`, section 6.4) re-reads the template program's
ProgramData. If `deploy_slot` is `Some(s)` and the header slot is not `s`, the code has been
upgraded since registration: refused with `TemplateChanged`. The admin re-registers it as a new
template after review. Equipped items are not touched by staleness (rule 3 already restricts who
can upgrade); the site shows "upgraded since registration" from the same read (06).

## 4. Items

### 4.1 `create_item(template_id, params, royalty_bps)`

Accounts: `author` (signer, mut, pays), `config` (mut), `template`, `minter` (`["minter"]`),
`item_mint` (mut, `["item-mint", config.items_minted]`), `item` (init), `recipient_holding`
(mut; the author's holding of the new mint, created here), the token program and its event
authority, `system_program`.

Checks: template `Active` and `open_authoring` (`TemplateClosed`); every used field inside
`field_min..=field_max`, unused fields zero (`ParamOutOfRange`); `royalty_bps <=
MAX_ROYALTY_BPS` (`RoyaltyTooHigh`).

Effects: create the item mint (2.4), mint 1 to the author, revoke the mint authority, write
`Item` with `level 1`, `source Authored`, the manifest copied from the template, `author =` the
signer. `config.items_minted += 1`. Event `ItemCreated { item, item_mint, template_id, params,
author, royalty_bps, level, source, ts }`.

### 4.2 `mint_loot(template_id, params, recipient)` (CPI only)

Accounts: `war_caller` (signer: `["armory-caller"]` under `<WAR_ID>`, the armory hard-codes
`<WAR_ID>` as a constant, as the kit hard-codes `LAUNCH_ID`, upstream hooks-v2 section 4.1),
`payer` (signer, mut), then as `create_item`.

Checks: `war_caller` is that PDA at its canonical bump (`NotWarCaller`); template `Active` and
`loot_enabled` (`TemplateClosed`); params in range (`ParamOutOfRange`). The war program chooses
`params` from its loot table and randomness (05); the armory only guarantees the ceiling.

Effects: as `create_item`, with `royalty_bps = template.loot_royalty_bps`, `source Loot`,
`author = war_caller`. The item token goes to `recipient`'s holding. Event `ItemCreated`.

### 4.3 What an item is worth to the token program

01 runs an equipped item from the slot record, not from `Item`. At equip time (6.4) the armory
passes, and 01 stores in the slot: `item`, `program`, `template_id`, `royalty_bps`,
`royalty_owner_bump`, the manifest (`cut_buy_bps`, `cut_sell_bps`, `flags`, `data_len`), and
`params`. With those, a transfer needs no armory account: the royalty holding is
`["holding", mint, create_program_address(["royalty", item, royalty_owner_bump], ARMORY_ID)]`.

## 5. Royalties

### 5.1 How it accrues (01's side, restated so both parts agree)

On every transfer where an equipped item answers cuts `c1..ck` (`k <= 2`, since one of upstream's
three deltas, `MAX_DELTAS` in `crates/bordrless-hook`, is the royalty), the token program adds a
delta of `floor(sum(c) * royalty_bps / 10_000)` to the item's royalty holding and reduces the
item's own cut recipients by the same amount, so the royalty comes out of what the item collected,
never out of the trader. 00 rule 1 holds: the hook did not pay; the token program applied a cut.
The order against Bordrless's protocol share is DECISIONS H4 (recommendation: protocol share first,
in the DEX, as today).

### 5.2 `claim_royalty(amount)`

Accounts: `claimant` (signer), `item`, `item_holding` (`["holding", item.item_mint, claimant]`),
`royalty_owner` (`["royalty", item]`), `token_mint`, `royalty_holding` (mut,
`["holding", token_mint, royalty_owner]`), `destination` (mut, the claimant's holding of
`token_mint`, created if missing), the token program, its event authority, and the token's slot
accounts (see I-01-4).

Checks: `item_holding.mint == item.item_mint`, `owner == claimant`, `amount == 1`, not frozen
(`NotItemOwner`). `amount <= royalty_holding.amount` (`InsufficientRoyalty`); `u64::MAX` claims
everything.

Effects: CPI transfer signed by `royalty_owner`. Event `RoyaltyClaimed { item, token_mint,
claimant, amount, ts }`.

Whoever holds the item at claim time takes everything accrued so far, including what accrued under
an earlier owner. The site shows unclaimed royalties on the item's listing (06), so a buyer sees
what comes with it.

## 6. Equip rules

A slot's kind, bounds, equip rule, notice and data range are in the mint (01) and never change
(00 rule 2). The armory decides when its item changes and signs the change as `SlotAuthority`
(`["slots", token_mint]`), which is the only key 01 accepts for `set_slot_item`.

### 6.1 Compatibility (checked at `propose` and again at `execute`)

For item `I` and slot `S` of token `T`:

| Check | Error |
| --- | --- |
| `S.equip_rule != Locked` and `S.kind != Locked` | `SlotLocked` |
| template of `I` is `Active` and not stale (3.3) | `TemplateClosed`, `TemplateChanged` |
| template kind == `S.kind` | `KindMismatch` |
| `I.cut_buy_bps <= S.bounds.max_cut_buy_bps`, same for sell | `OverBounds` |
| `I.flags` has no `MAY_REFUSE`, `MAY_BURN`, `MAY_SET_FEE` the slot's bounds forbid | `OverBounds` |
| `I.data_len <= S.data_len` | `DataRangeTooSmall` |
| `I.pool_reads <= S.bounds.max_pool_reads` | `OverBounds` |
| `I.royalty_bps <= MAX_ROYALTY_BPS` (always true; kept against a lowered ceiling) | `RoyaltyTooHigh` |
| the item is not already in another slot of `T` | `AlreadyEquipped` |

`None` (emptying a slot) needs only the first check and `S.bounds.may_be_empty` (01).

### 6.2 `propose(slot, item: Option<Pubkey>)`

Accounts: `proposer` (signer, mut), `token_mint`, `slot_state` (init_if_needed), `proposal`
(init, nonce `slot_state.next_nonce`), `item` (when `Some`), its `template`, the template's
`programdata`, `system_program`.

Checks: 6.1; `slot_state.open_proposal` is `None` (`ProposalOpen`); the slot's rule is `Vote` or
`Performance` (`SlotLocked`).

Effects: write the proposal, `open_proposal = Some(nonce)`, `next_nonce += 1`. Event
`ProposalCreated { mint, slot, nonce, proposer, item, vote_end, executable_at }`.

### 6.3 `vote(support: bool, amount: u64)`

Accounts: `voter` (signer, mut), `proposal` (mut), `vote_lock` (init), `holding` (mut, the
voter's holding of the token), `token_mint`, `slot_authority`, the token program,
`system_program`.

Checks: `now < vote_end` (`VotingClosed`); `0 < amount <= holding.amount` (`InvalidAmount`);
`holding.owner == voter`.

Effects: CPI `set_vote_lock(holding, amount, until = proposal.vote_end)` (I-01-3), signed by the
voter and `SlotAuthority`. The token program keeps `max(amount)` and `max(until)` of overlapping
locks, so one lock can back votes on proposals for different slots, and refuses any transfer or
burn that would take the holding's amount below the locked amount before `until`. The tokens never
move, so no slot item runs on a vote. Add `amount` to `votes_for` or `votes_against`. Event
`Voted { proposal, voter, support, amount }`.

Votes cannot be flash-borrowed: the lock lasts until `vote_end`, after the transaction.

### 6.4 `finalize(proposal)` and `execute(proposal)`

`finalize` (anyone, `now >= vote_end`): quorum and majority.

```
eligible = token_mint.supply - pool_base_vault.amount - launch_holding.amount   // upstream excludes
                                                                               // the same two (hooks-v2 0.14)
passed   = votes_for > votes_against
           and (votes_for + votes_against) * 10_000 >= eligible * VOTE_QUORUM_BPS
```

Accounts add the launch, its pool's base vault and the launch PDA's holding, bound to
`token_mint` through the `Launch` account (`["launch", mint]` under `<LAUNCH_ID>`). Sets `Passed`
or `Failed`; on `Failed`, `open_proposal = None`. Event `ProposalFinalized`.

`execute` (anyone, `status == Passed`, `now >= executable_at`, else `NotExecutable`):

1. 6.1 again (an item retired or a template upgraded during the notice is refused: the proposal
   becomes `Failed` through `fail_stale`, below).
2. Create the item's royalty holding for `token_mint` if missing (CPI `create_holding`, the
   executor pays).
3. CPI `set_slot_item` (I-01-1) signed by `SlotAuthority`.
4. Old item's `equipped_count -= 1`, new item's `+= 1`.
5. `status = Executed`, `open_proposal = None`. Event `SlotEquipped { mint, slot, old_item,
   new_item, by: Vote, ts }`.

The items program may need per-slot accounts prepared before the item runs (its extra-accounts
registry, 04). `execute` takes them as remaining accounts and checks the registry exists at the
address 01 and 04 agree on (`HookRegistryMissing`); it does not create them (that is an ordinary
transaction anyone can send before `execute`).

`fail_stale(proposal)` (anyone): a `Passed` proposal whose 6.1 checks now fail becomes `Failed`;
`open_proposal = None`.

### 6.5 `cancel(proposal)`

The proposer, while `Open` and no votes are cast. `Cancelled`, `open_proposal = None`.

### 6.6 `close_proposal`, `close_vote`

Anyone, once the proposal is `Failed`, `Executed` or `Cancelled`: rent back to the proposer, and to
each voter for their `VoteLock`. Locks in the token program expire by time on their own.

### 6.7 Performance rule

A `Performance` slot accepts votes exactly as a `Vote` slot (otherwise it would never hold anything
but its launch item), and in addition reverts to its **launch item** (stored in the slot by 01 at
launch) when its condition holds.

The condition is stored in the slot's `rule_data` (01), fixed at launch:

| Field | Meaning |
| --- | --- |
| `metric` | `QuoteVolume` 0 (quote volume over the window), `Twap` 1 (time-weighted price), `SwapCount` 2 |
| `window_secs` | the window measured, at least `MIN_TWAP_SECS` |
| `base_window_secs` | the window it is compared against (longer) |
| `op` | `Below` 0, `Above` 1 |
| `ratio_bps` | threshold: condition is `metric(window) op metric(base_window) * ratio_bps / 10_000`, normalised per second |
| `hold_secs` | how long the condition must hold before the revert |

Every metric is read from the pool's observation ring (03), which must carry cumulative quote
volume and cumulative swap count beside the price accumulator (I-03-1).

`check_performance(slot)` (anyone, paid nothing: it moves no funds): reads the launch, its pool and
its observations; evaluates the condition; if true and `condition_since` is `None`, sets it to
`now`; if false, clears it. If `now - condition_since >= hold_secs` and the slot does not already run
its launch item: CPI `set_slot_item` with the launch item (6.4 steps 2 to 4), clear any open
proposal (`Cancelled`), event `SlotEquipped { by: Performance }`. A revert has no notice: the
launch item is what holders bought into.

## 7. Locked slots

`Locked` rule or kind: `propose` and `check_performance` refuse with `SlotLocked`. The kit
(`<KIT_ID>`) is only ever equipped in a locked slot at launch by 03; it is not an item and the
armory never sees it.

## 8. Forge

### 8.1 `forge(item_a, item_b)`

Accounts: `forger` (signer, mut), `config` (mut), `item_a`, `item_b` (mut, closed), their mints
(mut) and the forger's holdings of them (mut), `template`, `minter`, the new `item_mint` and `item`,
the forger's holding of the new mint, the token program, its event authority, `system_program`.

Checks:

| Check | Error |
| --- | --- |
| `item_a != item_b`, both held by the forger (amount 1, not frozen, owner the forger) | `NotItemOwner` |
| same `template_id`; template `Active` and `forge_enabled` | `TemplateMismatch`, `TemplateClosed` |
| `equipped_count == 0` on both | `ItemEquipped` |
| `max(level_a, level_b) + 1 <= template.max_level` | `MaxLevel` |

Effects:

1. Burn both item tokens (token `burn` by the forger; the item mints have no hook) and close both
   `Item` accounts, rent to the forger. Unclaimed royalties stay in their holdings and become
   unclaimable: the forger is told to claim first (the site claims in the same transaction, 06).
2. New params: for each used field `i`,
   `p[i] = clamp(combine_rule[i](a[i], b[i]), field_min[i], field_max[i])`.
3. New item: `level = max(level_a, level_b) + 1`, `royalty_bps = max(royalty_a, royalty_b)`
   (both already at most `MAX_ROYALTY_BPS`), `source Forged`, `author = forger`, manifest from the
   template with the new params.
4. Event `ItemForged { burned: [item_a, item_b], item, template_id, params, level, forger, ts }`
   and `ItemCreated`.

### 8.2 Combine rules

Deterministic, no randomness, always clamped to the template's ceiling. Each template in 04 names
the rule per field.

| Rule | Value | `combine(a, b)` |
| --- | --- | --- |
| `Max` | 0 | `max(a, b)` |
| `Min` | 1 | `min(a, b)` (for fields where lower is stronger, such as a notice or a threshold) |
| `SumCapped` | 2 | `a + b` (checked), then clamped |
| `MaxPlusStep` | 3 | `max(a, b) + step`, where `step` is `FORGE_STEP_<TEMPLATE>_<FIELD>`, a per-template constant defined in 04 and to set |
| `Keep` | 4 | `a` (fields that identify a target or a mode and must not blend) |

`Keep` fields must be equal in both items, else `TemplateMismatch` (a Raid on token X cannot be
forged with a Raid on token Y).

## 9. Call depth

| Path | Height (top = 1) |
| --- | --- |
| `create_item`, `forge` | armory 1, token `create_mint` / `mint_to` / `burn` / `set_authority` 2 |
| `vote` | armory 1, token `set_vote_lock` 2 |
| `execute`, `check_performance` | armory 1, token `set_slot_item` / `create_holding` 2 |
| `claim_royalty` | armory 1, token `transfer` 2, the token's slot items 3 unless I-01-4 skips them |
| `mint_loot` | war 1, armory 2, token 3 |

All within Solana's 5. Compute and bytes for each: to measure (07).

## 10. Errors

`TemplateUpgradeable`, `ProgramDataMissing`, `InvalidTemplateProgram`, `InvalidKind`,
`InvalidFlags`, `InvalidDataLen`, `InvalidSchema`, `InvalidCutField`, `RoyaltyTooHigh`,
`TemplateClosed`, `TemplateChanged`, `TemplateMismatch`, `ParamOutOfRange`, `NotWarCaller`,
`NotItemOwner`, `InsufficientRoyalty`, `SlotLocked`, `KindMismatch`, `OverBounds`,
`DataRangeTooSmall`, `AlreadyEquipped`, `ProposalOpen`, `VotingClosed`, `InvalidAmount`,
`NotExecutable`, `HookRegistryMissing`, `ItemEquipped`, `MaxLevel`, `NotAdmin`.

## 11. Events

`TemplateRegistered`, `TemplateRetired`, `ItemCreated`, `ItemForged`, `RoyaltyClaimed`,
`ProposalCreated`, `Voted`, `ProposalFinalized`, `SlotEquipped`, `PerformanceCondition { mint,
slot, true_since: Option<i64> }`. All by self-CPI (`emit_cpi!`), as upstream.

## 12. Interfaces

### From 01 (token program)

- **I-01-1** `set_slot_item(slot: u8, item: Option<SlotItem>)`, signed by `SlotAuthority`
  (`["slots", mint]` under `<ARMORY_ID>`, canonical bump). `SlotItem { item, program, template_id,
  royalty_bps, royalty_owner_bump, cut_buy_bps, cut_sell_bps, flags, data_len, params }`. 01
  stores it in the slot. Refused when the slot is `Locked`.
- **I-01-2** The slot record exposes `kind`, `equip_rule`, `notice_secs`, `data_offset`,
  `data_len`, `bounds { max_cut_buy_bps, max_cut_sell_bps, may_refuse, may_burn, may_set_fee,
  max_pool_reads, may_be_empty }`, `rule_data` (6.7) and `launch_item`, all fixed at launch.
- **I-01-3** `set_vote_lock(amount, until)` on a holding, signed by its owner and the mint's
  `SlotAuthority`; transfers and burns below the locked amount before `until` fail (`VoteLocked`).
  Uses part of `Holding.reserved` (upstream 16 bytes, `state.rs:98`). Fallback if refused: escrow
  holdings owned by the proposal, with every template in 04 exempting them.
- **I-01-4** A royalty claim must not pay the token's own slot items (a Half-Life-style exit fee on
  a royalty claim would tax the royalty). Requested: the token program skips slot items on a
  transfer whose source owner equals `create_program_address(["royalty", item, bump], ARMORY_ID)`
  for the `item` and `bump` passed with it, as upstream Half-Life exempts the launch PDA and
  the furnace.
- **I-01-5** The royalty delta of 5.1, computed by 01 from the slot's `royalty_bps`.

### From 03 (DEX and launch)

- **I-03-1** The observation ring carries cumulative price, quote volume and swap count, readable
  without the DEX's signer.
- **I-03-2** `create_launch` writes each slot's launch item through 01 and does not create
  `SlotState` (lazy).

### From 04 (templates)

- **I-04-1** For every template: `kind`, `flags`, `data_len`, `field_count`, each field's meaning,
  `field_min`, `field_max` (as parameter names to set), combine rule, `cut_fields`, `pool_reads`,
  `max_level`, `loot_enabled`, `forge_enabled`, and every `FORGE_STEP_*` it uses.
- **I-04-2** The address and seeds of the items program's extra-accounts registry per (mint, slot),
  which `execute` checks.

### From 05 (war)

- **I-05-1** `mint_loot` is called only by `hookwars_war`, signing as `["armory-caller"]`, with the
  recipient's wallet and a template and params from its loot table.

### To 00

- Add `PARAM_FIELDS` (layout, set in M2) and `FORGE_STEP_*` (per template, 04) to section 6.
