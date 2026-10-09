# War

Every units token can open a **war chest**: a vault only code can spend, funded by the token's own fees. Hooks never pay anyone, so every reward in the war layer comes from a chest or a season pool, through instructions anyone can call.

## The war chest

| Funded by | Spent on |
| --- | --- |
| The companion's war share of the creator fee | Sieges (spot buys of a rival) |
| A share of raid trades' fees | Counter-strikes (buy back and burn) |
| Treaty and tribute payments (streamed to holders) | Bounties for raid points |

The chest never spends more than its balance less rent, and every spend is capped per call and spaced in time. The test suite checks the chest's books after every step.

## Attack and defense

| Action | What happens | Guard |
| --- | --- | --- |
| **Siege** | When raid volume from a rival passes the War orders threshold, anyone can crank `siege`: the chest spot-buys the rival and holds it as captured territory | Spend cap, interval, output no worse than the pool's own quote, waits while the rival trades at a premium |
| **Counter-strike** | When your short-window price falls far below the long-window price, the chest buys back and burns | Same limits as companion buybacks; a sandwich around it costs more than it makes |
| **Raze** | Sells captured territory | Rate-limited, and waits while spot is too far below the time-weighted price |
| **Return** | A treaty that grants peace returns captured tokens to the rival's chest | Needs a treaty both sides equipped |
| **Shield** | During a raid, holders who arrived from the raider pay more when they sell | Bounded by its slot |
| **Wall** | While under siege, a temporary max wallet | Lifts when the siege ends |

A siege cannot target a token whose kit pays holder rewards (the kit refuses program-owned holders). Lifting that limit is Planned.

## Points, bounties and loot

- **Raid points** live in the buyer's holding. They travel with the tokens and net out when a raider sells back, so a round trip earns nothing.
- **Bounties** pay SOL from the chest per point, capped per point and per claim.
- **Loot**: raid tickets roll for loot. The roll requests randomness from an oracle and never uses the slot hash. The reveal mints a real item: a template plus parameters from the season's loot table, inside the template's ceiling. The production randomness oracle is not chosen yet.

## Quests

Quests are checked on chain only (raid points spent, forges done) and grant loot tickets. Markers live in a per-holding account, so moving tokens cannot farm them.

## Seasons

| Step | Who | What |
| --- | --- | --- |
| Open | Anyone | Starts the season with a published score formula |
| Submit | Anyone | Names a candidate winner with its counters |
| Challenge | Anyone, within the window | Replaces the candidate with a higher score |
| Finalize | Anyone | The last unbeaten candidate wins |

Each check is constant time. Raid volume counts toward the score only up to a multiple of what the chest was funded that season, so self-raid loops cannot buy a season. The winner's chest receives a share of the protocol's fees for the next season, from the prize vault.

## Boss events

A **Boss** item (template 44) on the protocol's boss token counts raids against it per source token. Paying source chests from the season pool is Planned.
