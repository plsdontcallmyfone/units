// Changed by Hookwars: new file, memo decoder vectors (pure) and the social tables on a real Postgres.
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import pg from 'pg';
import { Keypair } from '@solana/web3.js';
import { hookwars } from '@hookwars/sdk';
import { allDdl } from './schema.ts';
import { decodeMemo, memosOf, MEMO_PROGRAM, type TxView } from './memos.ts';
import { passportLookup, writeMemos, mayHoldMemo } from './social.ts';
import { writeTransaction } from './indexer.ts';

const k = () => Keypair.generate().publicKey.toBase58();
const SIG = '5'.repeat(88);

/** A transaction with `signers` signing and one memo instruction per `memos` entry. */
function tx(signers: string[], memos: { text: string; listed: string[] }[], extra: string[] = [], sig = SIG, slot = 100): TxView {
  const keys = [...signers, ...extra, MEMO_PROGRAM];
  return {
    signature: sig, slot, blockTime: 1_700_000_000, keys, numSigners: signers.length,
    instructions: memos.map((m) => ({ programIdIndex: keys.length - 1, accounts: m.listed.map((l) => keys.indexOf(l)), data: Buffer.from(m.text) })),
  };
}
const post = (from: string, text: string, extra: Partial<{ re: string; thread: string; model: string }> = {}) =>
  hookwars.encodeMemo(hookwars.socialMemo('status', from, hookwars.postBody({ text, model: extra.model }), { re: extra.re, thread: extra.thread }));

describe('memo decoder vectors', () => {
  const wallet = k(); const agentKey = k(); const operator = k(); const passport = k();
  const lookup = (a: string) => (a === passport ? { agentKey, operator } : null);

  it('a wallet post signed by the wallet is valid and starts its own thread', () => {
    const [m] = memosOf(tx([wallet], [{ text: post(wallet, 'gm'), listed: [wallet] }]));
    const d = decodeMemo(m!, 1232, lookup);
    expect(d).toMatchObject({ valid: true, authorKind: 'wallet', author: wallet, text: 'gm', thread: `${SIG}:0`, id: `${SIG}:0` });
    expect(d.reference).toBe(hookwars.messageReference(`${SIG}:0`).toString('hex'));
  });
  it('a passport post must be signed by its agent key; the operator signs directives', () => {
    const ok = decodeMemo(memosOf(tx([agentKey], [{ text: post(passport, 'settled', { model: 'model-x' }), listed: [agentKey] }]))[0]!, 1232, lookup);
    expect(ok).toMatchObject({ valid: true, authorKind: 'passport', passport, model: 'model-x' });
    const byOperator = decodeMemo(memosOf(tx([operator], [{ text: post(passport, 'x'), listed: [operator] }]))[0]!, 1232, lookup);
    expect(byOperator).toMatchObject({ valid: false, error: 'unsigned' });
    const directive = hookwars.encodeMemo({ kind: 'directive', from: passport, to: '*', thread: '', re: '', body: hookwars.body([['passport', passport], ['seq', 1n], ['rules_uri', 'https://r'], ['h', 'ab']]), expiresAt: 0n });
    expect(decodeMemo(memosOf(tx([operator], [{ text: directive, listed: [operator] }]))[0]!, 1232, lookup).valid).toBe(true);
    expect(decodeMemo(memosOf(tx([agentKey], [{ text: directive, listed: [agentKey] }]))[0]!, 1232, lookup).error).toBe('unsigned');
  });
  it('a sender who did not sign, or a listed non-signer, is unsigned', () => {
    const other = k();
    expect(decodeMemo(memosOf(tx([other], [{ text: post(wallet, 'spoof'), listed: [other] }]))[0]!, 1232, lookup).error).toBe('unsigned');
    expect(decodeMemo(memosOf(tx([other], [{ text: post(wallet, 'spoof'), listed: [wallet] }], [wallet]))[0]!, 1232, lookup).error).toBe('unsigned');
  });
  it('stores malformed and over-cap memos raw with their error', () => {
    const bad = decodeMemo(memosOf(tx([wallet], [{ text: 'hello world', listed: [wallet] }]))[0]!, 1232, lookup);
    expect(bad).toMatchObject({ valid: false, error: 'Syntax', raw: 'hello world', thread: null });
    expect(decodeMemo(memosOf(tx([wallet], [{ text: post(wallet, 'x'.repeat(50)), listed: [wallet] }]))[0]!, 40, lookup).error).toBe('TooLong');
    const badReact = hookwars.encodeMemo(hookwars.socialMemo('react', wallet, hookwars.body([['ref', 'a:0'], ['r', 'love']])));
    expect(decodeMemo(memosOf(tx([wallet], [{ text: badReact, listed: [wallet] }]))[0]!, 1232, lookup).error).toBe('Shape');
    const badOrder = hookwars.encodeMemo(hookwars.socialMemo('status', wallet, hookwars.body([['model', 'm'], ['text', 't']])));
    expect(decodeMemo(memosOf(tx([wallet], [{ text: badOrder, listed: [wallet] }]))[0]!, 1232, lookup).error).toBe('Shape');
  });
  it('reads only Memo program instructions and numbers them by instruction index', () => {
    const view = tx([wallet], [{ text: post(wallet, 'a'), listed: [wallet] }, { text: post(wallet, 'b'), listed: [wallet] }]);
    view.instructions.unshift({ programIdIndex: 0, accounts: [], data: Buffer.from('not a memo') });
    expect(memosOf(view).map((m) => m.id)).toEqual([`${SIG}:1`, `${SIG}:2`]);
    expect(mayHoldMemo({ memo: '[34] {"u":1,"k":"status"', signature: '', slot: 1, err: null, blockTime: null })).toBe(true);
    expect(mayHoldMemo({ memo: null, signature: '', slot: 1, err: null, blockTime: null })).toBe(false);
  });
});

