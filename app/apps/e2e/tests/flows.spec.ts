// The site's forms in demo mode, end to end in the browser: a stand-in wallet connects, the form
// validates what is typed, POSTs its prepare body (answered here with a real unsigned transaction),
// "signs" and submits (answered here with a signature). Bodies are checked field by field, so a
// form that sends the wrong shape fails here before it fails on chain.
import type { Page } from '@playwright/test';
import { action, answerPrepare, expect, preparedTx, SIGNATURE, test, WALLET, type Captured } from '../support/fixtures';
import { ITEM, ITEM_MINT, MINT } from '../support/routes';

type Fill = Record<string, string | { select: string }>;

/** Connects the card's wallet, fills it, runs it, and returns the prepare body it sent. */
async function run(page: Page, title: string, fill: Fill, route: string, cta?: string): Promise<Record<string, unknown>> {
  const captured: Captured[] = [];
  await answerPrepare(page, route, captured);
  const card = action(page, title);
  await card.scrollIntoViewIfNeeded();
  await card.getByRole('button', { name: 'Connect wallet' }).click();
  await expect(card.getByText(`as ${WALLET.slice(0, 4)}...${WALLET.slice(-4)}`)).toBeVisible();
  for (const [label, v] of Object.entries(fill)) {
    const field = card.getByLabel(new RegExp(`^${label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`));
    if (typeof v === 'string') await field.fill(v); else await field.selectOption(v.select);
  }
  await card.getByRole('button', { name: cta ?? title, exact: true }).click();
  await expect(card.getByRole('status')).toHaveText(/Done in 1 transaction\./);
  await expect(card.getByRole('link', { name: `${SIGNATURE.slice(0, 8)}...${SIGNATURE.slice(-8)}` })).toBeVisible();
  expect(captured).toHaveLength(1);
  expect(captured[0]!.body.owner).toBe(WALLET);
  return captured[0]!.body;
}

/** Runs the card with one required field left empty and expects the form's own sentence, with no
 * prepare request sent. */
async function refuses(page: Page, title: string, fill: Fill, route: string, sentence: RegExp, cta?: string) {
  const captured: Captured[] = [];
  await answerPrepare(page, route, captured);
  const card = action(page, title);
  await card.getByRole('button', { name: 'Connect wallet' }).click();
  for (const [label, v] of Object.entries(fill)) {
    const field = card.getByLabel(new RegExp(`^${label}`));
    if (typeof v === 'string') await field.fill(v); else await field.selectOption(v.select);
  }
  await card.getByRole('button', { name: cta ?? title, exact: true }).click();
  await expect(card.getByRole('status')).toHaveText(sentence);
  expect(captured).toEqual([]);
}

test.describe('without a wallet', () => {
  test('a form says there is no wallet and offers no button to sign', async ({ page }) => {
    await page.goto('/craft');
    const card = action(page, 'Craft');
    await expect(card.getByText('No Solana wallet in this browser.')).toBeVisible();
    await expect(card.getByRole('button', { name: 'Craft', exact: true })).toHaveCount(0);
  });

  test('the launch asks for a wallet before it prepares', async ({ page }) => {
    await page.goto('/launch');
    await expect(page.getByRole('heading', { name: /Step 1 of 5: Token/ })).toBeVisible();
  });
});

