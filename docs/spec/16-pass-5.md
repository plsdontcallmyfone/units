# 16. Pass 5: protocol deferred items

Branch `p5`, from main `43b65d3`. Scope: the security review 3 items spec 15 section 2 left for
the lead, and the protocol items earlier passes deferred. Every fixed item has a regression test
that runs the attack and sees it fail. Owner values added here are params behind the existing
admin timelock; TEST values in `programs/tests` are not decisions.

## 1. Status

| Item | Status | Change | Test |
| --- | --- | --- | --- |
| L-6 directive memo binds the constraints | done | `units-memo` `DirectiveBody` gains `c` (hex `sha256(borsh(DirectiveConstraints))`), in the canonical order after `h`; a body without it is refused (`Shape`). `set_directive` recomputes the hash from its argument (`constraints_hash_hex`) and refuses a memo for the same passport and sequence whose `c` differs (`MemoMismatch`). App: `directiveConstraintsHashHex` (sdk), the agents runtime writes `c` and checks it against the Directive account | `directives.rs` `p5_l6_the_memo_commits_to_the_enforced_constraints`; `units-memo` `directive_body_round_trip`; agents `memo.test.ts` |
| I-3 ForgeLevel badge per wallet | done | a ForgeLevel claim creates `["badge-claim", badge id, item]` (third criterion account, writable, no data); a second claim with the same item fails `AlreadyClaimed`. Lamports sent to the marker first do not block the claim (top up and assign). sdk `badgeItemClaimAddress` | `social.rs` `a_forge_level_badge_opens_after_the_delay_and_cannot_move` (the item moves to a second wallet) |
| I-6 book flushed by place and cancel | done | `BookParams.min_rest_secs` (`BOOK_MIN_REST_SECS`, owner value); `Order.placed_at`; `cancel` refuses before `placed_at + min_rest_secs` (`RestTooShort`) and now takes `config`; `place` refuses an expiry inside the rest (an owner could otherwise crank its own expired order for the bounty) | `secfix3.rs` `p5_i6_an_order_rests_before_its_owner_can_cancel_it` |
| I-2 commission bounty to the first submitter | done | new `take_submission`: the item's current holder becomes the submission's submitter (and so the bounty's recipient) while the commission is open. Chosen over paying whoever holds the item at `pay_commission`, which would pay a listing or lease escrow; the bounty stays a builder's, the builder being whoever holds the item. sdk `marketTakeSubmission` | `market.rs` `p5_i2_the_items_holder_takes_over_its_submission` |
| I-4 rivalry wins farmed | done | a win counts only when the rival raided back (`theirs > 0`, both ledgers) and at most once per rival per season (`WarState.last_win_rival`, `last_win_season`, from `reserved`, now 28 bytes, same size) | `war_expansion.rs` `p5_i4_rivalry_wins_need_a_contest_and_count_once_per_rival_per_season` |
| M-11 settle drops in raw units | done (measure), cap deferred | `SETTLE_CRANK` drops are measured in bridged SOL settled (the quote side), never in raw units of the item's token. Token-side cuts drop nothing until they are priced; see 2.2 | `integ3.rs` `settle_takes_the_protocol_fee_and_author_share_wears_the_item_and_drops_to_its_holder` (a token-side settle drops 0) |
| M-7 boss weighting by window volume | done (cap), accumulation deferred | `claim_boss_share` counts a source's sealed volume only up to `raid_volume_per_funded` times what its chest received in that season (the M-B cap of the score); what the cap withholds stays in the pool. See 2.1 | `war_expansion.rs` `the_boss_pool_takes_its_share_and_pays_source_chests_by_volume` (an unfunded source claims 0) |
| `SEASON_FINISH` drop and `TREATIES_HELD` counter | deferred | see 2.3 | |
| R35 siege exemption marker | deferred, R10 stands | see 2.4 | |
| arsenal 2 request 5 (settle creating payout holdings) | deferred, unchanged reason | the token's `create_holding` needs the owner's account, which settle cannot name for every module (13) | |
| I-6 boss ledger (integration 2) | not needed | the boss pool reads the `RaidLedger`; pass 4b and this pass keep it | |
| `LaunchConfig` with slots | deferred | see 2.5 | |
| war suites off `war_items_stub` / `war_armory_stub` | deferred | see 2.6 | |
| Hook Lab pool callbacks | deferred | see 2.7 | |

## 2. Designs left

### 2.1 M-7 season accumulation

The cap closes the cheap attack (volume washed by a token whose chest holds nothing). What stays
is that the seal reads the boss ledger's two windows, so volume older than two windows is not
counted and a full table can push old sources out. The full fix is spec 15's: items' Raid mark into
the boss token also CPIs war `note_boss_raid(season, source, volume)` signed by items'
`["war-caller"]`, adding to an eviction-free per-season table in the `BossPool`, and the seal is
allowed only within `boss_seal_window_secs` after `ends_at`. It changes the items program's Raid
callback (a call level inside a swap) and needs its budget measured; it belongs to an items lane.

### 2.2 M-11 token-side value and a per-recipient cap

Token-side cuts should drop by their value: price the cut through the launch pool's TWAP
(`hookwars_common::observations::window_read`, as the raze floor does) and pass the result to
`drop`. That adds the pool and its observations to the craft suffix of `settle` (two accounts) and a
window read to its budget. A per-recipient season cap (`DropRule.per_recipient_cap`, a
`["drop-tally", season, source, recipient]` account) needs one more account in the shared drop
suffix every caller passes (items, war, book, market); it is a suffix layout change for all four.

### 2.3 Season finish and treaties held

Skill counters and drops credit wallets (R42, R43). The season winner is a token, and no war treaty
step is signed by or names a wallet, so there is no protocol-verified wallet to credit. Crediting the
launch's creator or the treaty's proposer is a product decision (it rewards a role, not an action
the protocol saw that wallet take); left for the owner.

### 2.4 R35

R35 replaces R10's refusal only "if it fits the measured budgets": the kit must treat a war chest
holding a foreign kit token as an excluded owner, which needs the rival chest's mint derivation in
the kit's accounts on every transfer of that token (D-7). Not measured in this pass; R10's refusal
(`SiegeTargetHasRewards`) stands.

### 2.5 `LaunchConfig` with slots

03 section 782 left it for time, not for a limit. It changes the `LaunchConfig` layout and adds a
slot comparison to `prepare_launch`; prepared launches keep inline slots meanwhile. Not a security
item; next launch lane.

### 2.6 War suites on the real programs

Pass 4a listed `war_items_stub` and `war_armory_stub` as replaceable by the real items and armory.
The war suites build many worlds through the stubs' direct writes (raid ranges, ledgers, equip
states); moving them is a test refactor across the war files, and `war_e2e.rs` already runs war on
the real programs. Left for a war test lane.

