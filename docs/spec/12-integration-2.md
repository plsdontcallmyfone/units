# units spec 12: integration pass 2 notes

Status: branch `integ2`, 2026-10-09 and 2026-10-10. What this pass changed in programs owned by
earlier lanes, how each integration request was answered, and where the result differs from the
request. Requests come from 09 section 21 (agents), 10 section 17 (market and social), the end of 08
(arsenal 2), the security reviews and the app worker (app/INTEGRATION.md section 4).

## 1. Fixed failures

- **Agents suites (5 tests).** Security fix M-2 (`proposal_min_bps`, TEST 100) made every treaty
  proposal by an agent key that held no tokens fail with `BelowProposalThreshold`; `bond_refusals`
  then read a proposal that was never made. The suites now mint each proposer 100,000 units of the
  token (`treaty_world(.., proposers)`) and the budgets suite mints its key 1,000,000. No program
  change.
- **The third ignored test** is `tests/devnet_plan.rs` (`#[ignore]` on purpose: it writes
  `scripts/devnet/init-plan.json`, run it explicitly per docs/DEVNET.md), with the two Studio
  fixtures.

## 2. Agents (09 section 21)

| # | Status | Where |
| --- | --- | --- |
| 1 badge equip by the agents caller (R28) | done | armory `cpi::is_badge`, `cpi::agents_armory_caller`, `process_equip_launch` |
| 2 Soulbound item for the admin; 42 refused off a badge | done (at `propose`, through `check_proposed`) | armory `create_item`, `check_proposed` (`NotBadge`) |
| 3 armory record calls | done | `create_item`, `create_composite`, `forge`, `claim_royalty`, `execute`, `equip_launch` |
| 4 war record calls and the inbox wrap | done except `finalize_season` | war `siege`, `counter_strike`, `raze`, `share_treaty_inflow`, `split_protocol_fees`, `submit_candidate`, `claim_bounty`, `reveal` |
| 5 items record call | done | `settle_equip` (`CRANK`, value = quote settled) |
| 6 proposal close bond guard | done | armory `close_proposal` (`BondStillPosted`) |

Deviations:

- **Record suffix shape.** The optional accounts are five trailing remaining accounts in this order:
  `[<AGENTS_ID>, ["agents-caller"] under the calling program, agents event authority, passport (mut),
  actor]` (`hookwars_common::agents_record`). The request named three; the event authority and the
  actor must be in the transaction anyway, so they are part of the suffix. The suffix is recognised
  by its first two keys; without it every instruction behaves as before. The discriminator is the
  constant `RECORD_DISCRIMINATOR`, checked against `hookwars_agents::instruction::Record` in
  `tests/integ2.rs`.
- **`finalize_season` records nothing:** it has no signer and no actor to attribute.
- **`close_proposal` takes two more accounts:** `bond_mark` (address checked,
  `["bond-mark", proposal]` under `<AGENTS_ID>`) and `bond` (optional; required when the mark
  exists). App and SDK must pass them.

## 3. Market and social (10 section 17)

| Request | Status |
| --- | --- |
| I-1 token protocol sources, social spends | done: `MARKET_ID`, `SOCIAL_ID` in `PROTOCOL_SOURCE_PROGRAMS`; `SpendToken` uses `transfer_from_protocol` with the guild treasury seeds |
| I-2 badges as Soulbound slot mints | not done (below) |
| I-3 `revert_for_lease_end` and the equip gate | done, gate at `propose` only (below) |
| I-4 claim refusal while listed | done (`ItemListed` when the claimant is the market escrow) |
| I-5 author and claim counters | done: `AuthorCounter`, `ClaimCounter`, `init_counters`; social criteria `ItemsAuthored`, `RoyaltiesClaimed` |
| I-6 boss ledger | not done (optional in the request; the raid ledger table is unchanged) |
| I-7 lessor rent at settlement | done |
| I-8 template author share | not done: no `template_author_bps` is stored on templates yet (spec 11 owns the split) |
| I-9 war coalitions, boss pool, rivalry budgets | not done |
| I-10 seasonal meta, lineage, Hook Lab, `expire_treaty` | not done |
| I-11 kit chest markers | not done |
| I-12 app IDLs and decoders | app lane |

Notes:

- **I-3 gate only at `propose`.** `propose` takes `["lease", item]` under `<MARKET_ID>` as its first
  remaining account when it names an item. When that account is a `Lease`, it must be `Active` and
  name this token and slot, else `ItemLeasedElsewhere`. `execute` is not gated: its account list
  (`EquipCtx`) is shared with `equip_launch`, the launchpad and the agents caller, and one more
  account there shifts every caller's layout. A lease cannot be revoked early, so a proposal made
  under an active lease stays valid until the lease ends, when `end_lease` reverts the slot.
