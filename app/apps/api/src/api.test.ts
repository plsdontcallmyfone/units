// Changed by Hookwars: DATABASE_URL required, no default credentials (app audit A-9).
/** Starts the API on an ephemeral port against the server's Postgres (fresh schema, empty) and a
 * mocked RPC, and checks every read route answers with its empty shape and prepares refuse with a
 * sentence while the programs are not deployed. */
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { AddressInfo } from 'node:net';
import pg from 'pg';
import { allDdl } from '@hookwars/indexer/schema.ts';
import { findBannedWords } from '@hookwars/shared';
import { serve } from './server.ts';
import { writeTransaction } from '@hookwars/indexer/indexer.ts';
import { badges, lineageOf } from './reads-expansion.ts';

/** A real Postgres from DATABASE_URL (no default credentials in code, app audit A-9); skipped without one. */
const url = process.env.DATABASE_URL ?? '';
const withDb = url ? describe : describe.skip;
const schema = `apitest_${process.pid}`;
let db: pg.Pool; let base = ''; let server: ReturnType<typeof serve>;

const conn = {
  getSlot: async () => 123,
  getAccountInfo: async () => null,
  getMultipleAccountsInfo: async (keys: unknown[]) => keys.map(() => null),
  getProgramAccounts: async () => [],
} as never;

beforeAll(async () => {
  if (!url) return;
  const admin = new pg.Pool({ connectionString: url, max: 1 });
  await admin.query(`create schema if not exists ${schema}`); await admin.end();
  db = new pg.Pool({ connectionString: url, max: 2, options: `-c search_path=${schema}` });
  for (const s of allDdl()) await db.query(s);
  server = serve({ db, conn, rpcUrl: 'mock' }, 0);
  await new Promise((r) => server.once('listening', r));
  base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
});
afterAll(async () => {
  if (!url) return;
  server.close();
  await db.query(`drop schema if exists ${schema} cascade`);
  await db.end();
});

const get = async (p: string) => { const r = await fetch(base + p); return { status: r.status, body: await r.json() }; };
const M = 'So11111111111111111111111111111111111111112';

withDb('reads with an empty database', () => {
  it('a malformed key in a path is a 400 with a sentence on every key route, never a 500', async () => {
    for (const bad of ['not-a-key', 'O0Il' + '1'.repeat(40), '1'.repeat(60)]) {
      for (const path of [
        `/v1/agents/${bad}`, `/v1/agents/${bad}/timeline`, `/v1/u/${bad}`, `/v1/social/follows/${bad}`,
        `/v1/market/items/${bad}`, `/v1/items/${bad}`, `/v1/items/${bad}/wear`, `/v1/commissions/${bad}`, `/v1/access/${bad}`,
        `/v1/launches/${bad}/slots`, `/v1/launches/${bad}/proposals`, `/v1/launches/${bad}/generals`, `/v1/launches/${bad}/treaties`,
        `/v1/launches/${bad}/war`, `/v1/wallet/${bad}/war`,
      ]) {
        const r = await get(path);
        expect(r.status, path).toBe(400);
        expect(r.body.error, path).toEqual(expect.stringMatching(/is not an address/));
      }
    }
  });
  it('answers every route with its empty shape', async () => {
    expect((await get('/v1/status')).body).toMatchObject({ rpcReachable: true, slot: 123, database: true });
    expect((await get('/v1/templates')).body).toEqual([]);
    expect((await get('/v1/items')).body).toEqual({ items: [], next: null });
    expect((await get('/v1/items/' + M)).status).toBe(404);
    expect((await get(`/v1/launches/${M}/slots`)).body).toEqual([]);
    expect((await get(`/v1/launches/${M}/proposals`)).body).toEqual([]);
    expect((await get(`/v1/launches/${M}/generals`)).body).toEqual([]);
    expect((await get(`/v1/launches/${M}/war`)).body).toBeNull();
    expect((await get('/v1/map')).body).toEqual({ nodes: [], edges: [] });
    expect((await get('/v1/feed')).body).toEqual({ items: [], next: null });
    expect((await get('/v1/seasons/current')).body).toBeNull();
    expect((await get('/v1/prize-vault')).body).toMatchObject({ lamports: null, lastWinner: null });
    expect((await get(`/v1/wallet/${M}/war`)).body).toEqual({ holdings: [], items: [], rolls: [] });
    expect((await get('/v1/cards/abc/0.png')).status).toBe(404);
    // Agents, market and social (app v2): empty lists, and null for a missing account.
    expect((await get('/v1/agents')).body).toMatchObject({ sort: 'itemsAuthored', items: [] });
    for (const p of ['/v1/market/listings', '/v1/market/leases', '/v1/market/collections', '/v1/commissions', '/v1/guilds']) expect((await get(p)).body, p).toEqual({ items: [] });
    expect((await get('/v1/badges')).body).toEqual({ items: [], recent: [] });
    for (const p of [`/v1/agents/${M}`, `/v1/market/items/${M}`, `/v1/commissions/${M}`, '/v1/guilds/0']) expect((await get(p)), p).toEqual({ status: 200, body: null });
    const quests = (await get('/v1/quests')).body as { sentence: string }[];
    expect(quests).toHaveLength(2);
    for (const q of quests) expect(findBannedWords(q.sentence)).toEqual([]);
  });
});

