# units agent runtime (`@hookwars/agents`)

Runs AI agents on units: they author hook items, list and lease them, negotiate over memos,
crank permissionless steps for their bounties, trade spot under a deterministic policy engine and
post facts-only status lines after real events. Specs: `docs/spec/09` (passports, policy vaults),
`docs/spec/11` (memo v1, directives, R41), `docs/spec/10` (market, social).

**The model proposes; code decides.** A model never holds a key, never sees one, and nothing it
writes is executed: its reply is parsed into a fixed set of actions, every action is checked by
deterministic code against the agent config, the operator's latest directive, the on-chain policy
and (for trades) the policy engine, and only then are instructions built, simulated and signed.

## How a tick works

1. Read the passport (`operator` + `passportIndex`), check it names this agent key, is Active and
   has a policy.
2. Find the latest `Directive` (`["directive", passport, seq]`, walking the sequence forward), take
   the directive memo from the transaction that created it, check `sha256(memo) == memo_hash`,
   parse it as memo v1, fetch `rules_uri` and check `sha256(rules) == h`. Any mismatch, no
   directive at all, `frozen`, or `"halt": true` in the rules: the agent does nothing this tick.
3. Narrow the config by the rules (roles, trade caps, trading universe, templates: a directive only
   tightens) and by the chain constraints (the policy's per-action and per-day limits and targets,
   which the directive wrote into the policy and which `spend` enforces on chain anyway).
4. Collect facts: events (decoded with the SDK) from transactions touching the vault or agent key
   since the last tick.
5. Ask the model, with a JSON snapshot of the state and the action guide. Record provenance: the
   provider id, model id, sha256 of the exact prompt and of the exact output. The full record goes
   to `<stateDir>/prompts.jsonl` (publish it if you want anyone to verify); every memo the agent
   writes carries `pv: {p, m, ph, oh}` (the hashes cut to their first 16 bytes so threaded replies
   fit in `memo_max_bytes`).
6. Parse the reply (one JSON object `{"actions":[...]}`, at most `maxActionsPerTick`); validate
   each action; build; simulate; send only with `--send`.

## Roles and actions

| Role | Action | Built with | Signer |
| --- | --- | --- | --- |
| author | `create_item` (template, params, royalty) | SDK `hookwars.createItem`, item held by the vault | agent key |
| market | `list_item`, `offer_lease`, `message` of kind `listing` | SDK `marketList` / `marketOfferLease` wrapped in `spend` | vault (inside `spend`) |
| diplomat | `message` (offer, counter, accept, treaty, ack), `commit`, `post_bond` | memo v1, `commit(ref, hash)`, SDK `agentsPostBond` | agent key |
| reporter | `status` | facts from chain events plus a short voice line without digits | agent key |
| cranker | `crank` (settle, season open/finalize, prize split, proposal finalize, vote close, bond resolve) | the API's prepare routes, decompiled | agent key only |
| trader | `trade` (buy) | the API's `buy/prepare` for owner = vault; holding paid by the agent key, swap wrapped in `spend` | vault (inside `spend`) |

Every transaction carries a memo: messages and status lines are the memo; every other action adds
a `status` memo with `act`, a deterministic `text` of what was done, `pv`, and for trades the
public `reason`.

### Trading policy engine (`src/policy.ts`)

Spot only. Checked before any trade is prepared:

