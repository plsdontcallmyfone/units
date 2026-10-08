# Hookwars spec 05: war (`hookwars_war`)

Status: specification, 2026-10-08. Nothing is built. Follows `00-overview.md`; names, seeds and
parameters are the overview's. Parameters this part needs that the overview does not list yet are
in section 13 for the overview to adopt.

Upstream references, cited by file and line at `43688f3`:

- companion steps: `programs/bordrless_companion/src/instructions/steps.rs` (buyback 357-480, its
  fee bound 315-319, pool share 326-333, reference catch-up 336-347)
- companion limits: `programs/bordrless_companion/src/constants.rs` (`MAX_BOUNTY_BPS` 15,
  `BUYBACK_POOL_SHARE_BPS` 24, `MAX_PREMIUM_BPS` 27, `BUYBACK_AFTER_LAUNCH` 38,
  `BUYBACK_SLIPPAGE_BPS` 40), the reasoning in `docs/companions.md` 38-49
- calling by built instruction: `programs/bordrless_companion/src/invoke.rs` 1-33
- DEX slippage on what arrived: `programs/bordrless_swap/src/instructions/swap.rs` 413
- protocol fee collection: `programs/bordrless_swap/src/instructions/pool.rs` 646 (collector is
  `address`-checked) and 675 (`process_collect_protocol_fees_sol`)
- kit owners rule: `docs/hooks-v2.md` section 4.5 (lines 458-480)

## 1. What the war program does

`hookwars_war` holds each token's **war chest** and spends it only through the instructions below.
Every spending instruction is permissionless, checks on chain that it is due, is capped per call and
per interval, and pays its sender a bounty of at most `MAX_CRANK_BOUNTY_BPS` of what it moves
(overview rule 5). It never holds a user's signature for anything but that user's own claim.

Spot only: a siege is a spot buy, a counter-strike is a spot buy and a burn, a raze is a spot sell,
a bounty and a loot roll pay for raid activity already recorded. Nothing pays on a season's outcome
except the season prize, which is a share of protocol fees that already exist (section 10).

## 2. Accounts

