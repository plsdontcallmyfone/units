# units spec 14: protocol pass 4b (war side)

Branch `p4b` from main `3d39bb6`. Scope: `programs/hookwars_war`, `programs/bordrless_kit`, their
tests and new test files. Built and tested on server B (`/root/hw-p4b`, `CARGO_TARGET_DIR
/root/cache-p4b`, every cargo command under `/root/build.lock`). Every figure below is from a test
run on this branch; none is a proposal for production. Every new parameter is "to set".

## Test counts

Full workspace on server B after the last build of every program: 559 passed, 0 failed, 10 ignored
(main `3d39bb6`: 550 passed). New: `war_expansion.rs` 6, `war_e2e.rs` 2, `budgets_war.rs` 1. The
math vectors (`programs/tests/vectors/hookwars-math.json` and its app mirror) were re-rendered: the
two `split_protocol_fees` metas gained the `boss_pool` placeholder. `cargo clippy -p hookwars-war
-- -D warnings` stops in `hookwars-common` at a line this pass did not write
(`eco_cpi`, `s.len() >= CRAFT_HEAD + 1`, `int_plus_one`), as on main.

## Per item

| Item | Status | Where |
| --- | --- | --- |
| 10 section 17 I-9: boss pool and `claim_boss_share` (11.1) | done | `instructions/boss.rs`, `split_protocol_fees` optional `boss_pool`, tests `war_expansion.rs` |
| I-9: coalitions with a shared chest (8) | done | `instructions/coalition.rs`, tests `war_expansion.rs`, `budgets_war.rs` |
| I-9: rivalry budgets (11.3, R36) | done (badge as an event, see below) | `instructions/rivalry.rs`, ring-fence in `attack.rs`, `bounty.rs`, `coalition.rs` |
| I-11 / R35: siege exemption marker | deferred, R10 stands | reasons below |
| I-6: boss ledger | not needed by war; unchanged (items) | the boss pool reads the `RaidLedger` |
| 11 E-4 war part: drops | done for raid reveal (`RAID_REVEAL`) and quest claim (`QUEST_CLAIM`); season finish deferred | `common.rs` `split_economy` / `economy_effects`, `loot.rs`, `quests.rs` |
| 11 E-6 war part: social counters | done for `RAIDS` (reveal); `TREATIES_HELD` deferred | same |
| `budgets_war` re-measured | done | `budgets_war.rs::war_budgets`, new `war_expansion_budgets` |
| e2e L-D (re-equip a Pool item by vote on a slot launch, then swap; real launchpad, armory, items) | done, with a finding for pass 4a | `war_e2e.rs::l_d_a_pool_item_re_equipped_by_vote_trades_at_once` |
| review 2 M-B live self-raid loop through the real Raid callbacks | done | `war_e2e.rs::m_b_a_live_self_raid_loop_cannot_buy_the_season_score` |
| war-suite stand-ins replaced by the real items and armory | not done in the war suites; the new e2e file runs war on the real programs | below |

## Boss events (10 section 11.1)

- `BossPool` at `["boss", season le]`: `boss_mint`, `funded`, `paid`, `sealed`, `total_volume`,
  `to_share`, one `BossSource { mint, volume, claimed }` per raid-ledger entry (`RAID_TABLE_LEN`).
- `init_boss_pool(season, boss_mint)`: admin only (the protocol names the boss token), the running
  season or a later one.
- `split_protocol_fees` takes an optional `boss_pool` (account index 7, after `winner_season`;
  the program id when absent, so old callers are unchanged). When given it must be the running
  season's and unsealed; it receives `boss_share_bps` (new `WarParams` field, TEST 1,000) of the
  same amount the winner's share is taken from; the treasury gets the rest less the crank bounty.
  `valid()` requires `season_prize_share_bps + boss_share_bps <= 10,000`.
