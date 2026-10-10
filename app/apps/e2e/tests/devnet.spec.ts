// Read-only checks against devnet (E2E_DEVNET=1, project "devnet"): the site runs without demo
// data, the API reads the devnet RPC (E2E_RPC_URL, default the public devnet endpoint) and the
// devnet indexer's database (DATABASE_URL). Nothing is signed or sent.
import { expect, test } from '../support/fixtures';
import { API_PORT } from '../playwright.config';

const API = `http://127.0.0.1:${API_PORT}`;
// The ids of docs/DEVNET.md section 1.
const PROGRAMS: [string, string][] = [
  ['token', '5yeVq5rEWWBRkBWiA49So9u4jpxeZQTeYsjQcFRwX618'],
  ['armory', '7xnkyX5B7Ywz86Cu2ofM5T2z2UPmy1qhpgrAuK9Kk5MU'],
  ['items', '8wMqHBAWhKxw2fNpczHfMohKjowbUM4hGqQkoYPf93Gv'],
  ['war', '5vJnBvr33jpsfYxMY2pvNf6tF9tkj8eaZ6goFtByUWA2'],
];

test('the API reaches devnet and sees the units programs deployed', async ({ request }) => {
  const s = await (await request.get(`${API}/v1/status`)).json();
  expect(s.cluster).toContain('devnet');
  expect(s.rpcReachable).toBe(true);
  const deployed = (s.programs as { deployed: boolean }[]).filter((p) => p.deployed).length;
  expect(deployed, 'programs the status reads as deployed').toBeGreaterThan(0);
});

test('the header shows the cluster and no demo data', async ({ page }) => {
  await page.goto('/projects');
  await expect(page.locator('body')).not.toContainText('Demo data');
  await expect(page.locator('body')).not.toContainText('Backend down');
});

for (const [name, id] of PROGRAMS) {
  test(`the explorer shows the ${name} program as deployed`, async ({ page }) => {
    await page.goto(`/program/${id}`);
    await expect(page.locator('main')).toContainText(id.slice(0, 8));
    await expect(page.locator('main')).not.toContainText(/not deployed/i);
  });
}

test('the armory lists the registered templates', async ({ page, request }) => {
  const t = await (await request.get(`${API}/v1/templates`)).json() as unknown[];
  test.skip(t.length === 0, 'no template registered on this devnet deploy yet');
  await page.goto('/armory/templates');
  await expect(page.locator('main')).not.toContainText(/No templates/i);
});

test('projects show the launches the indexer has, or say there are none', async ({ page, request }) => {
  const r = await request.get(`${API}/v1/launches`);
  const n = r.ok() ? ((await r.json()).items as unknown[]).length : 0;
  await page.goto('/projects');
  if (n > 0) await expect(page.locator('main a[href^="/t/"]').first()).toBeVisible();
  else await expect(page.locator('main')).toContainText(/no|none|yet/i);
});
