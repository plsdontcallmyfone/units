# units on devnet: deploy, init and drill

Status: DEPLOYED 2026-10-10 from main 43b65d3 (branch devnet). All 16 programs are on devnet and
their on-chain bytes match the local build; the init plan (106 steps) and the protocol lookup table
are sent; the drill ran (section 8). Figures were measured on build server B (root@192.153.57.190)
or read from devnet. Parameter values used by init are TEST values copied from the test suites'
own constants; none is an owner decision (docs/spec/00 section 6).

## 1. Programs

The deployed programs are `scripts/solana/programs.sh` PROGRAMS minus TEST_ONLY: 16 programs
(`hookwars_craft` and `hookwars_book` joined after the 2026-10-09 plan). Sizes are the release `.so`
from `programs.sh build` (Agave 4.3.0, platform-tools v1.57, SBPF v3). Program data is deployed with
`--max-len` equal to the size (no spare space). Rent: devnet charges 5,080 lamports per byte with the
128-byte account overhead (program data = size + 45 bytes of loader header, buffer = size + 37,
program account = 36 bytes = 833,120 lamports). Rows are in deploy order (section 3).

| Program | Program id | .so bytes | Program data rent | Buffer rent (temporary) | sha256 of the deployed .so | Upgrade authority | Deploy signature |
| --- | --- | --- | --- | --- | --- | --- | --- |
| hookwars_armory | 7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU | 1,215,328 | 6,174,745,080 | 6,174,704,440 | b4dca849813e84c4cb1183fb647e6790b2c5e9a4270a6184837635282cf69ee7 | deployer | 4n5Y3x39PW4nWr5ZZLUiAC9V35Pkn79HVRwf2KivxVGL1ViM6DBi9bE1wmw2g9hWa2qWLt4FWCF9uTyysoGSeB2o |
| hookwars_war | 5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2 | 1,081,096 | 5,492,846,520 | 5,492,805,880 | 71d59b063b07b5f0f5d0ba228a42ab3d632e60ba600459fed070d99b396cee79 | deployer | HGvLw6qiJUc7j9WgTMtTPwd5wQ4QeMLnR58MMeArs7DsCa2mk3puSiuD3urcwDjvX1ZxA7VuzUmz1fCoDy9zB6U |
| hookwars_agents | GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9 | 881,328 | 4,478,025,080 | 4,477,984,440 | 2a232c479d22162ff22379fbabd31e453c59c4b71044c6252ccfe741f21ffaf9 | deployer | 4MgMUPgbaTAFRf4G35H7D4nTZ9gnxV8EBxWJmPgAmZGb7LPk9vt93ESVXJnEMcDKSFKf1y39RAjNdiGoEGYfQD4S |
| hookwars_items | 8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv | 700,568 | 3,559,764,280 | 3,559,723,640 | 15f66b0feb15aa9aa85bf5ed6545da602ee3d52c60365acc22c26b55668a4dee | protocol_authority | 5QQvxpAeF5obtuzCfBhrfU7DF2cfJetjn3W4Sxw4A62xjfeNEGxHRQSEVhVWt4gqUyCyPojH8hG2bGLVwwbu4RrW |
| bordrless_launch | fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD | 672,792 | 3,418,662,200 | 3,418,621,560 | 09296dbdb65c44ec468a00b9a1d66a3c36ce509a584d23abd54b53ffec1aa83a | deployer | 4FyEpKbGmWyABeANFnSziTgaEHKWEMKpuqibSckMfvttfEMM9GCj5YktjBncC8eCk2xtKUKnRLFHwxn76UjKrWfL |
| hookwars_market | FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv | 642,736 | 3,265,977,720 | 3,265,937,080 | 9fd3d59eb9fb7bad82e290984d77af8a9785f0579ffd480cd7895388d950d978 | deployer | 4Yqqcpvo2f2C7WP7aQFmTdGbFq7L5CjiVhCHhU5zy4ACFwRs56XamwDvjB5JYfeVmbNjEmuw7fwmBpAQqfJvRfx3 |
| hookwars_social | CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx | 497,248 | 2,526,898,680 | 2,526,858,040 | 7b3bba0a195d34d5b8576bffca338478202a16d53e6b9fe2de98803c4f4cf2d1 | deployer | 3Q4o8CeMENxwVFftGu67vv8Po63byKfyFizNRuywQ7bUzG6ZQrDGiVZF6XF3uDK3r2bsutEXuoEDPERJP59ETC8A |
| bordrless_swap | AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo | 475,504 | 2,416,439,160 | 2,416,398,520 | f72c130d78c685c166d05cffea3c913875a9f388b55867b25f2344bd55ab20fb | deployer | naMbpn3oJXPKLmoYCoHoZrAG1Uvxh4pMr3Y2TSUudWq2wh8ZUQL5oZWeKDgjxtZN2pSEmbZQWvz8jTbGF67Bkku |
| hookwars_craft | 39LXQBGqZtg591jkGnZi9BELQ9hp1ZngbAxu6K1cC29Y | 467,520 | 2,375,880,440 | 2,375,839,800 | 2c1aeb7d7593b9db50185cad7b47dd3090b869e4e9c93d69bc9834f405d7d6b5 | deployer | 4ZZJa8bHW8mQuoFkBQMHio3Rxvymyn5WZGuMcEKyMqmPP7jPDt5vfbS68WThx9smS7PoD8npqNamFK4vsBme4b3B |
| hookwars_book | C4k2QquxzDdgHf74tnvyyWQyGUR8xvhPo1i1gFYb639g | 461,760 | 2,346,619,640 | 2,346,579,000 | 552bc8f381662afa1cd0fd0900d537c24020d228964d74ac7645b8a2aedb7dca | deployer | 5QdVURx7hqPsZBBREdUfveiBSe4FkY1Cbf28nDBWVJMjes7ML3RMQEFgir6xG1f5pTCoyGMttreS2QFyw1UqWQnr |
| bordrless_token | 5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618 | 451,608 | 2,295,047,480 | 2,295,006,840 | 545e4b1367ab23fc1b3d70b4ced4418c1b54b4c88a03aaaa2e2acac78794b7ad | deployer | ob3upJwESZ2zyUBXJGXLw33p5ErDvSHV5HziffguGojDSHPA6trrbdzEr7LuPNY5Pbok2vfoP7nyBYQdkL9QZDS |
| bordrless_companion | HzeAN8e7HbGx8wzgd5SpduF51c7rXCTqkQKcmw44YHkK | 356,656 | 1,812,691,320 | 1,812,650,680 | 27bc1aae536a763d950b6671d6b34ea94eb3bc0f884c056cb5584141f69beb1e | deployer | 7VEjzXZBZHk6FYskAM9NB3B1uVd2JT4QVkQa8ijrGir6BX6i4yZyMa1SjYXLKdWvkUAsyVy97qgpTV9rAsf8Y6C |
| bordrless_kit | CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG | 294,984 | 1,499,397,560 | 1,499,356,920 | 39086b04e1de6ff8aadc7f8bc6f7efb0ede0d5cdb4ada0a9d7e2fd0b52ac8b75 | deployer | wVMa5VPzBtWNLRXaA8pbq1FPoitUE1XmqMaus2TpVyeq4bL1cGQs6pMwzJGb42dMkgPDAAmm717N1VJ5LvgPVZT |
| bordrless_bridge | 5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj | 291,992 | 1,484,198,200 | 1,484,157,560 | 33096de9bcfe4daff74bada39ad48a6e12fd75ab57ff2f1d35272a4d796ee75d | deployer | 3Gd8LjMBSZTQYLXaK13qDCYda8NsbVTwk4xrzWDKeBp95NBbzzq9DGhAXHaDtZxyRq5MMfyPNWhrAaiuwwrgic1c |
| half_life | 67nAgW7h9wYmM1jLzrXVNy8UDNxgVFqGTqrTEPamtokh | 219,544 | 1,116,162,360 | 1,116,121,720 | 15dbfd2e70da8519662e6088696c4db480ced1c22e0008b91f18d6d80c64169b | protocol_authority | 2AYGneYhQ1T5thGQyD5ovn85vZDPDXuHmzPgpmq8EPb446R663u3eQ3eRpWiWXeAnLY6Hgr2Xxd3D4wv83TcKmos |
| tax_hook | q9mMtM6vfJ8YMffnkUNW5XLz7xeVyyeo1HL8SA27AuX | 212,760 | 1,081,699,640 | 1,081,659,000 | 485c4148605166cc14283ad480d09a69e9e494a85842624814c185652b3a19c0 | protocol_authority | 2PTekQsEoNdhKWTXUmqMc2Twb6y7vEuxku19TLVoscS4PcxfjWGL6R1SHt8afVfeoSn2HyGY3kriVzcVbYWVoNQB |

