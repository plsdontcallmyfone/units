# units on devnet: deploy, init and drill

Status: prepared 2026-10-09, nothing sent to any cluster yet. Every figure below was measured on
build server B (root@192.153.57.190) from main 5ebe7dc, or read from devnet with read-only RPC
calls (`solana rent <bytes> -u devnet`). Parameter values used by init are TEST values copied from
the test suites' own constants; none is an owner decision (docs/spec/00 section 6).

## 1. Programs

The deployed programs are `scripts/solana/programs.sh` PROGRAMS minus TEST_ONLY: 14 programs.
Sizes are the release `.so` from `programs.sh build` (Agave 4.3.0, platform-tools v1.57, SBPF v3);
the build reported no stack-frame warnings. Program data is deployed with `--max-len` equal to the
size (no spare space). Rent figures are lamports read from devnet for the exact account sizes:
program data = size + 45 bytes of loader header, buffer = size + 37, program account = 36.

| Program | Program id | .so bytes | Program data rent | Buffer rent (temporary) | sha256 (first 16) | Upgrade authority |
| --- | --- | --- | --- | --- | --- | --- |
| bordrless_token | 5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618 | 451,440 | 2,294,194,040 | 2,294,153,400 | 6b2f28775c5a2492 | deployer |
| bordrless_swap | AhmowBwJF7E1uDQ3quQz8xMKevre3i8kYPBEbkhAedvo | 475,504 | 2,416,439,160 | 2,416,398,520 | f72c130d78c685c1 | deployer |
| bordrless_bridge | 5TzyKXK6tzSrkkRdximMebWoV4rRjuyzEwCnisS6DKwj | 291,992 | 1,484,198,200 | 1,484,157,560 | 33096de9bcfe4daf | deployer |
| bordrless_launch | fBvY7neytvwSuJLF1Sur5tHk7vkWyPzjyfDVDm1m2qD | 668,504 | 3,396,879,160 | 3,396,838,520 | 15821768ccda9155 | deployer |
| bordrless_kit | CLEEZe3v8Sqa45J1VdmfKkjxqFGSj44MxA5prQH3xTLG | 293,960 | 1,494,195,640 | 1,494,155,000 | 52571176f5f16b01 | deployer |
| tax_hook | q9mMtM6vfJ8YMffnkUNW5XLz7xeVyyeo1HL8SA27AuX | 212,760 | 1,081,699,640 | 1,081,659,000 | 485c4148605166cc | protocol_authority |
| half_life | 67nAgW7h9wYmM1jLzrXVNy8UDNxgVFqGTqrTEPamtokh | 219,544 | 1,116,162,360 | 1,116,121,720 | 15dbfd2e70da8519 | protocol_authority |
| bordrless_companion | HzeAN8e7HbGx8wzgd5SpduF51c7rXCTqkQKcmw44YHkK | 356,656 | 1,812,691,320 | 1,812,650,680 | 27bc1aae536a763d | deployer |
| hookwars_armory | 7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU | 792,888 | 4,028,749,880 | 4,028,709,240 | 4cba4ae53de22e97 | deployer |
| hookwars_items | 8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv | 622,648 | 3,163,930,680 | 3,163,890,040 | 258045d0243e3f0f | protocol_authority |
| hookwars_war | 5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2 | 799,792 | 4,063,822,200 | 4,063,781,560 | ceac8a1075135b89 | deployer |
| hookwars_market | FikEwNXoXqRWteX4kpCT8dJ34o8hWQ8w49whhZiqS2vv | 470,792 | 2,392,502,200 | 2,392,461,560 | 0ecc990ecfe67d32 | deployer |
| hookwars_social | CKf4SjuiYxy4C2eSjk6oSQb2AnqC3ADoDTm8d323jWAx | 407,280 | 2,069,861,240 | 2,069,820,600 | b793fb8e2920a39e | deployer |
| hookwars_agents | GUTwa3zv83CKoq3TNYL9W1bJeUSEBVxR3MkGdxiXnZJ9 | 738,440 | 3,752,154,040 | 3,752,113,400 | d3b4f17eada49333 | deployer |

