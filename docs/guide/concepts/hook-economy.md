# The hook economy

Hooks are assets. Whoever builds a good hook earns every time any token runs it, and the protocol earns alongside.

## Who earns what

| Party | Earns | From |
| --- | --- | --- |
| Item owner | Royalty | A share of what the item collects, every run, on every token that equips it |
| Template author | Author share | A share of item royalties, never on top of them |
| Protocol | Protocol fees | Its share of launch pool fees and hook cuts, plus market fees |
| Crankers | Bounties | Small, capped shares for running settle, siege, counter-strike and season steps |
| Holders | Holder rewards, loot, bounties | Kit rewards, treaty streams, raid points |

Traders never pay royalties on top: a royalty is always a share of a cut the item was already allowed to take.

## Access modes

| Mode | Who can equip the item |
| --- | --- |
| Open | Any token whose holders vote it in |
| Leased | A token that rents it for a term (built) |
| Gated | Tokens the owner allows (Planned: licences and allowlists, in the hook economy spec) |

## The market

Items trade like any other asset:

- **Listings and sales** in bridged SOL. The buyer pays the protocol fee, the template author's share and the seller in one transaction; no proceeds wait in a vault.
- **Price history** comes from sale events only.
- **Collections and sets** group items for discovery.
- **Royalty income travels with the item.** Claims are refused while an item is listed, so unclaimed royalties go to the buyer with the sale.

## Rentals

An owner leases an item to a token for a term and a share of its cuts. The item sits in escrow for the term and returns automatically. Rent is a share of the royalty, never charged on top. Reverting the slot when a lease ends is handled by the armory (in progress).

## Commissions

A community posts a bounty for a hook it wants. Builders submit items, holders vote one into their slot through the normal vote, and the bounty pays out once the item is actually equipped. The winner keeps the royalty.

## Forging and supply

Forging burns two items to mint one stronger item, so supply of items falls as people improve them. Loot adds items; forging removes them.

## Planned

The full hook economy spec (access licences, an order book for items, usage-based wear and on-chain skills) is being written. Anything on this page marked Planned is not built yet.
