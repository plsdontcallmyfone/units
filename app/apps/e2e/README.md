# units end-to-end tests

Playwright Test (`@playwright/test` 1.64.0, pinned) runs the site (`apps/web`) and the API
(`apps/api`) as they ship, and drives them in Chromium on a desktop and a phone profile (Pixel 7).
Axe-core (`@axe-core/playwright` 4.13.0, pinned) checks accessibility on the main pages.

It is not part of `pnpm -r test` (it needs a browser and two servers); run it on its own.

## What it checks

| File | What |
| --- | --- |
| `tests/routes.spec.ts` | Every page file of the site (the list is checked against `apps/web/app`) renders in demo mode with no console error, page error or failing same-origin request, no em dash, no monospace UI text (code blocks excepted), and no horizontal scroll at phone width |
| `tests/nav.spec.ts` | The header menus open, close on Escape and lead to their pages; every menu link is a checked page; the phone menu sheet |
| `tests/flows.spec.ts` | The forms in the browser with a stand-in wallet: the staged launch (checks, plan, simulate, sign), propose and vote, raid, marketplace list and collections, item access (Licensed terms), craft, the order book, coalitions, agent registration, the governance queue; each form's validation sentence; the request body each form sends, field by field; explorer search, docs, token page reads; the no-wallet states |
| `tests/api.spec.ts` | The API's reads and prepares with the request fixture: shapes the pages depend on, 4xx with a sentence (never a 500 or a stack) for bad input, and the site proxy's path allow-list and body cap |
| `tests/a11y.spec.ts` | Axe on 13 main pages; a serious or critical violation not in `support/a11y-baseline.json` fails |
| `tests/devnet.spec.ts` | Opt-in, read-only, against devnet: the API reaches devnet and sees the programs deployed, the explorer shows them, the armory and projects show what the indexer has; nothing is signed or sent |

The stand-in wallet (`support/fixtures.ts`) is injected as `window.solana`. It holds no key that
exists anywhere: "signing" returns the transactions unchanged, and the test answers the site's
`/api/v1/<route>/prepare` and `/api/v1/submit` routes itself, with a real unsigned v0 transaction
and a signature. Nothing reaches a cluster from the default projects.

## Run it

From `app/`:

```sh
pnpm install
pnpm -C apps/e2e exec playwright install --with-deps chromium   # once per machine
DATABASE_URL=postgres://... pnpm -C apps/e2e test:e2e             # builds the site, then runs
pnpm -C apps/e2e test:e2e:only                                    # without rebuilding the site
pnpm -C apps/e2e report                                           # the HTML report
```

The suite starts the API on 9965 and the site on 9964 (the app's port block is 9960 to 9969). It
refuses to start when either port is taken; choose others with `E2E_WEB_PORT` and `E2E_API_PORT`,
or set `E2E_REUSE=1` to test servers that are already running there. `E2E_RPC_URL` sets the API's
RPC (default the public devnet endpoint). Set `CI=1` for one retry and traces on the retry.

Devnet, read-only (the API reads devnet and the devnet indexer's database):

```sh
DATABASE_URL=<devnet indexer db> E2E_RPC_URL=https://api.devnet.solana.com pnpm -C apps/e2e test:e2e:devnet
```

On build server B (`ssh -i ~/.ssh/hookwars_build root@192.153.57.190`), inside the build lock:

```sh
cd /root/<your copy>/app
flock /root/build.lock bash -lc 'export DATABASE_URL=$(cat /root/hw-appv2-db.url); pnpm install --frozen-lockfile && pnpm -C apps/e2e exec playwright install --with-deps chromium && CI=1 pnpm -C apps/e2e test:e2e'
```

After fixing an accessibility finding, rewrite the baseline with
`E2E_A11Y_RECORD=1 pnpm -C apps/e2e test:e2e:only --project=desktop tests/a11y.spec.ts`.

## Known failures

Real app bugs the suite found. Each is a `test.fixme` with its reason (in `support/known.ts` for
the route checks, inline for the others); fix the page, remove the entry, and the test must pass.

| Test | Reason |
| --- | --- |
| nav: the pages added in app pass 5 are in the menu | The header (`components/site-header.tsx`) has no entry for `/governance` or `/war/coalitions`; app pass 5 added them to `components/nav.tsx`, which the layout no longer renders |
| flows: licence buy appears for a Licensed item | Demo data has no Licensed item (`lib/mock.ts` answers every `/v1/access` read with mode 0), so the form never renders in demo mode; covered by the API test of `licences/buy` and the devnet project |
