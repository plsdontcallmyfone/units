# Build a hook template

There are two ways to build: create **items** from existing templates, or write a new **template**.

## Create an item

Pick a template, set its parameters inside the floors and ceilings, set your royalty (capped by governance) and mint. The item is yours: equip it, sell it, rent it, or let communities vote it in. Your royalty is paid every time it runs.

A **composite** combines up to four modules into one item. The armory checks the combination: one fee override per side, cuts and burns summed within bounds, each module in its own memory slice.

## Write a template

The protocol's own templates live in one program, one file per template. An **external template** is its own program, registered after the Hook Lab checks it (below). Either way a template implements:

| Part | What it is |
| --- | --- |
| Shape | Slot kind, parameter fields, floor and ceiling per field, forge rule per field |
| Manifest | The most it may cut per side, may it refuse, may it burn, memory bytes, pool flags |
| Callbacks | Token side (transfer, mint, burn, touch) and pool side (before and after swap) |
| Validation | Extra parameter rules beyond floor and ceiling |
| Site sentence | One sentence with placeholders, shown on the item page |

### Rules every template follows

- Hooks never pay out. Money leaves only through settle, claims and payouts from program-owned vaults.
- One cut per transfer, into the equip vault.
- A pool callback always answers, even when it does nothing.
- Read other markets with time-weighted prices over at least `MIN_TWAP_SECS`. A failed read means no effect.
- Read and write only your own memory range.
- Make no calls to other programs from a callback.

### Tests a template needs

1. **Behaviour**: every branch, with exact amounts.
2. **Abuse**: the named attack fails or costs more than it earns.
3. **Forge**: combining never passes a ceiling.
4. **Invariants**: supply equals holdings, cuts add up, vaults match their records, after every step.

### The Hook Lab

Anyone, including an agent, can bring a template as its own program. The path:

1. **Check it locally.** `hooklab check <crate>` builds the crate with the pinned toolchain and runs the property suite (manifest bounds, answers on every callback, no payouts, memory ranges, invariants) in a local validator. Start from `examples/hook-template`.
2. **Submit it to the lab service.** Send the crate as a tarball, or a public GitHub repository at a full commit. The service runs it in two stages:
   - **Build**, inside a sandbox with no network and no signing key: the submitter's build scripts run here and nothing else does. Cargo reads only the lab's primed crate cache.
   - **Check**, outside the sandbox: the suite runs on the built program, and the lab signs the report. The report records what was built: the source tree hash, the `Cargo.lock` hash, the toolchain versions and the program's code hash.
3. **Deploy** the program with no upgrade authority. The armory refuses an upgradeable template.
4. **Post the bond** with `submit_template` (program, code hash, report hash). A high enough Builder level pays a smaller bond.
5. **The admin decides.** Approval returns the bond, and the admin registers the program, which waits out the admin timelock like every admin change. A rejection returns the bond or forfeits it.

Once registered, items of the template are created, equipped and paid out like any other: the slot calls the template's own program, and its equip vault settles by the same split.

**Reproducing a build.** Anyone can rebuild the same source with the toolchain the report names and compare the program's executable hash with the report's code hash. The lab does not yet produce container-verified builds; the recorded hashes are what make a rebuild checkable.
