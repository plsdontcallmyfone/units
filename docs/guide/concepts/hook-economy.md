# The hook economy

Hooks are assets. Builders earn every time any token runs their work, and the protocol earns alongside. Traders never pay a hidden charge: every fee and royalty comes out of a cut the item was already allowed to take, or out of a payment someone signed.

> Built: royalties, settlement, the item market, rentals, commissions, every access mode, licences, the token-side protocol fee, levels, materials, recipes, wear and the order book. The numbers each of them uses are governance settings, set by the owner before mainnet.

The [Economy](/economy) page shows all of it live, per period: protocol revenue by source, where settled cuts went, builders and holders, the market, crafting and the book, crank bounties and war money, each summed from chain events.

## Access modes

The item's holder chooses who may equip it.

| Mode | Who may equip it | What the token pays | Ends when |
| --- | --- | --- | --- |
| Open | Any token whose slot fits | Nothing up front; the royalty comes out of the item's own cuts | The token unequips it |
| Gated | Tokens the holder approved | Nothing up front | Unequip, or the holder revokes after the slot's notice |
| Licensed | Tokens holding a live licence | The licence price, once per term | The licence expires or is revoked |
| Leased | The one token named by an active lease | The lease fee and a rent share | The lease ends |
| Exclusive | One token at a time, first to equip | Nothing up front | That token unequips it |

Exclusive combines with Gated and Licensed: an exclusive licence is the premium tier.

## The fee waterfall

Every hook run is settled by a permissionless `settle` step. The split always sums exactly to the amount collected; integer splits round down and the remainder goes to the last recipient.

```
token-side cut (b)                      pool-side cut (p)
  protocol fee                            protocol share taken by the DEX on the swap
  ── then, on what is left ──             ── then ──
  royalty                                 royalty
    author share   -> template author        author share
    rent           -> lessor, if leased     rent
    holder         -> item owner            holder
  bounty           -> whoever settled       bounty
  rest             -> the template's destination (burn, war chest, partner, collector)
```

The protocol is paid first and only once: on the pool side the DEX already takes its share before the cut reaches the vault; on the token side a protocol fee (the armory's `item_protocol_bps`) is taken first at settlement, in the token's own units.

## Per market action

| Action | Who pays | Split |
| --- | --- | --- |
| Item sale | Buyer | Protocol fee, author resale share, rest to seller |
| Licence | Licensee | Protocol fee, template author share, rest to item holder |
| Lease fee | Lessee | All to the lessor; the protocol takes no share of lease fees today |
| Order book fill | Taker | Taker fee to protocol; maker fee may be zero |
| Recipe | Crafter | Fee split between protocol and the season pool |

Item runs, licences, order book fills and recipes emit a `ProtocolFee` event with their source; the DEX share is in every `Swapped` event and the market fee in every `Sold` event. So the chain alone answers how much the protocol earned from each activity. Item-run fees are paid in the token's own units, so the [Economy](/economy) page lists them per token instead of adding them to SOL.

## The market

- Listings and sales in bridged SOL, settled in one transaction.
- Price history from sale events only.
- Collections and sets.
- Royalty income travels with the item: claims are refused while it is listed.
- Rentals: an item leased for a term and a share of its cuts, returned automatically.
- Commissions: a community posts a bounty for a hook; it pays once the winning item is equipped by vote.

## Levels

Levels are computed from counters only protocol programs can move, after their own effects, so every counted action paid its normal fees. A level is never stored as something you can buy.

| Skill | Counted from |
| --- | --- |
| Builder | Items authored, templates registered |
| Crafter | Items crafted, repairs |
| Trader | Order book fills |
| Diplomat | Treaties that held |

Levels unlock higher licence prices, reduced Hook Lab bonds, higher-tier recipes and new order book markets. Each threshold is set by governance.

## Materials, recipes and wear

| Part | How it works |
| --- | --- |
| Materials | Fungible units tokens that enter only through drops from activity that already pays fees (settlement, raids, quests, season results), capped per season |
| Recipes | Burn materials and pay a fee to craft an item, or to repair one |
| Wear | Items can carry charges. Each run that applies an effect uses one. At zero the item goes dormant: it answers nothing and never blocks a trade. Repair restores charges |

No emission, recipe or charge value ships until a faucet and sink simulation shows supply stays bounded and repair is worth it for items in real use.

## Order book

| Book | Trades | Orders |
| --- | --- | --- |
| Material book | A material against bridged SOL | Limit buys and sells |
| Class book | Items of one template and minimum level | Standing bids for any item matching the class; a holder sells a fitting item into one |

Every order is fully escrowed. No margin and no shorting. An order rests a minimum time before its owner can cancel it, so a book cannot be flushed by placing and cancelling.
