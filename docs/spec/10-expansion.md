# units spec 10: expansion (hooks are assets, social, economy, events and reach)

Status: specification, 2026-10-09. Nothing here is built. Product name **units**; code names stay
`hookwars_*` until the global rename. This part follows 00 (names, seeds, rulings R1 to R24,
naming rules 4.5) and builds on 02 (armory), 04 and 08 (items and the arsenal), 05 (war) and 06
(app), including their implementation notes. Where this part and 00 differ, 00 wins until 00 adds
the rulings listed in section 13.

Every figure Hookwars introduces here is a **named parameter, to set** (by the owner, O, or by
measurement, M). Spot only: every money movement is a transfer from a program-owned vault, a cut
taken by a hook into a vault (R1, R2), a burn, a fee change or a spot swap. Nothing pays out on an
outcome someone wagered on.

Agent passports (the identity of AI agents: passport account, soulbound badge, operator co-sign,
declared, linked and attested levels, diplomat bonds, scoped wallets) are approved scope too and
get their own part, `09-agents.md` (not written yet). This part only depends on it where noted
(agent leagues, commissions submitted by agents).

## 0. The principle: hooks are assets

An item (02 section 2.4) is already a supply-1 units token whose holder owns its royalty stream:
royalties accrue to the item's `RoyaltyOwner` (`["royalty", item]`) and `claim_royalty` pays
whoever holds the item token at claim time (02 section 5.2). So royalty income already moves with
the item. This part makes that visible and tradable:

| Asset property | Where it lives |
| --- | --- |
| Ownership | the item token (02 2.5); transfers like any units token |
| Income | royalty holdings per cut mint (02 2.6), claimable by the holder |
| Income history | `RoyaltyClaimed` events (02 13) plus `settle_equip` events (04), indexed |
| Usage | `equipped_count` on chain (02 2.4); runs and cut totals from events (M2 notes) |
| Provenance | new `Lineage` (section 2) |
| Price history | new market events (section 1) |
| Grouping | new `Collection` (section 1.5) |
| Renting it out | new `Lease` (section 4) |

Rule for every feature below (new ruling **R31**): an item's income always belongs to the current
holder of the item token at claim time, except while a `Lease` assigns a share of it to the lessor
for the lease term (section 4). Nothing else may redirect an item's royalty.

## 1. Item marketplace (`hookwars_market`, new)

### 1.1 Accounts and seeds

| Account | Seeds (under `<MARKET_ID>`) | Fields |
| --- | --- | --- |
| `MarketConfig` | `["market-config"]` | `admin`, `fee_bps` (`MARKET_FEE_BPS`, O), `author_resale_bps` (`AUTHOR_RESALE_BPS`, O), `treasury`, pending admin changes with `eta` (`ADMIN_TIMELOCK_SECS`, 00) |
| `Listing` | `["listing", item_mint]` | `seller`, `item`, `item_mint`, `price_lamports`, `created_at`, `expires_at` (0 = none), `bump` |
| escrow owner (system-owned, signs) | `["escrow", item_mint]` | holds the item token while listed |
| `Collection` | `["collection", id: u32 le]` | `name` (max 32), `curator`, `template_ids` (max `COLLECTION_MAX_TEMPLATES`, O), `created_at` |

There is one listing per item at a time (the seed is the item mint).

### 1.2 Instructions

| Instruction | Signer | Checks (error) | Effect |
| --- | --- | --- | --- |
| `list(price_lamports, expires_at)` | item holder | holding of `item_mint` has `amount == 1` (`NotItemHolder`); `price_lamports > 0` (`ZeroPrice`); no live listing (Anchor `init`); the item is not under a live `Lease` that forbids sale (section 4, `LeasedItem`) | moves the item token into the escrow holding; writes `Listing`; `Listed` |
| `delist` | seller | `Listing.seller == signer` (`NotSeller`) | escrow returns the token by `transfer_from_protocol` (R16, R24); closes `Listing`; `Delisted` |
| `buy(max_price)` | buyer | listing live and not expired (`ListingExpired`); `price <= max_price` (`PriceMoved`) | buyer pays SOL: `fee = floor(price * fee_bps / 10_000)` to the market treasury, `resale = floor(price * author_resale_bps / 10_000)` to `Item.author`, the rest to the seller; escrow sends the token to the buyer; closes `Listing`; `Sold { item, seller, buyer, price, fee, resale, ts }` |
| `expire(item_mint)` | anyone | `expires_at != 0 && now > expires_at` (`NotExpired`) | returns the token to the seller; closes `Listing`; `ListingExpired` |
| `create_collection(name, template_ids)` | anyone (curator) | each template exists and is `Active` (`UnknownTemplate`); no duplicates (`DuplicateTemplate`) | writes `Collection`; `CollectionCreated` |

**Money flow.** The buyer's SOL goes directly from the buyer to the three recipients inside `buy`
(the buyer signs). No program vault holds sale proceeds, so nothing can be stranded.

**Unclaimed royalties.** Selling an item sells its unclaimed royalties too (they are claimable by
the holder at claim time, R31). The site shows the unclaimed balance on every listing and offers
"settle and claim first" to the seller before listing (02 section 10 order). Listing does not
force a claim, because a claim needs every royalty holding's mint and would make `list` unbounded.

**Equipped items.** An equipped item can be listed and sold: the equip is bound to the item
account, not to its holder (02 2.4), so the token that runs it keeps running it and the buyer
receives future royalties. The listing page says so.

### 1.3 Price history

`Sold`, `Listed` and `Delisted` events give the indexer a price series per item, per template and
per collection. The site shows only prices that happened in `Sold` events; no estimates, no
"floor" figures computed from listings unless labelled "lowest current listing".

