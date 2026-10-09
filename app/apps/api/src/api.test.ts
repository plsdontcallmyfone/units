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

/** A real Postgres from DATABASE_URL (no default credentials in code, app audit A-9); skipped without one. */
const url = process.env.DATABASE_URL ?? '';
const withDb = url ? describe : describe.skip;
const schema = `apitest_${process.pid}`;
let db: pg.Pool; let base = ''; let server: ReturnType<typeof serve>;

const conn = {
  getSlot: async () => 123,
  getAccountInfo: async () => null,
  getMultipleAccountsInfo: async (keys: unknown[]) => keys.map(() => null),
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