Each program account also holds 833,120 lamports (36 bytes). Deployer:
`AHQeXNyMe6cyvNYCs6DiuHcwb1DXdu6BR2YBADthAgTV`; protocol authority:
`CFi9xajnSxM1WMSndVoyQHmfm6DuEdzFodfjRfhTuzxa` (both keypairs in ~/.config/hookwars/program-keys).

The hooks the armory and launchpad check (`hookwars_items`, `half_life`, `tax_hook`) must be
upgradeable only by a key in `HOOK_UPGRADE_AUTHORITIES`, so they are deployed with the protocol
authority as upgrade authority; the plan generator runs init on that exact arrangement.

`randomness_stub` is test-only and is not in the budget. `hookwars_war` is initialized with the
stub's id as its randomness program (the suites' TEST value) because no randomness oracle is chosen
(decision D-4); loot `roll`/`reveal` therefore cannot complete on devnet until D-4 is decided and
the war config's randomness program is changed through the admin timelock. Deploying the stub on
devnet would make loot rolls predictable and is not planned.

## 2. Budget

| Item | Lamports | SOL |
| --- | --- | --- |
| Program data rent, 14 programs (kept) | 34,567,479,760 | 34.567 |
| Program account rent, 14 x 833,120 (kept) | 11,663,680 | 0.012 |
| **Kept by the deployment** | **34,579,143,440** | **34.579** |
| Largest buffer held during one upload (hookwars_war, returned after) | 4,063,781,560 | 4.064 |
| Write transactions, at least ceil(size / 1,232) each, 5,529 total, at 5,000 lamports | at least 27,645,000 | at least 0.028 |
| Init plan, 55 steps (configs, 45 templates, the Soulbound item), measured in LiteSVM | 182,891,480 | 0.183 |

Peak balance needed when deploying one program at a time with buffers closed after each: the kept
amount for everything already deployed plus the program being deployed plus its buffer. Deploying
in the order of section 3 and ending with hookwars_agents, the peak is the full kept amount plus
the largest buffer: **38.643 SOL** (34,579,143,440 + 4,063,781,560 lamports). With the init plan (0.183 SOL) and at least 0.028 SOL of write fees the run needs **at least 38.854 SOL** at its peak; the buffer's 4.064 SOL comes back after the last upload, leaving about 34.79 SOL committed.

## 3. Funding plan and run order

Owner approval (2026-10-09): borrow devnet SOL from the Instance devnet deployer
(`HzGbDT...kacHB`, key `~/.config/solana/instance-devnet-deployer.json`, passed explicitly with
`-k`), only at deploy time, and return what is left. Its balance read on 2026-10-09 was 17.44 SOL,
which is **less than the 38.854 SOL peak**: the deploy needs about 21.4 SOL more (38.854 less 17.44) from the
devnet faucet (rate-limited from this Mac and both build servers on 2026-10-09) or from the owner.
Do not start the deploy until the deployer holds the peak amount, or deploy in two sessions (the
script is resumable; programs already matching are skipped).

Run order (on a build server, keys in `HOOKWARS_KEYS`):

1. Build: `scripts/solana/programs.sh build <the 14 programs>`; check the sha256 values above.
2. Fund the deployer (transfer from the Instance deployer with explicit `-k`, `-u devnet`).
3. `HOOKWARS_KEYS=... scripts/devnet/deploy.sh` (dry run), then `... deploy.sh --send`.
4. Generate and check the init plan: `cargo test -p bordrless-program-tests --test devnet_plan --
   --ignored --nocapture` (runs the plan in LiteSVM first and writes `scripts/devnet/init-plan.json`).
5. `node scripts/devnet/send-plan.mjs` (dry run), then `--send`. Resumable: steps whose account
   exists are skipped (existence is read in one batched call; the public devnet RPC rate-limits
   bursts of single reads).
6. `node --experimental-strip-types scripts/devnet/lookup-table.mjs --send`; record the printed
   `PROTOCOL_LOOKUP_TABLE`.
7. Return the unused SOL to the Instance deployer.
8. The drill (section 5), step by step, recording every signature.

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

## 5. Drill plan (not run)

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
`app/packages/shared`). IDLs for `hookwars_market`, `hookwars_social` and `hookwars_agents` are not
yet in `idl/` or the SDK: run `scripts/solana/programs.sh idl` and sync them before the app uses
those programs.