- `seal_boss_pool(season)`: permissionless from the season's `ends_at`. Each source's volume is both
  windows of its ledger entry (`volume + prev_volume`; the Boss item's window is the season length).
  The Boss item names only the ledger (no war config extra), so its ledger stays at season 0; seal
  accepts season 0 or the pool's season and refuses a ledger rolled to another season.
  Deviation from I-9's wording: `total_volume` is the sum of the recorded sources, not
  `outbound_volume_season`, so every funded lamport is claimable (a ninth live source is not
  recorded by the item, I-6, and so has no share either way).
- `claim_boss_share(season)` for a war state's token: `to_share * volume / total_volume`, once per
  source, into the chest's bridged-SOL holding (`wrap_sol`). Booked as the new
  `WarState.received_other`, never as season funding, so a boss share does not raise the M-B score
  cap. Rounding dust stays in the pool (the test checks `funded - paid <= 1` for two sources).
- The pool's lamports are debited directly; the CPI to the bridge names the pool as an extra
  writable account so the runtime sees the debit with the chest's credit (without it the claim
  failed `UnbalancedInstruction`).

## Coalitions (10 section 8)

- `Coalition` at `["coalition", id le]`, shared chest `["coalition-chest", id le]` (a system-owned
  PDA with a bridged-SOL holding). Layout constants: `COALITION_MIN_MEMBERS` 2,
  `COALITION_MAX_MEMBERS` 5 (5 measured to fit, table below), `COALITION_CAPTURED` 2.
- `form_coalition(id, term_secs)`: permissionless; each member passes `mint, Coalition item,
  template, war state`; each item (template 43, program items) is in one of the member's slots and
  names `id`; members distinct, all war tokens; `0 < term_secs <= season_secs`.
- `contribute(amount)`: permissionless crank; the member's item still equipped and naming the id;
  `amount <= max_contribution_bps` of the chest less a live rivalry's ring-fenced budget; once per
  `contribute_interval_secs` (new `WarParams` field, TEST 3,600) per member; only before the term
  ends. Moves bridged SOL chest to shared chest (unwrap, transfer, wrap). The member books
  `WarState.sent_coalition` (in `expected_balance`); the coalition keeps its own solvency totals.
- `coalition_siege(rival)`: the guards of `siege` with the coalition chest spending: a named member's
  War orders and its own raid ledger make it due (threshold, window), the coalition's own
  `siege_interval_secs`, the premium wait, the sandwich bound, R10 (no holder-reward rival), a member
  is never a target (`SelfSiege`), the rival's war state marked besieged by the shared chest (M-4).
  Deviation: a joint siege adds to no member's season counters (one siege, several members: no
  double counting).
- `coalition_raze(rival)`: during the term the raze rate limit; after the term no rate limit (still
  the pool-share bound and the M-3 floor, window `min_twap_secs`), because the rate limit alone
  never reaches zero and dissolving waits for it. Bounty `max_crank_bounty_bps` of the proceeds.
- `dissolve_coalition`: permissionless after the term once nothing captured is left; pro rata of
  contributions (equal shares when nobody contributed), rounding to the last member; each member
  books `received_other`. The client lists each key once: the program holds every listed account
  info on its 32 KiB heap and five `wrap_sol`s repeat most of theirs (with repeats a five-member
  dissolve ran out of heap).

## Rivalries (10 section 11.3, R36)

- `open_rivalry`: permissionless when the token equips a Rivalry item (template 45). The rival is the
  first target in the items program's `EquipState` of that slot, decoded by hand (items depends on
  war; the discriminator is pinned in `war_expansion.rs`). Budget = the item's `budget_bps` of the
  chest at opening; one rivalry at a time (`WarState.rivalry`).
- Ring-fence while live: a siege of the rival and a counter-strike while the rival's chest besieges
  this token may use the whole chest and draw the budget first (`rivalry.spent`); every other siege,
  counter-strike, bounty and coalition contribution sees the chest less the unspent budget
  (`siege_base`, `counter_base`). The lamports never move because of the rivalry.
