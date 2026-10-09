# Programs

units is a set of Solana programs. The ids below are program ids from the repository's `Anchor.toml`. units is **not deployed** to any cluster yet; these are not live addresses.

| Program | Role | Program id |
| --- | --- | --- |
| Token | Mints, holdings, transfers; slot tables, item calls, holder memory, vote locks, protocol transfers | `5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618` |
| DEX | Constant-product pools, pool hooks, price observations in every pool, multi-hop routes | `AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo` |
| Launchpad | Curve launches, slot launches, forwarding swaps to pool items, graduation | `fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD` |
| Kit | Launch rules in a locked slot: holder rewards, max wallet, creator and early-buyer locks | `CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG` |
| Companion | A launch's creator as a program: buybacks, holder shares, vesting, war chest funding | `HzeAN8e7HbGx8wzgd5SpduF51c7rXCTqkQKcmw44YHkK` |
| Bridge | SPL, Token-2022 and SOL in and out of the standard, one for one | `5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj` |
| Armory | Templates, items, royalties, votes, equips, performance reverts, forging, loot minting | `7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU` |
| Items | Every template's code, composites, raid ledger, settlement, payouts | `8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv` |
| War | War chests, siege, counter-strike, raze, bounties, loot, quests, seasons, prize vault | `5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2` |
| Market | Item listings and sales, collections, rentals, commissions | `FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv` |
| Social | Achievement badges, guild halls | `CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx` |
| Agents | Passports, badges, proof levels, track records, policy wallets, diplomat bonds | `GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9` |
| Half-Life | Standalone exit-fee hook kept from the base | `67nAgW7h9wYmM1jLzrXVNy8UDNxgVFqGTqrTEPamtokh` |
| Example fee hook | Standalone transfer-fee example hook kept from the base | `q9mMtM6vfJ8YMffnkUNW5XLz7xeVyyeo1HL8SA27AuX` |

Test-only programs (testers and stand-ins) share some of these ids in the test suite and are never deployed.

## Built on

units forks the Bordrless token standard, DEX and launchpad (Apache-2.0). The token, DEX, launchpad, kit and companion are extended; the bridge is unchanged.
