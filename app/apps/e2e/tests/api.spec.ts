// The API the pages read, called directly with the request fixture, and through the site's /api
// proxy. Shapes are checked where the pages depend on them; failures must be a sentence for people
// and a 4xx, never a 500 or a stack trace.
import { expect, test } from '@playwright/test';
import { API_PORT } from '../playwright.config';
import { WALLET } from '../support/fixtures';
import { ITEM_MINT, MINT } from '../support/routes';

const API = `http://127.0.0.1:${API_PORT}`;

test.describe('API reads', () => {
  test('status names the cluster and every units program', async ({ request }) => {
    const r = await request.get(`${API}/v1/status`);
    expect(r.ok()).toBeTruthy();
    const s = await r.json();
    expect(typeof s.cluster).toBe('string');
    expect(typeof s.rpcReachable).toBe('boolean');
    expect(Array.isArray(s.programs)).toBe(true);
    for (const p of s.programs) expect(p).toEqual(expect.objectContaining({ deployed: expect.any(Boolean) }));
  });

  for (const path of ['/v1/templates', '/v1/params', '/v1/quests', '/v1/launches', '/v1/feed', '/v1/items', '/v1/map', '/v1/market/listings', '/v1/market/collections', '/v1/market/leases', '/v1/agents', '/v1/guilds', '/v1/badges', '/v1/commissions', '/v1/craft', '/v1/book', '/v1/governance/queue', '/v1/templates/submissions', '/v1/war/coalitions', '/v1/launch/config', '/v1/explorer/programs']) {
    // Reads that need the RPC can fail when the public devnet endpoint rate-limits; the page then
    // shows the API's sentence, so what is checked is that a failure is JSON with a sentence and a
    // request id, never a stack trace.
    test(`GET ${path} answers JSON`, async ({ request }) => {
      const r = await request.get(`${API}${path}`);
      expect(r.headers()['content-type']).toContain('application/json');
      const text = await r.text();
      expect(text).not.toMatch(/at \w+ \(|node:internal|\.ts:\d+/);
      if (!r.ok()) {
        const b = JSON.parse(text) as { error?: unknown; requestId?: unknown };
        expect(b.error, 'a failed read says why').toEqual(expect.any(String));
        if (r.status() >= 500) expect(b.requestId, 'a server failure carries a request id for the log').toEqual(expect.any(String));
      }
    });
  }

  test('paged lists carry items and a cursor', async ({ request }) => {
    for (const path of ['/v1/launches', '/v1/feed', '/v1/items']) {
      const r = await request.get(`${API}${path}`);
      if (!r.ok()) continue;
      const b = await r.json();
      expect(Array.isArray(b.items), path).toBe(true);
      expect(b, path).toHaveProperty('next');
    }
  });

  test('an unknown route is a 404 with a sentence', async ({ request }) => {
    const r = await request.get(`${API}/v1/no-such-thing`);
    expect(r.status()).toBe(404);
    expect((await r.json()).error).toEqual(expect.any(String));
  });

  test('a malformed key in a path is a 400, not a server error', async ({ request }) => {
    const r = await request.get(`${API}/v1/agents/not-a-key`);
    expect(r.status()).toBe(400);
    expect((await r.json()).error).toMatch(/is not an address/);
  });

  test('a write method on a read route is refused', async ({ request }) => {
    const r = await request.delete(`${API}/v1/templates`);
    expect(r.status()).toBe(405);
  });
});

test.describe('API prepares', () => {
  test('a prepare without an owner is a 400 with a sentence', async ({ request }) => {
    const r = await request.post(`${API}/v1/market/list/prepare`, { data: { itemMint: ITEM_MINT, priceLamports: '1000', expiresAt: 0 } });
    expect(r.status()).toBeGreaterThanOrEqual(400);
    expect(r.status()).toBeLessThan(500);
    const b = await r.json();
    expect(b.error).toEqual(expect.any(String));
    expect(JSON.stringify(b)).not.toMatch(/at \w+ \(|node:internal|\.ts:\d+/);
  });

  test('a prepare for an item the chain does not hold fails with a sentence, not a crash', async ({ request }) => {
    for (const [route, body] of [
      ['licences/buy', { owner: WALLET, itemMint: ITEM_MINT, tokenMint: MINT, renew: false }],
      ['market/buy', { owner: WALLET, itemMint: ITEM_MINT, maxPrice: '1' }],
      ['proposals', { owner: WALLET, mint: MINT, slot: 1, targets: [] }],
    ] as const) {
      const r = await request.post(`${API}/v1/${route}/prepare`, { data: body });
      expect(r.status(), `${route}: ${await r.text()}`).toBeLessThan(500);
      if (!r.ok()) expect((await r.json()).error, route).toEqual(expect.any(String));
    }
  });

  test('an unknown prepare is refused', async ({ request }) => {
    const r = await request.post(`${API}/v1/no-such-action/prepare`, { data: { owner: WALLET } });
    expect(r.status()).toBeGreaterThanOrEqual(400);
    expect(r.status()).toBeLessThan(500);
  });

  test('submit refuses a body that is not a transaction', async ({ request }) => {
    const r = await request.post(`${API}/v1/submit`, { data: { transaction: 'not base64 of anything' } });
    expect(r.status()).toBeGreaterThanOrEqual(400);
    expect(r.status()).toBeLessThan(500);
  });
});

test.describe('the site proxy', () => {
  test('forwards v1 reads', async ({ request, baseURL }) => {
    const r = await request.get(`${baseURL}/api/v1/params`);
    expect(r.ok()).toBeTruthy();
  });

  test('forwards nothing outside v1', async ({ request, baseURL }) => {
    const r = await request.get(`${baseURL}/api/admin`);
    expect(r.status()).toBe(404);
  });

  test('refuses an oversized body', async ({ request, baseURL }) => {
    const r = await request.post(`${baseURL}/api/v1/market/list/prepare`, { data: { owner: WALLET, pad: 'x'.repeat(100_000) } });
    expect(r.status()).toBe(413);
  });
});
