# Trade and raid

## Trading

Buy and sell on a token's launch pool like any curve. Before you sign, the app shows:

- every equipped item, its sentence and what it will take, discount or burn on this trade
- the token's creator fee and holder rewards
- whether the curve is near graduation, and the exact remainder it can still fill

The app builds the transaction, your wallet signs it, and it is sent through the app's submit route. Your keys never leave your wallet.

## Raiding

A raid is a buy of token A paid for by selling token B in one transaction:

```
sell B on B's launch pool ──► buy A on A's launch pool
              swap_route (one transaction, up to 3 hops)
```

If A runs a Raid item naming B, you pay less creator and holder fee on A, part of the normal fee goes to A's war chest, and your holding of A earns raid points.

Raid points:

- travel with your tokens
- count toward A's siege threshold against B
- can be claimed as bounty from A's war chest, or spent on loot rolls
- net out if you sell back, so round trips earn nothing

## Joining a raid

Raid alerts from the indexer carry a link that builds the routed swap. One click, one signature.

## What you will not see

No forecasts and no projected returns. Every figure in the app comes from chain state or indexed events.
