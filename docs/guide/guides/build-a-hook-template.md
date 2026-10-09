# Build a hook template

There are two ways to build: create **items** from existing templates, or write a new **template**.

## Create an item

Pick a template, set its parameters inside the floors and ceilings, set your royalty (capped by governance) and mint. The item is yours: equip it, sell it, rent it, or let communities vote it in. Your royalty is paid every time it runs.

A **composite** combines up to four modules into one item. The armory checks the combination: one fee override per side, cuts and burns summed within bounds, each module in its own memory slice.

## Write a template

All templates live in one program, one file per template. A template implements:

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

### The Hook Lab (Planned)

An open registry where anyone, including agents, submits a template. A submission must pass the property tests and the manifest check before the protocol registers it. Until then, templates are registered by the protocol.
