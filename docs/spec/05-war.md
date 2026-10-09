# Hookwars spec 05: war (`hookwars_war`)

Status: specification, 2026-10-08, revised after the integration rulings (00 section 9). Nothing is
built. Follows `00-overview.md`; names, seeds and parameters are the overview's. Rulings applied
here: R2, R3, R10, R13, R14, R15, D-9.

Upstream references, cited by file and line at `43688f3`:

- companion steps: `programs/bordrless_companion/src/instructions/steps.rs` (buyback 357-480, its
  fee bound 315-319, pool share 326-333 with the sandwich argument 321-325)
- companion limits: `programs/bordrless_companion/src/constants.rs` (`MAX_BOUNTY_BPS` 15,
  `BUYBACK_POOL_SHARE_BPS` 24, `BUYBACK_AFTER_LAUNCH` 38, `BUYBACK_SLIPPAGE_BPS` 40), reasoning in
  `docs/companions.md` 38-49
- calling by built instruction: `programs/bordrless_companion/src/invoke.rs` 1-33
- DEX slippage on what arrived: `programs/bordrless_swap/src/instructions/swap.rs` 413
- protocol fee collection: `programs/bordrless_swap/src/instructions/pool.rs` 646 (collector
  `address`-checked), 675 (`process_collect_protocol_fees_sol`)
- kit share: `programs/bordrless_kit/src/instructions/share.rs` 17-58 (sharer signs, source is a
  reward-mint holding, `MIN_SHARE_LAMPORTS`, streamed over one hour); kit owners rule
  `docs/hooks-v2.md` 4.5

## 1. What the war program does

`hookwars_war` holds each token's **war chest** and spends it only through the instructions below.
Every spending instruction is permissionless, checks on chain that it is due, is capped per call and
per interval, and pays its sender a crank bounty of at most `MAX_CRANK_BOUNTY_BPS` of what it moves
(00 rule 5). It holds no user's signature for anything but that user's own claim.

Spot only: a siege is a spot buy, a counter-strike is a spot buy and a burn, a raze is a spot sell,
a bounty and a loot roll pay for raid activity already recorded. The season prize is a share of
protocol fees that already exist (section 10.4), not a payout on a bet.

**What this program does not do (R2, R3).** It writes no raid marks and no raid volume: those are
written inside swaps by `hookwars_items` into the `RaidLedger`, which this program only reads. It
pays no royalties: `settle_equip` (04) does. There is no `record_marks` and no `pay_royalties` here.

## 2. Accounts

### 2.1 `WarConfig` at `["war-config"]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump`, `prize_vault_bump` | u8 | |
| `admin` | Pubkey | `<PROTOCOL_AUTHORITY>` |
| `protocol_treasury` | Pubkey | wallet that receives protocol fees after the season prize share |
| `randomness_program` | Pubkey | the oracle adapter chosen under D-4 (section 8.2) |
| `current_season` | u32 | season now running (0 before the first) |
| `last_winner` | Option<Pubkey> | mint that won the last finalized season |
| `pending` | Option<PendingConfig> | a proposed change to `admin`, `protocol_treasury` or `randomness_program`, with `eta` (section 11) |
| `reserved` | [u8; 64] | |

### 2.2 `WarChest` at `["war-chest", mint]`

A **system-owned address with no data**, like the companion's creator: only this program signs for
it. It holds a bridged-SOL holding (the spendable balance) and holdings of captured rival tokens;
its own token only for the instant between a counter-strike's buy and burn. Its lamports never fall
below the rent-exempt minimum of a zero-data account.

### 2.3 `TreatyInbox` at `["treaty-inbox", mint]`

System-owned, no data, signed by this program. Its bridged-SOL holding receives what partners'
Treaty and Tribute items pay this token (R13; `settle_equip` in 04 pays here, not into the chest
holding). Kept apart from the chest so that **every lamport in it belongs to holders** and none can
be spent on war. Section 6.6 streams it out.

