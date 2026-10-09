# Templates

Every hook in units is one of these audited templates plus its own parameters. Values in braces are an item's parameters; each sits inside the template's floor and ceiling.

| Id | Name | Family | Slot kind | What it does |
| --- | --- | --- | --- | --- |
| 1 | Raid | War | Pool + token half | Buyers who sold {target} pay {discount} less creator and holder fee; part of the fee goes to the war chest; buyers earn raid points |
| 2 | Shield | War | Pool + token half | Holders who came from {target} pay {sell_cut} extra when they sell during a raid |
| 3 | Wall | Defense | Defense | While under siege, no wallet may hold more than {max_wallet} |
| 4 | Spy | Relation | Pool | When {target} rises {trigger} over {window}, sellers pay {cut} |
| 5 | Treaty | Relation | Relation (pool half) | Every buy of either token sends {share} to the other token's holders, once both equip it |
| 6 | Tribute | Relation | Relation (pool half) | Every buy of us sends {bps} to {target}'s holders as tribute |
| 7 | Half-Life | Fee | Fee | Selling or sending costs {max_fee} of the tokens moved, halving every period held |
| 8 | Transfer Fee | Fee | Fee | Every transfer pays {fee} to {target}, with an optional max wallet |
| 9 | War orders | War | War | Sets the siege threshold, siege spend cap and counter-strike trigger |
| 10 | Size Tiers | Fee | Pool | Trades under {t1} pay {cut1}, up to {t2} pay {cut2}, larger trades pay {cut3} |
| 11 | Side Skew | Fee | Pool | Buys pay {buy_cut}, sells pay {sell_cut} |
| 12 | Launch Decay | Fee | Pool | Trades pay {start_cut} at launch, falling to {end_cut} over {decay} |
| 13 | Velocity Fee | Fee | Pool | Past {swaps} trades in {window}, each extra trade adds {cut}, up to {max} |
| 14 | Impact Fee | Fee | Pool | Trades that move the price pay {cut} per 1% moved, up to {max} |
| 15 | Volatility Fee | Fee | Pool | When the price moves more than {trigger} from its average, trades pay {cut} |
| 16 | Rush Hour | Fee | Pool | From {start}:00 UTC for {hours} hours trades pay {inside}; otherwise {outside} |
| 17 | Cooldown | Defense | Defense | After buying, a wallet waits {cooldown} before selling or sending |
| 18 | Daily Sell Cap | Defense | Defense | A wallet may sell or send at most {cap} of its holding per day |
| 19 | Max Transaction | Defense | Defense | No single transfer may move more than {max_tx} of the supply |
| 20 | Flash Guard | Defense | Defense | Tokens cannot be sold within {min_slots} slots of being bought |
| 21 | Dump Brake | Defense | Pool | While the price is more than {drop} under its average, sells pay {cut} |
| 22 | Dust Guard | Defense | Defense | Wallet sends under {min_amount} are refused |
| 23 | Guest List | Defense | Pool | For the first {open_after}, only holders of at least {min_hold} {target} can buy |
| 24 | Loyalty Pot | Reward | Pool + token half | Sells pay {cut} into a pot; wallets that held the whole period claim a share |
| 25 | Holder Stream | Reward | Pool | {buy_cut} of buys and {sell_cut} of sells stream to all holders |
| 26 | Streak | Reward | Reward | Buy on consecutive days without selling to build a streak, up to {max} |
| 27 | Ally Pass | Relation | Pool | Holders of at least {min_hold} {target} pay {discount} less on buys |
| 28 | Embargo | Relation | Pool | Buyers who arrive by selling {target} pay {cut} extra |
| 29 | Mercenary | War | Pool + token half | Any buyer arriving from another token earns {points} raid points per unit |
| 30 | Garrison | War | Pool | While under siege, buyers pay {discount} less |
| 31 | War Levy | War | Pool | While a raid of at least {trigger} is under way, sells pay {cut} to the war chest |
| 32 | Sell Burn | Burn | Pool | {burn} of every sell is burned |
| 33 | Target Burn | Burn | Pool | {burn} of every trade is burned until supply reaches {target} of the start |
| 34 | Gift Ember | Burn | Fee | {cut} of every wallet-to-wallet send is burned |
| 35 | Rank Badge | Social | Reward | Every {unit} bought earns a unit; each {step} units is a rank, up to {max} |
| 36 | Referral | Social | Pool | Buyers who name a referrer send {cut} of their buys to that referrer |
| 37 | Sell Ladder | Fee | Fee | Selling a bigger share of your holding at once costs more, up to {max} |
| 38 | First Blood | Social | Pool | The first buy of at least {min} each day pays {discount} less |
| 39 | Guild Tag | Social | Reward | Holders can wear a guild tag |
| 40 | Patience | Reward | Pool | Wallets that held for {min_age} pay {discount} less when they sell |
| 41 | Composite | Utility | Host slot's kind | Runs up to four modules in one slot |
| 42 | Soulbound | Defense | Defense | Refuses every transfer; used for agent and achievement badges |
| 43 | Coalition | Relation | Relation | Names the coalition a token belongs to and caps its chest contribution |
| 44 | Boss | War | Pool | Counts inbound raids on the boss token per source token |
| 45 | Rivalry | Relation | Relation | Names a rival for a fixed term; the result feeds the season score |

Items 42 to 45 are configuration or badge templates and cannot be part of a composite.
