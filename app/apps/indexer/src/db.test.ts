// Changed by Hookwars: skipped without DATABASE_URL (app audit A-9).
/**
 * Against a real Postgres (DATABASE_URL, the server's hookwars_app by default): migrations apply,
 * writing one transaction twice yields the same rows (idempotent), state handlers fill slots,
 * hook data and items. Uses a scratch schema and drops it after.
 */
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import pg from 'pg';
import { Keypair } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { allDdl } from './schema.ts';
import { writeTransaction } from './indexer.ts';

/** A real Postgres from DATABASE_URL (no default credentials in code, app audit A-9); skipped without one. */
const url = process.env.DATABASE_URL ?? '';
const withDb = url ? describe : describe.skip;
const schema = `test_${process.pid}`;
let db: pg.Pool;
let reachable = true;

beforeAll(async () => {
  if (!url) return;
  db = new pg.Pool({ connectionString: url, max: 1, options: `-c search_path=${schema}` });
  try {
    await db.query(`create schema if not exists ${schema}`);
    for (const s of allDdl()) await db.query(s);
  } catch (e) {
    reachable = false;
    if (process.env.SKIP_DB !== '1') throw e;
  }
});
afterAll(async () => {
  if (!url) return;
  if (reachable) await db.query(`drop schema if exists ${schema} cascade`);
  await db.end();
});

const k = () => Keypair.generate().publicKey.toBase58();

describe.runIf(process.env.SKIP_DB !== '1')('indexer writes', () => {
  it('is idempotent and fills state', async () => {
    if (!reachable) return;
    const mint = k(), item = k(), itemMint = k(), author = k(), holding = k(), owner = k();
    const events: hookwars.TxEvent[] = [
      { ordinal: 0, program: 'token', programId: '', name: 'SlotsInitialized', via: 'cpi', data: { mint, slotAuthority: k(), slots: [{ kind: 5, equipRule: 0, maxCutBps: 0, mayRefuse: false, mayWriteData: true, mayAnswerTouch: false, dataOffset: 0, dataLen: 32, equipVault: k(), lockedProgram: k() }] } },
      { ordinal: 1, program: 'token', programId: '', name: 'HookDataWritten', via: 'cpi', data: { mint, holding, owner, data: '0x' + '00'.repeat(64) } },
      { ordinal: 2, program: 'armory', programId: '', name: 'ItemCreated', via: 'cpi', data: { item, itemMint, templateId: 1, params: [1, 2, 3], manifest: { kind: 4 }, author, royaltyBps: 100, level: 1, source: 0, ts: '5' } },
      { ordinal: 3, program: 'items', programId: '', name: 'RaidMarked', via: 'log', data: { mint, rival: k(), trader: owner, volume: '1000', points: 3, lootTicket: true } },
    ];
    for (let i = 0; i < 2; i++) {
      const c = await db.connect();
      await writeTransaction(c, 'sigA', 100, 1_700_000_000, events, false, ['token']);
      c.release();
    }
    const n = await db.query('select count(*)::int as n from events');
    expect(n.rows[0].n).toBe(4);
    expect((await db.query('select count(*)::int as n from raids')).rows[0].n).toBe(1);
    expect((await db.query('select data_len from slots where mint = $1', [mint])).rows[0].data_len).toBe(32);
    expect((await db.query('select owner from item_owners where item_mint = $1', [itemMint])).rows[0].owner).toBe(author);
    expect((await db.query('select source from items where item = $1', [item])).rows[0].source).toBe('authored');
  });
});
