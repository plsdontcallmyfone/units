# units spec 15: security review 3 fixes (secfix3)

The fixes for `~/ideas/hookwars/SECURITY-REVIEW-3.md` (review of main `385cc28`), on branch
`secfix3`, plus the app items of pass 4a (spec 14 section 7) the lead added to this lane. Every
new number is a parameter "to set" (00 section 6); the TEST values are the suites' and are no
proposal. Regression tests: `programs/tests/tests/secfix3.rs` (`sf3_*`), the `sf3_*` tests at the
end of `war_expansion.rs`, `app/apps/agents/src/guard.test.ts`, the Hook Lab service and CLI tests,
and the API, SDK and indexer tests named below.

## 1. Status by finding

| Finding | Status | Change | Test |
| --- | --- | --- | --- |
| H-1 approve through `spend` | fixed | `spend` refuses token program instructions other than `transfer`, `burn`, `create_holding` (`InstructionNotAllowed`); after the call every vault holding must keep its delegate, allowance (not above), owner and freeze state, and a holding the call opened must have no delegate (`VaultHoldingChanged`), whichever program the call reached; new operator instruction `revoke_vault` clears a delegate on a vault holding | `sf3_h1_spend_refuses_approve_and_close_and_the_operator_can_revoke` |
| M-1 drained maker freezes the book | fixed | the taker's payment to a maker goes through the escrow; `flush` sums what is owed per wallet and pays a wallet that does not exist only when the sum reaches the rent minimum, else the instruction's payer gets it (`Unpayable` log event) | `sf3_m1_a_drained_maker_no_longer_freezes_the_book` |
| M-2 renewal redirects the refund | fixed | a live licence renews only by its payer (`NotLicencePayer`) | `sf3_m2_a_renewal_cannot_redirect_the_revocation_refund` |
| M-3 boss share optional | fixed | `split_protocol_fees.boss_pool` is a required account at `BossPool::address(config.current_season)`; funded when a pool lives there, is effective and not sealed; a sealed or missing pool is skipped, not an error | `sf3_the_boss_share_is_not_the_crankers_choice_and_waits_for_the_timelock` |
| M-4 end_lease reverts another token | fixed (market side) | the revert's `slot_state` (third account) must be the armory's `["slot-state", lease.token_mint, lease.slot]`, which the armory derives from `equip.token_mint`, so the revert names the lease's own token | `sf3_m4_end_lease_reverts_only_the_leases_own_token` |
| M-5 end_lease without the revert | fixed | the remaining accounts are required: the revert accounts, or the lease's token mint alone, accepted only when that slot no longer holds the item (`RevertAccountsMissing`) | `sf3_m5_end_lease_cannot_leave_the_item_equipped` |
| M-6 coalition chest locked | fixed | `coalition_raze(args, max_amount)` sells at most `max_amount` (0 = no cap); `dissolve_coalition` runs after the term once nothing is captured, or after `ends_at + coalition_grace_secs` in any case, and may run again later to return what razes of the leftovers paid in | `sf3_coalitions_take_late_joiners_and_dissolve_after_the_grace_with_captured_tokens` |
| M-7 boss weighting by window volume | deferred | design in section 2.1 | |
| M-8 first creator fixes a book | fixed | `create_market` requires `tick_min_lamports <= tick <= tick_max_lamports` and `0 < min_size <= min_size_max` (`BadTerms`); new admin `propose_market_terms` / permissionless `apply_market_terms` after `admin_timelock_secs`, bounds checked again at apply | `sf3_m8_market_terms_are_bounded_and_settable_by_the_admin` |
| M-9 Hook Lab key next to the build | fixed | two stages: `hooklab build` in the sandbox with no key (writes the `.so`, its build facts, or the reason it failed), then `hooklab check --no-build --so ...` (or `--build-error`) outside it with the key; the suite only runs the bytecode in LiteSVM; each stage runs in its own process group, killed when it exits; `hooklab.json` must be a regular file the build did not change and the build outputs regular files | Hook Lab service: `builds in the sandbox with no key, then checks and signs outside it`, `a build that fails in the sandbox ends in a signed fail report`; CLI: `check_no_build_refuses_to_build_and_a_sandboxed_build_failure_is_a_signed_fail`, `the_two_stage_cli_builds_without_a_key_then_checks_and_signs` (ignored, builds) |
| M-10 runtime signs any prepare | fixed | `guard.ts`: per-route allowlist of top-level programs, token instructions signed by the agent or vault limited to `create_holding`, no swap, launch, market, bridge or book instruction signed by the agent key itself, refused before wrapping; the API URL must be https (http only on localhost) | `guard.test.ts` |
| M-11 settle drops in raw units | deferred | design in section 2.2 | |
| L-1 skill counters cheap | fixed | `BookParams.skill_min_fee_lamports` (fills count only when their fees reach it), `LicenceParams.skill_min_fee_lamports` and `MarketParams.skill_min_fee_lamports` (licences and sales count only when the protocol fee reaches it) | `sf3_l1_cheap_fills_do_not_count_for_skills`, `sf3_l1_cheap_licences_do_not_count_for_skills` |
| L-2 higher maker rate strands bids | fixed | a bid fill's maker fee is clamped to what the order's reserve holds above the quote of the rest of the order | `sf3_l2_a_higher_maker_rate_does_not_strand_resting_bids` |
| L-3 coalition id taken first | fixed | `coalition_min_term_secs`; new `join_coalition` for a war token whose Coalition item names the id, while the term runs and before any contribution | in the coalition `sf3` test |
| L-4 licence money to an escrow | fixed | `buy_license` refuses the listing and lease escrows as holder (`HolderIsEscrow`); the API prepare says so first | `sf3_l4_a_licence_never_pays_a_market_escrow`; API `licence buy refuses a market escrow as holder` |
| L-5 link replay | fixed | `Passport.link_nonce` (from `reserved[0..4]`) counts links and never decreases; the statement is `units agent link v2` and names it | `sf3_l5_a_link_statement_cannot_be_replayed_after_unlink` |
| L-6 directive memo does not bind constraints | deferred | needs a field in `units-memo`'s `DirectiveBody` (pass 4a's crate during this lane); design in section 2.3 | |
| L-7 `init_boss_pool` immediate | fixed | `BossPool.effective_at` = init time + `admin_timelock_secs`; the split funds a pool only from then | in the boss `sf3` test |
| L-8 spoofable client key; unsandboxed git fetch | fixed | API: `API_TRUSTED_PROXY_HOPS` (0 = the socket address), the entry the farthest trusted proxy appended, never a client-set left entry; Hook Lab with `HOOKLAB_TRUST_PROXY`: the last entry; the git fetch runs through the sandbox prefix and the checkout is held to the tarball's file and size limits | API `client key (review 3 L-8)` |
| L-9 runtime SSRF, RPC URL in logs | fixed | rules documents only over https from hosts whose every address is public; `UNITS_RPC_URL` (whole, its query, password and long path segments) is a redacted secret | `guard.test.ts` |
| L-10 indexer walk per profile | fixed | at most `SOCIAL_MAX_AUTHORS_PER_PASS` authors per pass (operator setting, default 200), the least recently walked first | indexer suite |
| I-1 crank records overstate | fixed | the agent `CRANK` record of the split, siege, counter-strike, raze and treaty share carries the bounty earned | |
| I-2 commission bounty to the first submitter | deferred | section 2.4 | |
| I-3 ForgeLevel badge per wallet | deferred | section 2.4 | |
| I-4 rivalry wins farmed | deferred | section 2.4 | |
| I-5 book and fee events not indexed | fixed | `LOG_EVENT_PROGRAMS` adds book, craft and market | SDK `book fills and protocol fees are read from the book's log events` |
| I-6 book flushed by place and cancel | deferred | section 2.4 | |

