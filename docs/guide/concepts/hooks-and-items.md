# Hooks and items

## Templates

A **template** is audited hook code. All templates live in one program, so loot, forging and composites only ever produce new parameters, never new code. A template defines:

- its slot kind and the callbacks it runs on
- its parameter fields, each with a floor and a ceiling
- how each field changes when two items are forged
- its manifest: the most it may cut, whether it may refuse or burn, how many bytes of holder memory it needs

Templates are registered by the protocol. A template's program must be immutable or upgradeable only by the protocol's managed key, and its code hash is recorded. If the code changes, the template can no longer be equipped.

See every template in [Templates](../reference/templates.md).

## Items

An **item** is a template plus a set of parameters, minted as a supply-1 token. Whoever holds that token owns the item.

| Field | Meaning |
| --- | --- |
| Template | Which audited code it runs |
| Parameters | Its settings, each inside the template's floor and ceiling |
| Author | Who created it (fixed) |
| Owner | Whoever holds the item token |
| Royalty | The owner's share of what the item collects, fixed at creation and capped by governance |
| Level | Raised by forging |

Items come from three places: created by an author, dropped as loot, or forged from two items.

## Forging

`forge` burns two items of the same template and mints one. Each parameter moves by its template's rule (toward its ceiling, toward its floor, or kept), clamped to the ceiling. The new item is one level higher. Claim royalties before forging: unclaimed royalties on burned items are lost.

## Composites

A **composite** is one item that runs up to `MAX_MODULES` modules (build value 4) inside one slot:

- Modules run in order inside one call: no extra call depth.
- All cuts merge into one cut. Each module's share is recorded and settled to that module's own destination.
- Refusals combine with AND: any module refusing refuses the operation.
- One fee override per side. Each module gets its own slice of holder memory.
- One royalty, paid to the composite's owner.

The combined manifest is computed and checked against the slot's bounds like any other item.

## What an item can and cannot do

| Can | Cannot |
| --- | --- |
| Take a cut of a transfer or a swap, up to its slot's bound | Take more than the amount moved |
| Refuse a transfer | Use the trader's signature |
| Burn part of a swap (Pool slots that allow it) | Pay anyone out directly |
| Discount the launch's own creator and holder fees | Lower the protocol's fee |
| Read other tokens' time-weighted prices and the trade's route | Call another token when that token trades |
| Keep state in each holder's range | Read or write another slot's bytes |