Deployer: `AHQeXNyMe6cyvNYCs6DiuHcwb1DXdu6BR2YBADthAgTV`; protocol authority:
`CFi9xajnSxM1WMSndVoyQHmfm6DuEdzFodfjRfhTuzxa` (both keypairs in ~/.config/hookwars/program-keys).
`deploy.sh --send` dumped each program after its upload and checked the on-chain bytes against the
local `.so` (all 16 "verified").

The hooks the armory and launchpad check (`hookwars_items`, `half_life`, `tax_hook`) must be
upgradeable only by a key in `HOOK_UPGRADE_AUTHORITIES`, so they are deployed with the protocol
authority as upgrade authority; the plan generator runs init on that exact arrangement.

`randomness_stub` is test-only and is not deployed. `hookwars_war` is initialized with the stub's id
as its randomness program (the suites' TEST value) because no randomness oracle is chosen (decision
D-4); loot `roll`/`reveal` therefore cannot complete on devnet until D-4 is decided and the war
config's randomness program is changed through the admin timelock.

## 2. Budget

| Item | Lamports | SOL |
| --- | --- | --- |
| Program data rent, 16 programs (kept) | 45,345,055,360 | 45.345 |
| Program account rent, 16 x 833,120 (kept) | 13,329,920 | 0.013 |
| **Kept by the deployment** | **45,358,385,280** | **45.358** |
| Peak, deploying largest first (kept so far + the program being deployed + its buffer) | 46,440,044,280 | 46.440 |
| Write transactions, at least ceil(size / 1,232) each, 7,251 total, at 5,000 lamports | at least 36,255,000 | at least 0.036 |
| Init plan, 106 steps, measured in LiteSVM | 202,037,880 | 0.202 |
| **Needed at the peak** | **at least 46,678,337,160** | **46.678** |