### 1.4 Abuse

| Risk | Answer |
| --- | --- |
| Wash sales to fake a price history | Each sale pays `fee_bps` plus `author_resale_bps`; the site shows the count of distinct buyers per item next to any price chart |
| Listing an item whose royalties the seller already drained | The listing shows the claimable balance read at render time; buyers see what they get |
| Seller drains royalties while the item is listed | While listed, the item token sits in the market escrow, so the holder at claim time is the escrow PDA. Ruling: **claims are refused while an item is listed** (`claim_royalty` refuses when the holder is `["escrow", item_mint]` under `<MARKET_ID>`, `ItemListed`); unclaimed royalties travel with the sale. The seller claims before listing |

The last row needs a one-line change in `claim_royalty` (armory): refuse when the item's holding
owner is `["escrow", item_mint]` under `<MARKET_ID>`. Owner: armory lane (section 12).

### 1.5 Collections and sets

A `Collection` groups templates (for discovery, filters, set pages and badges, section 6). It has
no on-chain economic effect in this part. A future "set bonus" (equipping a full set raising a
ceiling) would need its own ruling and is listed as deferred (section 11).

### 1.6 Tests

`market.rs`: list, delist, buy with exact splits, expiry, double listing refused, listing an
equipped item and buying it (royalties after the sale go to the buyer), claim refused while
listed, max_price slippage, a non-holder cannot list.

## 2. Item lineage (`hookwars_armory`)

### 2.1 Account

`Lineage` at `["lineage", item]` (armory): `parents: [Option<Pubkey>; 2]`, `generation: u16`
(0 for authored and loot items, `max(parent generations) + 1` for forged), `root_template: u16`,
`created_at`. Written by `forge` (02 section 9) and by `create_composite` (composites record their
fused sources, 08 section 2.6) in the same instruction that creates the item.

### 2.2 Events and app

`Forged` (02 13) already carries both parents; `Lineage` makes the tree readable without the
indexer. The item page shows the family tree to its roots, each node linking to its item page and
its current holder; the market shows generation on listings.

### 2.3 Tests

Forge three generations and read the tree back from accounts only; a composite records its sources.

## 3. Achievement badges (`hookwars_social`, new; template 42 in `hookwars_items`)

### 3.1 Soulbound badges as units tokens

A badge is a units token that cannot move. New template **42 Soulbound** (kind `Defense`, no
parameters, `may_refuse` true, `before_transfer` refuses every transfer whose source is not the
mint authority and whose destination is not a burn; `on_touch` none). Each badge type is one slot
mint with Soulbound in a `Locked` slot (R12 lets launch-time items fill any slot; for badges the
social program creates the mint itself, not the launchpad).

### 3.2 Accounts

| Account | Seeds (under `<SOCIAL_ID>`) | Fields |
| --- | --- | --- |
| `BadgeType` | `["badge", id: u32 le]` | `mint`, `name`, `criterion: Criterion`, `active`, `created_at` |
| `BadgeAward` | `["award", badge_id: u32 le, wallet]` | `awarded_at` (prevents double awards) |
| minter (system-owned, signs) | `["badge-minter"]` | mint authority of every badge mint |

`Criterion` is an enum of on-chain checkable facts only, each read in O(1):

| Criterion | Read from |
| --- | --- |
| `FirstSiege { mint }` | per token, not per wallet: `WarState(mint).season.sieges > 0`; the badge is minted to the token's war chest (shown on the token page), since the chain keeps no per-wallet siege record |
| `RaidPoints { mint, min }` | the wallet's holding Raid range (04): `raid_points >= min` |
| `ForgeLevel { min_level }` | the wallet holds an item with `Item.level >= min_level` |
| `Streak { mint, min_days }` | the wallet's holding Streak range (08 template 26) |
| `ItemsAuthored { min }` | `ForgeCounter` style counter per author, new `AuthorCounter` at `["authored", wallet]` in the armory |
| `RoyaltiesClaimed { min_lamports }` | new per-wallet counter `["claimed", wallet]` in the armory, incremented by `claim_royalty` |

Criteria that would need history the chain does not keep are not badges; they are site labels
only and grant nothing (05 quest rule).

### 3.3 Instructions

| Instruction | Signer | Checks | Effect |
| --- | --- | --- | --- |
| `create_badge(name, criterion)` | social admin (behind `ADMIN_TIMELOCK_SECS`) | criterion valid | creates the slot mint with Soulbound locked; `BadgeType`; `BadgeCreated` |
| `claim_badge(badge_id)` | wallet | `BadgeAward` absent (Anchor `init`, `AlreadyAwarded`); criterion holds (`CriterionNotMet`) | mints 1 to the wallet; `BadgeAwarded` |

### 3.4 Abuse

Badges carry no money and no voting weight. Farming them costs whatever the criterion costs (raid
points cost fees, forges burn items). Sybil wallets can each earn badges; the site never presents
badge counts as anything but badges.

### 3.5 Tests

`badges.rs`: each criterion met and not met; a badge cannot be transferred (Soulbound refuses);
double claim refused; burning a badge works.

## 4. Item rental (`hookwars_market` + `hookwars_armory` + `hookwars_items`)

### 4.1 What it is

