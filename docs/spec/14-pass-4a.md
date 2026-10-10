# units spec 14: protocol pass 4a

Status: built on branch `p4a` (from main `3d39bb6`), 2026-10-10. Server A, LiteSVM. Every value the
suites use is a TEST value, none is a decision (00 section 6). Scope: the armory, the items program,
the token program, `crates/units-memo`, test-only stubs and their tests; one contiguous change in
the launchpad (external pool cuts, section 3); `crates/hookwars-common` appended only (module
`access`).

## 1. Review 1 L-1: the admin queue

| Item | Status | Shape |
| --- | --- | --- |
| registration and retirement behind the timelock | done | `queue_admin(action_hash)` (admin) opens `QueuedAction` at `["queued", action_hash]`, ready after `admin_timelock_secs`; `cancel_admin` closes it. A gated instruction recomputes `sha256(instruction data \|\| bound keys)` from its own arguments, checks the entry (address, hash, admin, ready) and closes it to the admin (`NotQueued`, `Timelock`). Bound keys: the template program for `register_template` and `register_external_template`; none for the rest |
| the newer setters | done | `set_template_economy`, `set_item_protocol_bps` (now `AdminQueued` accounts: admin, config, queued), `register_preset` and the new `set_access_params` are gated the same way |
| harnesses and devnet | done | `programs/tests/src/armory.rs`: `queue_ix`, `queued_for`, `fill_queued`, `Hw::queue`, `Hw::ready`, `Hw::send_gated`, `Hw::send_gated_all`; `Hw::new` queues its nine registrations and waits once. `devnet_plan.rs` queues every registration and `set_access_params`, adds one wait step (`wait_secs` = `TEST_PARAMS.admin_timelock_secs`) and then applies; a queue step also has `skip_if` (its template) so a rerun after the registration skips it. `scripts/devnet/send-plan.mjs` reads `skip_if` and sleeps through a wait step only when a later step still has to be sent. `init-plan.json` and `params.test.json` regenerated on server A |
| Hook Lab | done | the report carries the `register_external_template` instruction (queue entry filled), its `queue_admin` instruction and the manifest ceiling (`register::armory_manifest`) |

Not gated (deviation): `propose_admin`/`accept_admin` and `propose_params`/`apply_params` keep their
own timelocks; `settle_submission` is admin-direct (it only moves the bond).

## 2. E-1 access modes and E-8 level gates

| Item | Status | Shape and deviations |
| --- | --- | --- |
| accounts | done | `AccessPolicy` `["access", item]` (mode, exclusive, `licence_terms`, `holder_at_set`, `updated_at`), `Approval` `["approval", item, token_mint]` (`revoke_after`, 0 = live), `LicenceTerms` (price, term, per, `max_live`). Deviation: the mode is mirrored in `Item.access_mode` and `Item.exclusive` (from reserved bytes), so an equip path that sees `Open` needs no further account and every existing caller stays valid; a new item takes `Template.default_access` |
| `set_access(mode, exclusive, licence_terms)` | done | the holder (holding of the item mint, amount 1), or the operator or agent key of the passport whose `["agent-vault", passport]` holds it (agent suffix `[<AGENTS_ID>, passport, directive]`: the live directive's `allowed_access_modes` and `max_licence_price`, `frozen` refuses: `DirectiveForbids`). Mode allowed by the template (`AccessNotAllowed`; `Leased` is the market's flow and is not set here); terms only for `Licensed`, within `[licence_min_secs, licence_max_secs]`, `max_live` 1 when exclusive (`BadLicenceTerms`); a `Lease` that exists keeps the mode (`ItemLeasedElsewhere`). A listed item is held by the market escrow, so the holder check refuses it (`NotItemHolder`, deviation from `ItemListed`) |
| `approve(token_mint)`, `revoke_approval` | done | Gated only (`WrongAccessMode`); revocation sets `revoke_after = now + the longest notice_secs of the token's slots holding the item` (their `SlotState` accounts in remaining accounts, `WrongAccount` when one is missing); immediate when the item is not equipped there |
| the equip row | done | `check_access` at `equip_launch`, `propose` and `execute` (never on a revert, R38): `Open` and `Exclusive` need nothing; `Gated` a live `Approval`, `Licensed` a live market `License`, `Leased` an `Active` `Lease` naming this token and slot, passed as the proof suffix `[<ARMORY_ID>, proof]` at the end of the remaining accounts, before the agent suffix (`AccessDenied`); exclusive needs `equipped_count == 0` (`ExclusiveInUse`). `fail_stale` does not treat a lapsed access as stale |
| `enforce_access(slot)` | done | anyone; Gated: `revoke_after` passed; Licensed: not live and `now >= lapse + slot notice` (lapse = `revoked_at` or `ends_at`); else `AccessStillValid`. The performance-revert path: settled vault required, back to the launch item, or empty when the launch item is this item; `AccessEnforced` and `EquipApplied` (`by` = 4) |
| licence reads | done | `hookwars_common::access::read_license` reads the market's `License` raw (owner, address, discriminator pinned in `pass4a.rs`), because the market depends on the armory |
| `buy_license` reading `AccessPolicy` | not done | the market is outside this lane; `buy_license` still reads `LicenceOffer` (the one-line account swap of 11 E-1 stays a request) |
| E-8 `set_access` tier | done | a licence priced above `licence_tier_1_lamports` while `licence_tier_1_level > 0` needs `Builder >= licence_tier_1_level` from social's `["skills"]` and `["profile", wallet]` (level suffix `[<SOCIAL_ID>, skills, profile]`, read raw, `LevelTooLow`) |
| E-8 template submission | done | `submit_template(template_program, code_hash, uri_hash)` posts `TemplateSubmission` at `["submission-tpl", program]` with `lab_bond_lamports`, less `lab_bond_discount_bps` at `Builder >= lab_bond_discount_level`; `settle_submission(approved, forfeit)` refunds (approval needs the registered `Template` with the same program and code hash) or forfeits to the admin |
| numbers | to set | `ArmoryConfig.access: AccessParams` (from reserved bytes): `licence_tier_1_lamports`, `licence_tier_1_level`, `lab_bond_lamports`, `lab_bond_discount_level`, `lab_bond_discount_bps`, `licence_min_secs`, `licence_max_secs`; all 0 until `set_access_params` (licences cannot be offered while `licence_max_secs` is 0). TEST values: `TEST_ACCESS` |