- **`revert_for_lease_end(slot, item)`** (armory, `#[event_cpi]`): accounts `market_caller` (signer,
  `["market-caller"]` under `<MARKET_ID>`), `config`, `slot_state`, then `EquipCtx` (old item the
  leased one, new item the slot's launch item or none) and the L-D refresh tail. It does nothing
  when the slot no longer holds the item or the item is the launch item; otherwise it reverts to the
  launch item and emits `EquipApplied` with `by = 3` (`LEASE_END`). The slot must be settled first
  (`close_equip` refuses an unsettled vault), like every other equip change.
- **Market `end_lease` changed** (economy lane: possible merge conflict): with remaining accounts it
  CPIs `revert_for_lease_end`, the first remaining account `["market-caller"]`, the rest the
  armory's accounts in order. Without them it behaves as before.
- **I-7 shape.** `settle_equip` takes an optional suffix before the agent suffix:
  `[<MARKET_ID>, ["lease", item], lessor's token holding, lessor's quote holding]`. With an `Active`
  lease naming this token and slot, `floor(royalty * rent_bps / 10_000)` on each side goes to the
  lessor and the rest to the royalty holding (R32). A missing lessor holding leaves its share in the
  royalty holding: the item returns to the lessor at the lease's end, so nothing is lost; for the
  same reason a cranker that omits the suffix only delays the lessor. New event `LeaseRentPaid`;
  `EquipSettled.royalty_token` and `royalty_quote` now report the holder's part after rent.
- **I-5 shape.** Each counter is an optional trailing account: `create_item` (last remaining account,
  before the agent suffix), `create_composite` (after the module templates), `claim_royalty` (after
  the token extras). Passing the counter is in the wallet's own interest (it feeds badges), so it is
  optional. `claim_royalty` adds only bridged-SOL lamports. `init_counters(wallet)` is
  permissionless and idempotent. `claim_royalty` also stopped passing the suffixes to the token as
  extras (a bug found on the way).
- **I-2 not done.** Social badges stay plain mints frozen in their holdings, which already cannot
  move or burn. A Soulbound slot mint needs two changes this pass did not make: Soulbound binds only
  on mints whose freeze authority is `AGENTS_SIGNER` (social's minter would need adding to
  `soulbound::is_badge`), and an equip path for social (an armory caller like the agents one).

## 4. Arsenal 2 requests (end of 08)

| # | Status |
| --- | --- |
| 1 registries carry `extra_sources` | done (`equip::registry_list`) |
| 2 module conflicts | done inside one composite: Shield with Patience, Mercenary with Raid, two Loyalty Pots, two First Bloods (`ModuleConflict`). Across slots of one mint not done: the armory sees only the proposed item's accounts at `propose` |
| 3 war accepts a Mercenary slot | no change needed: war's `raid_slot` checks the slot's shape (items program, `ANSWERS_TOUCH`, `RAID_RANGE_LEN + 1` bytes), which a Mercenary slot has. Social `RaidPoints` now reads a Mercenary range too. Not exercised end to end (the war suites run a stand-in items program) |
| 4 silent items | covered by the H-A fix (integ) |
| 5 settle creates pot and Referral holdings | not done: `settle_equip` has no account for the holding's owner, which the token's `create_holding` needs. A missing holding is skipped and stays owed (L-C); anyone creates it with the token's `create_holding` and settles again (request 6 asks the launchpad and app to do this at equip) |
| 6 launchpad and app | app lane |
| 7 Loyalty Pot slot moves | done: `reslot_loyalty(slot)`, permissionless, moves the pot when its slot no longer holds a Loyalty Pot and `slot` does (also repairs a wrong slot given to `init_loyalty`) |

## 5. Security cross-branch

- **Kit payees (review 1 H-2 carried to items).** On a kit token (kit hook or a Locked kit slot) a
  token-side payout goes only to a key on the curve or a vault derivable from the mint (war chest,
  Loyalty pot, Referral owner, treaty inbox). `init_equip` refuses other targets (`BadTargets`);
  `settle_equip` leaves such a cut owed instead of paying it (only an equip made before this check
  can reach it). Today only Transfer Fee pays a target on the token side.
- **Review 1 L-1 (template registration and retirement behind the admin timelock): not done.** The
  design: `register_template` writes `Pending`, a permissionless `activate_template` after
  `admin_timelock_secs`; `retire_template` schedules, a permissionless apply retires. It changes
  every harness that registers templates (each registration then waits the TEST timelock, which
  moves the clock under worlds that hold timed state) and the devnet init plan, so it needs its own
  lane with the app.
- The e2e L-D test (re-equip a Pool item by vote on a slot launch, then swap) and the live
  self-raid loop test (review 2 M-B) were not written in this pass.

## 6. App worker request (app/INTEGRATION.md section 4)

`hookwars_items` had three `#[error_code]` enums (`ItemsError`, `SoulboundError` at offset 7000,
`ArsenalError` at offset 7100), which stops the IDL builder. They are one `ItemsError` now; Anchor
1.2 accepts explicit discriminants, so `SoulboundTransfer = 1000` (code 7000) and
`GuestListClosed = 1100` (7100) onward keep every number. `ArsenalError` and `SoulboundError`
remain as aliases of `ItemsError`. `tests/integ2.rs::merged_items_error_codes_keep_their_numbers`
pins the codes.

## 7. For the app lane

- `propose`: append `["lease", item]` under `<MARKET_ID>` when an item is named.
- `close_proposal`: `bond_mark`, `bond`.
- Optional record suffix (section 2), lease suffix on `settle_equip`, counter accounts (section 3).
- New instructions: armory `revert_for_lease_end`, `init_counters`; items `reslot_loyalty`; market
  `end_lease` remaining accounts. New accounts `AuthorCounter`, `ClaimCounter`; new event
  `LeaseRentPaid`; new social criteria `ItemsAuthored`, `RoyaltiesClaimed`; new armory errors
  `NotBadge`, `BondStillPosted`, `ItemListed`, `ItemLeasedElsewhere`, `NotMarketCaller`.

## 8. Not consolidated

The duplicate test stand-ins at shared ids (`items_stub`, `war_items_stub`, `pool_item_stub`;
`armory_stub`, `war_armory_stub`; `launch_stub`) were left as they are: each suite that loads one
relies on its fixed answers, and replacing them with the real programs changes those suites'
setups. Not done in this pass.