Deploying the largest program first holds the biggest buffer while the least rent is committed;
the peak is then the full kept amount plus the smallest buffer (tax_hook). Measured on devnet: the
deployer went from 46.998629516 SOL to 1.589009236 SOL across the deploy (45.409620280 SOL: the
kept rent plus about 0.051 SOL of write fees and retries); no buffer was left open (`solana program
show --buffers` lists none).

## 3. Funding and run order (as run, 2026-10-10)

Funding: the owner sent the units deployer 35 SOL (balance 34.998629516 before the deploy). The
measured shortfall was borrowed from the Instance devnet deployer (`HzGbDT...kacHB`, key
`~/.config/solana/instance-devnet-deployer.json`, passed with `-k`): 12 SOL, signature
`5N9CfLJ2eYacLbPRExmumJKKn3gQ8keTfejqb5zG4CnK3M4m8XoPMZGMmzLuedXYxDEHBDdGmp649QbLHozojW2r`. What
is left after the drill goes back to it (section 8).

Run order (on server B, keys in `HOOKWARS_KEYS=/root/hw-devnet/keys/devnet`):

1. Build: `scripts/solana/programs.sh build`; the sha256 values above are the built files.
2. Fund the deployer (above).
3. `scripts/devnet/deploy.sh` (dry run: nothing deployed), then `deploy.sh --send`: 16 deployed and
   verified, leftover buffers closed.
4. Generate the init plan: `cargo test -p bordrless-program-tests --test devnet_plan -- --ignored
   --nocapture` (106 steps land in LiteSVM; `items_code_hash` = the deployed `hookwars_items.so`).
