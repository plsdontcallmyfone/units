// Every page renders in demo mode: no console errors, no page errors, no failing same-origin
// request, no em dash, no monospace UI text, and no horizontal scroll at phone width.
import { readdirSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, pageFacts, test } from '../support/fixtures';
import { ROUTES } from '../support/routes';
import { KNOWN } from '../support/known';

const APP = fileURLToPath(new URL('../../web/app', import.meta.url));

function pageRoutes(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) { if (name !== 'api') out.push(...pageRoutes(p)); }
    else if (name === 'page.tsx') out.push('/' + relative(APP, dir).split('\\').join('/'));
  }
  return out.map((r) => (r === '/' ? r : r.replace(/\/$/, '')));
}

test('the route list covers every page file of the site', async ({}, info) => {
  test.skip(info.project.name !== 'desktop', 'one check is enough');
  const files = pageRoutes(APP).map((r) => (r === '/' ? '/' : r)).sort();
  expect(ROUTES.map(([r]) => r).sort()).toEqual(files);
});

for (const [route, url] of ROUTES) {
  test(`${route} renders cleanly`, async ({ page, problems }, info) => {
    const known = KNOWN[route]?.projects.includes(info.project.name) ? KNOWN[route] : undefined;
    if (known && !known.checks) test.fixme(true, known.reason);
    const skip = new Set(known?.checks ?? []);
    if (known?.checks) info.annotations.push({ type: 'known', description: known.reason });
    const res = await page.goto(url);
    expect(res?.status(), 'the page answers').toBeLessThan(400);
    await expect(page.getByRole('main').first()).toBeVisible();
    if (!skip.has('one-main')) expect.soft(await page.getByRole('main').count(), 'main landmarks').toBe(1);
    await expect(page.getByRole('navigation', { name: 'Primary' })).toBeVisible();
    // Let client reads and effects run.
    await page.waitForLoadState('networkidle', { timeout: 15_000 }).catch(() => undefined);
    const facts = await pageFacts(page);
    expect.soft(problems, 'console errors, page errors and failed requests').toEqual([]);
    expect.soft(facts.emDash, 'text with an em dash').toEqual([]);
    expect.soft(facts.mono, 'UI text in a monospace face').toEqual([]);
    if (info.project.name === 'mobile') expect.soft(facts.overflow, 'horizontal overflow at phone width (px)').toBeLessThanOrEqual(1);
  });
}
