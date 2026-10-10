// Accessibility smoke with axe-core on the main pages. Serious and critical violations are checked
// against a recorded baseline (support/a11y-baseline.json): a page that gains a new one fails, the
// ones already recorded are listed in the report and in README.md until the pages are fixed. Run
// with E2E_A11Y_RECORD=1 to rewrite the baseline after a fix.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '../support/fixtures';
import { ITEM, MINT } from '../support/routes';

const PAGES = ['/projects', '/launch', `/t/${MINT}`, '/war', '/armory', `/armory/items/${ITEM}`, '/marketplace', '/craft', '/book', '/agents', '/governance', '/explorer', '/docs'];
const FILE = fileURLToPath(new URL('../support/a11y-baseline.json', import.meta.url));
const RECORD = process.env.E2E_A11Y_RECORD === '1';

type Baseline = Record<string, string[]>;
const baseline: Baseline = (() => { try { return JSON.parse(readFileSync(FILE, 'utf8')) as Baseline; } catch { return {}; } })();

test.describe.configure({ mode: 'serial' });
test.skip(({ isMobile }) => isMobile, 'one browser is enough for axe');

const found: Baseline = {};
for (const path of PAGES) {
  test(`axe: ${path}`, async ({ page }, info) => {
    await page.goto(path);
    await page.waitForLoadState('networkidle', { timeout: 15_000 }).catch(() => undefined);
    const r = await new AxeBuilder({ page }).withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa']).analyze();
    const bad = r.violations.filter((v) => v.impact === 'serious' || v.impact === 'critical');
    const ids = [...new Set(bad.map((v) => v.id))].sort();
    found[path.replace(MINT, '[mint]').replace(ITEM, '[item]')] = ids;
    await info.attach('axe-violations.json', { body: JSON.stringify(bad.map((v) => ({ id: v.id, impact: v.impact, help: v.help, nodes: v.nodes.length, sample: v.nodes[0]?.target })), null, 2), contentType: 'application/json' });
    if (RECORD) return;
    const known = new Set(baseline[path.replace(MINT, '[mint]').replace(ITEM, '[item]')] ?? []);
    expect(ids.filter((id) => !known.has(id)), 'serious or critical axe violations not in the baseline').toEqual([]);
  });
}

test.afterAll(() => {
  if (RECORD) writeFileSync(FILE, JSON.stringify({ ...baseline, ...found }, null, 2) + '\n');
});