### 2.4 `WarState` at `["war", mint]` (owned by this program): war-side facts only

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump`, `chest_bump`, `inbox_bump` | u8 | |
| `mint`, `launch` | Pubkey | the token and its `Launch` |
| `last_seen_balance` | u64 | chest bridged-SOL balance after the last write (section 4) |
| `funded_total` | u64 | bridged SOL received, all sources |
| `spent_siege`, `spent_counter`, `paid_bounties`, `paid_cranks`, `razed_proceeds` | u64 | running totals |
| `treaty_shared_total` | u64 | streamed to holders from the inbox |
| `last_siege_at`, `last_counter_at`, `last_treaty_tick` | i64 | spacing and accrual |
| `under_siege_until` | i64 | set when another token's chest besieges this one; read by Defense items (04) |
| `siege_by_chest` | Pubkey | the besieging token's war chest while `under_siege_until > now`, else default; read by Shield and Wall (04) |
| `captured` | [Captured; `MAX_CAPTURED`] | |
| `season_id` | u32 | season the `season` counters belong to |
| `season`, `prev_season` | SeasonCounters | current, and the frozen previous season (10.1) |
| `reserved` | [u8; 64] | |

`Captured { rival_mint, amount: u64, cost: u64, captured_at: i64, raze_window_start: i64,
raze_window_base: u64, razed_in_window: u64 }`; an empty entry has `rival_mint` default and frees
when `amount` reaches 0.

`SeasonCounters { raid_volume_won: u64, sieges: u32, siege_spend: u64, times_besieged: u32,
counter_strikes: u32, treaty_secs: u64 }`. Values only, never marked to market, so no one raises a
score by moving a price at the end of a season.

### 2.5 The raid ledger (owned by `hookwars_items`, read here; R3)

`RaidLedger` at `["raid-ledger", mint]` under `<ITEMS_ID>`, written by the Raid pool half inside
swaps (04). This program needs from it, read-only:

| Field | Meaning |
| --- | --- |
| `inbound: [RaidWindow; RAID_TABLE_LEN]` | per rival X: quote volume of buys of **this** token paid for by selling X (this token raiding X's holders) |
| `RaidWindow { rival_mint, window_start: i64, volume: u64, prev_volume: u64 }` | current fixed window of `RAID_WINDOW_SECS` and the previous one |
| `season: u32`, `season_volume: u64` | total inbound raid volume this season (feeds `raid_volume_won`) |

Rolling volume at time `t`: `volume + prev_volume * (window_start + RAID_WINDOW_SECS - t) / RAID_WINDOW_SECS`
(previous window weighted by its overlap with the last `RAID_WINDOW_SECS`). A new rival takes only
an entry whose windows have both ended; a live entry is never evicted, so a flood of dust rivals
cannot erase a real raid. 04 owns the writing rule; this is what the reads below assume.

## 3. Which tokens can have a war chest

`init_war` needs a `Launch` under `<LAUNCH_ID>` (counter-strike buys on the launch pool). Bridged
tokens and ordinary pools have none.

**Holder-reward tokens (R10).** With holder rewards on, the kit refuses destinations off the ed25519
curve (`docs/hooks-v2.md` 4.5), so a war chest cannot hold such a token. Consequences in v1:

- `siege` refuses a rival whose kit has holder rewards on: `SiegeTargetHasRewards` (6.1);
- `counter_strike` on a token whose own kit has holder rewards on would fail at the delivery; for
  such tokens the War orders' counter-strike fields must be zero, and `counter_strike` refuses with
  `OwnTokenHasRewards` before any swap. The companion's own buyback (which the kit exempts) remains
  their buyback.

Exempting war chests in the kit is open (D-7).

## 4. Funding

| Source | Arrives in |
| --- | --- |
| Companion `war_bps` (03) | the chest's bridged-SOL holding, at each `claim_fees` |
| Pool items whose destination is the war chest (Shield, Spy, Raid toll; 04) | the chest's bridged-SOL holding, by `settle_equip` (04, R2) |
| Treaty and Tribute payments from partners | the **treaty inbox**, by `settle_equip` (R13); never the chest |
| Anyone | a plain transfer of bridged SOL to the chest holding |

`record_funding(mint)`: permissionless, no bounty. Adds the chest balance's increase since
`last_seen_balance` to `funded_total`, then sets `last_seen_balance`. Every spend calls it first and
updates `last_seen_balance` after, so totals stay exact without trusting senders. Event `WarFunded`.

## 5. Setup

### `init_war(mint)`

Accounts: `payer` (signer, writable), `mint`, `launch` (under `<LAUNCH_ID>`, `launch.mint == mint`),
`war_state` (init `["war", mint]`), `war_chest`, `treaty_inbox`, the chest's and the inbox's
bridged-SOL holdings (created through the token program's `create_holding`, payer pays), token
program and event authority, system program, event authority, program.

Checks: the mint's slot table (01) has a `War` slot (00 4.1, kind 6), else `MissingWarSlot`. A
second call fails at `init` (`AlreadyInitialized`). The launchpad calls it in the launch's setup
when the config asks for a war chest (03); anyone can call it later for a token launched without
one. Event `WarChestCreated`.

## 6. War orders, attack and defense

### 6.0 Reading the War orders

The token's `War` slot holds a **War orders** item (template defined in 04; kind `War`, no
callbacks). Its parameters configure this program. Per R7 they are `u32` fields; this part reads
them by name, 04 fixes the field order:

| Field | Meaning | Bound |
| --- | --- | --- |
| `siege_threshold_units` | rolling raid volume against a rival needed for a siege, in `POINT_UNIT_LAMPORTS` | template floor and ceiling |
| `siege_twap_secs` | TWAP window for the rival premium check | at least `MIN_TWAP_SECS` |
| `siege_spend_bps` | spend per siege, of the chest | at most `SIEGE_MAX_SPEND_BPS` |
| `counter_drop_bps` | short TWAP below long TWAP by at least this | template bounds |
| `counter_short_secs`, `counter_long_secs` | the two windows, `short < long` | at least `MIN_TWAP_SECS` |
| `counter_interval_secs` | spacing of counter-strikes | at least `COUNTER_STRIKE_MIN_INTERVAL_SECS` |
| `counter_spend_bps` | spend per counter-strike, of the chest | at most `COUNTER_STRIKE_MAX_SPEND_BPS` |
| `raze_enabled` | 0 or 1 | |
| `peace_returns` | 0 or 1: return captured holdings to a treaty partner | |
| `bounty_rate` | lamports paid per raid point | template ceiling |
| `crank_bounty_bps` | crank bounty | at most `MAX_CRANK_BOUNTY_BPS` |

Every instruction below first checks: the `Item` account is the one the mint's slot table names in
its `War` slot, the item's `Template` is War orders under `<ITEMS_ID>`, and the template's code hash
matches (02). Else `WrongWarOrders`. An empty `War` slot means every war action is off
(`NoWarOrders`) while funding still accrues.

**Who decides war.** The War orders item is equipped by the `War` slot's equip rule (vote or
locked, 02). So the holders decide whether raze and peace returns are on, and how aggressive sieges
and counter-strikes are; the cranks only execute what the equipped orders already allow.

### 6.1 Common steps of every spend

As the companion's steps (`steps.rs` 357-480):

1. `record_funding`; roll `WarState` into the current season (10.1).
2. Read the War orders (6.0).
3. Compute the spend; zero is `NothingToDo`.
4. `bounty = spend * crank_bounty_bps / 10_000`; the rest is what the step moves.
5. Call other programs only by instructions built here with their own client builders, finding the
   accounts among those passed (`invoke.rs` 1-33); a missing one is `MissingAccount`.
6. Never spend more than the chest's bridged-SOL balance; never take the chest's lamports below its
   rent minimum (`ChestInsufficient`).
7. Pay the bounty as SOL (bridge `unwrap_sol`), update `paid_cranks` and `last_seen_balance`.

### 6.2 `siege(mint, rival_mint)`

The chest spot-buys the rival and keeps what it buys.

Accounts: `cranker` (signer, writable), `war_state` (w), `war_chest`, `mint`, War orders `item` and
`template`, `raid_ledger` (`["raid-ledger", mint]` under `<ITEMS_ID>`), `rival_launch`,
`rival_pool`, `rival_observations` (`["obs", rival_pool]`), `rival_war_state` (w, optional: present
when the rival has one), `rival_kit_config` (when the rival's slot table has a `Locked` kit slot);
remaining: the DEX swap's accounts for the rival pool with the rival's slot slices, the chest's
rival holding (created when missing, cranker pays), the bridge's `unwrap_sol` accounts.

| Check | Error |
| --- | --- |
| `rival_mint != mint` | `SelfSiege` |
| the rival's kit, if any, does not have holder rewards on (R10) | `SiegeTargetHasRewards` |
| rolling inbound raid volume from `rival_mint` in `raid_ledger` is at least `siege_threshold_units * POINT_UNIT_LAMPORTS` | `SiegeNotDue` |
| `now >= last_siege_at + SIEGE_INTERVAL_SECS` | `SiegeNotDue` |
| a `captured` entry exists for the rival or one is free | `CapturedTableFull` |
| rival spot price at most its TWAP over `siege_twap_secs` times `(10_000 + SIEGE_MAX_PREMIUM_BPS) / 10_000` | no buy; event `SiegeWaited` (like upstream `BuybackWaited`) |

Spend: `min(chest_balance * siege_spend_bps / 10_000, rival_quote_side * pool_share_bps(rival_launch) / 10_000)`
with upstream's `pool_share_bps` (`steps.rs` 326-333). Upstream's sandwich argument then holds on
the rival's pool (`steps.rs` 321-325): pumping the rival by `p` before a buy of `v` and selling after
gains about `p * v` and costs about `p * Q * (buy + sell) / 2` in fees, so at half that bound the
sandwich loses.

Minimum out: the rival pool's own quote for `spend` net of `buy_fee_bound(rival_launch)`
(`steps.rs` 315-319), less `SIEGE_SLIPPAGE_BPS`, checked by the DEX against what arrived (`swap.rs`
413). The rival's token items run on the delivery; a rival Wall that refuses the chest makes the
whole step fail with nothing spent.

Effects: `captured[rival] += bought`, `cost += spend`; `season.sieges += 1`, `season.siege_spend += spend`;
`spent_siege += spend`; `last_siege_at = now`; when `rival_war_state` is passed:
`rival.under_siege_until = now + SIEGE_INTERVAL_SECS`, `rival.siege_by_chest = WarChest(mint)`, `rival.season.times_besieged += 1` (this
program owns every `WarState`, so it may write the rival's). Event `SiegeExecuted`.

### 6.3 `counter_strike(mint)`

The chest buys its own token on its launch pool and burns it, after a fall.

Accounts: `cranker`, `war_state` (w), `war_chest`, `mint`, War orders `item` and `template`,
`launch`, `pool`, `observations` (`["obs", pool]`), `kit_config` when the mint has a kit slot;
remaining: the DEX swap's accounts, the chest's token holding (created when missing), the token
`burn`'s accounts with the mint's slot slices, the bridge's `unwrap_sol` accounts.

| Check | Error |
| --- | --- |
| the mint's own kit, if any, does not have holder rewards on (section 3) | `OwnTokenHasRewards` |
| `counter_spend_bps > 0` | `NoWarOrders` |
| `twap(short) * 10_000 <= twap(long) * (10_000 - counter_drop_bps)` (03's TWAP read; `NoObservations` when the ring does not cover `long`) | `CounterNotDue` |
| `now >= last_counter_at + counter_interval_secs` and `now >= launch.created_at + BUYBACK_AFTER_LAUNCH` | `CounterNotDue` |

Spend: `min(chest_balance * counter_spend_bps / 10_000, quote_side * pool_share_bps(launch) / 10_000)`,
the companion's sandwich bound on our own pool. Minimum out as 6.2 with `BUYBACK_SLIPPAGE_BPS`.
No premium wait is needed: the trigger requires the short TWAP **below** the long one, so a
front-runner who pushes the price up first either turns the trigger false or meets the same quote
cap.

Effects: burn everything bought; `season.counter_strikes += 1`; `spent_counter += spend`;
`last_counter_at = now`. Event `CounterStrikeExecuted`.

### 6.4 `raze(mint, rival_mint)`

Sells captured rival tokens back into the rival's pool, slowly. **Permissionless crank, gated by the
War orders:** only when `raze_enabled == 1` (else `RazeDisabled`), so the decision is the holders'
(through the `War` slot's equip rule) and execution is anyone's.

Rate: windows of `RAZE_INTERVAL_SECS`. At a window's first raze, `raze_window_base = captured.amount`;
the window may sell at most `raze_window_base * RAZE_MAX_BPS_PER_INTERVAL / 10_000` (`RazeLimit`).
One call sells at most the remaining allowance and at most `pool_share_bps(rival_launch)` of the
rival pool's base side. Minimum out: the rival pool's quote for the sell net of its sell fee bound,
less `SIEGE_SLIPPAGE_BPS`. Proceeds return to the chest; the bounty comes from them. The rival's
items run on the sell (a rival Shield may charge it; that is the rival's defense).

Effects: `captured.amount -= sold`, `razed_in_window += sold`, `razed_proceeds += got`. Event `Razed`.

### 6.5 `return_captured(mint, rival_mint)` (peace)

Sends every captured rival token to the **rival's** war chest. **Permissionless, gated twice:** our
War orders have `peace_returns == 1` (else `PeaceReturnsOff`), and the same Treaty item is equipped
in both mints' slot tables (04 3.5: one item names both parties; each side's equip rule consented)
(else `NoTreaty`).

Accounts: `cranker`, `war_state` (w), `war_chest`, `mint`, `rival_mint` (its `Mint` read for the slot
table), the Treaty `item`, our War orders `item` and `template`, `rival_war_chest`, the token
`transfer`'s accounts with the rival's slot slices. No bounty (no SOL moves); the cranker pays rent
of the rival chest's holding if missing. A rival kit with holder rewards would refuse the PDA
destination; then the step fails and the holding stays captured.

Effects: entry cleared. Event `CapturedReturned`.

### 6.6 `share_treaty_inflow(mint)` (R13)

Streams what partners paid this token to its holders, through the kit's `share`.

Accounts: `cranker`, `war_state` (w), `treaty_inbox`, its bridged-SOL holding (w), the mint's
`kit_config` (w), the kit's `share` accounts (`share.rs` 17-43: reward mint, reward vault, token
program, token event authority), the bridge's `unwrap_sol` accounts.

1. `amount = inbox balance`. Below `MIN_SHARE_LAMPORTS` (upstream kit): `NothingToDo`.
2. `bounty = amount * MAX_CRANK_BOUNTY_BPS / 10_000` (the War orders may not exist on a treaty-only
   token, so the overview ceiling is used directly); `share = amount - bounty`.
3. CPI kit `share(share)` with `sharer = treaty_inbox` (signed by this program) and `source` the
   inbox's holding. The kit streams it over its hour (`share.rs` 45-49), so a buy just before
   cannot capture it.
4. If the kit refuses because no holder is eligible (`NoEligibleHolders`) the step fails and the
   SOL waits in the inbox.

A mint without a kit with holder rewards cannot use treaties as payee: the Treaty template's
validation (04) refuses a party whose kit has holder rewards off, so the inbox of such a mint never
receives. `treaty_shared_total += share`. Event `TreatyInflowShared`.

### 6.7 `accrue_treaty_time(mint)`

Permissionless, no bounty. For each Treaty item equipped in the mint's slots **and** in its
partner's (passed as accounts), adds `min(now, season end) - max(last_treaty_tick, season start)` to
`season.treaty_secs`; sets `last_treaty_tick = now`. The site appends it to any war transaction.
Event `TreatyTimeAccrued`.

## 7. Bounties

`claim_bounty(mint)`: a holder turns raid points into SOL from the chest.

Accounts: `owner` (signer), `holding` (`["holding", mint, owner]`), `mint`, `war_state` (w),
`war_chest`, War orders `item` and `template`, the token program's `touch` accounts (01) with the
Raid slot's slice, the bridge's `unwrap_sol` accounts.

1. Read the holding's Raid range (offset and length from the mint's slot table; layout 04 3.1:
   tag, `season`, `points`, `tickets`). A range of another season counts 0. Zero points:
   `NothingToDo`.
2. `pay = min(points * bounty_rate, BOUNTY_MAX_PER_CLAIM, chest_balance)`;
   `spent = ceil(pay / bounty_rate)`, never above `points`.
3. `touch` the holding, signed by `["war-signer"]`, payload `SpendRaidPoints { amount: spent }`
   (01 `touch`, 04 accepts it only from that signer).
4. Re-read the range: points fell by exactly `spent`, else `TouchMismatch`.
5. Pay `pay` as SOL to `owner`; `paid_bounties += pay`. Event `BountyClaimed`. No crank bounty.

## 8. Loot

### 8.1 Tickets

A counter in the Raid range (04): the Raid token half adds one per raid buy of at least
`LOOT_MIN_RAID_LAMPORTS`; `claim_quest` adds one (section 9). Tickets move with tokens, pro rata,
as raid points do (04 3.1).

### 8.2 `roll(mint, nonce)`, then `reveal(mint, nonce)`

Randomness comes from the oracle chosen under D-4. **Never** a slot hash, a recent blockhash or
anything the trader or a cranker chooses.

**How a roll names its oracle account.** `RollRequest` stores `oracle_program`, which must equal
`WarConfig.randomness_program` at `roll` time, and `oracle_account`, the randomness account the
adapter's request instruction creates or commits for this roll. `reveal` must pass exactly that
account (`WrongRandomness` otherwise). Changing `randomness_program` (section 11) does not affect
requests already made.

`RollRequest` at `["roll", holding, nonce: u64 le]`: `owner`, `mint`, `holding`, `season: u32`,
`requested_slot: u64`, `oracle_program`, `oracle_account`, `bump`.

`roll`: accounts `owner` (signer, w), `holding`, `mint`, `roll_request` (init, owner pays rent), the
oracle adapter's request accounts, the `touch` accounts.
1. `touch` with `SpendTicket` (04 subtracts one or fails).
2. Write `RollRequest`. 3. One CPI to the adapter's request instruction. Event `RollRequested`.

`reveal`: permissionless, no bounty (nothing in the chest moves).
1. The oracle account reports a value produced after `requested_slot` for this request, else
   `RandomnessNotReady` or `WrongRandomness`.
2. From the season's `LootTable` (8.3): template by weight from the first bytes, then each parameter
   uniformly in its range from disjoint later bytes.
3. CPI the armory's `mint_loot(owner, template_id, params)`, signed by `["loot-signer"]`, the only
   signer it accepts (00 4.3).
4. Close `roll_request`, rent to `owner`. Event `RollRevealed`.

`cancel_roll(mint, nonce)`: the owner, after `ROLL_EXPIRY_SECS` with no fulfilment; rent returned,
**ticket not refunded** (a refund would let a trader cancel rolls whose value showed up badly). Event
`RollCancelled`. D-4 must check whether the chosen oracle exposes the value before `reveal`.

### 8.3 `LootTable` at `["loot", season: u32 le]`

`season`; `entries: [LootEntry; LOOT_TABLE_LEN]` with `LootEntry { template_id: u16, weight: u32,
ranges: [ParamRange; PARAM_FIELDS] }`, `ParamRange { min: u32, max: u32 }`, each inside the
template's floor and ceiling (checked at proposal and again by `mint_loot`); `eta: i64`. Set through
the timelock (section 11). The admin chooses rarity, never code (00 rule 4).

## 9. Quests

Only quests whose condition this program checks, each granting one loot ticket into the named mint's
Raid range. A quest must be impossible to farm by moving tokens between wallets, which rules out any
quest whose proof travels with the tokens and is not consumed.

| Id | Name | Condition | Once per |
| --- | --- | --- | --- |
| 1 | `Raid` | the holding's Raid range holds at least `QUEST_RAID_POINTS` points of the current season; the claim **spends** them (`touch` `SpendRaidPoints`) | `QUEST_PERIOD_SECS` per (season, mint, owner) |
| 2 | `Forge` | the armory's per-wallet forge counter (02) is above the value recorded at the last `Forge` claim | `QUEST_PERIOD_SECS` per (season, owner); the ticket goes to the mint the owner names |

Rejected for v1: **`Hold`** (balance held for a time). Its proof is an age stamp that travels with
the tokens (Half-Life blends age to the receiver, 04 3.7), so one bag moved between wallets would
claim once per wallet per period. Revisit if a non-travelling marker is ever added.

**Where the "once per period" marker lives: a PDA, not hook data.** With the kit in a `Locked`
slot only 32 bytes remain for item ranges (R8), and Raid, Shield and Half-Life already use all of
them (04 C4). So:

`QuestMark` at `["quest", season: u32 le, mint, owner]` (for `Forge`, `mint` is the default key):
`last_period: u32`, `last_forge_count: u64`, `bump`. Rent paid by the claimant at the first claim;
`close_quest_mark` returns it to the owner once the season is finalized.

`claim_quest(quest_id, mint, period)`: accounts `owner` (signer, w), `holding` (of `mint`), `mint`,
`quest_mark` (init if needed), the armory's forge counter for `Forge`, the `touch` accounts. Checks
the condition and `period == current period` and `period > last_period`
(`QuestAlreadyClaimed`); then `touch` with the payloads `SpendRaidPoints` (quest 1 only)
and `AddTicket { amount: 1 }` (both quests). Event `QuestClaimed`.

The quest definitions above are constants of the program; `/v1/quests` (06) prints them from this
table.

## 10. Seasons

### 10.1 Counters and rollover

Every instruction that writes a `WarState` first rolls it: if `season_id < current_season`, copy
`season` into `prev_season` when `season_id == current_season - 1` (else zero both), zero `season`,
set `season_id`. A score for season `n` is therefore read from one `WarState` in O(1).
`raid_volume_won` is taken from the `RaidLedger`'s `season_volume` when its `season` matches, copied
into `season.raid_volume_won` at each roll and at `submit_candidate`.

### 10.2 `Season` at `["season", number: u32 le]`

| Field | Meaning |
| --- | --- |
| `number`, `starts_at`, `ends_at` | `ends_at = starts_at + SEASON_SECS` |
| `weights: ScoreWeights` | one `u64` per counter: `raid_volume_won`, `sieges`, `siege_spend`, `times_besieged`, `counter_strikes`, `treaty_secs` |
| `penalize_besieged: bool` | when set, `times_besieged * weight` is subtracted instead of added |
| `eta` | from the timelock (section 11) |
| `leader: Option<Pubkey>`, `leader_score: i128`, `finalized`, `prize_paid: u64` | |

**The score, on chain:**

```
score = raid_volume_won * w_raid + sieges * w_sieges + siege_spend * w_spend
      + counter_strikes * w_counter + treaty_secs * w_treaty
      + (penalize_besieged ? -1 : +1) * times_besieged * w_besieged
