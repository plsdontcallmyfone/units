# Hookwars spec 13: integration pass 3

Status: built on branch `integ3` (from main `7b48360`, after the economy merge), 2026-10-10. Server
A, LiteSVM. Every value the suites use is a TEST value, none is a decision (00 section 6).

## 1. Main green after the economy merge

| Item | Status |
| --- | --- |
| `crates/hookwars-common/src/lib.rs` did not compile: the economy merge left the `market` module open (`read_lease` lost its closing `}` and the module its `}`), "unclosed delimiter" | fixed (commit `423384f`). Main still has it until this branch merges |
| stale `app_vectors` | RESULT_VECTORS |
| the five agents bond tests the economy branch reported | RESULT_AGENTS |

## 2. Economy integration requests (11, end)

The economy accounts every caller passes are optional suffixes recognised by their first key, so
every existing caller and suite stays valid. Order at the end of the remaining accounts (client
side): `[..., rent suffix, craft suffix, fee suffix, social suffix, agents suffix]`; each program
splits them from the end. Hand-built CPIs in `hookwars_common::eco_cpi` (craft depends on armory
and social, social on armory and war, so neither may be a dependency of the armory or war): `disc`,
`split_tagged`, `init_wear`, `wear`, `drop`, `record_wallet`, `wear_dormant` (reads craft's `Wear`
bytes), the client helpers `init_wear_metas` and `social_metas`, and the craft `drop_source` names.
A unit test pins `disc("mint_crafted")` to the value 11 gives.

