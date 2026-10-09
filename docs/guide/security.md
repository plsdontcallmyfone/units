# Security

units is unaudited by any third party and not deployed. This page states the security model, what has been reviewed internally, and what is still open.

## The model

| Rule | How it is enforced |
| --- | --- |
| Hooks never get your signature | The token program and launchpad call items with their own signer; your signature never reaches an item |
| A hook never takes more than the amount | The token program checks every answer against the slot's bound and the amount |
| Hooks never pay out | Payouts leave only program-owned vaults, through settle, claims and payouts that verify their source |
| Bounds never change | A slot's kind, bounds, equip rule, notice and memory range are fixed at launch |
| Item code never changes under you | Templates must be immutable or upgradeable only by the managed key; code hash is checked at equip |
| No live calls between tokens | Relations read state already on chain; a token is never called because another traded |
| No spot-price reads | Other markets are read as time-weighted prices over a minimum window |
| No forged routes | The DEX fills the route; trader-supplied data is never trusted for it |

## What an item can do

Take a bounded cut, refuse a transfer, burn part of a swap where allowed, discount the launch's own fees, write its own memory range. A refusal cannot be caught at run time, so templates that declare they never refuse are tested for it.

## Internal reviews

| Review | Scope | Result |
| --- | --- | --- |
| Security review 1 | Token, armory, war, DEX, kit, companion | 2 high, 6 medium, 4 low, 3 info. All high and medium fixed with regression tests. Three lows deferred (template registration timelock, re-checking equipped items after a template upgrade, kit check cost) |
| Security review 2 | Items, launchpad slot launches, DEX routes, war on real types | 1 high, 2 medium, 6 low, 3 info. All high, medium and low fixed |
| Economic fuzz audit | 64 random sequences, 14,720 steps across trades, items, votes, settlement and war | No invariant violations; no free money in 321 round-trip probes |
| App and dependency audit | Site, API, indexer, bots, npm and Rust dependencies | 1 critical (web framework version), 1 high, 4 medium, 4 low. All fixed; dependency audit clean |

The suite checks after every step that supply equals holdings, cuts add up, vaults match their records, royalties are exact, war chests never overspend and pool reserves stay consistent.

## Known limits

- Sieges cannot target tokens whose kit pays holder rewards.
- Size Tiers and Max Transaction can be split across several transfers in one transaction.
- Loot randomness oracle not chosen yet.
- Several parameters (royalty cap, vote period, season length, prize share, siege and bounty caps) are set by governance and not yet decided.

## Upgrade authority

All programs are upgradeable by a single key today. Before mainnet, upgrade authority moves to a multisig behind a timelock. Do not treat any program as immutable until then.

## Reporting

Report vulnerabilities privately to the maintainers through the repository's security contact before disclosing.