- `settle_rivalry`: permissionless after the end, or at once when the item is no longer equipped (an
  early end, no win). The result compares this token's ledger entry for the rival with the rival's
  ledger entry for this token (both windows of each, so the last two raid windows before settling:
  the ledgers keep no more). A win adds one to `SeasonCounters.rivalry_wins`, weighted by the new
  `ScoreWeights.rivalry_wins`. Nothing moves between the chests (the test checks both balances).
- Badge: the result is the `RivalrySettled { won }` event for the token's page. A soulbound badge in
  social is per wallet; a token badge needs a social change (request to the social owner).

## R35 siege exemption marker (10 section 12.2, I-11): deferred

Not built and so not measured. The marker itself is cheap (`["chest-marker", chest]` at
`init_war`), but the kit must read it as a derived extra on every kit-token transfer to keep war
chests out of holder rewards: the kit registry grows from 2 to 3 extras, a `Locked` kit slot's
`extra_count` is fixed when the token is created (existing kit tokens could not take it), and every
client resolving kit-token transfers (launchpad slices, swap hops, the SDK) changes. Those live in
the launchpad, token and app passes. R10 stands: a siege refuses a holder-reward rival.

## E-4 and E-6 (war parts)

- Suffix layout at the end of a step's remaining accounts, after the agents record suffix is split
  off: `[..., craft drop suffix (10), social suffix (5)]`, each recognised by its first account
  (`hookwars_common::eco_cpi::{DROP_SUFFIX, SOCIAL_SUFFIX}`). Builders `client::drop_suffix`,
  `client::social_suffix`, `client::drop_common` (the token program and its event authority, before
  the suffix, for a step whose accounts lack them).
- `reveal`: `drop(RAID_REVEAL, measured 1)` to the raider and `record_wallet(RAIDS, 1)`; the drop
  suffix's recipient must be the roll's owner (`WrongAccount`). The armory's `mint_loot` now receives
  only the accounts before the suffixes (before: every remaining account, the agents suffix
  included).
- `claim_quest`: `drop(QUEST_CLAIM, 1)` to the claimant.
- Deferred: `SEASON_FINISH` (the winner is a token, not a wallet) and `TREATIES_HELD` (no war treaty
  step is signed by or names a wallet).
- Setup the test makes, which devnet needs too: `WAR_ID` in `CraftConfig.callers` and in
  `SkillTable.callers`, a material and a `["drop", source]` rule.

## e2e L-D and the finding for pass 4a

The test launches a slot token with a Size Tiers item in a Pool slot (vote rule), buys, proposes and
votes the slot to a second Size Tiers item, finalizes with the launch accounts (M-1), settles the old
item's pool cuts (the armory refuses to unequip a slot with unsettled vaults), executes, and then
buys and sells with the new item's accounts and no `refresh_pool_registry`. The registry names the
new item after `execute` (154,257 CU).

Finding: the armory's `refresh_after_equip` forwards `remaining[3..]` after the system program, but
the launchpad's `refresh_pool_registry` is an `#[event_cpi]` instruction that expects its event
authority and program there. The tail the armory documents (`[launch program, launch, pool
registry, item registries...]`, and `Hw::refresh_tail`) fails against the real launchpad
(`AccountNotEnoughKeys`; the test asserts it). It works when the client inserts the launchpad's
event authority and program before the item registries. Fix for pass 4a: append both inside
`refresh_after_equip` (or document the tail). The stand-in launchpad of the armory suites has no
event-cpi accounts, which is why the armory suites pass.

## M-B live loop

Raids through the real Raid callbacks on the real launchpad: three wash loops (raid, sell the raided
tokens back, buy the rival back) each leave `outbound_volume_season` at 0; two kept raids count.
After the season the score is 0 (the chest received nothing that season); after real funding `F`
the score is exactly `F * raid_volume_per_funded`, below the kept volume.

## Stand-ins (task 4)