| Request | Status | Shape and deviations |
| --- | --- | --- |
| E-1 access modes (`AccessPolicy`, `Approval`, `set_access`, `approve`, `revoke_approval`, `enforce_access`, the `check_fits` row) | deferred | the largest request (new accounts, four instructions, checks in `equip_launch`, `propose`, `execute`, the licence read and the directive read); `execute` shares `EquipCtx` with the launchpad and the agents caller, so a new account there moves every caller (the same reason 12 gated I-3 only at `propose`). `Template.default_access` and `allowed_access` exist (E-7) so the layout does not move again |
| E-2 protocol fee in `settle_equip` (R37) | done | `ArmoryConfig.item_protocol_bps` (from its reserved bytes, so `ArmoryParams` and `init` are unchanged) set by the admin with `set_item_protocol_bps` (bounded by `max_royalty_bps`). Token side only: `protocol = floor(x * bps / 10_000)` first, then royalty and bounty as before. Fee suffix `[Template, admin's token holding, author's token holding, author's quote holding]`, required once the bps is above 0. Deviation: paid to the armory admin's holding, not an items config `treasury` (items has no config); no `protocol_fees_total` (the `ProtocolFee` events carry it); `reference` is zero (settle has no reference argument). A missing admin holding leaves the share in the royalty holding |
| E-3 items read craft wear | done | the armory's `create_item`, `create_composite`, `mint_loot` and `mint_crafted` open the `Wear` (`init_wear`, `max_charges = Template.charges_on_create`) when it is above 0, signed by the armory's `["craft-caller"]`; the init-wear suffix is then required (`WearAccountsMissing`) and `Item.has_wear` is set. `init_equip` takes a new last argument `wear: bool` (the armory passes `Item.has_wear`) and the registry ends with the `Wear`. The engine reads it after the module extras; while dormant every callback answers the default (no cut, no data, the pool's default answer), never a refusal (R38). `settle_equip` calls `wear(runs since the last settle)`: new `EquipState.runs_at_settle` (from reserved bytes); an item that wears and has run needs the craft suffix. A template's charges apply to items made after they are set |
| E-4 drops | done for `settle_equip`; deferred for war | settle: craft suffix `[craft program, craft config, craft event authority, items' ["craft-caller"], Wear, drop rule, material, material mint, craft minter, recipient, recipient's material holding, recipient's holding of the item mint]` (12 accounts); `SETTLE_CRANK`, measured = the token-side amount settled; the drop is made only when the last account shows the recipient holds the item (deviation: the request named no proof account). War `reveal` (`RAID_REVEAL`), season finish and quest claim: deferred, the war suites run stand-ins of the armory and items and would need craft loaded; war budgets in 07 were measured without the 10 more accounts |
| E-5 armory `mint_crafted` | done | accounts as `MintLoot` with `craft_signer` (checked against `["craft-signer"]` under `<CRAFT_ID>`, `NotCraftSigner`) and `owner` = the crafter; `source = CRAFTED` (3); event `ItemCrafted`. Deviation: fields are not drawn: each is the midpoint of the recipe's range (`min == max` gives that value); the loot randomness adapter is war's and is not reachable from a CPI made inside craft. `<CRAFT_ID>` is `39LXQBGqZtg591jkGnZi9BELQ9hp1ZngbAxu6K1cC29Y` (the id in 11's request is not the declared one) |
| E-6 counters | done for armory and market; deferred for war | armory `create_item`, `create_composite` (`ITEMS_AUTHORED`, the author), `register_template` (`TEMPLATES_REGISTERED`, the admin); market `buy` (`ITEMS_SOLD`, the seller). Social suffix `[<SOCIAL_ID>, skills, profile (mut), social event authority, the caller's ["social-caller"]]`; a wallet without a profile is a no-op. War `TREATIES_HELD` and `RAIDS`: deferred with E-4's war part |
| E-7 template fields | done | `Template.author_bps`, `default_access`, `allowed_access`, `charges_on_create` (reserved 32 to 24), set by the admin with `set_template_economy` (admin-direct like registration; see L-1). Licences pay `Template.author_bps` when it is above 0, else `LicenceConfig.author_bps` (deviation: a fallback for templates set before) |
| E-8 level gates | deferred | both gates sit on instructions not built (`set_access` is E-1, `submit_template` is the Hook Lab lane) |
| E-9 overview | done | 00 section 4.3 seeds (licence, memo, craft, book, the craft and social callers, the craft output signer) and section 6 economy parameters |
| callers | done in the harness and the devnet plan | craft `callers` = armory and items, `output_program` = the armory; social `callers` add the armory (`Ew::wired`, `devnet_plan.rs`) |

## 3. Leftovers from 12

| Item | Status |
| --- | --- |
| spec 10 I-2 author share at settle (R34) | done with E-2: `bps(royalty, Template.author_bps)` on both sides, before the rent, to the registrant's holdings (`AuthorSharePaid`). Deviation: only when the fee suffix is passed; it is required only once `ITEM_PROTOCOL_BPS` is above 0, so until then a cranker may leave the author's share in the royalty holding |
| I-6 boss ledger, I-8 to I-11 (coalitions with a shared chest, boss pool and `claim_boss_share`, rivalry budgets, seasonal meta R33, siege exemption marker R35) | deferred: each is a war state change with its own budget measurement; not reached in this pass |
| arsenal 2 request 5 (settle creating payout holdings) | deferred, unchanged reason: the token's `create_holding` needs the owner's account, which settle cannot name for every module |
| review 1 L-1 (registration and retirement behind the timelock) | deferred: unchanged reason (every harness and the devnet plan). The two new setters (`set_template_economy`, `set_item_protocol_bps`) are admin-direct and belong in the same change |
| e2e L-D test, live self-raid loop (review 2 M-B) | deferred |
| duplicate stubs | deferred (12 section 8) |
| 5-account record suffix across callers | confirmed: every caller splits it with `agents_record::split`, which checks the first two keys and takes five; the new economy suffixes sit before it |

## 4. Wave F (08)

| Item | Status |
| --- | --- |
| `fuse(targets: Vec<(u8, u8)>, royalty_bps)` | done: `CreateComposite` accounts; remaining = the components' templates, then `(item, item mint, holder's holding)` per component, then the usual suffixes. Each component must be held by the signer (`NotItemHolder`), not equipped (`ItemEquipped`), not a composite; it is burned and listed in `CompositeItem.provenance`; the modules then pass the same 2.10 validation as `create_composite`. Deviation: unclaimed royalties of a component cannot be proven empty (its royalty holdings are one per token it was ever equipped on); the holder claims first |
| forge of composites | deferred |
| presets: `register_preset(id, template_ids, name)`, `mint_composite(preset_id, modules, royalty_bps)` | done: `Preset` at `["preset", id]` (the module templates in order, 2 to `MAX_MODULES`, no Composite); `mint_composite` takes `CreateComposite` accounts with the preset first in the remaining accounts, refuses modules out of the preset's order (`InvalidSchema`) and otherwise runs `create_composite` (2.10 at each mint). Deviations: admin-direct (L-1); 2.10 runs at mint, not at registration (it needs params, which each mint chooses); the ten presets of 08 section 4.8 are not registered by the devnet plan (several of their templates need war state); `mint_loot` of a preset not built |

## 5. Tests

`tests/integ3.rs` (new, `Ew::wired`): RESULT_TESTS

## 6. For the app lane

- New instructions: armory `set_template_economy`, `set_item_protocol_bps`, `mint_crafted`,
  `fuse`. Items `init_equip` gains `wear: bool` (armory CPI only).
- New fields: `ArmoryConfig.item_protocol_bps`; `Template.author_bps`, `default_access`,
  `allowed_access`, `charges_on_create`; `Item.has_wear`; `EquipState.runs_at_settle`.
- New events: `TemplateEconomySet`, `ItemProtocolBpsSet`, `ItemCrafted` (armory); `ProtocolFee`,
  `AuthorSharePaid` (items). New armory errors `WearAccountsMissing`, `NotCraftSigner`,
  `NotItemHolder`.
- Suffixes: init-wear on item creation when the template wears; social on `create_item`,
  `create_composite`, `register_template`, market `buy`; fee and craft suffixes on `settle_equip`.

## 7. Result

RESULT_TOTAL