## 3. External templates (Hook Lab gaps 1 to 4)

| Gap | Status | Shape |
| --- | --- | --- |
| 1 registration | done | `register_external_template(args, manifest)`: an id no built-in template (and not the Composite) uses, a program that is not the items program, a non-zero code hash, the program check of 00 rule 3 (immutable, or upgradeable only by `HOOK_UPGRADE_AUTHORITIES`, pinned by deploy slot), a manifest whose kind is the template's, no War kind, no mint flag, no foreign reads, no forging. `Template.external` and `Template.ext_manifest` (from reserved bytes) |
| 2 authoring and equip | done | `create_item` checks fields against the template's floors and ceilings and gives the item the declared manifest; `check_fits` is unchanged; `apply_equip` puts the template's own program in the slot (`set_slot_item(program)`); `init_equip` writes the registry `[Item, equip vault when it cuts, EquipState, the launchpad's ["hook-authority", items], the items program]` (the vault is account 6 of a token callback, as the Hook Lab ABI says) and accepts any distinct targets up to `max_targets`. The token program needed no change: it calls the slot's program with the existing ABI |
| 3 settlement | done | `settle_equip` takes the vault balance as what an external template collected (its program cannot write `EquipState`), pays it by the same waterfall (protocol share, royalty with author and rent shares, bounty), the rest to the equip's first target or, with none, to the item's royalty holding |
| 4 pool side | done | the launchpad already calls the slot's program; after an external program answers a cut it calls the new items `record_pool_cut(mint, slot, item, cut, side)` signed by its `["hook-authority", items]`, with the last three accounts of the slot's slice; items refuses a non-external equip, another mint's state or another signer. `append_pool_items` accepts an item registry owned by the items program (Changed by Hookwars lines in `bordrless_launch`) |
| Hook Lab pinned test | done | `the_armory_refuses_an_external_template_id_today` replaced by `the_armory_registers_and_equips_the_starter_end_to_end` (queue, register, author, equip, cut, settle against the real armory) |

Test-only program `programs/ext_template` (`DwYAoZnHU2piMTpTLbPksSbmkisfpvR99LwKs7E2vgzm`, keypair
in the program-keys directory): the starter's transfer cut (`params[0]`) and a pool cut
(`params[1]`).

### 3.1 The pool registry refresh after an equip (found by pass 4b)