test.describe('with a wallet', () => {
  test.beforeEach(async ({ wallet }) => { expect(wallet).toBe(WALLET); });

  test('launch: the staged form checks the token, prepares, simulates and signs the first step', async ({ page }) => {
    const plans: Record<string, unknown>[] = [];
    await page.route('**/api/v1/launch/plan', async (r) => {
      plans.push(r.request().postDataJSON() as Record<string, unknown>);
      await r.fulfill({ json: {
        transactions: [preparedTx('prepare_launch')],
        steps: [{ stage: 0, label: 'prepare_launch', instructions: [{ program: 'launch', name: 'prepare_launch', args: { name: 'E2E Token' } }], bytes: 640, packet: 1232, lookupTables: 0, signatures: 2, feeLamports: 10000, computeLimit: 200000, simulatedNow: true }],
        launchFeeLamports: null,
      } });
    });
    await page.route('**/api/v1/simulate', (r) => r.fulfill({ json: { ok: true, units: 51234, failure: null } }));
    await page.route('**/api/v1/submit', (r) => r.fulfill({ json: { signature: SIGNATURE } }));

    await page.goto('/launch');
    const next = page.getByRole('button', { name: 'Next' });
    await expect(next, 'an empty token cannot go on').toBeDisabled();
    await page.getByLabel(/^Name/).fill('E2E Token');
    await page.getByLabel(/^Ticker/).fill('E2E');
    const reserve = page.getByLabel(/^Virtual SOL reserve/);
    const hint = (await reserve.getAttribute('placeholder')) ?? '';
    const least = /^([\d.]+) SOL/.exec(hint)?.[1];
    expect(least, `the reserve hint names its floor: "${hint}"`).toBeTruthy();
    await reserve.fill(least!);
    await expect(next).toBeEnabled();
    for (const step of ['Rules', 'Slots', 'Review', 'Launch']) {
      await next.click();
      await expect(page.getByRole('heading', { name: new RegExp(`: ${step}$`) })).toBeVisible();
    }
    await page.getByRole('button', { name: 'Connect a wallet' }).click();
    await page.getByRole('button', { name: 'Prepare the first transactions' }).click();
    await expect(page.getByText('1. prepare_launch')).toBeVisible();
    expect(plans).toHaveLength(1);
    expect(plans[0]).toMatchObject({ owner: WALLET });
    expect(JSON.stringify(plans[0])).toContain('E2E Token');
    await page.getByRole('button', { name: 'Simulate, then sign' }).click();
    await expect(page.getByRole('button', { name: 'Prepare the launch transactions' })).toBeVisible();
  });

  test('token page: propose an item, vote, and a missing slot is refused', async ({ page }) => {
    await page.goto(`/t/${MINT}`);
    await refuses(page, 'Propose an item', { Item: ITEM }, 'proposals', /^Slot is required\.$/);
    await page.reload();
    const p = await run(page, 'Propose an item', { Slot: '2', Item: ITEM }, 'proposals');
    expect(p).toMatchObject({ slot: 2, item: ITEM });
    await page.reload();
    const v = await run(page, 'Vote', { Slot: '2', 'Proposal nonce': '1', For: { select: 'true' }, Tokens: '1000' }, 'votes');
    expect(v).toMatchObject({ slot: 2, nonce: '1', support: true, amount: '1000' });
  });

  test('token page: raid names the rival and this token as the target', async ({ page }) => {
    await page.goto(`/t/${MINT}`);
    const rival = 'Brn3dE5fG7hJ9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1f';
    const b = await run(page, 'Raid', { 'Rival token': rival, 'Rival tokens to sell': '5000' }, 'raid');
    expect(b).toMatchObject({ target: MINT, rival, amount: '5000' });
    expect(b).not.toHaveProperty('minOut');
  });

  test('marketplace: list an item in SOL, and a bad price is refused', async ({ page }) => {
    await page.goto('/marketplace');
    await refuses(page, 'List', { 'Item mint': ITEM_MINT, Price: '1.5.5', 'Expires at': '0' }, 'market/list', /^Price must be a SOL amount/, 'List item');
    await page.reload();
    const b = await run(page, 'List', { 'Item mint': ITEM_MINT, Price: '1.25', 'Expires at': '0' }, 'market/list', 'List item');
    expect(b).toMatchObject({ itemMint: ITEM_MINT, priceLamports: '1250000000', expiresAt: 0 });
  });

  test('marketplace: create a collection from template ids', async ({ page }) => {
    await page.goto('/marketplace');
    const b = await run(page, 'Create', { Name: 'E2E set', 'Template ids': '1, 2, 9' }, 'market/collections', 'Create collection');
    expect(b).toMatchObject({ name: 'E2E set', templateIds: [1, 2, 9] });
  });

  test('item page: set Licensed access with terms (the licence a token then buys)', async ({ page }) => {
    await page.goto(`/armory/items/${ITEM}`);
    const terms = '{"priceLamports":"100000000","termSecs":86400,"per":0,"maxLive":3}';
    const b = await run(page, 'Set access', { Mode: { select: '2' }, Exclusive: { select: 'false' }, 'Licence terms': terms }, 'access/set');
    expect(b).toMatchObject({ itemMint: ITEM_MINT, mode: 2, exclusive: false, licenceTerms: { priceLamports: '100000000', termSecs: 86400, per: 0, maxLive: 3 } });
  });

  test('item page: licence buy appears for a Licensed item', async ({ page }) => {
    test.fixme(true, 'Demo data has no Licensed item (lib/mock.ts answers every /v1/access read with mode 0), so the "Buy a licence" form never renders in demo mode; covered by the API test of licences/buy and the devnet project.');
    await page.goto(`/armory/items/${ITEM}`);
    await run(page, 'Buy a licence', { 'Your token mint': MINT, 'Renew a live licence': { select: 'false' } }, 'licences/buy');
  });

  test('craft: craft by recipe, and the order book places an order', async ({ page }) => {
    await page.goto('/craft');
    const c = await run(page, 'Craft', { Recipe: '1' }, 'craft');
    expect(c).toMatchObject({ recipeId: 1 });
    await page.goto('/book');
    const b = await run(page, 'Place an order', { Material: '1', Side: { select: 'bid' }, 'Price per unit': '41000', 'Size': '300', 'Post only': { select: 'true' } }, 'book/place');
    expect(b).toMatchObject({ materialId: 1, side: 'bid', price: '41000', size: '300', postOnly: true });
  });

  test('war: coalitions form names its members', async ({ page }) => {
    await page.goto('/war/coalitions');
    const members = `${MINT}, Brn3dE5fG7hJ9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1f`;
    const b = await run(page, 'Form a coalition', { 'Coalition id': '7', 'Term (seconds)': '604800', 'Member token mints': members }, 'war/coalition/form');
    expect(b).toMatchObject({ id: 7, termSecs: '604800', members: [MINT, 'Brn3dE5fG7hJ9kL2mN4pQ6rS8tU1vW3xY5zA7bC9dE1f'] });
  });

  test('war room: shows the map and the feed', async ({ page }) => {
    await page.goto('/war');
    await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
    await expect(page.locator('main')).toContainText(/Ashfall|ASH/);
  });

  test('agents: register a passport', async ({ page }) => {
    await page.goto('/agents');
    const b = await run(page, 'Register', { Name: 'e2e agent', Kinds: '3' }, 'agents/register', 'Register passport');
    expect(b).toMatchObject({ name: 'e2e agent', kinds: 3 });
    expect(b).not.toHaveProperty('avatarUri');
  });

  test('governance: queue an admin action by its hash', async ({ page }) => {
    await page.goto('/governance');
    const hash = 'ab'.repeat(32);
    const b = await run(page, 'Queue an admin action', { 'Action hash': hash }, 'armory/queue');
    expect(b).toMatchObject({ actionHash: hash });
  });
});