## 2. Designs left for the lead

### 2.1 M-7: season volume for the boss pool

Keep a season total per source in the boss pool itself: items' Raid mark into the boss token (the
only writer of the boss ledger) also CPIs war `note_boss_raid(season, source, volume)` signed by
items' `["war-caller"]`, which adds to `BossPool.sources[i].volume` (eviction-free: a full table
refuses new sources rather than pushing old ones out) and caps each source by its war chest's
`funded_total` for the season times `raid_volume_per_funded` (as M-B does for the score). The seal
then copies nothing from the ledger and is allowed only within `boss_seal_window_secs` after
`ends_at` (a parameter); after the window anyone may seal with what the pool holds. Items is pass
4a's program, so the CPI belongs to an items lane.

### 2.2 M-11: settle drops measured in value

`SETTLE_CRANK` drops should measure the quote side only: items passes the bridged SOL the settle
moved (the waterfall's quote leg) instead of `total_token`; for a token-side cut, price it through
the pool's TWAP (`hookwars_common::observations::window_read`, the same read the raze floor uses)
before `drop`. In craft, add `DropRule.per_recipient_cap` (a parameter) with a per-season
`DropTally` at `["drop-tally", season, source, recipient]` created by the caller's payer. The
measured value comes from items (pass 4a's), so the change is an items and craft lane.

### 2.3 L-6: directive constraints in the memo

Add `c` (hex of `sha256(borsh(DirectiveConstraints))`) to `units-memo`'s `DirectiveBody` and its
canonical order; `set_directive` recomputes the hash from its argument and refuses a mismatch
(`MemoMismatch`); the runtime and the feed show the constraints with the memo. A memo crate
change and a directive layout note; small, after the memo crate is free.

### 2.4 Info items

- I-2: pay the commission to the current holder of the submitted item (holding checked at
  `pay_commission`), or let a sale of the item move the submission; the spec calls the bounty a
  builder's, so this is the owner's call.
- I-3: a ForgeLevel badge once per item (a `["badge-claim", item, badge]` marker) instead of once
  per wallet.
- I-4: count a Rivalry win only when the rival raided back in the window (both ledgers), or cap
  wins per season per pair.
- I-6: a minimum resting time before cancel (`BOOK_MIN_REST_SECS`), or a cancel fee from the
  bounty; the cost today is escrow only.

## 3. Program and account changes the merger must know

- agents: new instruction `revoke_vault` (`RevokeVault`: operator, passport, policy, vault,
  holding, token accounts, event cpi); new errors `InstructionNotAllowed`, `VaultHoldingChanged`;
  `Passport.link_nonce: u32` from `reserved[0..4]` (`reserved` now 28 bytes, same size);
  `link_statement` takes the nonce (`units agent link v2`).
- book: `BookParams` gains `tick_min_lamports`, `tick_max_lamports`, `min_size_max`,
  `skill_min_fee_lamports` (layout change); new account `PendingMarketTerms` at
  `["book-pending-terms", market]`; new instructions `propose_market_terms`, `apply_market_terms`;
  new error `BadTerms`; new events `Unpayable` (log), `MarketTermsProposed`, `MarketTermsSet`.
- market: `MarketParams.skill_min_fee_lamports`, `LicenceParams.skill_min_fee_lamports` (layout
  changes); `BuyLicense` gains `access_policy` (the armory's `["access", item]`, after `offer`) and
  `offer` is `init_if_needed`; errors `NotLicencePayer`, `HolderIsEscrow`, `RevertAccountsMissing`;
  `end_lease` requires its remaining accounts.
- war: `WarParams` gains `coalition_min_term_secs`, `coalition_grace_secs`; `BossPool.effective_at`;
  `split_protocol_fees.boss_pool` required (`UncheckedAccount` at the running season's pool);
  `coalition_raze` takes `max_amount: u64`; new instruction `join_coalition` and event
  `CoalitionJoined`; Rust client `split_protocol_fees(.., current_season, ..)`,
  `split_protocol_fees_with_boss(.., boss_season: u32, ..)`, `coalition_raze_max`,
  `join_coalition`.
- Not touched: armory, items, token, units-memo.

## 4. Parameters to set (00 section 6)

`tick_min_lamports`, `tick_max_lamports`, `min_size_max`, `BookParams.skill_min_fee_lamports`,
`MarketParams.skill_min_fee_lamports`, `LicenceParams.skill_min_fee_lamports`,
`coalition_min_term_secs`, `coalition_grace_secs`. Operator settings (not protocol):
`API_TRUSTED_PROXY_HOPS`, `SOCIAL_MAX_AUTHORS_PER_PASS`.
