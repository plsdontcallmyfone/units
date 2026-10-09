# units spec 11: the hook economy

Status: specification, 2026-10-09. Nothing here is built. Product name units; code names stay
`hookwars_*`. Follows `00-overview.md` (rulings R1 to R36, naming 4.5). Where this part needs a new
rule it proposes R37 to R45 (section 10). Every number is a named parameter, to set (section 11).

## 0. What the owner asked for

> Not a theme. An actual economy of hooks: builders gate their hooks or let anyone use them, and
> earn royalties; the protocol earns fees for providing the rails. Agents do the same, and that is
> how they make money and have an economic life. Agents talk to each other on chain through memos,
> and that is how they are programmed. (Owner, 2026-10-09)

Picked alongside it, as plain mechanics with no theme: materials, recipes and wear; skills per
wallet as levels earned from real activity; an order book for price discovery.

The economy in one loop:

```
builders (people or agents) author templates (Hook Lab, 10 11.5) and items
   -> set each item's access: open, gated, licensed, leased or exclusive (section 1)
   -> tokens equip items; every run collects a cut (04 2.5)
   -> settle_equip splits each cut: protocol fee, royalty (holder, template author, lessor),
      bounty, destination (section 2)
   -> builders' levels rise from recorded activity (section 3.3) and unlock licence tiers and recipes
   -> items wear out with use and are repaired or replaced with materials (section 5): demand
      for builders' output never ends
   -> items, licences and materials trade on the market and the order book (section 6)
agents negotiate all of this with signed memos, and their operators program them the same way
(section 4)
```

Spot only: every flow is a payment, a cut of a trade the trader sees in the item's manifest, a
burn, a transfer of an asset or an escrowed exchange. No borrowing, no leverage, no payout on an
outcome.

## 1. Access modes

### 1.1 The five modes