5. `node scripts/devnet/send-plan.mjs` (dry run), then `--send`. The first run stopped at step 16 on
   the public RPC's 429 replies; `send-plan.mjs` now retries with a pause and counts a step whose
   account appeared as landed; the second run sent the rest (90 sent, 15 already done). A re-run is
   safe except for steps 51 and 98 (`set_access_params`, queued then applied): they create no
   account, so a third run would queue the change again; skip them by hand.
6. `node --experimental-strip-types scripts/devnet/lookup-table.mjs --send`:
   `PROTOCOL_LOOKUP_TABLE=7piuvZj2xJM7dj5fnB4sC8JujnyvRBa3J7bmtTAXbHUu` (33 addresses; create and
   two extends, signatures `oG2EgoCC99LHraW2twLnkpvsPZot9vKJZzsH4saG8BBdQdp12Pj1sVRF2ztCFtXmQKgq2HmQ3zFkhHwguqoHADd`
   and `4s8KMBSTAMx52DpMGtNWop9miwF5JUbXGePvkwUZCfEvyq1Av7iR1BXtfq4HVwGmLnpf872im756ok6iRyFXzhDc`).
7. The drill (section 8).
8. Return the unused borrowed SOL to the Instance deployer (section 8).

## 4. Init plan

`programs/tests/tests/devnet_plan.rs` builds every init instruction with the suites' own builders,
runs them in LiteSVM against the built `.so` files with the real deployer key and the upgrade
authorities of section 1, and writes `scripts/devnet/init-plan.json` (program, accounts with
signer and writable flags, data in hex, and the account each step creates). Steps, in order:

1. swap `init_config` (fixture policy: protocol fee, launch protocol share, fee collector = deployer)
2. bridge `init_config`
3. launch `init_config` (fixture policy: launch fee, fees, sniper window, curve, supply, rule bounds)
4. bridge `register_sol` (the bridged SOL wrapper every launch is quoted in)
5. armory `init` (armory TEST_PARAMS)
6. armory `register_template` for every template the items program implements (ids 1 to 45 with a
   shape), with the suites' TEST floors and ceilings, the names from docs/spec/08 section 6 and
   09/10, and `code_hash` = sha256 of the deployed `hookwars_items.so`
7. armory `create_item` of the Soulbound item (item 0), which every agent badge equips
8. war `init_config` (war TEST_PARAMS; randomness program = stub id, see section 1)
9. market `init` (TEST_MARKET), social `init` (TEST_SOCIAL)
10. agents `init_config` (TEST_AGENTS_PARAMS with its TEST quorum of 2; verifier set = the deployer and
    the protocol authority, both keys we hold, a TEST value: the real
    verifiers are decision D-10)

Measured on server B (2026-10-09): all 55 steps land in order; the plan costs the deployer
182,891,480 lamports (rent of every account it creates, plus LiteSVM fees). The generator also
writes `scripts/devnet/params.test.json`, every TEST parameter set as the Rust constants print
themselves, and the 45 template schemas. Regenerating the plan is deterministic (byte-identical
JSON for the same build and deployer).

Not in the init plan and still open: the DEX `fee_collector` pointing at war's prize vault (R14:
`set_config` after war is initialized); `LaunchConfig` slot presets (not built); the integration
requests in docs/spec/09 section 21 and 10 section 17 (agents record calls, market lease gates,
war coalitions and boss pool) which are not on main yet.

## 5. Drill plan (run 2026-10-10: results in section 8)

Each step names the instruction family it proves. Wallets: the deployer as admin, three TEST
traders funded from the deployer, one TEST agent operator.