Fixed: `refresh_after_equip` (used by `execute`, `check_performance`, `revert_for_lease_end` and
`enforce_access`) called the launchpad's `refresh_pool_registry`, an `#[event_cpi]` instruction,
without its event authority and program, so the documented tail failed against the real launchpad
(`AccountNotEnoughKeys`). The tail is now `[launch program, launch, pool registry, the launchpad's
event authority, item registries...]`; the armory checks the event authority's address and passes
it and the launch program after the system program. `Hw::refresh_tail` returns the four accounts;
`war_e2e.rs` uses the documented tail (its workaround removed); `security.rs` pins the length 4.

## 4. Forge of composites (08 section 2.11)

Done: `forge` of two composites with the same module sequence (template ids, target slices, reads
in order; `ModulesMismatch`); each module combines by its template's forge rule (the pure
`hookwars_common::combine`) and is clamped; level = max + 1, capped at the lowest `max_level` of the
composite template and the module templates; every module template must allow forging. Remaining
accounts: both module lists (closed to the forger), the new list (created), each module's
`Template`. Provenance: the two burned items.

## 5. units-memo social kinds

Done: `kind::FOLLOW`, `UNFOLLOW`, `REACT`, `HIDE`, `kind::SOCIAL`, `kind::WITH_SOCIAL`, `REACTIONS`,
`Message::parse_kinds`, `follow_body`, `react_body`, `hide_body`, `social_message`,
`social_fields`, matching `app/packages/sdk/src/hookwars/memo.ts`. `Message::parse` still accepts
the core kinds only (as `parseMemo`'s default). Shared vectors: `crates/units-memo/vectors/social.json`
(the Rust test encodes and parses each byte for byte); an app test reading the same file is an app
pass item.

## 6. Stubs

| Stub | Verdict |
| --- | --- |
| `armory_stub` | kept: the token slot suites set arbitrary slot items and sign arbitrary `["slots", mint]` calls, which the real armory refuses by design |
| `items_stub` | kept: `protocol_vaults.rs` signs token payouts with arbitrary items seeds, including refused ones (R24) |
| `launch_stub` | kept: the armory and items suites create slot mints directly (not through `prepare_launch`) and forward pool callbacks without a pool; the real launchpad's `equip_prepared` needs a prepared launch |
| `pool_item_stub` | kept: the launchpad forwarding suites script refusals, wrong-side and over-bound answers, which no real item gives; `ext_template` now covers the real external path |
| `war_items_stub`, `war_armory_stub` (and the war suites' use of `items_stub`, `armory_stub`) | listed for the war lane (p4b's harness, not changed here): still used by `src/war.rs`, `loot.rs`, `budgets_war.rs`, `war_expansion.rs`; the real items program serves the Raid touch and the real armory `mint_loot`, and they duplicate `items_stub`/`armory_stub` at the same ids |

## 7. App changes for the app pass

- IDLs: armory gains `queue_admin`, `cancel_admin`, `set_access_params`, `set_access`, `approve`,
  `revoke_approval`, `enforce_access`, `submit_template`, `settle_submission`,
  `register_external_template`; `register_template`, `retire_template`, `set_template_economy`,
  `register_preset` take a trailing `queued` account; `set_item_protocol_bps` uses `AdminQueued`;
  new accounts `AccessPolicy`, `Approval`, `QueuedAction`, `TemplateSubmission`; `ArmoryConfig.access`,
  `Template.external`/`ext_manifest`, `Item.access_mode`/`exclusive`; events `AccessSet`,
  `Approved`, `ApprovalRevoked`, `AccessEnforced`, `AdminActionQueued`/`Cancelled`/`Applied`,
  `AccessParamsSet`, `TemplateSubmitted`, `SubmissionSettled`; `EquipApplied.by` 4. Items gains
  `record_pool_cut`.
- Prepares: `set_access` (with the agent and level suffixes), `approve`, `revoke_approval` (with
  the slot states), `enforce_access` (a crank, like the performance revert), and the access proof
  suffix on every equip path for a Gated, Licensed or Leased item.
- Item page: access mode, terms, approvals; the Hook Lab service shows the queue instruction.
- Execute, performance revert, lease-end revert and `enforce_access` on a slot launch's Pool or
  Relation slot: the refresh tail gains the launchpad's event authority as its 4th account.
- `memo.ts`: the comment saying `units-memo` refuses the social kinds is now wrong; add a test that
  reads `crates/units-memo/vectors/social.json`.

## 8. Tests

See the final report of the lane for counts. New suite `programs/tests/tests/pass4a.rs`.