### 2.7 Hook Lab pool callbacks

The armory accepts pool manifests for external templates (pass 4a); the Hook Lab suite still
writes a manifest with zero pool fields (`tools/hooklab/src/register.rs`) and runs only token
callbacks. Adding pool callbacks means a launch pool in the lab's LiteSVM world and a property set
for buy and sell cuts; left for a Hook Lab lane.

## 3. Program and account changes the merger must know

- `units-memo`: `DirectiveBody.c`; `directive_message(passport, seq, rules_uri, h, c)`.
- agents: `set_directive` checks `c` (`MemoMismatch`); new `handlers::directive::constraints_hash_hex`.
- social: `claim_badge` for ForgeLevel takes a third criterion account
  `["badge-claim", badge id, item]` (writable); new error `AlreadyClaimed`; seed `ITEM_CLAIM`.
- book: `BookParams.min_rest_secs` (layout change); `Order.placed_at` (layout change of the
  market's order vectors); `Cancel` gains `config` after `owner`; new error `RestTooShort`.
- market: new instruction `take_submission` (`TakeSubmission`: holder, commission, submission,
  item, item mint, holder's holding, event cpi).
- war: `WarState.last_win_rival`, `last_win_season` from `reserved` (same size).
- items: `settle` passes the quote side settled to the `SETTLE_CRANK` drop.
