# Relations

Tokens never call each other live. When token A trades, token B is not touched. Relations work from state that is already on chain at trade time: time-weighted prices, the trade's route and treaties both sides equipped.

## Reading other markets

Every launch pool keeps a ring of price observations inside the pool account (cumulative price, volume and swap count). Items read a rival's **time-weighted price** over at least `MIN_TWAP_SECS`, never the spot price, so a one-block spike moves the reading very little. A failed read means "no signal", never a failed trade.

## Rivalry

**Spy** watches one rival. When the rival rises past a trigger over a window, sellers of your token pay more. **Rivalry** (template 45) names a rival for a fixed term. Its result feeds the season score and a badge. Nothing moves between the two war chests based on the result.

## Raids

A raid is a buy that arrives by selling a rival. Because the DEX fills the route, the item knows exactly where the buyer came from.

| Item | Effect |
| --- | --- |
| Raid | Buyers who sold a named rival pay less creator and holder fee, a share of the normal fee goes to your war chest, and the buyer earns raid points |
| Ally Pass | Holders of an allied token pay less on buys |
| Embargo | Buyers arriving from a named token pay extra |
| Mercenary | Any buyer arriving from another token earns raid points |

A raid only counts when the route starts on the rival's own launch pool, so a pool the attacker made cannot fake one.

## Treaties and tribute

A **Treaty** is a two-sided item: every buy of either token sends a share to the other token's holders. It takes effect only when **both** tokens have equipped it under their own equip rules. Either side leaves by unequipping, after its notice.

**Tribute** is one-sided: a token pays a share of its buys to a partner. Payments land in the partner's treaty inbox and stream to holders through the kit, so a buy just before a payout cannot capture it.

Treaties can carry a term and need a renewal vote to continue (Planned, see the expansion spec).

## Coalitions

A **Coalition** item (template 43) names the coalition a token belongs to and caps how much of its war chest it may contribute to joint sieges. Coalition war chests are Planned; the item exists.