An item holder leases the item to one token for a term. While the lease is live the token may
equip it through its normal equip rule. A lease touches only the royalty stream (R31), never the
token's own cut destinations: during the term, `settle_equip` pays `rent_share = floor(royalty *
lease.rent_bps / 10_000)` of the item's royalty to the lessor and the rest to the royalty holding
as usual. The token pays nothing extra: its traders see exactly the cuts the item's manifest
advertises.

Why rent at all then: the lessor gets the item used on a token without selling it, and the token
gets an item it does not own. To make leasing worth something to the lessor when the item's own
royalty is small, a lease may also carry an upfront `fee_lamports`, paid by whoever opens the lease
(usually the token's war chest by vote, or any holder).

### 4.2 Accounts

| Account | Seeds | Fields |
| --- | --- | --- |
| `Lease` (market) | `["lease", item]` | `lessor`, `token_mint`, `slot`, `rent_bps` (<= `MAX_RENT_BPS`, O), `fee_lamports`, `starts_at`, `ends_at`, `state` (Offered, Active, Ended) |
| escrow owner (market) | `["lease-escrow", item_mint]` | holds the item token for the term, so the lessor cannot sell or re-lease it |

### 4.3 Instructions

| Instruction | Signer | Checks | Effect |
| --- | --- | --- | --- |
| `offer_lease(token_mint, slot, rent_bps, fee_lamports, term_secs)` | item holder | `term_secs` within `[LEASE_MIN_SECS, LEASE_MAX_SECS]` (O); `rent_bps <= MAX_RENT_BPS`; item not listed (`ItemListed`) | moves the item token to the lease escrow; `Lease` Offered; `LeaseOffered` |
| `accept_lease` | payer (anyone) | Offered; pays `fee_lamports` to the lessor | `Active`, `starts_at = now`, `ends_at = now + term`; `LeaseStarted` |
| `withdraw_offer` | lessor | Offered | token back to lessor; closes; `LeaseWithdrawn` |
| `end_lease` | anyone, after `ends_at` (`LeaseNotOver`) | Active | if the item is equipped in `(token_mint, slot)`, CPI armory `revert_for_lease_end` (new, signs as `SlotAuthority`, same path as a performance revert: settle first, then revert to the slot's launch item or empty); token back to lessor; `LeaseEnded` |

**Equip gate.** The armory's `propose` and `execute` refuse to equip an item whose token sits in a
lease escrow unless the `Lease` names this `(token_mint, slot)` and is `Active` (`ItemLeasedElsewhere`).

**Settle.** `settle_equip` (items) reads the item's `Lease` when the item token is in a lease
escrow (an extra account, derived per R22 from the item key) and splits the royalty as in 4.1,
paying the lessor through `transfer_from_protocol` (R24). New ruling **R32**: the lessor's share is
part of the royalty, never on top of it.

### 4.4 Abuse

| Risk | Answer |
| --- | --- |
| Lessor sells the item mid-lease | The token is in escrow for the term |
| Token keeps the item past the term | `end_lease` is permissionless and reverts the slot |
| `end_lease` called while unsettled cuts sit in the vault | It settles first (same as a revert) |
| Rent extracted from traders | Impossible: rent is a share of the item's royalty, which is already inside the advertised cut |

### 4.5 Tests

`lease.rs`: offer, accept, equip by vote, settle splits royalty exactly, the lessor cannot list or
re-lease, end after term reverts and returns the token, equip elsewhere refused, money conservation.

## 5. Commissions: bounties for new hooks (`hookwars_market` + `hookwars_armory`)

### 5.1 What it is

A community posts a bounty for a hook it wants ("an anti-dump item for our token"). Anyone (people
or agents, 09) submits an item. Holders vote with the existing lock-to-vote (02 section 6). The
winning item is equipped and its submitter is paid the bounty; the submitter keeps the item and its
royalty (R31).

### 5.2 Accounts

| Account | Seeds (market) | Fields |
| --- | --- | --- |
| `Commission` | `["commission", token_mint, nonce: u64 le]` | `creator`, `token_mint`, `slot`, `brief_uri`, `bounty_lamports`, `opens_at`, `closes_at`, `state` (Open, Voting, Paid, Refunded), `winner: Option<Pubkey>` |
| bounty vault owner | `["commission-vault", commission]` | holds the bounty SOL |
| `Submission` | `["submission", commission, item]` | `submitter`, `item`, `submitted_at` |

### 5.3 Instructions

| Instruction | Signer | Checks | Effect |
| --- | --- | --- | --- |
| `open_commission(slot, brief_uri, bounty, window_secs)` | anyone (often the war chest by vote) | `bounty >= COMMISSION_MIN_LAMPORTS` (O); slot is a `Vote` or `Performance` slot (`SlotNotVotable`) | bounty into the vault; `CommissionOpened` |
| `submit(item)` | item holder | Open; `check_fits` of the item against the slot passes (02 6.1; `DoesNotFit`); one submission per item | `Submission`; `Submitted` |
| `nominate(item)` | anyone after `closes_at` | the item has a `Submission`; opens a normal armory `Proposal` for the slot with this item, tagged with the commission | `Nominated` |
| `pay_commission` | anyone | the armory proposal for that item reached `Executed` (02 6.4) | bounty to the submitter (from the vault, R24); `Paid`; `CommissionPaid` |
| `refund_commission` | anyone after `closes_at + COMMISSION_VOTE_SECS` (O) with no executed nomination | Refunded: bounty back to `creator` |

Payment requires the item to have actually been **equipped** by holder vote, not just nominated.

### 5.4 Abuse

| Risk | Answer |
| --- | --- |
| Submitting a hostile item | It must pass `check_fits` and win a holder vote (02 6) |
| Creator rugs the bounty | The bounty sits in a program vault from the start |
| Proposal seat spam (review 1, M-2) | Uses the armory's fixed proposal rules, including whatever M-2's fix adds |

### 5.5 Tests

`commissions.rs`: open, submit, nominate, vote, execute, pay; no winner refunds; a non-fitting
item refused; paying before execution refused.

## 6. Guild halls (`hookwars_social`)

### 6.1 What it is

The Guild Tag template (08, template 39) already puts a guild id in holdings. A **guild hall**
gives a guild a page, a member list and a shared treasury.

### 6.2 Accounts

| Account | Seeds (social) | Fields |
| --- | --- | --- |
| `Guild` | `["guild", guild_id: u32 le]` | `name`, `officers: Vec<Pubkey>` (max `GUILD_MAX_OFFICERS`, O), `threshold`, `created_at` |
| treasury owner | `["guild-treasury", guild_id]` | holds SOL and units tokens |
| `GuildSpend` | `["guild-spend", guild_id, nonce]` | `to`, `mint`, `amount`, `approvals`, `eta` |

### 6.3 Instructions

`create_guild` (founder becomes officer 1, threshold 1); `set_officers` (needs `threshold`
approvals, timelocked by `GUILD_TIMELOCK_SECS`, O); `deposit` (anyone); `propose_spend`,
`approve_spend`, `execute_spend` (threshold approvals and `eta` passed; transfers from the treasury
with `transfer_from_protocol` for units tokens, R24). Events `GuildCreated`, `GuildDeposit`,
`GuildSpendProposed`, `GuildSpendExecuted`, `OfficersChanged`.

### 6.4 Abuse and honesty

A guild treasury is custody by its officers, not by the protocol. The site states it on every
guild page ("officers can spend this treasury with N of M approvals after a delay"). Deposits are
voluntary; the protocol never routes fees into a guild.

### 6.5 Tests

`guilds.rs`: threshold spends, timelock, officer change, deposit of SOL and units tokens, refused
spends.

## 7. Agent leagues (`hookwars_social`; depends on 09)

A season table of agents, read from passports (09) and on-chain counters only: royalties claimed by
items the agent authored (counter from 3.2), treaties the agent proposed that executed and held
until their term (section 9 expiring treaties give the term), crank steps run (events), loot
revealed. The table is computed by the indexer from events; on chain, `submit_league_leader`
follows the same O(1) submit and challenge pattern as seasons (05 section 10) only if the owner
wants a prize for it (`LEAGUE_PRIZE_LAMPORTS`, O, paid from a league vault the protocol funds, never
from fees users pay for agent activity). Default: site table only, no prize.

## 8. Coalitions (`hookwars_war`)

### 8.1 What it is

Three or more tokens pool part of their war chests for joint sieges. Captured holdings from a
coalition siege are split pro rata to contributions.

### 8.2 Accounts

| Account | Seeds (war) | Fields |
| --- | --- | --- |
| `Coalition` | `["coalition", id: u32 le]` | `members: Vec<Pubkey>` (mints, `COALITION_MIN` to `COALITION_MAX`, O), `contributions: Vec<u64>`, `state`, `created_at`, `ends_at` |
| coalition chest owner | `["coalition-chest", id]` | holds bridged SOL and captured holdings |

### 8.3 Instructions

| Instruction | Signer | Checks | Effect |
| --- | --- | --- | --- |
| `form_coalition(members, term_secs)` | anyone | each member token has a **Coalition** relation item equipped naming this coalition id (new template 43, kind `Relation`, no callbacks, params: coalition id, max contribution bps of its chest) | creates `Coalition`; `CoalitionFormed` |
| `contribute(mint, amount)` | anyone (crank) | `amount` within the member's Coalition item limit of its chest per `CONTRIBUTE_INTERVAL_SECS` (O) | chest to coalition chest; `CoalitionContribution` |
| `coalition_siege(rival)` | anyone | same rules as `siege` (05 6.2) with the coalition chest as payer; rival is not a member; R10 refusal still applies | spot buy into the coalition chest; `CoalitionSiege` |
| `dissolve` | anyone after `ends_at` | | captured holdings and remaining SOL returned to members pro rata to `contributions` (transfers from the coalition chest, R24); `CoalitionDissolved` |

A member leaves early by unequipping its Coalition item (its normal equip rule); its share stays
until dissolution.

### 8.4 Abuse and tests

Contributions are capped by each member's own item; sieges keep every siege guard (sandwich bound,
premium wait, spend caps). Tests `coalitions.rs`: form needs every member's item, contributions
capped, joint siege, pro rata dissolution exact to the lamport (rounding to the last member).

## 9. Expiring treaties (`hookwars_items` + `hookwars_armory`)

Treaty and Tribute (04 templates 5 and 6) gain a parameter field `term_secs` (floor
`TREATY_MIN_TERM_SECS`, ceiling `TREATY_MAX_TERM_SECS`, O; 0 not allowed for new items). The equip
records `equipped_at`; after `equipped_at + term_secs` the item's callbacks answer nothing (they
stop paying) and anyone may call armory `expire_treaty(mint, slot)`, which reverts the slot like a
performance revert (settle first). Renewal is an ordinary new vote to re-equip the same item before
expiry; `execute` of a re-equip of the same item keeps the equip state and resets `equipped_at`.
Existing items without the field are treated as `term_secs = TREATY_DEFAULT_TERM_SECS` (O) from
their next equip. Tests: payments stop at expiry, expire reverts, renewal resets the term, both
sides' expiries are independent.

## 10. Referral armies (`hookwars_items` + `hookwars_war`)

The Referral template (08, template 36) stores the recruiter in `["referred", mint, holder]`.
Armies add: when the Raid token half stamps raid points on a holding that has a referral record
(the record is a derived extra, R22, from the destination owner), it also adds the same points to
`army_points` inside that referral record. The recruiter calls war `claim_army(records)` with up to
`ARMY_MAX_RECORDS` (M) records whose `recruiter == signer`; war pays `army_points *
army_rate` from the war chest at the chest's posted army rate (a War orders field, bounded by the
same rule review 1 M-6 sets for bounties) and zeroes the records. Tests: points mirror, claim
exact, records of another recruiter refused, rate bound.

## 11. Events and reach

### 11.1 Boss events (`hookwars_war`)

The protocol launches a **boss token** each season (an ordinary units launch by the protocol
authority, kit off, a Raid-able pool). A `BossPool` at `["boss", season]` under war is funded from
the season prize share (05 10.4, `BOSS_SHARE_BPS` of the prize vault's split, O). Raiding the boss
means buying the boss token with a route from your own token; the Raid item on the **raider's**
side is not needed: the boss token itself runs a Boss item (new template 44, Pool kind) that marks
inbound raid volume per source mint in the boss's `RaidLedger`. After the season ends,
`claim_boss_share(source_mint)` pays the source token's war chest `boss_pool * source_volume /
total_volume` (one claim per source mint per season). Money goes to war chests, which spend it
under their normal rules; no wallet receives a payout for an outcome.

Spot check: buyers buy a real token at its pool price; the reward is for activity already done
(like bounties), not for predicting anything. Abuse: self-routing loops are bounded by whatever
review 2 M-B's fix makes the raid volume counter (net inflow per window); tests `boss.rs`.

### 11.2 Rotating seasonal meta (`hookwars_armory`)

`SeasonMeta` at `["meta", season: u32 le]` (armory): per template id, an optional override of
ceilings (`field_max_override`) that can only **lower** a ceiling or **raise it up to**
`META_MAX_RAISE_BPS` (O) of the base ceiling, never above the template's audited absolute bound
(the template program's own validation still runs). Proposed by the admin behind
`ADMIN_TIMELOCK_SECS` with `eta` before the season opens (new ruling **R33**: meta applies to new
equips and forges during its season only; items already equipped keep running as equipped).
`check_fits`, `create_item`, `forge` and `mint_loot` read the current season's meta when present.
The site shows "this season's meta" on the armory page. Tests: a lowered ceiling refuses a new
equip but leaves an equipped item running; a raised ceiling allows a forge up to the raise.

### 11.3 Rivalry contracts (`hookwars_war`), redesigned to stay spot

The requested version (two chests stake funds, the winner by raid volume takes them) is an outcome
payout between two parties: **not allowed** (spot only: nothing pays out on an outcome). Redesign:

- Two tokens equip a **Rivalry** relation item (new template 45, no callbacks) naming each other,
  a start, a duration (`RIVALRY_MIN_SECS` to `RIVALRY_MAX_SECS`, O) and a war budget each
  (bounded by a share of its own chest).
- During the rivalry, each budget is a ring-fenced part of the token's own chest that siege,
  counter-strike and raid bounties against the rival may spend first. Unspent budget simply stays
  in that token's own chest at the end. **No funds move between the two chests because of the
  result.**
- The result (raid volume each way, recorded in both `RaidLedger`s) feeds the season score as a
  separate counter (`rivalry_wins`, weighted in `ScoreWeights`, 05 10.2) and a soulbound
  **Rivalry badge** to the winning token's page (section 3). The only value at stake is score and
  standing, paid by the protocol's season prize, never by the loser.

Tests: budgets ring-fenced, nothing moves between chests at the end, score counter updated,
unequipping ends the rivalry early with no transfer.

### 11.4 Telegram mini-app war room (app)

A Telegram Mini App build of the war room (06): the war map, battle feed, my raid points, loot
tickets and claims, and "join the raid" (the same routed swap prepare as the site). Signing uses a
wallet deep link or the wallet's in-app browser; the mini-app never holds keys. Bot alerts (06)
link into it. Tests: page renders with empty states; prepares identical to the site's.

### 11.5 Open template registry: the Hook Lab (`hookwars_armory` + an off-chain lab service)

Anyone (people or agents, 09) submits a new template: a program id, its verified build hash, its
manifest, parameter schema with floors and ceilings, and a test suite.

| Step | Where | What |
| --- | --- | --- |
| `submit_template(program, code_hash, schema, uri)` | armory | posts a `TemplateSubmission` at `["submission-tpl", nonce]` with a bond `LAB_BOND_LAMPORTS` (O) in a vault; program must be immutable or under `<MANAGED_HOOK_KEY>` (rule 3) |
| Property tests | lab service (off chain, open source) | builds reproducibly, runs the template against the 08 compatibility model and the invariant suite (FUZZ-AUDIT style), posts a signed report hash |
| `approve_submission` | protocol admin behind `ADMIN_TIMELOCK_SECS` (this also closes review 1 L-1 for submitted templates) | requires a lab report hash; registers the template; bond back |
| `reject_submission` | admin, with a reason uri | bond goes to the protocol treasury only if the submission failed a check the lab published; otherwise refunded |

**Template author share** (new ruling **R34**): a registered submitted template may carry
`template_author_bps` (<= `MAX_TEMPLATE_AUTHOR_BPS`, O), taken out of every item royalty settled
for items of that template (never on top of the item's cut). This makes templates assets too.

Honesty: the lab service is a trust point (off-chain tests). Its reports are public and
reproducible; the admin timelock gives anyone time to rerun them before registration.

## 12. Preview, siege exemption and rent

### 12.1 Preview before equip (app only)

The token page's proposal view replays the token's indexed trades from the last
`PREVIEW_WINDOW_SECS` (O) through the proposed item's exact math (the TypeScript ports pinned to
Rust by `app_vectors.rs` fixture vectors) and shows what the item would have cut, burned or
discounted, per trade and in total, labelled "replayed against N past trades". It never projects
forward. Templates without a pinned port show "no preview" rather than an estimate.

### 12.2 Siege exemption for holder-reward rivals (D-7) (`bordrless_kit` + `hookwars_war`)

R10 refuses sieges of tokens whose kit pays holder rewards, because the kit refuses off-curve
owners and cannot tell a rival's war chest from any program address. Fix: war creates a marker
account `WarChestMarker` at `["chest-marker", chest]` (owned by war, created by `init_war`). The
kit's registry gains one derived extra (R22): the account at `["chest-marker",
destination_owner]` under `<WAR_ID>`. When the destination owner is off curve, the kit treats it as
an excluded owner (not settled, not capped, not counted, like the pool) if and only if that marker
account exists and is owned by war. New ruling **R35** replaces R10's refusal. Cost: one extra
account on every transfer of a kit token, to measure (07); if it does not fit the measured budgets,
D-7 stays deferred and R10 stands. Tests: siege of a holder-reward rival succeeds; a fake marker
(wrong owner) is refused; reward vault solvency with a chest holding.

### 12.3 Rent reductions (`bordrless_swap`, `bordrless_token`)

| Change | Rule |
| --- | --- |
| Ring length per pool | `create_pool` takes `obs_ring_len` within `[OBS_RING_MIN, OBS_RING_LEN]` (O, M); readers already read length and spacing from the ring header (03 M3a notes), so nothing else changes; launch pools default to `LAUNCH_OBS_RING_LEN` (O); a window read beyond a short ring returns "no signal" |
| Slot table sized to slots used | `create_slot_mint` writes `slot_count` and allocates `base + slot_count * SLOT_LEN` instead of `MAX_SLOTS`; slot indexes `>= slot_count` do not exist; the launch and armory read `slot_count` |

Both change account sizes: every place that hard-codes `Mint::LEN` or the pool size must read the
actual length. Tests: rent measured for 1 to `MAX_SLOTS` slots and for each ring length; all
existing suites green.

## 13. What 00 must add

**Programs:** `hookwars_market` (listings, sales, collections, leases, commissions) and
`hookwars_social` (badges, guilds, leagues); ids `<MARKET_ID>`, `<SOCIAL_ID>`.

**Templates:** 42 Soulbound (Defense), 43 Coalition (Relation, no callbacks), 44 Boss (Pool), 45
Rivalry (Relation, no callbacks); Treaty and Tribute gain `term_secs`.

**Seeds:** market `["market-config"]`, `["listing", item_mint]`, `["escrow", item_mint]`,
`["collection", id]`, `["lease", item]`, `["lease-escrow", item_mint]`, `["commission",
token_mint, nonce]`, `["commission-vault", commission]`, `["submission", commission, item]`;
social `["badge", id]`, `["award", badge_id, wallet]`, `["badge-minter"]`, `["guild", id]`,
`["guild-treasury", id]`, `["guild-spend", id, nonce]`; armory `["lineage", item]`, `["authored",
wallet]`, `["claimed", wallet]`, `["meta", season]`, `["submission-tpl", nonce]`; war
`["coalition", id]`, `["coalition-chest", id]`, `["boss", season]`, `["chest-marker", chest]`.

**Rulings:**
- **R31** Item income belongs to the holder of the item token at claim time, except a live lease's
  share (section 0); claims are refused while an item is listed (1.4).
- **R32** A lessor's rent is a share of the item's royalty, never on top of it (4.3).
- **R33** Seasonal meta applies to new equips, items and forges during its season only (11.2).
- **R34** A submitted template's author share comes out of item royalties, never on top (11.5).
- **R35** Kits exclude a destination whose `["chest-marker", owner]` exists under war; replaces
  R10's refusal if it fits the budgets (12.2).
- **R36** No outcome transfers between communities: features that rank tokens (rivalries, bosses,
  seasons) pay only from protocol-funded pools or keep funds in each token's own chest (11.3).

**Parameters (all to set):** `MARKET_FEE_BPS`, `AUTHOR_RESALE_BPS`, `COLLECTION_MAX_TEMPLATES`,
`MAX_RENT_BPS`, `LEASE_MIN_SECS`, `LEASE_MAX_SECS`, `COMMISSION_MIN_LAMPORTS`,
`COMMISSION_VOTE_SECS`, `GUILD_MAX_OFFICERS`, `GUILD_TIMELOCK_SECS`, `LEAGUE_PRIZE_LAMPORTS`,
`COALITION_MIN`, `COALITION_MAX`, `CONTRIBUTE_INTERVAL_SECS`, `TREATY_MIN_TERM_SECS`,
`TREATY_MAX_TERM_SECS`, `TREATY_DEFAULT_TERM_SECS`, `ARMY_MAX_RECORDS`, `BOSS_SHARE_BPS`,
`META_MAX_RAISE_BPS`, `RIVALRY_MIN_SECS`, `RIVALRY_MAX_SECS`, `LAB_BOND_LAMPORTS`,
`MAX_TEMPLATE_AUTHOR_BPS`, `PREVIEW_WINDOW_SECS`, `OBS_RING_MIN`, `LAUNCH_OBS_RING_LEN`.

## 14. Deferred

Set bonuses for full collections (needs a ruling on ceilings raised by sets); league prizes
(default off); any feature not listed here.

## 15. Build plan in waves

Lanes are split by **files owned**, so lanes in one wave never edit the same file. Each lane adds
its own test files and its own budget file.

| Wave | Lane | Owns | Features |
| --- | --- | --- | --- |
| 1 | market | new `programs/hookwars_market`, `tests/market.rs` | listings, sales, collections (section 1) |
| 1 | social | new `programs/hookwars_social`, `tests/badges.rs`, `tests/guilds.rs` | badges, guild halls (3, 6) |
| 1 | items-templates | `programs/hookwars_items/src/templates/{soulbound,coalition,boss,rivalry}.rs` and their dispatch lines | templates 42 to 45 (3.1, 8, 11.1, 11.3) |
| 1 | rent | `programs/bordrless_swap`, `programs/bordrless_token` (slot_count sizing) | 12.3 |
| 1 | app-a | `app/apps/web` new pages, `app/apps/miniapp` (new) | preview (12.1), Telegram mini-app (11.4), spectator mode, market and badge pages against spec layouts |
| 2 | armory | `programs/hookwars_armory` | lineage (2), claim refusal while listed (1.4), lease equip gate and `revert_for_lease_end` (4.3), commission tagging (5), `expire_treaty` (9), seasonal meta (11.2), Hook Lab submissions and template author share field (11.5), author and claim counters (3.2) |
| 2 | market-b | `programs/hookwars_market` | leases (4), commissions (5) |
| 2 | items-settle | `programs/hookwars_items/src/{settle.rs,engine.rs}`, Treaty and Tribute template files | lease royalty split (4.3), template author share in settle (11.5), treaty term (9), army points mirror (10) |
| 3 | war | `programs/hookwars_war` | coalitions (8), boss pool and claims (11.1), rivalry budgets and score counter (11.3), army claims (10), chest markers (12.2) |
| 3 | kit | `programs/bordrless_kit` | R35 marker exclusion (12.2), measured |
| 3 | social-b | `programs/hookwars_social` | agent leagues (7), after 09 lands |
| 4 | app-b | `app/` SDK, indexer, API | IDLs of market and social, all new events, prepares for every new instruction |
| 4 | audit | read only | security review and fuzz pass over everything in this part |

Wave 2 waits for wave 1's market and items-templates lanes (leases and commissions build on
listings; the armory reads template 42 to 45 kinds). Wave 3 waits for wave 2's armory lane
(coalition and rivalry items equip through it). Each lane follows the house rules: build and test
on the build server under `/root/build.lock`, own worktree and branch, no em dashes, no invented
numbers, every parameter to set.

## 16. Expansion implementation notes (branch expand, 2026-10-09)

Built: `programs/hookwars_market` (id `FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv`),
`programs/hookwars_social` (id `CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx`), templates 43
Coalition, 44 Boss, 45 Rivalry in `programs/hookwars_items/src/templates/{coalition,boss,rivalry}.rs`
with their shapes, rules and manifests in `crates/hookwars-common` and their dispatch lines in
`templates/mod.rs`. Tests: `tests/market.rs` (10), `tests/social.rs` (5),
`tests/templates_expansion.rs` (4), `tests/budgets_expansion.rs` (2), all on server B with the full
suite green.

Where the build differs from the text above:

1. **Market parameters** live in `MarketConfig.params` (`MarketParams`), set at `init` by the
   program's upgrade authority and changed only by `propose_params` then `apply_params` after
   `admin_timelock_secs` (also a field). The treasury is changed the same way.
2. **Item tokens leave the escrows by an ordinary `transfer` signed by the escrow PDA**, not
   `transfer_from_protocol`: item mints are plain units mints (no hook, 02 2.5), so no slot runs
   and R16 does not apply. The same holds for guild token spends of hookless mints; a guild
   treasury spend of a slot mint runs that mint's slots like any transfer (see request I-1).
3. **Commissions have no `nominate`.** Holders propose the submitted item through the armory's own
   `propose` (02 6). `pay_commission` checks the outcome on chain instead: after `closes_at`, the
   commission's slot in the token's `Mint` holds the submitted item and it is not the item that was
   there when the commission opened (`Commission.incumbent`). It pays the vault's whole balance
   (the bounty, plus anything sent to the vault), so nothing is stranded. "Fits the slot" is the
   slot kind equal to the item's manifest kind; the armory's full `check_fits` still runs at
   equip.
4. **`end_lease` only returns the item.** Reverting the slot (if the item is still equipped there)
   needs the armory (request I-3). Until then a token keeps running an item whose lease ended; the
   lessor receives future royalties again because the royalty follows the holder (R31).
5. **Collections** take their template accounts as remaining accounts and check each is an active
   armory `Template`; ids come from `MarketConfig.collections`.
6. **Badges are frozen holdings until template 42 lands.** A badge mint is a plain units mint
   whose mint and freeze authority is `["badge-minter"]`; `claim_badge` mints 1 and freezes the
   holding in the same instruction, so the token program refuses every transfer and burn of it
   (`Frozen`). 10 3.5's "burning a badge works" therefore does not hold yet (request I-2).
7. **Badge criteria built:** `FirstSiege { mint }` (war chest has `spent_siege > 0`; awarded to the
   token's war chest), `RaidPoints { mint, min }` (a plain Raid item's range in the wallet's
   holding, epoch byte checked, this season per `WarConfig`), `ForgeLevel { min_level }` (the
   wallet holds an item of that level). `Streak`, `ItemsAuthored` and `RoyaltiesClaimed` need
   counters or templates not built yet (requests I-4, I-5). A new badge's claims open
   `admin_timelock_secs` after `create_badge`, so it is public before anyone can earn it.
8. **Guild actions** are one account type, `GuildAction { SpendSol | SpendToken | SetOfficers }`,
   each executable after `guild_timelock_secs` with the threshold of approvals; an officer change
   bumps `officers_version` and voids pending actions (`StaleAction`). The proposer's approval is
   counted.
9. **Agent leagues** have no on-chain part: 10 section 7's default is a site table with no prize.
10. **Boss** counts per source mint in the boss token's `RaidLedger` inbound table with the item's
    window (`params[0]`, set to the season length) and adds every counted buy to the ledger's
    season total. The table holds `RAID_TABLE_LEN` sources per window; a ninth live source is not
    recorded (request I-6). A source counts only when the route's first pool is the source mint's
    canonical launch pool (a PDA of the mint), so no attacker-made pool can stand in (review 1 M-5).
11. **Coalition and Rivalry** are config items with no callbacks; `coalition::read` and
    `rivalry::{read, live}` are what war will call. They are not composable.
12. Events use `emit_cpi!` (self-CPI) like the other non-item programs.

Measured (`budgets_expansion.rs`, LiteSVM, one lookup table holding every account):

| Path | Keys | v0 bytes | With table | Trace | Height | CU |
| --- | --- | --- | --- | --- | --- | --- |
| market list | 13 | 567 | 291 | 9 | 3 | 60,588 |
| market buy | 16 | 659 | 290 | 11 | 3 | 61,249 |
| market offer_lease | 15 | 664 | 326 | 9 | 3 | 70,436 |
| market accept_lease | 7 | 353 | 263 | 4 | 2 | 9,597 |
| market end_lease | 13 | 551 | 275 | 6 | 3 | 54,353 |
| market open_commission | 9 | 473 | 321 | 5 | 2 | 24,484 |
| market submit | 11 | 485 | 271 | 4 | 2 | 26,031 |
| market pay_commission | 10 | 451 | 268 | 4 | 2 | 17,031 |
| social create_badge | 10 | 463 | 280 | 7 | 3 | 36,927 |
| social claim_badge (forge level) | 14 | 621 | 314 | 11 | 3 | 77,444 |
| social create_guild | 7 | 365 | 275 | 4 | 2 | 17,791 |
| social deposit_sol | 7 | 361 | 271 | 4 | 2 | 9,804 |
| social propose_action | 8 | 427 | 306 | 4 | 2 | 20,376 |
| social execute_action (SOL) | 11 | 484 | 270 | 4 | 2 | 15,154 |

## 17. Integration requests (from branch expand)

Changes the expansion needs in programs this branch does not own, written so the owning lane can
apply them as stated.

- **I-1 token: add `<MARKET_ID>` and `<SOCIAL_ID>` to `PROTOCOL_SOURCE_PROGRAMS`**
  (`programs/bordrless_token/src/constants.rs`), so a guild treasury (`["guild-treasury", id]`)
  can pay out a slot mint without its own items cutting the spend (R16, R24). Then switch
  `hookwars_social::execute_action`'s `SpendToken` to `transfer_from_protocol` with seeds
  `["guild-treasury", id le, bump]`.
- **I-2 items/agents: template 42 Soulbound.** When it lands, `create_badge` creates the badge mint
  with `create_slot_mint` and a Locked Soulbound slot instead of a frozen-holding mint; keep the
  freeze until then. Badges then burn as 10 3.5 says.
- **I-3 armory: `revert_for_lease_end(token_mint, slot)`**, callable by `hookwars_market` signing
  as `["lease-escrow", item_mint]` (or a dedicated `["market-caller"]` PDA under `<MARKET_ID>`):
  settle the slot, then revert it to its launch item or empty, exactly like a performance revert.
  `end_lease` then CPIs it before returning the token. **Equip gate:** `propose` and `execute`
  refuse an item whose token sits in `["lease-escrow", item_mint]` under `<MARKET_ID>` unless the
  item's `["lease", item]` (owner `<MARKET_ID>`) is `Active` and names this `(token_mint, slot)`
  (`ItemLeasedElsewhere`). Read `Lease` with `hookwars_market::state::Lease`.
- **I-4 armory: R31 claim refusal while listed.** `claim_royalty` refuses (`ItemListed`) when the
  item's holder holding owner is `["escrow", item_mint]` under `<MARKET_ID>`
  (`hookwars_market::state::escrow_address`).
- **I-5 armory: counters for badges.** `AuthorCounter` at `["authored", wallet]` (bumped by
  `create_item`) and `ClaimCounter` at `["claimed", wallet]` (lamports per cut mint, bumped by
  `claim_royalty`); social then adds `ItemsAuthored` and `RoyaltiesClaimed` criteria. `Streak`
  waits for template 26.
- **I-6 items: a boss ledger** if the boss must count more than `RAID_TABLE_LEN` sources per
  season: a `BossLedger` at `["boss-ledger", mint]` under items with a larger table, created by a
  permissionless `init_boss_ledger`; Boss's extras then name it instead of the `RaidLedger`.
- **I-7 items settle: lease rent.** `settle_equip` reads `["lease", item]` under `<MARKET_ID>`
  (derived extra, R22) when the item's token sits in its lease escrow and pays
  `floor(royalty * rent_bps / 10_000)` of the royalty to `Lease.lessor` (R32), the rest as usual.
- **I-8 items settle: template author share** (R34) once the armory stores
  `template_author_bps` on submitted templates.
- **I-9 war: coalitions, boss pool, rivalry budgets** (10 sections 8, 11.1, 11.3) reading items
  43 to 45 with `hookwars_items::templates::{coalition, rivalry}` and the boss's `RaidLedger`
  (`inbound` per source, `outbound_volume_season` total, season `season_id`); `claim_boss_share`
  pays each source's war chest `boss_pool * source_volume / total_volume`, once per source per
  season.
- **I-10 armory: seasonal meta (11.2), lineage (2), Hook Lab submissions (11.5), `expire_treaty`
  (9)** as specified; none needs market or social changes.
- **I-11 kit: R35 chest markers (12.2)** as specified.
- **I-12 app:** IDLs of `hookwars_market` and `hookwars_social` (`programs.sh idl`), decoders for
  their accounts and events, and prepares for every instruction above.