```

computed in `i128` with checked arithmetic over the token's counters for that season. The site
computes it identically from the `Season` and `WarState` accounts (06).

`open_season()`: permissionless, once `now >= Season.starts_at` (which the timelock put at least
`ADMIN_TIMELOCK_SECS` after the proposal), the previous season finalized, and the season's
`LootTable` past its `eta`. Increments `current_season`. Event `SeasonOpened`.

### 10.3 King of the hill, without scanning every token

- `submit_candidate(season, mint)`: from `ends_at` to `ends_at + CHALLENGE_SECS`. Reads that mint's
  `WarState` (rolled) and `RaidLedger`, computes the score; it becomes the leader if there is none
  (event `CandidateSubmitted`) or if its score is strictly greater, or equal with a lower mint key
  (event `CandidateChallenged`, naming the beaten leader). Else `NotHigherScore`. O(1) per call.
- `finalize_season(season)`: after `ends_at + CHALLENGE_SECS`; sets `finalized` and
  `WarConfig.last_winner`. Event `SeasonFinalized`.

The window never extends, so nobody keeps a season open by submitting.

### 10.4 The prize: a share of protocol fees (R14, confirmed)

The DEX's `fee_collector` is set once to `["prize-vault"]` under `<WAR_ID>`, a system-owned PDA;
`collect_protocol_fees_sol` already pays any `address`-checked wallet (`pool.rs` 646, 675). No DEX
code change.

`split_protocol_fees()`: permissionless. Of the vault's lamports above its rent minimum,
`SEASON_PRIZE_SHARE_BPS` goes to the last finalized winner's chest (wrapped to bridged SOL into its
holding) and the rest to `protocol_treasury`; a crank bounty of at most `MAX_CRANK_BOUNTY_BPS` comes
from the treasury part. With no winner yet, all goes to the treasury. The winner of season `n` earns
from fees collected while season `n + 1` runs; `Season(n).prize_paid` accumulates. Event `PrizePaid`.

The admin-only `collect_protocol_fees` into the collector's quote holding (`pool.rs` 578) bypasses
the split; after the change the operator uses only the SOL path (03 records this).

## 11. Admin setters behind the timelock (D-9)

Every power the admin has is proposed, waits `ADMIN_TIMELOCK_SECS`, then applies; the admin may
cancel before it applies. Nothing applies instantly.

| Power | Propose | Apply |
| --- | --- | --- |
| a season's weights and dates | `propose_season(number, starts_at, weights, penalize_besieged)` creates `Season` with `eta = now + ADMIN_TIMELOCK_SECS`; `starts_at >= eta` | `open_season` (10.2), permissionless |
| a season's loot table | `propose_loot_table(season, entries)` writes `LootTable` with `eta` | used by `reveal` only once `eta` has passed and only for its season; frozen when the season opens |
| `admin`, `protocol_treasury`, `randomness_program` | `propose_config(change)` writes `WarConfig.pending` with `eta` | `apply_config()`, permissionless after `eta` |
| any pending item | `cancel_pending(...)` by the admin before it applies | |

Events `SeasonProposed`, `LootTableProposed`, `ConfigProposed`, `ConfigApplied`, `PendingCancelled`.
A `Season` or `LootTable` cannot be edited after proposal, only cancelled and proposed again.

## 12. Events (R15: the names 06 uses)

All emitted by self-CPI (`emit_cpi!`), as upstream.

| Event | Fields |
| --- | --- |
| `WarChestCreated` | `mint, chest, treaty_inbox, war_state` |
| `WarFunded` | `mint, amount, balance, funded_total` |
| `SiegeExecuted` | `mint, rival_mint, spent, bought, bounty, cranker, captured_total` |
| `SiegeWaited` | `mint, rival_mint, rival_price, rival_twap` |
| `CounterStrikeExecuted` | `mint, spent, burned, bounty, cranker` |
| `Razed` | `mint, rival_mint, sold, got, bounty, cranker, captured_left` |
| `CapturedReturned` | `mint, rival_mint, amount, treaty_item` |
| `TreatyInflowShared` | `mint, amount, bounty, cranker` |
| `TreatyTimeAccrued` | `mint, treaty_item, secs, season` |
| `BountyClaimed` | `mint, owner, points, paid` |
| `RollRequested` | `roll, mint, owner, holding, season, requested_slot, oracle_program, oracle_account` |
| `RollRevealed` | `roll, mint, owner, item, template_id, params, season` |
| `RollCancelled` | `roll, mint, owner` |
| `QuestClaimed` | `quest_id, mint, owner, season, period` |
| `SeasonProposed` | `season, starts_at, ends_at, weights, penalize_besieged, eta` |
| `LootTableProposed` | `season, eta` (entries read from the account) |
| `SeasonOpened` | `season, starts_at, ends_at` |
| `CandidateSubmitted` | `season, mint, score, submitted_by` |
| `CandidateChallenged` | `season, mint, score, submitted_by, beaten` |
| `SeasonFinalized` | `season, winner, score` |
| `PrizePaid` | `season, winner, to_winner, to_treasury, bounty, cranker` |
| `ConfigProposed`, `ConfigApplied`, `PendingCancelled` | `change, eta` |

The raid mark event 06 calls `RaidMarked` is emitted by `hookwars_items` (R3; 04 names it
`RaidMarked`), not by this program.

## 13. Errors

`AlreadyInitialized`, `MissingWarSlot`, `NoWarOrders`, `WrongWarOrders`, `MissingAccount`,
`NothingToDo`, `ChestInsufficient`, `SelfSiege`, `SiegeTargetHasRewards`, `OwnTokenHasRewards`,
`SiegeNotDue`, `CapturedTableFull`, `CounterNotDue`, `NoObservations`, `RazeDisabled`, `RazeLimit`,
`PeaceReturnsOff`, `NoTreaty`, `TouchMismatch`, `RandomnessNotReady`, `WrongRandomness`,
`RollNotExpired`, `QuestAlreadyClaimed`, `QuestConditionUnmet`, `SeasonNotOpen`,
`SeasonNotEnded`, `ChallengeClosed`, `NotHigherScore`, `SeasonNotFinalized`, `TimelockNotPassed`,
`NotAdmin`, `MathOverflow`.

## 14. Call depth and compute

Measured values go in 07; these are the shapes.

| Path | Depth (top level 1) |
| --- | --- |
| `siege`: war, DEX swap, launchpad pool hook, pool items | 4 |
| `siege`: war, DEX swap, token transfer, rival token items | 4 |
| `counter_strike` burn: war, token burn, token items | 3 |
| `claim_bounty`, `claim_quest`, `roll`: war, token `touch`, Raid item | 3 |
| `share_treaty_inflow`: war, kit `share`, token transfer (reward mint has no hook) | 3 |
| `reveal`: war, armory `mint_loot`, token `mint_to` (item mints have no slots) | 3 |
| `split_protocol_fees`: war, bridge wrap | 2 |

**Measured (M4/M5):**

| Path | Keys | v0 bytes (lookup table) | Trace | Height | CU |
| --- | --- | --- | --- | --- | --- |
| `init_war` | 16 | 281 | 10 | 3 | 87,566 |
| `siege` (rival a war token, marked) | 37 | 346 | 22 | 4 | 312,139 |
| `raze` | 34 | 339 | 19 | 4 | 273,955 |
| `counter_strike` | 34 | 348 | 24 | 4 | 324,581 |
| `claim_bounty` | 24 | 304 | 12 | 4 | 117,702 |
| `roll` | 16 | 290 | 9 | 3 | 63,689 |
| `reveal` (armory stand-in, mints nothing) | 13 | 275 | 4 | 2 | 36,799 |
| `claim_quest` (Raid) | 15 | 286 | 10 | 3 | 79,183 |
| `submit_candidate` | 8 | 269 | 3 | 2 | 21,990 |
| `split_protocol_fees` (with a winner) | 20 | 304 | 11 | 4 | 88,099 |

Account sizes with the provisional layout constants (`MAX_CAPTURED` 8, `LOOT_TABLE_LEN` 8,
`PARAM_FIELDS` 11): `WarConfig` 551 bytes (rent 4,725,840 lamports), `WarState` 984 (7,739,520),
`Season` 178 (2,129,760), `LootTable` 775 (6,284,880), `RollRequest` 198 (2,268,960), `QuestMark` 90
(1,517,280). Heights include the self-CPI events. Measured by
`programs/tests/tests/budgets_war.rs::war_budgets` (LiteSVM, the M4/M5 branch, 2026-10-09), on
launch pools with no kit and slot tables of a War slot and a touch-only Raid slot; heavier rival or
token items add their own cost to `siege`, `raze` and `counter_strike`.

All within 5. A siege runs the rival's items: heavy rival items cost the siege more compute, and a
refusing rival Defense item makes sieges on it fail. Both are intended.

## 15. Interfaces with other parts

| From | What this part needs |
| --- | --- |
| 00 | seed `["treaty-inbox", mint]` under war; seed `["quest", season, mint, owner]` under war; parameter `ADMIN_TIMELOCK_SECS`; `QUEST_PERIOD_SECS`, `QUEST_RAID_POINTS` named under `QUEST_*` |
| 01 | `touch(holding, slot_index, payload)` forwarding the signer as authority, applying only that slot's range, emitting `HookDataWritten`; the `Mint` slot table readable (War slot item, kit slot, Treaty items) |
| 02 | `Item`, `Template` readable (template id, params, floors and ceilings, code hash); `mint_loot` accepting only `["loot-signer"]`; a per-wallet forge counter |
| 03 | `Observations` with a TWAP read that returns none when the ring does not cover the window; companion `war_bps` paid to the chest's bridged-SOL holding; `init_war` at launch; launch swap builders for a rival's pool with its slot slices; the `fee_collector` note (10.4) |
| 04 | the War orders template (kind `War`) with the fields of 6.0; `RaidLedger` with the fields of 2.5 including `season: u32` and `season_volume`; Raid range with `season_id: u32` (04 section 3, matches `current_season`); touch payloads `SpendRaidPoints`, `SpendTicket`, `AddTicket` accepted only from `["war-signer"]`; `settle_equip` paying Treaty and Tribute to the partner's `["treaty-inbox", partner]` holding and war-destined cuts to `["war-chest", mint]`; Treaty validation refusing a payee without kit holder rewards; Defense items reading `WarState.under_siege_until` |
| kit | none in v1 (R10); D-7 later |

## M4/M5 implementation notes

Built on branch `m4` (2026-10-09): `programs/hookwars_war`, the test-only stand-ins
`war_items_stub` (at the items id), `war_armory_stub` (at the armory id) and `randomness_stub`, and
the suites `war.rs`, `siege.rs`, `counter_strike.rs`, `loot.rs`, `quests.rs`, `seasons.rs`,
`budgets_war.rs` (53 tests) with the harness `programs/tests/src/war.rs`. Where the spec was open,
the choice its rulings imply was taken:

1. **War orders are recognised by kind, not id.** The armory numbers templates densely in
   registration order (02 2.3), so a template id is not a constant. The War slot's item must be an
   armory `Item` of a `Template` whose `kind` is `War` and whose `program` is the items program.
   Staleness of the template's code is the armory's check at equip (02 3.3: items already equipped
   keep running); the war program does not re-read ProgramData.
2. **Peace is gated by the Treaty, not by the War orders.** 04 3.9 has eleven War orders fields
   and no `peace_returns`; 04 3.5 gives the Treaty `returns_captured` (field 2). `return_captured`
   requires that field to be 1 and the same Treaty item in both mints' slot tables. The Treaty's
   template id is `WarConfig.treaty_template_id`, set at `init_config` and changed only through the
   timelock (same reason as 1).
3. **Names follow the owning part.** The siege threshold unit is `SIEGE_UNIT_LAMPORTS` (04 3.9,
   where 6.0 here said `POINT_UNIT_LAMPORTS`); the ledger's season fields are `season_id` and
   `outbound_volume_season` (04 2.9, where 2.5 here said `season` and `season_volume`).
4. **Every policy number is a config field behind the timelock** (`WarParams`, 21 fields).
   `init_config` is signed by the program's upgrade authority (upstream configs' rule); after it,
   only `propose_config` then `apply_config`. Validation: every bps at most 10,000, the crank bounty
   at most the companion's `MAX_BOUNTY_BPS`, every duration and unit positive.
5. **Layout constants are provisional:** `MAX_CAPTURED` 8 and `LOOT_TABLE_LEN` 8 (account sizes
   measured in section 14). `PARAM_FIELDS` 11 and `RAID_TABLE_LEN` 8 are assumed in
   `src/foreign.rs` until the armory and items crates fix them.
6. **Chest solvency is an identity.** `WarState::expected_balance()` is `funded_total +
   razed_proceeds - spent_siege - spent_counter - paid_bounties - paid_cranks`, and every suite
   asserts it equals the chest's balance (and `last_seen_balance`) after every step. Two rules keep
   it exact: `razed_proceeds` is gross (the raze bounty is in `paid_cranks`), and a treaty-inflow
   bounty is paid from the inbox, so it is not in `paid_cranks`.
7. **Season counters by season number, not by time.** A war step does not read the `Season`, so
   activity after `ends_at` and before the next season opens still counts toward the running season;
   the next season cannot open before the last is finalized, which bounds this to
   `CHALLENGE_SECS`. `raid_volume_won` is synced from the `RaidLedger` at `submit_candidate` (the
   only time it is read), not at every roll.
8. **Quest period:** `floor((now - Season.starts_at) / QUEST_PERIOD_SECS) + 1` (the first is 1).
9. **One `cancel_pending(what, season)`:** `what` 0 the config change, 1 a season, 2 a loot table;
   seasons and loot tables only before their season opens.
10. **`WarConfig.last_winner_season`** is stored with `last_winner`, so `split_protocol_fees` knows
    which `Season.prize_paid` to add to.
11. **The randomness adapter interface** is fixed in `src/oracle.rs` (D-4 stays open for the oracle
    behind it): `request_randomness(requester)` with the `RollRequest` PDA signing, and a
    `Randomness` account (`requester`, `request_slot`, `fulfilled`, `fulfilled_slot`, `value`).
    `reveal` takes a value only when fulfilled in a slot after the request. The draw: a template by
    weight from bytes 0..8, each field from its own two bytes (8 + 2i), so 11 fields use 30 of 32.
12. **`mint_loot`'s call:** data `owner, template_id: u16, params: [u32; PARAM_FIELDS]`; accounts
    `loot_signer` (signer), `payer` (the revealer), `owner`, then the remaining accounts in order.
    `RollRevealed.item` is the default key (the armory's `LootMinted` names the item).
13. **Token slices are the first remaining accounts**, counted by `SliceArgs` (`first`, `second`):
    the rival's delivery slice for `siege`, the input slice for `raze`, buy then burn slices for
    `counter_strike`, the rival's Locked slice for `return_captured`. Touches take a `raid_slot`
    index; a slot qualifies when its program is the items program, it answers touch and its range is
    12 bytes (11 plus the epoch byte).
14. **`return_captured` moves tokens with `transfer_from_protocol` (R16)**, so on the rival's mint
    only its Locked slot runs.
15. **Accounts other programs write are read by hand** (`siege`, `counter_strike`, `raze`): the
    rival's and our launch, pool and mint are `UncheckedAccount`, decoded with owner and
    discriminator checks, so no `Account<..>` exit writes back a pre-swap copy.
16. **Measured sandwich results** (`siege.rs`, `counter_strike.rs`, TEST launch: LP fee 0.3%,
    creator fee 1%): an attacker who bought 0.1, 0.5 or 1 SOL of the rival before a siege, and sold
    after, ended with 9.998, 9.985 and 9.969 SOL of 10 (at 0.5 and 1 SOL the premium check made the
    siege wait); around a counter-strike with 0.1, 0.5 and 2 SOL, 9.998, 9.991 and 9.963.
17. **Tests write what other branches own.** Slot tables are written into upstream launches' mints
    (the slot-aware launchpad is M3b); the War orders `Item`, `Template`, `ForgeCounter`,
    `RaidLedger` and `Observations` are written with `put` in the stand-ins' Anchor layouts; the
    treaty-inflow test writes a `WarState` directly for a kit launch, since the kit cannot sit in a
    Locked slot in this branch (R9 is the kitcomp branch).
18. **For the integrator:** M1's `programs/tests/tests/slots.rs` fails `cargo clippy -D warnings`
    (an unneeded `mut` at line 134, a slice clone at line 236); not changed here.

### Integration steps

1. **Armory (M2):** replace `foreign::{Item, Template, Manifest, ForgeCounter, disc::{ITEM,
   TEMPLATE, FORGE_COUNTER}}` and `PARAM_FIELDS` with the `hookwars_armory` crate's types; build
   `mint_loot` with the armory's client and pass `create_item`'s accounts after `owner`; fill
   `RollRevealed.item` from the armory's item derivation; delete `war_armory_stub` and switch the
   suites to the real armory (templates registered, items created, `ForgeCounter` from real forges).
2. **Items (M3b):** replace `foreign::{RaidLedger, RaidWindow, Mark, RAID_TABLE_LEN,
   disc::RAID_LEDGER}` and `common::WarTouch` with the `hookwars_items` crate's; check its Raid range
   layout against `common::RaidRange`; delete `war_items_stub`; give the Raid slot's real extras
   (`extra_count`) in the suites' touches.
3. **DEX (M3a):** replace `foreign::{ObservationHeader, Observation, Observations, OBS_HEADER_LEN,
   disc::OBSERVATIONS, spot_q64}` and the TWAP read with `bordrless_swap` / `bordrless_core`'s;
   confirm whether `Observations` is an Anchor account (this branch assumes its discriminator);
   replace the suites' `put_observations` with real swaps over time.
4. **Launchpad (M3b):** slot tables from `prepare_launch` and `init_war` in the launch's setup;
   delete the harness's `add_slot` patching.
5. **Companion (kitcomp):** add a funding test through `claim_fees` with `war_bps`.
6. **Kit (R9, D-7):** with the kit in a Locked slot, run the treaty-inflow test on a real war token
   (drop `put_war_state`).
7. **Deployment:** the DEX config's `fee_collector` set to `["prize-vault"]` under the war program
   (10.4); a randomness adapter program for the chosen oracle (D-4).
8. **Pin the precomputed discriminators** (`foreign::disc`, `oracle::REQUEST_RANDOMNESS`,
   `loot::MINT_LOOT`) against the owning crates' `Discriminator` in a host test once those crates
   are in the workspace.