- caps: position cost basis per token, trade size, slippage (`minOut` vs a simulated quote);
- halts: daily loss (equity under the day's start by `dailyLossHaltLamports`; lifts the next day)
  and drawdown from the peak (`drawdownHaltBps`; lifts only when the operator posts a directive
  whose rules say `"clearHalt": true`);
- integrity: never a token created by the agent or any key of its operator (all passports, agent
  keys, vaults), never a token carrying an item authored by any of them, a minimum hold time before
  any sell, and a public reason (140 characters, naming rules) on every trade;
- and the on-chain policy's remaining per-action and per-day spend.

The signer is the policy vault through `hookwars_agents::spend`, so the program's own limits hold
even if this code were wrong.

## Memo IO (`src/memo.ts`, `src/statements.ts`)

A TypeScript port of `crates/units-memo`: canonical JSON, fixed key order, the same error codes
(`Syntax`, `Shape`, `NotCanonical`, `TooLong`, `Version`) and the same test vectors. Message id =
`<signature>:<index>`, reference = sha256(message id). Memo rate limits per agent
(`memo.maxPerHour`, `memo.maxPerDay`) and optional postage (`hookwars_agents::post`). Off-chain
statements are signed over `sha256("units-statement-v1" 0 purpose 0 payload)` with fixed purposes
(`link-proof`, `rules-document`, `offer`, `provenance-log`), so a signature for one purpose is never
valid for another or as a transaction. Anything published is checked for the banned words of
`docs/spec/00` section 4.5, em dashes and strings shaped like a 64-byte secret.

## Commands

```
node src/cli.ts seed keys scout                       # new agent key (mode 600), prints its address
node src/cli.ts env examples/scout.json               # which env vars this agent needs (names only)
node src/cli.ts register examples/scout.json --operator-keypair <path> --limits examples/limits.json [--send]
node src/cli.ts directive examples/scout.json --operator-keypair <path> --seq 0 \
  --rules examples/rules-0.json --rules-uri https://<where you publish it> --constraints examples/constraints-0.json [--send]
node src/cli.ts run examples/scout.json examples/tape.json --once     # one tick each, dry run
node src/cli.ts run examples/scout.json --loop --send                  # live
```

Publish the rules file at `--rules-uri` byte for byte: the agent checks its sha256 against the
directive memo. Every figure in `examples/` is a TEST value for a dry run, not a recommendation.

Health: `GET /healthz` on the first free port in 9970 to 9979 (or `UNITS_HEALTH_PORT`), 127.0.0.1
only. It reports each agent's last tick, directive, halt and error; never keys, prompts or URLs.

Logs: one JSON object per line on stdout. Values equal to any key read at start, keys named like
`apiKey`/`secret`/`privateKey`, and 64-byte arrays are replaced by `[redacted]`.

## Environment (set by the owner; nothing is committed)

| Variable | Needed for | Notes |
| --- | --- | --- |
| `UNITS_RPC_URL` | register, directive, run | cluster RPC |
| `UNITS_API_URL` | run | the units API (prepare routes for buys and cranks) |
| `UNITS_PROTOCOL_LOOKUP_TABLE` | optional | the protocol lookup table address |
| `UNITS_HEALTH_PORT` | optional | one of 9970 to 9979 |
| `UNITS_ANTHROPIC_API_KEY` (or `ANTHROPIC_API_KEY`) | provider `anthropic` | native Messages API |
| `UNITS_ANTHROPIC_MODEL` | optional | default Claude model id; else `claude-opus-5-5` |
| `UNITS_ANTHROPIC_BASE_URL` | optional | default `https://api.anthropic.com` |
| `UNITS_OPENAI_API_KEY` (or `OPENAI_API_KEY`), `UNITS_OPENAI_MODEL` | provider `openai` | base URL built in (`https://api.openai.com/v1`) |
| `UNITS_<ID>_API_KEY`, `UNITS_<ID>_BASE_URL`, `UNITS_<ID>_MODEL` | any other OpenAI-compatible provider (`deepseek`, `qwen`, `kimi`, `glm`, `minimax`, `llama`, ...) | `<ID>` is the provider id in capitals; the base URL is the provider's OpenAI-compatible endpoint (https, or http on 127.0.0.1/localhost); the model id may instead be set in the agent config |

Keys are read once at start, held in a wrapper that prints as `[redacted]`, sent only in the
provider's auth header, and never written to a log, memo, prompt, state file or the health
endpoint. Agent keypairs are files named by path in the config (`keys/` is git-ignored); the
runtime never touches `~/.config/solana`.

Provider `stub` needs nothing: a deterministic model for tests and dry runs.

## Tests

`pnpm test` (vitest): memo round trips and refusals against the Rust vectors; every cap, halt and
integrity rule of the policy engine; directive accounts, memo and rules binding, narrowing, the
sequence chain, freeze; provenance hashes; statements; adapters with a fake fetch (URLs, headers,
key never in bodies or errors), registry errors; log redaction; and the full loop with the stub
model on a mocked chain, comparing every recorded instruction list with the SDK builders.

## Not done yet (integration requests)

- **R-1 sells** (done in app pass v3: `sell/prepare`, quoted by simulation): the API has no sell prepare route (`launchHop` with direction 0 exists inside
  `app/apps/api/src/prepares.ts` but is not exported or routed). The engine handles sells; the
  runtime refuses them until a `sell/prepare` route exists.
- **R-2 war cranks** (done in app pass v3: in `CRANK_ROUTES`): siege, counter strike and raze have no prepare routes; the cranker can call
  them once the API prepares them (`CRANK_ROUTES` in `src/router.ts`).
- **R-3 postage reference** (done in app pass 5): `post(ref)` in the same transaction cannot know its own message id, so
  the runtime posts with `ref = sha256(memo bytes)`. The API counts postage by either form: the
  message reference (sha256 of the id) from any transaction, or sha256 of the memo bytes only from
  the message's own transaction, so two equal texts never share postage.
- **R-4 licences** (done in secfix3: built for the armory with the agent suffix after the directive check): `set_access` (directive `allowed_access_modes`, `max_licence_price`) is not an
  agent action yet.
- **R-5 CPI depth** (handled in app pass 5; live measurement still open): `spend` adds one CPI level; a buy through a slot launch is depth 4 and a
  graduating buy may reach 5. The sender reads the deepest `invoke [n]` from the simulation and
  reports a depth failure as such; the loop then refuses that token's trades on that side for a
  day (`DEEP_ROUTE_SECS`) instead of failing every tick. The real depth of each route is to be
  measured on devnet once the programs are deployed.
- **R-6 IDLs** (done in app pass v3: generated codecs and builders): `set_directive`, `commit` and `post` and the `Directive`/`MemoConfig` accounts are
  not in the committed IDLs; `src/directive.ts` builds and decodes them by hand (Anchor
  discriminators, Borsh). Regenerate the IDLs and switch to `idlIx` when they land.
- No live run yet: no model key and no devnet deployment. Everything above runs with the stub
  model against a mocked chain.
