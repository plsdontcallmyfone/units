// Changed by Hookwars: new file, the social routes on a real Postgres (seeded through the indexer's own writers) and the memo prepares.
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import type { AddressInfo } from 'node:net';
import pg from 'pg';
import { Keypair, VersionedTransaction } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { findBannedWords } from '@hookwars/shared';
import { allDdl } from '@hookwars/indexer/schema.ts';
import { writeTransaction } from '@hookwars/indexer/indexer.ts';
import { decodeMemo, memosOf, MEMO_PROGRAM, type TxView } from '@hookwars/indexer/memos.ts';
import { passportLookup, writeMemos } from '@hookwars/indexer/social.ts';
import { prepare } from './prepares.ts';
import { serve } from './server.ts';
import { decodeSkills, level } from './social.ts';

const k = () => Keypair.generate().publicKey.toBase58();
const S = (n: number) => String(n).repeat(88).slice(0, 88);

const conn = {
  getSlot: async () => 123,
  getAccountInfo: async () => null,
  getMultipleAccountsInfo: async (keys: unknown[]) => keys.map(() => null),
  getProgramAccounts: async () => [],
  getLatestBlockhash: async () => ({ blockhash: '11111111111111111111111111111111', lastValidBlockHeight: 1 }),
  simulateTransaction: async () => ({ value: { err: null, logs: [], unitsConsumed: 1000 } }),
} as never;

function tx(signers: string[], text: string, sig: string, slot: number): TxView {
  const keys = [...signers, MEMO_PROGRAM];
  return { signature: sig, slot, blockTime: 1_700_000_000 + slot, keys, numSigners: signers.length, instructions: [{ programIdIndex: keys.length - 1, accounts: [0], data: Buffer.from(text) }] };
}

describe('levels and the skill table', () => {
  it('level() matches hookwars_common::economy::level', () => {
    expect(level([1n, 3n, 10n, 0n, 0n, 0n, 0n, 0n], 0n)).toBe(0);
    expect(level([1n, 3n, 10n, 0n, 0n, 0n, 0n, 0n], 3n)).toBe(2);
    expect(level([1n, 3n, 10n, 0n, 0n, 0n, 0n, 0n], 99n)).toBe(3);
    const d = Buffer.alloc(8 + 2 + 4 + 66);
    d.writeUInt32LE(1, 10); d[14] = 1; d[15] = 5; d.writeBigUInt64LE(1n, 16); d.writeBigUInt64LE(3n, 24);
    expect(decodeSkills(d)).toEqual([{ id: 1, counter: 5, thresholds: [1n, 3n, 0n, 0n, 0n, 0n, 0n, 0n] }]);
  });
});

describe('memo prepares', () => {
  it('builds one signed memo per action and refuses what the indexer would refuse', async () => {
    const owner = Keypair.generate().publicKey.toBase58();
    const r = await prepare(conn, 'social/post/prepare', { owner, text: 'gm', model: 'model-x' });
    const t = VersionedTransaction.deserialize(Buffer.from(r.transactions[0]!.transaction, 'base64'));
    const keys = t.message.staticAccountKeys.map((x) => x.toBase58());
    const memo = t.message.compiledInstructions.find((ix) => keys[ix.programIdIndex] === MEMO_PROGRAM)!;
    expect(Buffer.from(memo.data).toString()).toBe(`{"u":1,"k":"status","f":"${owner}","t":"*","th":"","re":"","b":{"text":"gm","model":"model-x"},"x":0}`);
    expect(memo.accountKeyIndexes.map((i) => keys[i])).toEqual([owner]);
    await expect(prepare(conn, 'social/follow/prepare', { owner, target: 'nope' })).rejects.toThrow('not an address');
    await expect(prepare(conn, 'social/react/prepare', { owner, ref: `${S(1)}:0`, reaction: 'love' })).rejects.toThrow('must be one of');
    await expect(prepare(conn, 'social/hide/prepare', { owner, ref: `${S(1)}:0`, reason: 'x' })).rejects.toThrow('social admin');
    await expect(prepare(conn, 'social/post/prepare', { owner, text: 'x', as: k() })).rejects.toThrow('No passport');
    const react = await prepare(conn, 'social/react/prepare', { owner, ref: `${S(1)}:0`, reaction: 'useful' });
    expect(react.transactions).toHaveLength(1);
  });
});

