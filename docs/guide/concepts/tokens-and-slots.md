# Tokens and slots

units tokens live on their own token program, built for hooks. A hook does not only approve or block a transfer: it can take part of the amount, burn part of a swap, set a fee and keep state for every holder.

## The slot table

Each token is created with a **slot table** of up to four slots. Every slot is fixed at launch:

| Field | Meaning |
| --- | --- |
| Kind | What the slot may hold: Fee, Reward, Defense, Relation, Pool, War or Locked |
| Bounds | The most any item in it may do: maximum cut, may it refuse, may it burn |
| Equip rule | Who may change the item: Locked, Vote or Performance |
| Notice | The delay between a change being decided and taking effect |
| Data range | Which bytes of each holder's memory the slot may use |

Bounds never change after launch. An item whose limits exceed the slot's bounds can never be equipped there.

## Slot kinds

| Kind | Runs on | Typical items |
| --- | --- | --- |
| Fee | Token transfers | Half-Life, Transfer Fee, Sell Ladder |
| Reward | Token transfers | Streak, Rank Badge, Guild Tag |
| Defense | Token transfers | Wall, Cooldown, Max Transaction |
| Relation | Transfers, and launch pool swaps when the template has a pool half | Treaty, Tribute, Rivalry |
| Pool | Launch pool swaps | Raid, Size Tiers, Sell Burn |
| War | Nothing: its parameters configure the war chest | War orders |
| Locked | As the launch rules kit does | The kit (holder rewards, max wallet, locks) |

## Equip rules

- **Locked**: the launch item stays for good.
- **Vote**: holders lock tokens to vote; a passing item is equipped after the notice period.
- **Performance**: accepts votes, and also reverts to the launch item when an on-chain condition (volume, time-weighted price, swap count) holds for long enough.

## Per-holder memory

Every holding carries 64 bytes of hook data. Each slot gets its own range, and an item can read and write only its range. The first byte of every range is an epoch byte the token program changes on every equip, so a new item never inherits the previous item's data. With the kit installed, the kit uses bytes 0 to 31 and items share the rest.

## Limits

A token runs at most `MAX_SLOTS` slots, and at most `MAX_CUTTING_SLOTS` of them may take a cut on one transfer. The build values are 4 slots (the kit plus 3 items) and 3 cutting slots (2 when the locked slot also cuts), set from the measurements in [Limits](../reference/limits.md).
