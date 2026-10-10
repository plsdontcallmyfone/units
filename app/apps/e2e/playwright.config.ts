// End-to-end tests of the units site (app/apps/web) and API (app/apps/api) with Playwright Test.
// Default projects run the site in demo mode (MOCK_DATA=1) against a local API; the opt-in devnet
// project (E2E_DEVNET=1) runs read-only checks against the devnet configuration. Ports come from
// the app's block 9960 to 9969 (CLAUDE.md); 9964 and 9965 by default so a developer's site on 9960
// and API on 9961 keep running.
import { execSync } from 'node:child_process';
import { defineConfig, devices } from '@playwright/test';

const CI = Boolean(process.env.CI);
const DEVNET = process.env.E2E_DEVNET === '1';
export const WEB_PORT = Number(process.env.E2E_WEB_PORT ?? 9964);
export const API_PORT = Number(process.env.E2E_API_PORT ?? 9965);
const REUSE = process.env.E2E_REUSE === '1';

// Ports are owned, not free: refuse to start on a port something else already holds, rather than
// testing (or killing) another session's server.
function owner(port: number): string {
  try { return execSync(`lsof -ti tcp:${port} -sTCP:LISTEN`, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] }).trim(); } catch { return ''; }
}
// Only the runner checks: worker processes load this file again while the servers are up.
if (!REUSE && process.env.TEST_WORKER_INDEX === undefined) {
  for (const port of [WEB_PORT, API_PORT]) {
    const pids = owner(port);
    if (pids) throw new Error(`Port ${port} is already in use by pid ${pids.split('\n').join(', ')}. Pick another with E2E_WEB_PORT / E2E_API_PORT (9960 to 9969), or set E2E_REUSE=1 to test the servers already there.`);
  }
}

const api = `http://127.0.0.1:${API_PORT}`;
const web = `http://127.0.0.1:${WEB_PORT}`;

export default defineConfig({
  testDir: './tests',
  fullyParallel: true,
  forbidOnly: CI,
  retries: CI ? 1 : 0,
  workers: CI ? 2 : undefined,
  timeout: 60_000,
  expect: { timeout: 10_000 },
  reporter: [['list'], ['html', { outputFolder: 'playwright-report', open: 'never' }]],
  outputDir: 'test-results',
  use: {
    baseURL: web,
    trace: 'on-first-retry',
    screenshot: 'only-on-failure',
  },
  projects: DEVNET
    ? [{ name: 'devnet', testMatch: /devnet\.spec\.ts/, use: { ...devices['Desktop Chrome'] } }]
    : [
        { name: 'desktop', testIgnore: /devnet\.spec\.ts/, use: { ...devices['Desktop Chrome'] } },
        { name: 'mobile', testIgnore: /(devnet|api)\.spec\.ts/, use: { ...devices['Pixel 7'] } },
      ],
  webServer: [
    {
      name: 'api',
      command: 'node ../api/src/main.ts',
      url: `${api}/v1/status`,
      reuseExistingServer: REUSE,
      timeout: 60_000,
      env: {
        PORT: String(API_PORT),
        HOST: '127.0.0.1',
        // DATABASE_URL comes from the environment (server B: /root/hw-appv2-db.url).
        ...(process.env.DATABASE_URL ? { DATABASE_URL: process.env.DATABASE_URL } : {}),
        RPC_URL: process.env.E2E_RPC_URL ?? 'https://api.devnet.solana.com',
        // Every page read comes from one client (the site's server) and the suite opens many pages
        // at once; the limiter itself is unit tested in apps/api (guard.test.ts).
        RATE_READ_CAPACITY: '100000', RATE_READ_PER_SEC: '10000',
        RATE_PREPARE_CAPACITY: '100000', RATE_PREPARE_PER_SEC: '10000',
      },
      stdout: 'ignore',
      stderr: 'pipe',
    },
    {
      name: 'web',
      // Built by `pnpm build:web` first (test:e2e does it); the production server is what ships.
      command: `pnpm -C ../web exec next start -p ${WEB_PORT} -H 127.0.0.1`,
      url: web,
      reuseExistingServer: REUSE,
      timeout: 120_000,
      env: { API_URL: api, ...(DEVNET ? {} : { MOCK_DATA: '1' }) },
      stdout: 'ignore',
      stderr: 'pipe',
    },
  ],
});