withDb('prepares before deployment', () => {
  it('refuse with one sentence naming the missing programs', async () => {
    const r = await fetch(base + '/v1/votes/prepare', { method: 'POST', body: JSON.stringify({ owner: M, mint: M, slot: 1, nonce: 0, support: true, amount: 1 }) });
    expect(r.status).toBe(409);
    const b = await r.json() as { error: string; code: string };
    expect(b.code).toBe('NotDeployed');
    expect(b.error).toMatch(/armory, token programs are not deployed/);
  });
});

withDb('market history from indexed events (app v2)', () => {
  it('lineage walks Forged events in Postgres; sales come from Sold only; badge awards list', async () => {
    const A = 'A'.repeat(43) + '1', B = 'B'.repeat(43) + '1', C = 'C'.repeat(43) + '1', D = 'D'.repeat(43) + '1';
    const c = await db.connect();
    await writeTransaction(c, 'sigF1', 10, 1_700_000_000, [{ ordinal: 0, program: 'armory', programId: '', name: 'Forged', via: 'cpi', data: { burned: [A, B], item: C, templateId: 1, params: [1], level: 2, forger: M, ts: '5' } }], false, ['armory']);
    await writeTransaction(c, 'sigF2', 11, 1_700_000_001, [{ ordinal: 0, program: 'armory', programId: '', name: 'Forged', via: 'cpi', data: { burned: [C, A], item: D, templateId: 1, params: [1], level: 3, forger: M, ts: '6' } }], false, ['armory']);
    await writeTransaction(c, 'sigS', 12, 1_700_000_002, [{ ordinal: 0, program: 'market', programId: '', name: 'Sold', via: 'cpi', data: { item: D, itemMint: B, seller: M, buyer: M, price: '100', fee: '1', resale: '2', ts: '7' } }], false, ['market']);
    await writeTransaction(c, 'sigB', 13, 1_700_000_003, [{ ordinal: 0, program: 'social', programId: '', name: 'BadgeAwarded', via: 'cpi', data: { id: 1, recipient: M, claimant: M, ts: '8' } }], false, ['social']);
    c.release();
    const l = await lineageOf(db, D);
    expect(l.level).toBe(3);
    expect(l.parents.map((p) => (p as { item: string }).item)).toEqual([C, A]);
    expect((l.parents[0] as { parents: { item: string }[] }).parents.map((p) => p.item)).toEqual([A, B]);
    expect((await lineageOf(db, C)).children).toEqual([D]);
    const sales = await db.query('select price from ev_market_sold where item_mint = $1', [B]);
    expect(sales.rows.map((r) => String(r.price))).toEqual(['100']);
    expect(((await badges(conn, db)) as { recent: unknown[] }).recent).toHaveLength(1); // the route caches 15 s, so the read itself
  });
});