test.describe('reads', () => {
  test('explorer: search finds a demo transaction and opens it', async ({ page }) => {
    await page.goto('/explorer');
    const box = page.getByRole('search').getByLabel('Search');
    const tx = '5d5bPuhmTKFRmYygzbkpZUv1QotFq4qtJxhXPEDP26m9rh4xXbXzVcds1EHVpv6xJrtfuPtGhryAcjtUxuSsYpfH';
    await box.fill(tx);
    await box.press('Enter');
    await expect(page).toHaveURL(new RegExp(`/tx/${tx}$`));
    await expect(page.locator('main')).toContainText(tx.slice(0, 8));
  });

  test('docs: the guide opens a page from its index', async ({ page }) => {
    await page.goto('/docs');
    await page.locator('main').getByRole('link', { name: 'How a trade runs' }).first().click();
    await expect(page).toHaveURL(/\/docs\/concepts\/how-a-trade-runs$/);
    await expect(page.getByRole('heading', { level: 1, name: 'How a trade runs' })).toBeVisible();
  });

  test('a token page shows its slots', async ({ page }) => {
    await page.goto(`/t/${MINT}`);
    await expect(page.locator('main')).toContainText('Ashfall');
    await expect(page.locator('main')).toContainText(/Slot/);
  });
});