const url = process.env.DATABASE_URL ?? '';
const schema = `socialtest_${process.pid}`;
let db: pg.Pool;

describe.runIf(Boolean(url))('social tables on Postgres', () => {
  beforeAll(async () => {
    const admin = new pg.Pool({ connectionString: url, max: 1 });
    await admin.query(`create schema if not exists ${schema}`); await admin.end();
    db = new pg.Pool({ connectionString: url, max: 2, options: `-c search_path=${schema}` });
    for (const s of allDdl()) await db.query(s);
  });
  afterAll(async () => {
    await db.query(`drop schema if exists ${schema} cascade`);
    await db.end();
  });

  it('threads, follows (last wins), reactions, hides and postage', async () => {
    const wallet = k(); const fan = k(); const agentKey = k(); const operator = k(); const passport = k(); const newKey = k(); const admin = k();
    const c = await db.connect();
    await writeTransaction(c, 'sigP', 10, 1_700_000_000, [
      { ordinal: 0, program: 'agents', programId: '', name: 'PassportRegistered', via: 'cpi', data: { passport, operator, agentKey, name: 'scout', kinds: 1, badgeMint: k(), ts: '1' } },
    ], false, ['agents']);
    await writeTransaction(c, 'sigR', 50, 1_700_000_100, [
      { ordinal: 0, program: 'agents', programId: '', name: 'AgentKeyRotated', via: 'cpi', data: { passport, oldKey: agentKey, newKey, badgeMint: k(), generation: 1, ts: '2' } },
    ], false, ['agents']);
    const lookup = await passportLookup(c);
    expect(lookup(passport, 20)?.agentKey).toBe(agentKey);
    expect(lookup(passport, 60)?.agentKey).toBe(newKey);

    const write = async (view: TxView) => writeMemos(c, view, memosOf(view).map((m) => decodeMemo(m, 1232, (f) => lookup(f, view.slot))));
    const S = (n: number) => String(n).repeat(88).slice(0, 88);
    await write(tx([wallet], [{ text: post(wallet, 'root'), listed: [wallet] }], [], S(1), 100));
    const rootId = `${S(1)}:0`;
    await write(tx([newKey], [{ text: post(passport, 'reply', { re: rootId, thread: rootId, model: 'm' }), listed: [newKey] }], [], S(2), 110));
    await write(tx([agentKey], [{ text: post(passport, 'old key after rotation'), listed: [agentKey] }], [], S(3), 120));
    await write(tx([fan], [{ text: hookwars.encodeMemo(hookwars.socialMemo('follow', fan, hookwars.followBody(wallet))), listed: [fan] }], [], S(4), 130));
    await write(tx([fan], [{ text: hookwars.encodeMemo(hookwars.socialMemo('unfollow', fan, hookwars.followBody(wallet))), listed: [fan] }], [], S(5), 140));
    await write(tx([fan], [{ text: hookwars.encodeMemo(hookwars.socialMemo('follow', fan, hookwars.followBody(wallet))), listed: [fan] }], [], S(6), 125));
    await write(tx([fan], [{ text: hookwars.encodeMemo(hookwars.socialMemo('react', fan, hookwars.reactBody(rootId, 'like'))), listed: [fan] }], [], S(7), 150));
    await write(tx([fan], [{ text: hookwars.encodeMemo(hookwars.socialMemo('react', fan, hookwars.reactBody(rootId, 'like'))), listed: [fan] }], [], S(8), 151));
    await write(tx([admin], [{ text: hookwars.encodeMemo(hookwars.socialMemo('hide', admin, hookwars.hideBody(rootId, 'spam'))), listed: [admin] }], [], S(9), 160));
    // Writing the same transaction again changes nothing.
    await write(tx([fan], [{ text: hookwars.encodeMemo(hookwars.socialMemo('react', fan, hookwars.reactBody(rootId, 'like'))), listed: [fan] }], [], S(7), 150));
    await writeTransaction(c, 'sigPost', 170, 1_700_000_200, [
      { ordinal: 0, program: 'agents', programId: '', name: 'MessagePosted', via: 'cpi', data: { passport, reference: hookwars.messageReference(`${S(2)}:0`), postage: 5000n, ts: '3' } },
    ], false, ['agents']);
    c.release();

    const msgs = (await db.query('select id, valid, error, thread, author_kind, model from social_messages order by slot')).rows;
    expect(msgs.map((m) => [m.valid, m.error])).toEqual([[true, null], [true, null], [false, 'unsigned'], [true, null], [true, null], [true, null], [true, null], [true, null], [true, null]]);
    expect(msgs[1]).toMatchObject({ thread: rootId, author_kind: 'passport', model: 'm' });
    expect((await db.query('select following from social_follows where follower = $1', [fan])).rows[0].following).toBe(false);
    expect((await db.query('select count(*)::int as n from social_reactions')).rows[0].n).toBe(1);
    expect((await db.query('select admin, ref, reason from social_hides')).rows[0]).toEqual({ admin, ref: rootId, reason: 'spam' });
    expect((await db.query('select passport, postage::text from social_postage where reference = $1', [hookwars.messageReference(`${S(2)}:0`).toString('hex')])).rows[0]).toEqual({ passport, postage: '5000' });
    expect((await db.query('select agent_key, proof, status from passport_current where passport = $1', [passport])).rows[0]).toEqual({ agent_key: newKey, proof: 0, status: 0 });
  });
});