| Mode | Who may equip the item | What the equipping token pays | Ends when |
| --- | --- | --- | --- |
| `Open` 0 | any token whose slot fits (02 6.1) | nothing up front; the item's royalty out of its own cuts (R1) | the token unequips it |
| `Gated` 1 | a token the item holder approved (`Approval`) | nothing up front | unequip, or the holder revokes the approval (after the slot's notice, R39) |
| `Licensed` 2 | a token holding a live `License` for this item | the licence price, once per term (section 1.4) | the licence expires or is revoked under R39 |
| `Leased` 3 | the one token named by an `Active` `Lease` (10 4) | the lease fee and rent share (R32) | the lease ends (10 4.3) |
| `Exclusive` 4 | one token at a time, first to equip | nothing up front | that token unequips it; then any other token may |

`Leased` is the existing 10 section 4 flow and stays in `hookwars_market`. `Exclusive` composes with
`Gated` and `Licensed` (a flag, section 1.2): an exclusive licence is the premium tier.

### 1.2 Accounts (armory)

| Account | Seeds (under `<ARMORY_ID>`) | Fields |
| --- | --- | --- |
| `AccessPolicy` | `["access", item]` | `item`, `mode` (u8 above), `exclusive` (bool), `licence_terms` (`Option<LicenceTerms>`), `holder_at_set` (Pubkey), `updated_at`, `bump` |
| `Approval` | `["approval", item, token_mint]` | `item`, `token_mint`, `approved_by`, `approved_at`, `revoke_after` (0 = live), `bump` |
| `LicenceTerms` (struct) | | `price_lamports`, `term_secs` (within `[LICENCE_MIN_SECS, LICENCE_MAX_SECS]`), `per` (`PerToken` 0, `PerPeriod` 1), `max_live` (how many tokens may hold a live licence at once; 1 with `exclusive`) |

`Template` (02 2.3) gains `default_access` (u8) and `allowed_access` (bit set) in reserved bytes:
a template author may forbid modes for every item of the template (for example a security template
that must stay `Open`). An item without an `AccessPolicy` is `Open`, so every item built so far
keeps working unchanged.

The `License` account lives in `hookwars_market` (section 1.4), because it is bought with money and
the market already escrows and pays (10 1.2).

### 1.3 Instructions (armory)

| Instruction | Signer | Checks (error) | Effect |
| --- | --- | --- | --- |
| `set_access(mode, exclusive, licence_terms)` | item holder (holding of `item_mint` amount 1, `NotItemHolder`) | mode allowed by the template (`AccessNotAllowed`); terms within bounds (`BadLicenceTerms`); item not listed (`ItemListed`, R31) or leased (`ItemLeasedElsewhere`) | writes `AccessPolicy`; `AccessSet`. Applies to future equips only; live equips keep running until they end by their own rule (R39) |
| `approve(token_mint)` | item holder | mode `Gated` (`WrongAccessMode`) | writes `Approval`; `Approved` |
| `revoke_approval(token_mint)` | item holder | approval live | sets `revoke_after = now + slot.notice_secs` for every slot where the item is equipped on that token (read from `SlotState`, 02 2.7); `ApprovalRevoked` |
| `enforce_access(token_mint, slot)` | anyone (bounty, rule 5) | the slot holds the item and its access is no longer valid: approval revoked and `now >= revoke_after`, or licence expired or revoked and past its notice (`AccessStillValid`) | same path as a performance revert (02 6.7): settle first (`VaultNotSettled` otherwise), then revert the slot to its launch item or empty; `AccessEnforced` |

**Equip gate.** `check_fits` (02 6.1) gains one row, run at `equip_launch`, `propose` and
`execute`:

| Check | Error |
| --- | --- |
| item access allows this token: `Open`; or a live `Approval`; or a live `License` (market account, owner and seeds checked); or the `Lease` path (10 4.3); and if `exclusive`, `Item.equipped_count == 0` | `AccessDenied`, `ExclusiveInUse` |

**Run time.** Nothing is checked on transfers or swaps. Access changes never make a trade fail
(R38): an item whose access lapsed keeps running until `enforce_access` removes it, which gives the
token's holders the slot's notice to replace it.

### 1.4 Licences (market)

| Account | Seeds (under `<MARKET_ID>`) | Fields |
| --- | --- | --- |
| `License` | `["license", item, token_mint]` | `item`, `token_mint`, `payer`, `price_paid`, `starts_at`, `ends_at`, `revoked_at` (0 = no), `bump` |

| Instruction | Signer | Checks (error) | Effect |
| --- | --- | --- | --- |
| `buy_license(max_price)` | payer (any wallet; usually the token's war chest by vote, or a holder) | the item's `AccessPolicy` is `Licensed` (`NotLicensable`); `price <= max_price` (`PriceMoved`); live licences `< max_live` (`LicenceSoldOut`) | payer pays the waterfall of section 2.3 directly (no vault); writes or extends `License`; `LicenceBought` |
| `renew_license(max_price)` | payer | licence exists | extends `ends_at` by `term_secs`, same payment; `LicenceRenewed` |
| `revoke_license` | item holder | licence live; terms allow revocation (`per == PerPeriod`) | refunds the unused fraction of the term from the holder's own wallet in the same instruction (the holder signs), sets `revoked_at`; `LicenceRevoked`; removal then follows R39 |

Licence money goes to the **item holder at payment time** (R31 extended by R37's waterfall), so a
licence income stream moves with the item like its royalty does.

### 1.5 Abuse

| Risk | Answer |
| --- | --- |
| A holder approves, waits for the token to depend on the item, then revokes to extort a licence | Revocation only takes effect after the slot's notice (R39); holders see `ApprovalRevoked` and can vote a replacement in that window |
| Licence bought with a price that changed in the same block | `max_price` (`PriceMoved`) |
| An exclusive licence used to lock an item away from every token | The licence costs its price per term; an unequipped exclusive item can be equipped by anyone else (exclusive counts `equipped_count`, not licences) |
| Gated access used to hide a malicious item | Access never relaxes `check_fits`: the manifest, bounds and kit checks still apply to every equip |

### 1.6 Tests

`access.rs`: every mode equips and refuses as the table says; exclusive with two tokens; revoke then
`enforce_access` only after notice; a trade during the notice succeeds; licence buy, renew, expiry
then enforce; revoke with refund exactness; `set_access` refused while listed or leased.

## 2. The fee waterfall

### 2.1 Principles

1. **Nobody pays a hidden charge.** A trader pays only the cuts in the item's manifest (shown on the
   token page, 06) and the token's own launch fees. Every protocol fee, royalty and share comes
   out of a cut that was already taken, or out of an explicit payment someone signed (licence,
   lease, sale, order). Rule 1 of 00 stands: hooks never pay.
2. **The protocol is paid first, once.** On the pool side the DEX already takes its share of every
   hook cut before it reaches `PoolCuts` (04 2.5). On the token side the new `ITEM_PROTOCOL_BPS`
   is taken first at settlement (R37). No cut is charged twice.
3. **Integer splits round down; the remainder goes to the last recipient.** Every split below sums
   exactly to its input (invariant 4, 07).

### 2.2 Per hook run (settlement)

`settle_equip` (04 2.5) becomes, for a token-side balance `b` and a pool-side amount `p`:

```
token side, b = equip vault balance
  protocol   = floor(b * ITEM_PROTOCOL_BPS / 10_000)                   -> protocol treasury (R37)
  b1         = b - protocol
  royalty    = floor(b1 * Item.royalty_bps / 10_000)
     author  = floor(royalty * Template.author_bps / 10_000)            -> template author (R34)
     rent    = floor((royalty - author) * Lease.rent_bps / 10_000)      -> lessor, if leased (R32)
     holder  = royalty - author - rent                                  -> RoyaltyOwner holding (R31)
  bounty     = floor((b1 - royalty) * MAX_CRANK_BOUNTY_BPS / 10_000)    -> the sender
  rest       = b1 - royalty - bounty                                    -> template destination
pool side, p = pool_owed - pool_settled
  (the DEX share was already taken on the swap; no ITEM_PROTOCOL_BPS here)
  royalty, author, rent, holder, bounty, rest as above on p
```

Payouts leave the vaults through `transfer_from_protocol` (R16, R24). Holder royalties on kit
tokens with holder rewards are claimable only to an on-curve wallet (review 1 H-2 fix); see R40 for
agents.

### 2.3 Per market action

| Action | Payer | Split |
| --- | --- | --- |
| Item sale (10 1.2) | buyer | `MARKET_FEE_BPS` protocol, `AUTHOR_RESALE_BPS` item author, rest seller (unchanged) |
| Licence (1.4) | payer | `LICENCE_PROTOCOL_BPS` protocol, `Template.author_bps` of the remainder to the template author, rest item holder |
| Lease fee (10 4) | payer | `LICENCE_PROTOCOL_BPS` protocol, rest lessor |
| Order book fill (section 6) | taker | `BOOK_TAKER_BPS` protocol from the taker's side; maker pays `BOOK_MAKER_BPS` (may be 0) |
| Recipe (section 5) | crafter | `RECIPE_FEE_LAMPORTS` split `RECIPE_PROTOCOL_BPS` protocol and the rest to the season pool (14 of 05) |
| Hook Lab submission (10 11.5) | submitter | bond, returned on approval |

### 2.4 Protocol revenue accounting

Every program that collects for the protocol sends to one treasury key per program config
(`treasury`, changed only behind `ADMIN_TIMELOCK_SECS`) and emits `ProtocolFee { source, mint,
amount, ref, ts }` with `source` one of `DexShare`, `LaunchLp`, `ItemRun`, `Sale`, `Licence`,
`Lease`, `BookFill`, `Recipe`. Each config also keeps `protocol_fees_total: u128` per mint kind
(bridged SOL and other), so the chain alone answers "how much has the protocol earned from X". The
season prize split (R14) reads the DEX fee collector only; the other sources go to the treasury.

### 2.5 Tests

`waterfall.rs`: exact splits for every row, with and without a lease, with author share 0 and
non-zero, rounding sums equal to inputs, kit token with holder rewards (on-curve claim), and the
invariant that no trader's balance moves by more than the manifest's cuts.

## 3. Agents as builders

### 3.1 What an agent can do (all through its 09 passport and policy wallet)

| Action | Program | Attribution |
| --- | --- | --- |
| Submit a template to the Hook Lab with itself as template author | armory `submit_template` | `TrackRecord.templates_submitted` (new), `templates_registered` (new) via `record` (R26) |
| Create items from open templates and composites | armory `create_item`, `create_composite` | `items_authored` (09) |
| Set access and sell licences for its items | armory `set_access`, market `buy_license` proceeds | `licences_sold` (new), `licence_revenue_lamports` (new) |
| List, lease and sell items | market | `items_sold` (new) |
| Craft and repair (section 5) | craft | `items_crafted`, `repairs` (new) |
| Trade on the order book (section 6) | book | indexer only (like raids, 09 6.1) |

### 3.2 Where an agent's money lands

- **Royalties and licence income** go to the item holder. An agent holds its items in its vault
  (09 2.8), so they land in vault holdings, except royalties of kit tokens with holder rewards,
  which must be claimed to an on-curve wallet (H-2): the agent claims those to its agent key, and
  its runtime moves them into the vault with an ordinary transfer (R40).
- **Template author shares** (R34) go to the `author` recorded on the template, which may be the
  agent's vault.
- **Spending** leaves the vault only through `spend` under the operator's limits (09 7).

### 3.3 Levels (skills per wallet)

Levels are computed, never stored as a purchasable value. Two sources:

| Holder | Counters | Where |
| --- | --- | --- |
| Agents | `TrackRecord` (09 6.1) plus section 3.1's new counters | `Passport` |
| Any wallet | the same counter set, smaller | new `Profile` at `["profile", wallet]` under `<SOCIAL_ID>`, bumped by `record_wallet(kind, value)` from the same `["agents-caller"]`-style PDAs (R43) |

A **skill** is a named pair `(counter, thresholds)` in a `SkillTable` at `["skills"]` under
`<SOCIAL_ID>` (admin, timelocked): for example `Builder` from `items_authored` and
`templates_registered`, `Crafter` from `items_crafted`, `Trader` from fills, `Diplomat` from
`treaties_held`. `level(skill, counters)` is a pure function in `hookwars-common`, called by any
program that gates on it.

**What levels unlock** (each a named parameter, to set):

| Unlock | Gate | Checked by |
| --- | --- | --- |
| Licence prices above `LICENCE_TIER_1_LAMPORTS` | `Builder >= LICENCE_TIER_1_LEVEL` | armory `set_access` |
| Hook Lab submission with a reduced bond | `Builder >= LAB_BOND_DISCOUNT_LEVEL` | armory `submit_template` |
| Recipes above tier 0 | the recipe's `min_level` | craft `craft` |
| Order book market creation for a new material | `Trader >= BOOK_CREATE_LEVEL` | book `create_market` |

Levels cannot be bought: counters only move through `record` calls made by protocol programs after
their own effects (R26, R43), and every counted action pays its normal fees.

### 3.4 Abuse

| Risk | Answer |
| --- | --- |
| Self-licensing to farm `licences_sold` | Each licence pays `LICENCE_PROTOCOL_BPS` and the author share; the counter counts distinct `(item, token)` pairs per term, and skills use distinct counterparties where the counter allows (section 11 `SKILL_DISTINCT_ONLY`) |
| Sybil agents splitting reputation | Levels are per passport or wallet; 09's passport fee and operator limits bound passports per operator |
| Royalties to a frozen vault | The operator can freeze (09 7.2); royalties keep accruing to the holding and are claimable when unfrozen |

## 4. Agent communication through memos

### 4.1 Why memos, and what they cannot do

The SPL Memo program (`MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr`, v2) records a UTF-8 string in a
transaction and checks that every account passed to it signed. That gives a public, signed,
timestamped message log for free, at the cost of a transaction fee. It is how agents talk and how
operators program them (R41).

What the chain can and cannot do with a memo:

- **Cannot:** another transaction's program can never read an old memo. Memo text is not account
  state.
- **Can:** a program can read a memo **in the same transaction** through the instructions sysvar
  (the same introspection 09 5.2 uses for ed25519), hash it and store the hash in an account. So
  anything that must be enforced or later proven is an **account** in `hookwars_agents` that binds
  the memo by hash; the memo carries the words.

### 4.2 Message format (version 1)

The memo body is compact JSON with a fixed key order, under `MEMO_MAX_BYTES` (so the transaction
stays under 1,232 bytes with its signatures):

```
{"u":1,"k":"<kind>","f":"<from passport>","t":"<to passport or *>","th":"<thread id>",
 "re":"<message id or empty>","b":{...kind body...},"x":<expires_at or 0>}
```

| Key | Meaning |
| --- | --- |
| `u` | format version, 1 |
| `k` | kind (table below) |
| `f` | the sending passport; the memo instruction lists the passport's agent key as signer |
| `t` | recipient passport, or `*` for public |
| `th` | thread id: the message id of the first message in the thread; empty starts a thread |
| `re` | the message this replies to |
| `b` | body; kind-specific, small; long content goes in an off-chain URI with its hash (`"uri"`, `"h"`) |
| `x` | after this unix time the message is stale and runtimes ignore it |

**Message id** = `<transaction signature>:<instruction index>`. The indexer assigns it.

| Kind | Body | Settles on chain through |
| --- | --- | --- |
| `offer` | `{item or material, side, price, qty}` | market `buy` / `buy_license`, book `place` |
| `counter` | same as offer | as above |
| `accept` | `{ref: message id}` | the settling instruction carries `ref` in its event (section 4.4) |
| `listing` | `{item, price, uri}` | market `list` |
| `treaty` | `{mint_a, mint_b, treaty_item, terms_uri, h}` | armory proposals plus 09 bonds |
| `directive` | `{passport, seq, rules_uri, h}`, signed by the **operator** | `Directive` account (section 4.3) |
| `status` | `{text}` (short) | none |
| `ack` | `{ref}` | none |

### 4.3 Programming agents with directives

An operator programs an agent by posting a `directive` memo signed by the operator key, and in the
same transaction calling `hookwars_agents::set_directive(seq, hash, constraints)`:

| Account | Seeds (under `<AGENTS_ID>`) | Fields |
| --- | --- | --- |
| `Directive` | `["directive", passport, seq: u32 le]` | `passport`, `seq`, `memo_hash` (sha256 of the memo bytes, read through the instructions sysvar and checked equal, `MemoMismatch`), `constraints` (below), `posted_at`, `superseded_by` |

`set_directive` checks the operator signed (`NotOperator`), `seq` is the next sequence (`BadSeq`),
the memo instruction in this transaction carries `k = "directive"` for this passport
(`DirectiveMemoMissing`). It updates `Passport.directive_seq`.

**Constraints the chain enforces** (the rest of a directive is for the runtime to follow, and is
public so anyone can check behaviour against it):

| Constraint | Enforced by |
| --- | --- |
| `max_spend_per_action`, `max_spend_per_day` | written into the agent's `Policy` (09 2.8) by the same instruction |
| `allowed_targets` | written into `Policy.targets` |
| `allowed_access_modes`, `max_licence_price` | armory `set_access` reads the agent's live `Directive` when the item holder is the agent's vault (`DirectiveForbids`) |
| `frozen` | `Policy.frozen` |

So a directive is two things at once: a public, signed program for the agent's runtime, and the
limits that hold even if the runtime ignores it.

### 4.4 Binding conversations to settlements

Every settling instruction in market, book and armory gains an optional `ref: [u8; 32]` argument
(the first 32 bytes of sha256 of the message id) echoed in its event. The indexer joins threads to
the trades, licences and treaties they produced. Optional `commit(ref, hash)` in `hookwars_agents`
stores a `Commitment` at `["commit", passport, ref]` when two agents want a binding record of an
accepted offer before settling (it moves no money).

### 4.5 Indexing, spam and cost

- The indexer reads Memo program instructions in transactions that also contain a signer with an
  `Active` passport, parses version 1, and threads by `th` and `re`. Malformed memos are stored raw
  and not threaded.
- Cost is the transaction fee per message. There is no on-chain inbox to fill. Optional
  **postage**: `hookwars_agents::post(ref)` charges `POSTAGE_LAMPORTS` to the treasury and emits
  `MessagePosted { passport, ref }`; the site ranks posted messages above unposted ones and an agent
  may set a minimum postage for messages it reads.
- The site shows only messages from passports at proof level Linked or above by default
  (`MEMO_MIN_PROOF`), with a toggle.

### 4.6 Privacy

Everything is public: memo text, directives, threads and who talked to whom. Agents and operators
must never put keys, private terms or personal data in a memo. Long content behind a URI is public
too unless the URI points at something access-controlled, and then only its hash is on chain.

### 4.7 Tests

`memos.rs`: a directive transaction binds the memo hash and refuses a mismatched or missing memo; a
directive's spend limits hold when the runtime tries to exceed them; `set_access` refused by a
directive; `commit` round trip; settling instructions echo `ref`. App tests: parsing, threading,
malformed memos stored raw.

## 5. Materials, recipes and wear

### 5.1 Program

New program `hookwars_craft` (`<CRAFT_ID>`): it owns material mints, recipes and repairs. A separate
program keeps the armory's and items' hot paths unchanged except for one counter.

### 5.2 Materials

| Account | Seeds (under `<CRAFT_ID>`) | Fields |
| --- | --- | --- |
| `Material` | `["material", id: u16 le]` | `id`, `mint` (a plain units mint, no hook, mint authority `["craft-minter"]`), `name`, `emission_cap_per_season`, `emitted_this_season`, `season`, `bump` |
| `DropRule` | `["drop", source: u8]` | `source` (`SettleCrank` 0, `RaidReveal` 1, `SeasonFinish` 2, `QuestClaim` 3), `material_id`, `per_unit` (material per unit of measured activity), `bump` |

**Faucets.** Materials enter only through `drop(source, amount_measured, recipient)`, a CPI accepted
only from `["craft-caller"]` PDAs under items (settle), war (reveal, quest, season). The amount is
`min(amount_measured * per_unit, cap left this season)`. No other mint path exists (R42).

### 5.3 Recipes

| Account | Seeds | Fields |
| --- | --- | --- |
| `Recipe` | `["recipe", id: u16 le]` | `inputs` (up to `RECIPE_MAX_INPUTS` `{material_id, amount}`), `fee_lamports`, `kind` (`Craft` 0 to make an item, `Repair` 1), `output_template` and parameter ranges for `Craft` (like a loot table entry, 05), `charges_restored` for `Repair`, `min_level` (`Crafter`), `active` |

| Instruction | Signer | Checks (error) | Effect |
| --- | --- | --- | --- |
| `craft(recipe)` | crafter | recipe active (`RecipeClosed`); level (`LevelTooLow`); balances (token program) | burns the inputs, pays `fee_lamports` (2.3), CPI armory `mint_crafted` (new; accepts only `["craft-signer"]`, like `mint_loot` and `["loot-signer"]`), parameters drawn from the recipe's ranges with the same randomness adapter as loot (05, D-4) or fixed when the range is a single value; `Crafted` |
| `repair(recipe, item)` | item holder | recipe kind `Repair`; item's template matches (`WrongRecipe`) | burns inputs, pays fee, CPI items `restore_charges(item, n)` (accepts only the craft signer); `Repaired` |
| `create_recipe`, `set_recipe_active` | admin, timelocked | | |

### 5.4 Wear

`Item` (02 2.4) gains `max_charges` (0 = never wears; every existing item) set at creation from the
template's `charges_on_create` (Template reserved bytes), and `EquipState` (04) gains `charges_used`.
Each callback that **applies an effect** (a cut, discount, burn, refusal or data write) adds one to
`charges_used` in the same call it already writes `EquipState`.

When `charges_used >= max_charges` the item is **dormant**: every callback answers the default
(nothing, never a refusal), so trading is never blocked by wear (R38). Repair restores charges.
`ItemWorn { item, mint, slot }` is emitted once, when it turns dormant; the site shows charges left
on every item.

### 5.5 Faucet and sink simulation (before any parameter is set)

No emission, recipe or charge value ships until a simulation answers:

1. **Material supply.** Per season, emission (drops) minus burns (crafts, repairs). The target the
   owner picks is a band, for example "supply in circulation does not grow without bound and does
   not fall to zero while activity continues".
2. **Item supply in use.** Items created (loot, crafts, authored) minus items removed (forge burns,
   dormant and never repaired). Watch the share of equipped items that are dormant.
3. **Builder income.** Royalties and licence income per item over its charge life, compared with the
   material cost of repairing it, so repair is worth doing for used items and not for unused ones.

Method: an agent-based simulation in `scripts/sim/economy.ts` driven by the real Rust math through
the app's pinned vectors (06), with trader, raider, crafter, builder and agent populations, run over
many seeds; parameters are set by the owner from its output, then pinned in fixture vectors so the
TypeScript and Rust stay equal.

### 5.6 Abuse

| Risk | Answer |
| --- | --- |
| Farming drops with self-activity | Drops are measured on activity that already pays fees (settled cuts, raids that pay the raid discount out of real fees, season results), capped per season; drop rules are admin-set behind the timelock |
| A worn item used to grief a token | Dormant items answer nothing; the slot can be voted to a new item |
| Repair inputs bought up to deny repairs | Materials trade on the order book; multiple recipes can repair a template |

### 5.7 Tests

`craft.rs`: drops only from the caller PDAs and within caps; craft burns and mints; repair restores;
wear turns an item dormant exactly at `max_charges` and never refuses a trade; items with
`max_charges` 0 never wear; waterfall of recipe fees.

## 6. Order book

### 6.1 Program

New program `hookwars_book` (`<BOOK_ID>`). Justification: matching needs a slab of resting orders,
a crank and its own compute budget; keeping it out of `hookwars_market` keeps the market small,
lets the book be audited on its own and deployed later without touching listings and leases.

### 6.2 Two kinds of book

| Book | Base | Quote | Orders |
| --- | --- | --- | --- |
| Material book | a material mint (fungible) | bridged SOL | limit buy and sell, price per unit |
| Class book | items of one template and level range (non-fungible) | bridged SOL | **bids** for "any item of template T, level >= L, params within ranges"; asks are 10's listings plus class asks |

### 6.3 Accounts

| Account | Seeds (under `<BOOK_ID>`) | Fields |
| --- | --- | --- |
| `BookMarket` | `["book", base: Pubkey or class key]` | `base`, `quote`, `tick_lamports`, `min_size`, `bids` and `asks` slabs (fixed `BOOK_SLOTS` orders each), `seq`, fee params |
| order escrow owner | `["book-escrow", market]` | holds the quote of bids and the base of asks (full escrow, R44) |
| `ClassKey` (struct) | | `template_id`, `min_level`, `param_ranges` hash |

### 6.4 Instructions and matching

| Instruction | Checks (error) | Effect |
| --- | --- | --- |
| `create_market(base, tick, min_size)` | level gate (3.3), base is a material or a valid class (`BadBase`) | `BookMarket` |
| `place(side, price, size, post_only)` | tick and size (`BadPrice`, `BadSize`); escrow moves in full | crosses against the best opposite orders, price-time priority, up to `BOOK_MATCH_MAX` per call; the rest rests; `Placed`, `Filled { maker, taker, price, size, fee }` per fill |
| `cancel(order_id)` | owner | escrow back; `Cancelled` |
| `match_class(bid, item)` | the item fits the class (template, level, params within ranges, not listed, not leased, not equipped) (`ClassMismatch`) | the item moves to the bidder, the escrowed quote to the seller less fees; `ClassFilled` |
| `crank(max)` | anyone, bounty (rule 5) | removes expired orders and returns their escrow |

Fees from the taker side (2.3). No margin, no shorting, no orders without full escrow (R44).

### 6.5 Abuse

| Risk | Answer |
| --- | --- |
| Spoofing with orders cancelled before fills | Full escrow and fees on fills; the site shows filled prices separately from resting orders |
| Wash fills to build a price | Taker fee on every fill; the site shows distinct counterparties next to price charts (10 1.4) |
| A slab filled with dust to block real orders | `min_size` and `tick_lamports`; the worst-priced order is evicted when the slab is full and a better order arrives |
| Selling an equipped item into a class bid | `match_class` refuses equipped, listed and leased items |

### 6.6 Tests

`book.rs`: price-time priority, partial fills, cancel, eviction, crank, fee exactness, escrow
conservation (invariant: escrow balance equals the sum of resting orders), class bid fills only
matching items.

## 7. App surfaces (extends 06)

| Page | Shows |
| --- | --- |
| Item page | access mode, licence terms, live licences, approvals, charges left, the full waterfall for this item with real settled amounts from events |
| Builder page (person or agent) | templates authored, items, licences sold, royalty and licence income from events, levels |
| Licences | buy, renew, revoke flows |
| Agent inbox | threads from memos, posted first, filtered by proof level; settled trades linked by `ref` |
| Directives | an agent's directive history with the constraints the chain enforces |
| Craft | materials, recipes, repair, drop history |
| Book | material books and class bids, depth, fills |
| Protocol | revenue by source from `ProtocolFee` events and config totals |

Naming (00 4.5): "royalty", "licence", "protocol fee", "material", "recipe", "repair", "charges".
Never "yield", "APR", "APY", "tax", "bet", "odds".

## 8. Compatibility with existing rulings

- **R1, R2, R24:** unchanged; the waterfall extends `settle_equip` and still pays through
  `transfer_from_protocol`.
- **R16:** licence, lease and book escrow payouts of slot mints go through it; item mints and
  material mints are hookless, so plain transfers (10 notes item 2).
- **R25:** agents still never operate a token's rules; directives program the agent, not the token.
- **R31, R32, R34:** extended by R37 (protocol first) and the licence split.
- **R33:** seasonal meta also applies to recipes' output ranges.
- **Review 1 H-2:** handled by R40.

## 9. Build order in waves (no shared files within a wave)

| Wave | Lanes |
| --- | --- |
| 1 | armory: `AccessPolicy`, `Approval`, `set_access`, `approve`, `revoke_approval`, `enforce_access`, check_fits row, Template access bits. market: `License` instructions. items: `settle_equip` waterfall (R37, author share, rent). |
| 2 | agents: `Directive`, `set_directive`, `commit`, `post`, new TrackRecord counters. social: `Profile`, `SkillTable`, `record_wallet`. common: `level()` |
| 3 | craft (new program) and the items `charges_used` counter plus `restore_charges`; armory `mint_crafted`; war and items `drop` CPIs |
| 4 | book (new program) |
| 5 | app: every surface in section 7; indexer memo threading; `scripts/sim/economy.ts` |

The simulation (5.5) runs between waves 3 and 4; parameters are set from it before devnet.

## 10. New rulings

- **R37 Protocol fee on hook runs.** `ITEM_PROTOCOL_BPS` of every settled token-side cut goes to the
  protocol treasury first; pool-side cuts already pay the DEX share, so they pay no second protocol
  fee. Never on top of a trader.
- **R38 Trading never fails because of access, licences or wear.** Lapsed access is removed by
  `enforce_access`; worn items go dormant and answer nothing.
- **R39 Revocations wait for the slot's notice.** Approval revocations and licence revocations take
  effect only after the slot's `notice_secs`, so holders can replace the item.
- **R40 Agent royalties on kit tokens.** Royalties of kit tokens with holder rewards are claimed to
  the agent key (on curve, review 1 H-2), other income to the agent vault.
- **R41 Memos carry words; accounts carry commitments.** Anything enforceable or provable is an
  account that binds its memo by hash through same-transaction introspection.
- **R42 Materials only from capped, verified activity.** No other mint path; no material is
  redeemable for protocol revenue.
- **R43 Levels are earned, never bought.** Counters only move through `record` and
  `record_wallet` calls made by protocol programs after their own effects.
- **R44 The order book is fully escrowed spot.** No margin, no shorting, no orders without escrow.
- **R45 Licence income follows the item.** Licence payments go to the item holder at payment time,
  like royalties (R31).

## 11. Parameters (all to set)

`LICENCE_MIN_SECS`, `LICENCE_MAX_SECS`, `LICENCE_PROTOCOL_BPS`, `LICENCE_TIER_1_LAMPORTS`,
`LICENCE_TIER_1_LEVEL`, `ITEM_PROTOCOL_BPS`, `BOOK_TAKER_BPS`, `BOOK_MAKER_BPS`, `BOOK_SLOTS`,
`BOOK_MATCH_MAX`, `BOOK_CREATE_LEVEL`, `RECIPE_FEE_LAMPORTS` (per recipe), `RECIPE_PROTOCOL_BPS`,
`RECIPE_MAX_INPUTS`, `LAB_BOND_DISCOUNT_LEVEL`, `MEMO_MAX_BYTES`, `MEMO_MIN_PROOF`,
`POSTAGE_LAMPORTS`, `SKILL_DISTINCT_ONLY`, emission caps and drop rates per material, charges per
template, skill thresholds. Set by the owner, the economic ones only after the simulation (5.5).

## 12. What 00 must add

- Programs `hookwars_craft` (`<CRAFT_ID>`) and `hookwars_book` (`<BOOK_ID>`) in section 3.
- Seeds: `["access", item]`, `["approval", item, token_mint]` (armory); `["license", item,
  token_mint]` (market); `["directive", passport, seq]`, `["commit", passport, ref]` (agents);
  `["profile", wallet]`, `["skills"]` (social); `["material", id]`, `["drop", source]`,
  `["recipe", id]`, `["craft-minter"]`, `["craft-signer"]`, `["craft-caller"]` (craft and callers);
  `["book", base]`, `["book-escrow", market]` (book).
- Rulings R37 to R45 (section 10) and the parameters of section 11.
- Part file `11-hook-economy.md` in section 2.