### 2.1 `WarConfig` at `["war-config"]`

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump` | u8 | |
| `admin` | Pubkey | `<PROTOCOL_AUTHORITY>`; opens seasons and sets loot tables (section 9.1) |
| `protocol_treasury` | Pubkey | wallet that receives the protocol's fees after the season prize share |
| `prize_vault_bump` | u8 | bump of `["prize-vault"]` (section 10.3) |
| `randomness_program` | Pubkey | the oracle adapter chosen under D-4 (section 8.2) |
| `current_season` | u32 | the season now running (0 before the first) |
| `last_winner` | Option<Pubkey> | the mint that won the last finalized season |
| `reserved` | [u8; 64] | |

### 2.2 `WarChest` at `["war-chest", mint]`

A **system-owned address with no data**, exactly like the companion's creator
(`constants.rs` "`PDA(["creator", mint])`"): only this program signs for it. It holds:

- a holding of bridged SOL (`["holding", BRIDGED_SOL_MINT, war_chest]`): the chest's spendable
  balance;
- holdings of captured tokens (`["holding", rival_mint, war_chest]`);
- holdings of its own token only for the instant between a counter-strike's buy and its burn.

Its lamports are never spent below the rent-exempt minimum of a zero-data account.

### 2.3 `WarState` at `["war", mint]` (owned by this program)

| Field | Type | Meaning |
| --- | --- | --- |
| `version`, `bump`, `chest_bump` | u8 | |
| `mint` | Pubkey | |
| `launch` | Pubkey | the token's `Launch` (launch pools only; section 3) |
| `funded_total` | u64 | bridged SOL received, all sources |
| `spent_siege`, `spent_counter`, `paid_bounties`, `paid_cranks` | u64 | running totals |
| `razed_proceeds` | u64 | |
| `last_siege_at`, `last_counter_at` | i64 | spacing |
| `captured` | [Captured; `MAX_CAPTURED`] | see below |
| `season_id` | u32 | season the counters below belong to |
| `season` | SeasonCounters | the current season's counters |
| `prev_season` | SeasonCounters | frozen copy of the previous season's counters (section 10.1) |
| `reserved` | [u8; 64] | |

`Captured { rival_mint: Pubkey, amount: u64, cost: u64, captured_at: i64, raze_window_start: i64,
raze_window_base: u64, razed_in_window: u64 }`. An empty entry has `rival_mint` default. A siege on
a rival with no free entry and no entry of its own fails with `CapturedTableFull`; entries free when
their `amount` reaches 0 (raze or peace return).

`SeasonCounters { raid_volume_won: u64, sieges: u32, siege_spend: u64, treaty_secs: u64,
counter_strikes: u32 }`. Values only, never mark-to-market, so nobody can raise a score by moving a
price at the end of a season.

### 2.4 The raid ledger (owned by `hookwars_items`, read here)

Raid volume is written **inside swaps**, by the pool-kind Raid item, and an account can only be
written by its owner program. A CPI from the item into this program would add a call level on
every raid buy (DEX 1, launchpad hook 2, item 3, war 4, and graduation deeper still). So the raid
volume table lives in an account owned by `hookwars_items` (04 defines it as `RaidLedger` at
`["raid-ledger", mint]` under `<ITEMS_ID>`), and this program **reads** it. What this part needs from
it:

| Field | Meaning |
| --- | --- |
| `inbound: [RaidWindow; RAID_TABLE_LEN]` | per rival whose holders raided **us** (sold rival to buy us) |
| `RaidWindow { rival_mint, window_start: i64, volume: u64, prev_volume: u64 }` | volume in quote lamports in the current fixed window of `RAID_WINDOW_SECS`, and the previous window's |
| eviction | a new rival takes the entry whose `window_start` is oldest when every entry has ended its window; otherwise it is not recorded (no eviction of a live window, so a flood of dust rivals cannot erase a real raid) |
| `outbound_volume_season: u64`, `season_id: u32` | raid volume **our** holders brought into us this season (feeds `raid_volume_won`) |

"Rolling" is two fixed windows: the volume used for a check at time `t` is
`volume + prev_volume * (window_start + RAID_WINDOW_SECS - t) / RAID_WINDOW_SECS` (the previous
window weighted by how much of it still overlaps the last `RAID_WINDOW_SECS`). It is cheap, needs no
ring per rival and cannot be reset by one trade.

A raid is **active against us** from rival X while that rolling volume from X is above zero.
Shield and Wall (04) use the same definition.

## 3. Which tokens can have a war chest

A war chest needs the token's launch pool (counter-strike buys there) and a `Launch` account.
`init_war` therefore requires a `Launch` under `<LAUNCH_ID>` for the mint. Bridged tokens and
ordinary pools have no war chest.

**Kit conflict (blocking, see section 14).** With holder rewards on, the kit refuses any destination
off the ed25519 curve (`docs/hooks-v2.md` 4.5): a PDA cannot hold the token. The war chest is a PDA,
so as specified upstream it can neither receive its own token in a counter-strike nor capture a
rival whose launch has holder rewards on. The companion only escapes this because the kit excludes
exactly the companion's creator (`docs/companions.md` 69-71). This part assumes the kit change in
section 14: the kit treats `["war-chest", mint]` under `<WAR_ID>` as an **excluded owner** (no
rewards, no cap, not counted in `eligible`) for any mint, proven by passing that mint's `WarState`.

## 4. Funding

| Source | How it arrives |
| --- | --- |
| Companion `war_bps` | 03 adds `war_bps` to the companion's `Split` (upstream `state.rs` "`Split`"); `claim_fees` moves that share of each claim, in bridged SOL, to the chest's bridged-SOL holding. `funded_total` is updated by `record_funding` (below) |
| Raid item fee share | the pool-kind Raid item answers a quote-side cut into the chest's bridged-SOL holding on raid buys (04). Bordrless's protocol share of cuts applies first (overview D, `docs/hooks-v2.md` 3.1) |
| Anyone | a plain transfer of bridged SOL to the chest holding |

`record_funding(mint)`: permissionless, no bounty. Reads the chest's bridged-SOL balance, adds the
increase since `last_seen_balance` (a field kept in `WarState`, updated on every spend) to
`funded_total`, emits `ChestFunded { mint, amount, balance }`. Spends call it first, so totals stay
exact without trusting senders.

## 5. Instructions: setup

### `init_war(mint)`

Accounts: `payer` (signer, writable), `mint`, `launch` (under `<LAUNCH_ID>`, `mint` matches),
`war_state` (init, `["war", mint]`), `war_chest` (`["war-chest", mint]`, not created: system
address), `chest_quote_holding` (created through the token program's `create_holding`, payer pays),
token program and its event authority, system program, event authority, program.

Checks: the mint's slot table (01) has a `War` slot (section 14) or `MissingWarSlot`. Idempotent:
`init` fails on an existing `WarState`, so a second call is `AlreadyInitialized`.

The launchpad (03) calls `init_war` in the launch's setup transaction when the launch config asks
for a war chest; a token launched without one can be given one later by anyone.

## 6. Instructions: attack and defense

All spending instructions share these steps, as companion steps do (`steps.rs` 357-480):

1. `record_funding`.
2. Read the equipped `War` slot item's parameters through the armory (02): `Item` account, its
   `Template` must be the War template of `<ITEMS_ID>`, and it must be the item the mint's slot
   table names. A mismatch is `WrongWarItem`.
3. Compute the spend; refuse 0 with `NothingToDo`.
4. Bounty first: `bounty = spend * bounty_bps / 10_000`, with `bounty_bps` the War item's parameter,
   at most `MAX_CRANK_BOUNTY_BPS`. The rest is what the step moves.
5. Call other programs only by instructions built here with their own client builders, finding the
   accounts among those passed (`invoke.rs` 1-33). A caller who leaves one out gets
   `MissingAccount`.
6. Never spend more than the chest's bridged-SOL balance; never leave the chest's lamports below
   its rent-exempt minimum.

### 6.1 `siege(mint, rival_mint)`

The chest spot-buys the rival and keeps what it buys.

Accounts: `cranker` (signer, writable), `war_state` (writable), `war_chest`, `raid_ledger`
(`["raid-ledger", mint]` under `<ITEMS_ID>`), the War slot's `item` and `template`, `mint` (for the
slot table), `rival_launch` (under `<LAUNCH_ID>`), `rival_pool`, `rival_observations`
(`["obs", rival_pool]` under `<SWAP_ID>`); remaining: the DEX swap's accounts for the rival pool
with the rival's hook slices, the chest's rival holding (created when missing, the cranker paying),
the bridge's `unwrap_sol` accounts for the bounty.

Checks, in order:

| Check | Error |
| --- | --- |
| `rival_mint != mint` | `SelfSiege` |
| rolling raid volume from `rival_mint` in `raid_ledger.inbound` is at least the War item's `siege_threshold` | `SiegeNotDue` |
| `now >= last_siege_at + SIEGE_INTERVAL_SECS` | `SiegeNotDue` |
| rival spot price is at most its TWAP over the War item's `siege_twap_secs` (at least `MIN_TWAP_SECS`) times `(10_000 + SIEGE_MAX_PREMIUM_BPS) / 10_000` | `RivalAbovePremium` (the chest never buys a rival into its own pump; no state change, like `BuybackWaited`) |
| a `captured` entry exists for the rival or one is free | `CapturedTableFull` |

Spend: `min(chest_balance * SIEGE_MAX_SPEND_BPS / 10_000, rival_quote_side * pool_share_bps(rival_launch) / 10_000)`
where `pool_share_bps` is upstream's (`steps.rs` 326-333): a quarter of a buy and a sell's fees, at
most `BUYBACK_POOL_SHARE_BPS`. Upstream's argument then holds on the rival's pool: someone who pumps
the rival by `p` before the siege buy of `v` and sells after gains about `p * v` and pays about
`p * Q * (buy + sell) / 2` in fees, so at half that bound the sandwich costs more than it makes
(`steps.rs` 321-325).

Minimum out: the rival pool's own quote for `spend` net of `buy_fee_bound(rival_launch)`
(`steps.rs` 315-319), less `SIEGE_SLIPPAGE_BPS`; the DEX checks it against what arrived (`swap.rs`
413). The rival's own token items run on the delivery: a rival Wall or kit max wallet may refuse
the buy, and then the whole step fails and nothing is spent.

Effects: `captured[rival].amount += bought`, `cost += spend`; `season.sieges += 1`,
`season.siege_spend += spend`; `spent_siege += spend`; `last_siege_at = now`; bounty unwrapped and
paid to `cranker` as SOL. Event `Sieged { mint, rival_mint, spent, bought, bounty, cranker }`.

### 6.2 `counter_strike(mint)`

The chest buys its own token on its launch pool and burns it, when the price has fallen.

Accounts: as companion `buyback` (`steps.rs` 349-356): `cranker`, `war_state`, `war_chest`, the
War slot `item` and `template`, `launch`, `pool`, `observations` (`["obs", pool]`); remaining: the
DEX swap's accounts, the chest's token holding (created when missing), the token `burn`'s accounts
with the mint's slot slices, the bridge's `unwrap_sol`.

Trigger: `twap(short) * 10_000 <= twap(long) * (10_000 - counter_drop_bps)` with `short` and
`long` the War item's `counter_short_secs` and `counter_long_secs` (both at least
`MIN_TWAP_SECS`, `short < long`), read from the pool's observation ring (03). Else
`CounterNotDue`. Spacing: `now >= last_counter_at + counter_interval_secs` (item parameter, floored
by `COUNTER_STRIKE_MIN_INTERVAL_SECS`), and `now >= launch.created_at + BUYBACK_AFTER_LAUNCH`
(upstream, so the sniper fee window is never bought into).

Spend: `min(chest_balance * COUNTER_STRIKE_MAX_SPEND_BPS / 10_000, quote_side * pool_share_bps(launch) / 10_000)`;
the same sandwich bound as the companion, on our own pool. Minimum out as in 6.1 with
`BUYBACK_SLIPPAGE_BPS`. Unlike the companion there is no reference price that waits on a premium:
the trigger already requires the price to be **below** its own longer TWAP, so the chest only buys
into a fall, and a front-runner who pushes the price up before the strike makes the trigger false
(the short TWAP rises) or makes the strike buy less (the cap is on quote spent).

Effects: burn everything bought, `season.counter_strikes += 1`, `spent_counter += spend`,
`last_counter_at = now`, bounty paid. Event `CounterStruck { mint, spent, burned, bounty, cranker }`.

### 6.3 `raze(mint, rival_mint)`

The chest sells captured rival tokens back into the rival's pool, slowly.

Allowed only when the War item's `raze_enabled` is set (decided by the token's equip rule, like any
item change). Else `RazeDisabled`.

Rate: the raze window is `RAZE_INTERVAL_SECS`. At the first raze of a window,
`raze_window_base = captured.amount`; each window may sell at most
`raze_window_base * RAZE_MAX_BPS_PER_INTERVAL / 10_000` in total (`RazeLimit`). Each call sells at
most that remaining allowance and at most `pool_share_bps(rival_launch)` of the rival pool's base
side, so one call is never a crash.

Minimum out: rival pool quote for the sell net of the rival's sell fee bound, less
`SIEGE_SLIPPAGE_BPS`. Proceeds (bridged SOL) return to the chest's holding; the rival's token items
run on the sell (a rival Shield may charge the chest its higher sell fee; that is the rival's
defense working).

Effects: `captured.amount -= sold`, `razed_in_window += sold`, `razed_proceeds += got`, bounty from
the proceeds. Event `Razed { mint, rival_mint, sold, got, bounty, cranker }`.

### 6.4 `return_captured(mint, rival_mint)` (peace)

Sends every captured rival token back to the **rival's** war chest when a treaty between the two
tokens is in force.

Accounts: `cranker`, `war_state` (writable), `war_chest`, `mint`, `rival_mint`, both mints' Treaty
items and templates, `rival_war_state` (`["war", rival_mint]`), `rival_war_chest`, the token
`transfer`'s accounts with the rival's slot slices.

Checks: both mints' slot tables have the **same** Treaty item equipped (04: one treaty item names
both parties) and its terms include `returns_captured`. Else `NoTreaty`. No bounty (it moves no SOL);
the transfer's rent, if the rival chest's holding is missing, is paid by the cranker.

Effects: `captured` entry cleared. Event `CapturedReturned { mint, rival_mint, amount }`.

### 6.5 `accrue_treaty_time(mint)`

Permissionless, no bounty. For each Treaty item equipped in the mint's slots (passed as accounts),
adds `now - last_treaty_tick` to `season.treaty_secs`, then sets `last_treaty_tick = now` (a field in
`WarState`). Ticks are capped at the season's end. Cheap enough to be appended by the site to any
war transaction.

## 7. Bounties

`claim_bounty(mint)`: a holder turns raid points into SOL from the chest.

Accounts: `owner` (signer: the holding's owner), `holding` (`["holding", mint, owner]`), `mint`,
`war_state` (writable), `war_chest`, the War slot `item` and `template`, the token program's
`touch` accounts (01) with the Raid slot's slice, the bridge's `unwrap_sol` accounts.

Steps:

1. Read the holding's hook data range of the **Raid** slot (offset and length from the mint's slot
   table, layout from 04). `points = raid_points` for the current season; a range stamped for an
   older season counts 0. `points == 0` is `NothingToDo`.
2. `pay = min(points * bounty_rate, BOUNTY_MAX_PER_CLAIM, chest_balance)`, with `bounty_rate`
   (lamports per point) the War item's parameter. Points spent:
   `spent = ceil(pay / bounty_rate)`, never more than `points`.
3. `touch` the holding through the token program, signed by this program's
   `["war-signer"]` PDA, with the payload `SpendRaidPoints { amount: spent }` (section 12). The Raid
   item checks the signer, subtracts, and answers the new range.
4. Re-read the range: the points must have fallen by exactly `spent`, else `TouchMismatch`.
5. Unwrap and pay `pay` to the owner. `paid_bounties += pay`. Event
   `BountyClaimed { mint, owner, points: spent, paid }`.

No crank bounty: the owner sends it.

## 8. Loot

### 8.1 Tickets

Tickets are a counter in the Raid slot's hook data range (04): the Raid item adds one on each raid
buy of at least `LOOT_MIN_RAID_LAMPORTS` while a raid is active, and `claim_quest` adds one (section
9). They travel with the tokens like raid points.

### 8.2 `roll(mint, nonce)` then `reveal(mint, nonce)`

Randomness comes from an oracle chosen under D-4. This part fixes only the interface: a request
account the oracle fills, read by `reveal`. **Never** the slot hash, a recent blockhash or anything
the trader or cranker chooses.

`roll`: accounts `owner` (signer, writable), `holding`, `mint`, `roll_request` (init,
`["roll", holding, nonce]`, owner pays rent), the oracle adapter's request accounts, the `touch`
accounts.

1. `touch` with `SpendTicket` (the Raid item subtracts one ticket or fails with its own error).
2. Record `RollRequest { owner, mint, season: current_season, requested_slot, oracle_request, revealed: false }`.
3. Call the oracle adapter's request instruction (one CPI).

`reveal`: permissionless (anyone may finish it). It pays no crank bounty, because nothing in the
chest moves; the site sends it with the owner's next transaction, or a keeper sends it.

1. The oracle request is fulfilled, its value was produced **after** `requested_slot`, and the
   request belongs to this `RollRequest`. Else `RandomnessNotReady` or `WrongRandomness`.
2. Pick from the season's `LootTable` (8.3): template by weight, then each parameter uniformly in its
   range, all from disjoint bytes of the value.
3. CPI the armory's `mint_loot(owner, template_id, params)` (02), signed by this program's
   `["loot-signer"]` PDA, which the armory accepts as the only loot minter.
4. Close `roll_request`, rent back to `owner`. Event `LootRevealed { mint, owner, item, template_id, season }`.

A request older than `ROLL_EXPIRY_SECS` that was never fulfilled may be closed by its owner with
`cancel_roll`; the ticket is **not** refunded (otherwise a trader could cancel bad-looking rolls
when the value is visible before reveal). Whether the value is visible before `reveal` depends on
the oracle; D-4 must check this.

### 8.3 `LootTable` at `["loot", season: u32 le]`

| Field | Meaning |
| --- | --- |
| `season` | |
| `entries: [LootEntry; LOOT_TABLE_LEN]` | `{ template_id: u16, weight: u32, ranges: [ParamRange; MAX_TEMPLATE_PARAMS] }` |
| `ParamRange { min: u64, max: u64 }` | each inside the template's ceiling, checked by `set_loot_table` against the armory's `Template` and again by `mint_loot` |
| `locked: bool` | set when the season opens; no edit afterwards |

`set_loot_table(season, entries)`: `WarConfig.admin` only, only before that season opens. The admin
chooses rarity, never code (overview rule 4). Event `LootTableSet`.

## 9. Quests

Only quests a program can check from chain state, each granting one loot ticket.

| Quest | Check |
| --- | --- |
| `Hold` | the holding's age stamp (a template that stamps arrival time, e.g. the Half-Life template, 04) is at least `QUEST_HOLD_SECS` old, and the balance is at least `QUEST_MIN_BALANCE` |
| `Raid` | raid points for the current season at least `QUEST_RAID_POINTS` |
| `Forge` | the armory's per-wallet forge counter (02) increased since the last `Forge` quest claim for this wallet |

`claim_quest(mint, quest)`: accounts `owner` (signer), `holding`, `mint`, the slot items and
templates read, the armory's forge counter for `Forge`, the `touch` accounts.

**Where the "once per period" marker lives: in the holding's hook data, not in a PDA.** The Raid
slot range carries `quest_period: u16` per quest kind (04): the period index since the season's
start, in `QUEST_PERIOD_SECS`. A PDA per wallet per token per period would cost every claimant rent
on every token; hook data costs nothing and is already passed. To stop a holder from farming by
moving tokens to fresh wallets, the Raid item writes the **receiver's** marker as the larger of the
sender's and the receiver's on every transfer (04), so a fresh wallet that receives tokens inherits
the period. The balance floor makes many tiny wallets useless for `Hold`.

`touch` payload `MarkQuest { kind, period }`; then `tickets += 1` in the same answer. Event
`QuestClaimed { mint, owner, kind, period }`.

## 10. Seasons

### 10.1 Counters and rollover

Every instruction that writes `WarState` first rolls it: if `season_id < WarConfig.current_season`,
copy `season` into `prev_season` (only when `season_id == current_season - 1`; otherwise zero both),
zero `season`, set `season_id`. So a `WarState` always shows the current season's counters and the
last one's frozen totals, and reading a score after a season ends is O(1). `raid_volume_won` is read
from the raid ledger's `outbound_volume_season` when its `season_id` matches.

### 10.2 `Season` at `["season", number: u32 le]`

| Field | Meaning |
| --- | --- |
| `number`, `starts_at`, `ends_at` | `ends_at = starts_at + SEASON_SECS` |
| `weights: ScoreWeights` | `{ raid_volume_won, sieges, siege_spend, treaty_secs, counter_strikes }`, each u64, fixed at open |
| `leader: Option<Pubkey>`, `leader_score: u128` | |
| `finalized: bool` | |
| `prize_paid: u64` | |

`score = sum(counter * weight)` in u128, over the token's counters for that season.

`open_season(weights)`: `WarConfig.admin` only, only after the previous season is finalized (or for
the first); locks the loot table; increments `current_season`. The weights are on chain before the
season starts and never change. Event `SeasonOpened`.

### 10.3 King of the hill, without scanning every token

- `submit_candidate(season, mint)`: from `ends_at` until `ends_at + CHALLENGE_SECS`. Reads that
  mint's `WarState`, takes the counters for `season` (`prev_season` when it has rolled, `season`
  when it has not), computes the score. If there is no leader, or the score is strictly greater, or
  equal with a lower mint key (deterministic tie break), it becomes the leader. Else
  `NotHigherScore`. Each call is O(1).
- `finalize_season(season)`: after `ends_at + CHALLENGE_SECS`. Sets `finalized`,
  `WarConfig.last_winner = leader`. Event `SeasonFinalized { season, winner, score }`.

Anyone who believes another token scored more submits it during the window. The window does not
extend on submissions, so nobody can keep a season open by submitting.

### 10.4 The prize: a share of protocol fees

Recommendation: point the DEX's `fee_collector` at a war-program vault instead of changing the DEX.
`collect_protocol_fees_sol` already pays any `address`-checked wallet (`pool.rs` 646, 675), and a
system-owned PDA receives lamports like a wallet.

- `PrizeVault` at `["prize-vault"]`: system-owned, no data. The DEX config's `fee_collector` is set
  to it once (an admin action on the DEX, no code change).
- `split_protocol_fees()`: permissionless, with a crank bounty. Of the vault's lamports above its
  rent minimum, `SEASON_PRIZE_SHARE_BPS` goes to the **last finalized winner's** war chest (wrapped
  into its bridged-SOL holding), the rest to `WarConfig.protocol_treasury`. With no winner yet,
  everything goes to the treasury. Event `ProtocolFeesSplit { to_winner, to_treasury, winner }`.

So the winner of season `n` earns a share of all protocol fees collected while season `n + 1`
runs. The alternative, a vault the protocol funds by hand, was rejected: it depends on someone
remembering to fund it, and the share would not be verifiable on chain.

The admin-only `collect_protocol_fees` (into the collector's quote holding, `pool.rs` 578) bypasses
this split; after the change the admin uses only the SOL path.

## 11. Call depth and compute

Measured values go in 07; these are the shapes to measure.

| Path | Depth (top-level is 1) |
| --- | --- |
| `siege`: war, DEX swap, launchpad pool hook, pool-kind items | 4 |
| `siege`: war, DEX swap, token transfer, rival token items | 4 |
| `counter_strike` burn: war, token burn, token items | 3 |
| `claim_bounty` / `claim_quest` / `roll`: war, token `touch`, Raid item | 3 |
| `reveal`: war, armory `mint_loot`, token `mint_to` (an item mint has no slots) | 3 |
| `split_protocol_fees`: war, bridge wrap | 2 |

All stay within Solana's 5. A siege runs the **rival's** items, which this program does not
control: a rival with heavy items makes sieges on it cost more compute, and a rival whose Defense
item refuses the chest makes sieges on it fail. Both are intended.

## 12. Interfaces with other parts

| From | What this part needs |
| --- | --- |
| 00 | a slot kind `War` (section 14); parameters in section 13; seeds `["war-signer"]`, `["loot-signer"]`, `["prize-vault"]`, `["war-config"]`, `["loot", season]` added to 4.3 |
| 01 | `touch(holding, slot_index, payload: Vec<u8>)`: calls only that slot's item, passes the touch's signer as `authority` and the payload, applies the answered range, emits an event; refuses when the slot's item is not a template that accepts the signer |
| 02 | `Item` and `Template` readable by this program (template id, params, ceilings); `mint_loot(owner, template_id, params)` accepting `["loot-signer"]` under `<WAR_ID>` as the only loot minter; a per-wallet forge counter for the `Forge` quest |
| 03 | `Observations` at `["obs", pool]` with a TWAP read over any window at least `MIN_TWAP_SECS`; companion `war_bps` paying the chest's bridged-SOL holding; `init_war` called at launch when the config asks; launch client builders usable for a rival's launch pool with its slot slices |
| 04 | the War template's parameters (`siege_threshold`, `siege_twap_secs`, `counter_drop_bps`, `counter_short_secs`, `counter_long_secs`, `counter_interval_secs`, `raze_enabled`, `bounty_rate`, `bounty_bps`); the Raid template's range layout (`season_id`, `raid_points`, `tickets`, `quest_period[kinds]`), its touch payloads `SpendRaidPoints`, `SpendTicket`, `MarkQuest` accepted only from `["war-signer"]`; `RaidLedger` (section 2.4) owned by `<ITEMS_ID>`; the receiver-marker rule for quests; the Treaty item naming both parties with `returns_captured` |
| kit | the excluded-owner change in section 14 |

## 13. Parameters this part adds (for 00 section 6)

All to set.

| Name | Meaning | Set by |
| --- | --- | --- |
| `MAX_CAPTURED` | captured entries per chest | M |
| `RAID_TABLE_LEN`, `RAID_WINDOW_SECS` | raid ledger size and window (04 owns the account) | M, O |
| `SIEGE_MAX_PREMIUM_BPS` | how far above its TWAP a rival may be for a siege to buy | O |
| `SIEGE_SLIPPAGE_BPS` | minimum-out margin for siege and raze | O |
| `COUNTER_STRIKE_MIN_INTERVAL_SECS` | floor under the item's counter-strike interval | O |
| `ROLL_EXPIRY_SECS` | when an unfulfilled roll may be cancelled | O |
| `LOOT_TABLE_LEN`, `MAX_TEMPLATE_PARAMS` | loot table size, parameters per template (02 owns the latter) | M |
| `QUEST_PERIOD_SECS`, `QUEST_HOLD_SECS`, `QUEST_MIN_BALANCE`, `QUEST_RAID_POINTS` | quests | O |

## 14. Conflicts and assumptions for other parts

1. **Slot kind `War`.** Siege, counter-strike, raze and bounty rates are parameters of an equipped
   item, but no overview kind fits: they never run as a hook. Proposed: kind `War` (value 6), no
   callbacks, never called by the token program, read by this program; equip rules apply as for any
   slot. Without it, the War parameters would have to sit on a Relation or Pool item.
2. **Kit excluded owner.** Section 3: the kit must treat `["war-chest", mint]` under `<WAR_ID>`
   as excluded for every mint (proven by the mint's `WarState` passed as an extra), or war chests
   cannot hold tokens with holder rewards on. The overview lists the kit as "kept"; this is a change.
3. **Raid ledger ownership.** Raid volume is written inside swaps, so it is owned by
   `hookwars_items` (section 2.4), not by `WarState`. 04 must define it with the fields above.
4. **Touch signer and payload.** 01's `touch` must forward an authority and a payload to one slot's
   item; 04's Raid item must accept spends only from `["war-signer"]` under `<WAR_ID>`.
5. **Fee collector.** Section 10.4 sets the DEX `fee_collector` to `["prize-vault"]` under
   `<WAR_ID>`; 03 should state that no DEX code change is needed and that the admin quote-holding
   path is no longer used.
6. **Season admin.** `WarConfig.admin` sets season weights and loot tables before each season. They
   are public and locked before the season starts, but it is still an admin power; DECISIONS should
   record whether the owner wants it timelocked.
