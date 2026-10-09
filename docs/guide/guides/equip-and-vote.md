# Equip and vote

Holders decide what runs in their token's Vote and Performance slots.

## The flow

```
propose ──► vote (lock tokens) ──► finalize ──► notice period ──► execute
```

| Step | Who | What |
| --- | --- | --- |
| Propose | A holder above the proposal threshold | Names an item for a slot. The proposer's tokens stay locked until the vote ends. One open proposal per slot. |
| Vote | Holders | Lock tokens in place for or against. Locked tokens cannot be sold or sent until the vote ends. |
| Finalize | Anyone | After the vote period: passes if quorum is met and more votes are for than against |
| Execute | Anyone | After the notice period: settles the old item, equips the new one |

Votes lock tokens where they are, so the token's own items never run on a vote, and a vote cannot be flash-borrowed.

## Before an item is accepted

The armory checks, at propose and again at execute:

- the item's template kind matches the slot kind
- the item's manifest fits inside the slot's bounds
- its holder-memory needs fit the slot's range
- its template code has not changed since registration

A proposal that no longer fits fails. Only a real fit failure marks a proposal stale; missing accounts are an error, so nobody can fail a passed vote by leaving accounts out.

## Performance slots

A Performance slot accepts votes like a Vote slot. It also carries a condition fixed at launch, for example "volume below its long average for three days". When the condition holds for its hold time, anyone can crank a revert to the launch item.

## Notice

The notice period gives holders time to react before a new item takes effect. It is fixed per slot at launch, within governance bounds.

## After an equip

Equipping a Pool or Relation item refreshes the launch pool's item registry in the same step, so trading never pauses.