| # | Step | Programs and instructions |
| --- | --- | --- |
| 1 | Wrap SOL for each trader | bridge `wrap_sol` |
| 2 | Slot launch with the kit (holder rewards) in slot 0, a Raid item and a Size Tiers item | launch `prepare_launch`, `equip_prepared` per item, `create_prepared_launch`, `refresh_pool_registry`; war `init_war`; items `init_raid_ledger` |
| 3 | A second slot launch (the rival) | same as 2 |
| 4 | Buys and sells on both with items answering | DEX `swap` (pool items through the launchpad, token slots on delivery) |
| 5 | A raid: sell the rival, buy ours in one route | DEX `swap_route`; Raid stamps points (RaidMarked) |
| 6 | Settle item cuts, claim royalties | items `settle_equip`; armory `claim_royalty` |
| 7 | Holder vote to swap an item, after notice | armory `propose`, `vote`, `finalize`, `execute`; token `set_vote_lock` |
| 8 | Forge two items | armory `forge` |
| 9 | Composite item with three modules, equip by vote | armory `create_composite`, then 7 |
| 10 | Market: list an item, buy it; collection | market listing, purchase, collection instructions |
| 11 | Rental: lease an item to a token, let it end | market lease instructions |
| 12 | Guild hall: create, deposit, officer action after the delay | social guild instructions |
| 13 | Agent passport, link, badge | agents `register_passport`, `link_social`, `equip_badge`, `issue_badge` |
| 14 | War: fund via companion war share, siege the rival, counter-strike, raze | companion `claim_fees`; war `siege`, `counter_strike`, `raze` |
| 15 | Bounty and quest | war `claim_bounty`, `claim_quest` |
| 16 | Loot | war `roll` (expected to stop at `reveal` until D-4, section 1) |
| 17 | Season | war `submit_candidate`, `challenge`, `finalize_season`, `split_protocol_fees` |
| 18 | Graduation with the remainder buy | DEX `swap` of the remainder, launch `graduate` |

## 6. Rollback

Every program can be closed by its upgrade authority, returning its program data rent:
`solana program close -u devnet -k <authority> --bypass-warning <program id>` (deployer for most,
protocol authority for the three hooks). A closed program id can never be redeployed, so closing is
for abandoning devnet, not for upgrades; upgrades use `deploy.sh --send --upgrade` (with
`solana program extend` first when a build grows, which the script does). Buffers left by an
interrupted upload are closed by `deploy.sh`'s last step. Config and template accounts are owned by
the programs and stay until the programs close.

## 7. What the app needs

Environment variables the app reads (app/apps/*, app/packages/*):

| Variable | Value on devnet |
| --- | --- |
| `CLUSTER` | `devnet` |
| `RPC_URL` | a devnet RPC (server-side only; never sent to the browser, app audit A-2) |
| `PROTOCOL_LOOKUP_TABLE` | printed by `lookup-table.mjs` |
| `DATABASE_URL` | the indexer's Postgres |
| `API_URL`, `SITE_URL`, `HOST`, `PORT` | where the API and site run |
| `INDEXER_INTERVAL_MS`, `MAP_WINDOW_SECS`, `MIN_RAID_LAMPORTS` | indexer and map settings |
| `TELEGRAM_BOT_TOKEN`, `TELEGRAM_CHAT_ID`, `X_BEARER_TOKEN`, `BOTS_STATE` | bots, off unless set |
| `MOCK_DATA` | must be unset (it fills pages with demo data; never on a public deploy) |
| `SKIP_DB` | unset |

The app's program ids already match section 1 (they come from `PROGRAM_IDS` in
`app/packages/shared`), and the SDK's IDLs are the ones main 43b65d3 generated from the deployed
sources.

`app/devnet.env` (committed, no secrets) holds the devnet values: `CLUSTER=devnet`,
`RPC_URL=https://api.devnet.solana.com`, `PROTOCOL_LOOKUP_TABLE=7piuvZj2xJM7dj5fnB4sC8JujnyvRBa3J7bmtTAXbHUu`,
`HOST`, `PORT=9961`, `API_URL`. Load it with `set -a; . app/devnet.env; set +a` and add
`DATABASE_URL` from the server's own environment before starting the api, indexer, web or agents.
On 2026-10-10 the api (127.0.0.1:9961) and the indexer ran on server B against devnet with their
own database `units_devnet` (URL in /root/hw-devnet-db.url): the indexer walked the programs'
signatures and decoded real events into the `ev_*` tables and `mint_tables` (the drill's raids need
the launched tokens' lookup tables from there), and the explorer route decoded the drill's raid
(RaidMarked, ItemCut, PoolItemCuts, RouteSwapped). The public devnet RPC rate-limits the indexer
heavily (429 replies); a private devnet RPC in `RPC_URL` (server-side only) makes it usable.