const url = process.env.DATABASE_URL ?? '';
const schema = `socialapi_${process.pid}`;
let db: pg.Pool; let base = ''; let server: ReturnType<typeof serve>;
const get = async (p: string) => { const r = await fetch(base + p); return { status: r.status, body: await r.json() as any }; };

describe.runIf(Boolean(url))('social routes on Postgres', () => {
  const wallet = k(); const fan = k(); const operator = k(); const agentKey = k(); const passport = k(); const admin = k(); const lowKey = k(); const low = k();
  const rootId = `${S(1)}:0`;

  beforeAll(async () => {
    const a = new pg.Pool({ connectionString: url, max: 1 });
    await a.query(`create schema if not exists ${schema}`); await a.end();
    db = new pg.Pool({ connectionString: url, max: 3, options: `-c search_path=${schema}` });
    for (const s of allDdl()) await db.query(s);
    process.env.SOCIAL_ADMINS = admin;
    process.env.MEMO_MIN_PROOF = '1';
    const c = await db.connect();
    await writeTransaction(c, 'sigP', 10, 1_700_000_000, [
      { ordinal: 0, program: 'agents', programId: '', name: 'PassportRegistered', via: 'cpi', data: { passport, operator, agentKey, name: 'scout', kinds: 1, badgeMint: k(), ts: '1' } },
      { ordinal: 1, program: 'agents', programId: '', name: 'ProofChanged', via: 'cpi', data: { passport, old: 0, new: 1, ts: '1' } },
      { ordinal: 2, program: 'agents', programId: '', name: 'PassportRegistered', via: 'cpi', data: { passport: low, operator, agentKey: lowKey, name: 'unlinked', kinds: 1, badgeMint: k(), ts: '1' } },
      { ordinal: 3, program: 'agents', programId: '', name: 'TreatyHeld', via: 'cpi', data: { bond: k(), passport, ts: '1' } },
    ], false, ['agents']);
    await writeTransaction(c, 'sigRaid', 20, 1_700_000_010, [
      { ordinal: 0, program: 'items', programId: '', name: 'RaidMarked', via: 'log', data: { mint: k(), rival: k(), trader: wallet, volume: '7000', points: 1, lootTicket: false } },
    ], false, ['items']);
    const lookup = await passportLookup(c);
    const write = async (v: TxView) => writeMemos(c, v, memosOf(v).map((m) => decodeMemo(m, 1232, (f) => lookup(f, v.slot))));
    const memo = (kind: hookwars.MemoKind, from: string, b: hookwars.MemoValue, o: { re?: string; thread?: string } = {}) => hookwars.encodeMemo(hookwars.socialMemo(kind, from, b, o));
    await write(tx([wallet], memo('status', wallet, hookwars.postBody({ text: 'root post' })), S(1), 100));
    await write(tx([agentKey], memo('status', passport, hookwars.postBody({ text: 'agent reply', model: 'model-x' }), { re: rootId, thread: rootId }), S(2), 110));
    await write(tx([lowKey], memo('status', low, hookwars.postBody({ text: 'below the proof floor' })), S(3), 115));
    await write(tx([fan], memo('follow', fan, hookwars.followBody(passport)), S(4), 120));
    await write(tx([fan], memo('react', fan, hookwars.reactBody(rootId, 'like')), S(5), 125));
    await write(tx([wallet], memo('status', wallet, hookwars.postBody({ text: 'spam' })), S(6), 130));
    await write(tx([admin], memo('hide', admin, hookwars.hideBody(`${S(6)}:0`, 'spam')), S(7), 140));
    await write(tx([fan], memo('hide', fan, hookwars.hideBody(rootId, 'not an admin')), S(8), 141));
    c.release();
    server = serve({ db, conn, rpcUrl: 'mock' }, 0);
    await new Promise((r) => server.once('listening', r));
    base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  });
  afterAll(async () => {
    server?.close();
    delete process.env.SOCIAL_ADMINS; delete process.env.MEMO_MIN_PROOF;
    await db.query(`drop schema if exists ${schema} cascade`);
    await db.end();
  });

  it('global feed: roots only, hidden and below-floor posts left out, toggle shows them', async () => {
    const f = (await get('/v1/social/feed')).body;
    expect(f.items.map((i: any) => i.text)).toEqual(['root post']);
    expect(f.items[0]).toMatchObject({ reactions: { like: 1, useful: 0, disagree: 0 }, replies: 1, postageLamports: null });
    expect(f.filter).toMatchObject({ minProof: 1, proofFilterApplied: true });
    const all = (await get('/v1/social/feed?all=1&roots=0')).body;
    expect(all.items.map((i: any) => i.text).sort()).toEqual(['agent reply', 'below the proof floor', 'root post']);
  });

  it('following feed from signed follows', async () => {
    const f = (await get(`/v1/social/feed?scope=following&viewer=${fan}&roots=0`)).body;
    expect(f.items.map((i: any) => [i.text, i.passportName, i.model])).toEqual([['agent reply', 'scout', 'model-x']]);
    expect((await get(`/v1/social/feed?scope=following&viewer=bad`)).status).toBe(400);
  });

  it('thread, timeline, follows, hides record', async () => {
    const t = (await get(`/v1/social/threads/${rootId}`)).body;
    expect(t.messages.map((m: any) => m.text)).toEqual(['root post', 'agent reply']);
    expect((await get(`/v1/social/threads/${S(9)}:0`)).status).toBe(404);
    const tl = (await get(`/v1/agents/${passport}/timeline`)).body;
    expect(tl.agent).toMatchObject({ name: 'scout', proof: 1 });
    expect(tl.items.map((i: any) => i.type)).toEqual(['memo', 'event', 'event', 'event']);
    expect(tl.items[0].model).toBe('model-x');
    expect((await get(`/v1/social/follows/${passport}`)).body).toMatchObject({ followers: 1, following: 0 });
    const h = (await get('/v1/social/hides')).body;
    expect(h.items.map((i: any) => [i.admin, i.applied])).toEqual([[fan, false], [admin, true]]);
  });

  it('wallet profile and leaderboards hold only chain facts', async () => {
    const u = (await get(`/v1/u/${wallet}`)).body;
    expect(u.levels).toMatchObject({ profile: null, counters: null });
    expect(u.raids).toEqual({ count: 1, volumeLamports: '7000' });
    expect(u.holdings).toEqual([]);
    expect(u.posts.items.map((i: any) => i.text)).toEqual(['root post']);
    const raids = (await get('/v1/social/leaderboards?board=raids')).body;
    expect(raids.items[0]).toMatchObject({ rank: 1, who: wallet, value: '7000' });
    const tr = (await get('/v1/social/leaderboards?board=treaties')).body;
    expect(tr.items[0]).toMatchObject({ who: passport, value: '1', agent: { name: 'scout' } });
    const lv = (await get('/v1/social/leaderboards?board=levels')).body;
    expect(lv.items).toEqual([]);
    expect(lv.note).toContain('No skill table');
    for (const b of ['royalties', 'authors', 'cranks']) expect((await get(`/v1/social/leaderboards?board=${b}`)).body.items).toEqual([]);
    const live = (await get('/v1/social/live?since=0')).body;
    expect(live.items.length).toBeGreaterThan(0);
    expect(findBannedWords(JSON.stringify([u, raids, tr, live]))).toEqual([]);
  });
});

