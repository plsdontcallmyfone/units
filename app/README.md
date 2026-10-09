# units app

Everything off chain for units: the SDK, shared types and math, the indexer, the API, the site and
the alert bots (docs/spec/06-app.md). Internal package names still say `hookwars`; the product name
is units.

| Path | What | Run |
| --- | --- | --- |
| `packages/shared` | API types, parameters, templates and their site sentences, math, hook-data decoding | `pnpm -C packages/shared test` |
| `packages/sdk` | the upstream SDK with units program ids, plus `hookwars.*`: decoders, events, builders, slot slices | `pnpm -C packages/sdk test` |
| `apps/indexer` | Postgres schema, per-program cursors, self-CPI and log events, state tables | `node src/main.ts migrate`, `once`, `run` |
| `apps/api` | read routes and prepares (06 3.3) | `node src/main.ts` (PORT, RPC_URL, DATABASE_URL) |
| `apps/web` | the site (Next.js) | `next build`, `next start` (API_URL) |
| `apps/bots` | Telegram and X alerts, disabled unless tokens are set | `node src/main.ts` |

Node 22 runs the TypeScript sources directly (type stripping). `INTEGRATION.md` lists what must be
checked once the programs merge. Derived from the Bordrless SDK (Apache-2.0); see NOTICE.