Not replaced in `war.rs`, `siege.rs`, `counter_strike.rs`, `loot.rs`, `quests.rs`, `seasons.rs`,
`budgets_war.rs`: the war harness builds tokens by writing slot tables directly (no launch, no
armory equip), so the real items' `touch` (which loads the item, template and equip state through
the slot's registry extras) and the real armory's `mint_loot` (config, templates, item mints) have
nothing to read. Moving them means rebuilding `WarWorld` on real slot launches as `war_e2e.rs` does.
The war paths on the real programs are covered by `war_e2e.rs` (and `e2e.rs`). No stub became
unreferenced: `war_items_stub` (Raid `touch`) and `war_armory_stub` (`mint_loot`, `LootLog`) are
still loaded by `WarWorld`, and `war_expansion.rs` uses `war_armory_stub::LootLog`.

## Budgets (v0, one lookup table, 1.4M limit; `budgets_war.rs`)

| path | keys | v0 bytes | trace | height | CU |
| --- | --- | --- | --- | --- | --- |
| init_war | 16 | 281 | 10 | 3 | 87,707 |
| siege | 36 | 344 | 22 | 4 | 289,635 |
| raze | 34 | 339 | 19 | 4 | 254,649 |
| counter_strike | 33 | 346 | 24 | 4 | 319,342 |
| claim_bounty | 24 | 304 | 12 | 4 | 121,361 |
| roll | 16 | 290 | 9 | 3 | 66,892 |
| reveal (stub armory) | 13 | 275 | 4 | 2 | 34,104 |
| claim_quest (raid) | 15 | 286 | 10 | 3 | 83,009 |
| submit_candidate | 8 | 269 | 3 | 2 | 24,096 |
| split_protocol_fees | 20 | 305 | 11 | 4 | 82,722 |
| form_coalition (5) | 23 | 311 | 4 | 2 | 92,736 |
| contribute (first, creates the shared holding) | 23 | 325 | 17 | 4 | 191,179 |
| contribute | 23 | 325 | 14 | 4 | 158,011 to 166,999 |
| coalition_siege | 36 | 344 | 22 | 4 | 287,617 |
| coalition_raze (after the term) | 31 | 333 | 19 | 4 | 244,107 |
| dissolve_coalition (5) | 32 | 312 | 38 | 4 | 401,332 |
| open_rivalry | 11 | 270 | 3 | 2 | 46,417 |
| settle_rivalry | 9 | 266 | 3 | 2 | 27,821 |

Legacy transactions in `war_expansion.rs` (CU only): `split_protocol_fees` with the boss pool
32,502; `claim_boss_share` 91,150; `reveal` with the drop and `RAIDS` 145,802.

## Account and layout changes (for the app pass)

- `WarParams` += `boss_share_bps: u16`, `contribute_interval_secs: i64` (appended).
- `SeasonCounters` += `rivalry_wins: u32`; `ScoreWeights` += `rivalry_wins: u64`.
- `WarState` += `received_other: u64`, `sent_coalition: u64`, `rivalry: RivalryBudget` (before
  `reserved`); `expected_balance` = funded + razed + received_other - sent_coalition - siege -
  counter - bounties - cranks.
- New accounts `BossPool`, `Coalition`; new instructions `init_boss_pool`, `seal_boss_pool`,
  `claim_boss_share`, `form_coalition`, `contribute`, `coalition_siege`, `coalition_raze`,
  `dissolve_coalition`, `open_rivalry`, `settle_rivalry`; new events `BossPoolOpened`,
  `BossPoolFunded`, `BossPoolSealed`, `BossShareClaimed`, `CoalitionFormed`,
  `CoalitionContributed`, `CoalitionSiegeExecuted`, `CoalitionRazed`, `CoalitionDissolved`,
  `RivalryOpened`, `RivalrySettled`; new errors after `WrongAccount`.
- `split_protocol_fees` has the optional `boss_pool` at index 7 (pass the program id when absent).
- `scripts/devnet/params.test.json` is written by the ignored `devnet_plan` test and still shows the
  old `WarParams` string; regenerate it when the devnet plan is next run.
